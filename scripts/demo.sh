#!/usr/bin/env bash
# `make demo`: run the desktop app against a throwaway vault filled with mock
# data. Never touches your real vault, Keychain or login items.
#
#   scripts/demo.sh          seed (first run only) and open the app
#   scripts/demo.sh seed     (re)create the demo vault and exit
#   scripts/demo.sh env      print the env vars to use the demo vault from a shell
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEMO="$ROOT/.dev/demo"
BIN="${BIN:-$ROOT/target/debug}"
PORT="${DEMO_PORT:-18931}"
PASSWORD="demo-password"

export VAULTWRAP_HOME="$DEMO/vault"
export VAULTWRAP_KEYCHAIN_FILE="$DEMO/keychain.json"
export VAULTWRAP_DEMO=1
export VAULTWRAP_SKIP_SETUP=1
export PATH="$ROOT/scripts/demo/bin:$PATH" # fake `az` for the Azure mock

vw() { "$BIN/vaultwrap" "$@"; }

daemon_pid() { vw status 2>/dev/null | sed -n 's/^daemon *running (pid \([0-9]*\).*/\1/p'; }

stop_daemon() {
  local pid
  pid="$(daemon_pid || true)"
  if [ -n "$pid" ]; then kill "$pid" 2>/dev/null || true; fi
}

start_mock() {
  if ! curl -s -o /dev/null "http://127.0.0.1:$PORT/secrets" 2>/dev/null; then
    python3 "$ROOT/scripts/demo/azure_mock.py" "$PORT" &
    MOCK_PID=$!
    for _ in $(seq 1 50); do
      curl -s -o /dev/null "http://127.0.0.1:$PORT/secrets" && break
      sleep 0.1
    done
  fi
}

write_files() {
  mkdir -p "$DEMO/files"
  cat >"$DEMO/files/storefront.dev.env" <<'ENV'
# Shared team .env for Acme's storefront (dev)
DATABASE_URL=postgres://localhost:5432/storefront_dev
SHOPIFY_API_KEY=shpka_demo_dev_0000
SHOPIFY_API_SECRET=shpss_demo_dev_0000
ALGOLIA_APP_ID=DEMO123
ALGOLIA_ADMIN_KEY=algolia-demo-admin-dev
ENV
  cat >"$DEMO/files/storefront.staging.env" <<'ENV'
DATABASE_URL=postgres://storefront:demo@acme-staging.internal:5432/storefront
SHOPIFY_API_KEY=shpka_demo_staging_0000
SHOPIFY_API_SECRET=shpss_demo_staging_0000
ALGOLIA_APP_ID=DEMO123
ALGOLIA_ADMIN_KEY=algolia-demo-admin-staging
ENV
  # Pretend password-manager CLI output (JSON) for the command provider.
  cat >"$DEMO/files/vault-items.dev.json" <<'JSON'
{"OPENAI_API_KEY": "demo-openai-dev", "MAPBOX_TOKEN": "pk.demo-dev"}
JSON
  cat >"$DEMO/files/vault-items.staging.json" <<'JSON'
{"OPENAI_API_KEY": "demo-openai-staging", "MAPBOX_TOKEN": "pk.demo-staging"}
JSON
}

seed() {
  stop_daemon
  rm -rf "$DEMO"
  mkdir -p "$DEMO"
  write_files
  echo "Seeding demo vault in $DEMO"
  printf '%s\n' "$PASSWORD" | vw init --password-stdin >/dev/null

  # chally: Azure Key Vault (served by the local mock)
  vw env add chally/api/dev >/dev/null
  vw env add chally/api/staging >/dev/null
  vw env add chally/api/prod --protect >/dev/null
  vw env add chally/web/staging >/dev/null
  vw env add chally/web/prod --protect >/dev/null
  vw provider add chally/kv azure-keyvault \
    --vault-url https://chally-kv.vault.azure.net \
    --set "vault_url=http://127.0.0.1:$PORT" >/dev/null
  vw source add kv -e chally/api/staging --prefix '{env}-' >/dev/null
  vw source add kv -e chally/api/prod --prefix '{env}-' >/dev/null
  vw source add kv -e chally/web/staging --prefix 'web-{env}-' >/dev/null
  vw source add kv -e chally/web/prod --prefix 'web-{env}-' >/dev/null
  vw set -e chally/api/dev \
    DATABASE_URL=postgres://localhost:5432/chally_dev \
    REDIS_URL=redis://localhost:6379 \
    STRIPE_SECRET_KEY=demo-stripe-local-key \
    LOG_LEVEL=debug >/dev/null
  vw set -e chally/api/staging LOG_LEVEL=debug SENTRY_ENVIRONMENT=staging >/dev/null
  vw set -e chally/api/prod LOG_LEVEL=info SENTRY_ENVIRONMENT=production >/dev/null
  vw set -e chally/web/staging NEXT_PUBLIC_POSTHOG_KEY=phc_demo_staging >/dev/null

  # acme: a shared .env file plus a password-manager CLI
  vw env add acme/storefront/dev >/dev/null
  vw env add acme/storefront/staging >/dev/null
  vw provider add acme/shared-env dotenv \
    --path "$DEMO/files/{project}.{env}.env" >/dev/null
  vw provider add acme/team-vault command --format json \
    --command "cat '$DEMO/files/vault-items.{env}.json'" >/dev/null
  for e in dev staging; do
    vw source add shared-env -e "acme/storefront/$e" >/dev/null
    vw source add team-vault -e "acme/storefront/$e" >/dev/null
  done
  vw set -e acme/storefront/dev PORT=3000 SHOPIFY_API_KEY=shpka_my_own_dev_key >/dev/null

  # northwind: no providers, just local values
  vw env add northwind/mobile-app/dev >/dev/null
  vw set -e northwind/mobile-app/dev \
    API_BASE_URL=http://localhost:8080 \
    FIREBASE_API_KEY=demo-firebase-key \
    EXPO_PUBLIC_ENV=development >/dev/null

  echo "Demo vault ready. Master password: $PASSWORD"
}

case "${1:-app}" in
  seed)
    trap 'stop_daemon; [ -n "${MOCK_PID:-}" ] && kill "$MOCK_PID" 2>/dev/null; true' EXIT
    start_mock
    seed
    vw ls -e chally/api/staging
    ;;
  env)
    echo "export VAULTWRAP_HOME='$VAULTWRAP_HOME' VAULTWRAP_KEYCHAIN_FILE='$VAULTWRAP_KEYCHAIN_FILE' PATH='$ROOT/scripts/demo/bin':\"\$PATH\""
    ;;
  app)
    trap 'stop_daemon; [ -n "${MOCK_PID:-}" ] && kill "$MOCK_PID" 2>/dev/null; true' EXIT
    start_mock
    [ -f "$VAULTWRAP_HOME/vault.db" ] || seed
    echo "Opening Vaultwrap (demo). Master password: $PASSWORD. Quit from the menu bar to stop."
    "$BIN/vaultwrap-desktop"
    ;;
  *)
    echo "usage: $0 [app|seed|env]" >&2
    exit 2
    ;;
esac
