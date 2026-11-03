# vaultwrap

Your `.env` files, out of your repos and into one encrypted vault. Each run gets its secrets injected.

```bash
vaultwrap run -- npm run dev        # secrets injected into this process only
cd ~/code/chally-api                # or: auto-loaded into your shell, like direnv
```

Built for developers who juggle several clients or companies. Each **org** can pull from its own
secret store (Azure Key Vault, a shared `.env`, 1Password, Doppler, AWS... anything with a CLI).
Each **project** has **environments** (staging, prod...) that layer those sources together.

```
org: chally ─── providers: kv-staging (Azure Key Vault), kv-prod (Azure Key Vault)
   └─ project: api
        ├─ staging  ← kv-staging [prefix api-] + local overrides
        └─ prod     ← kv-prod    [prefix api-]               [protected]
org: acme   ─── providers: op (1Password CLI)
   └─ project: web
        └─ dev      ← op + local
```

## How it works

- **One binary**, `vaultwrap`, is the CLI, the background daemon, and the service installer.
- **Encrypted vault**: a SQLCipher (encrypted SQLite) database. Names, values, provider credentials and
  cached remote values are all encrypted at rest.
- **Daemon** (`vaultwrap daemon`) runs in the background, starts at login (launchd/systemd), and is the only
  process that opens the vault. Clients talk to it over a private Unix socket (mode `0600`, same-user only).
  It starts automatically on first use if the service isn't installed.
- **Unlocks with your Mac login.** The vault key lives in the macOS Keychain (`dev.vaultwrap`), so there's
  no daily password. Your master password is the recovery path: it opens the vault on a new Mac or if
  the Keychain entry is removed, and you can reset it with your Mac password. See
  [Keys and passwords](USAGE.md#keys-and-passwords).
- **Remote values are cached** in memory (5 min default), and encrypted in the vault for **offline** fallback.
- **Protected environments** (prod) are never auto-loaded into shells and ask before `run`.

## Install (macOS)

```bash
make install        # builds dist/Vaultwrap-<version>.pkg and runs the installer
```

Or build `make dist` and double-click `dist/Vaultwrap-<version>.pkg` (or drag the app from the `.dmg`).
The installer sets up everything:

- **Vaultwrap.app** in Applications. It lives in the menu bar (lock icon), and closing the window keeps it running.
- **`vaultwrap`** command at `/usr/local/bin/vaultwrap`. This is the CLI bundled inside the app.
- **Background service.** The vault daemon starts at login, so terminals and hooks work without the app open.
- The app opens once installed so you can create your vault.

Optional: auto-load secrets when you `cd` into a linked project. Add to `~/.zshrc`:

```bash
eval "$(vaultwrap hook zsh)"
```

Uninstall with `make uninstall` (keeps your vault) or `./scripts/uninstall.sh --purge`.

CLI only (Linux, or no app): `make install-cli && vaultwrap service install`.

## Development

```bash
make help        # everything below
make demo        # the app with mock orgs, Azure Key Vault, .env and CLI sources
make dev         # run the desktop app against a throwaway vault in .dev/
make cli ARGS="ls"   # run the CLI against the same dev vault
make daemon      # foreground daemon with logs
make check       # fmt + clippy + tests
make dist        # .app, .pkg and .dmg in dist/
```

Dev targets never touch your real vault, Keychain or login items. `make demo` seeds `.dev/demo` with
three orgs (Azure Key Vault served by a local mock, a shared `.env` file, a password-manager-style CLI)
and opens the app. The demo master password is `demo-password`. `make demo-seed` starts it over.

## Quick start

```bash
vaultwrap init                                # choose a recovery password; the key goes in Keychain
vaultwrap env add chally/api/staging          # creates the org + project too
vaultwrap env add chally/api/prod --protect

cd ~/code/chally-api
vaultwrap link chally/api -e staging          # writes .vaultwrap.toml (commit it — no secrets)
vaultwrap import .env && rm .env              # move an existing .env into the vault
vaultwrap set STRIPE_KEY                      # prompts, so it never hits shell history

vaultwrap run -- npm run dev                  # staging
vaultwrap -e prod run -- ./migrate.sh         # prod (asks to confirm)
vaultwrap ls                                  # what's set, and where each value comes from
```

## Connect a central store

```bash
# Azure Key Vault, using your `az login` session
vaultwrap provider add chally/kv azure --vault-url https://chally-kv.vault.azure.net

# ...or a service principal (the secret is stored encrypted in the vault)
vaultwrap provider add chally/kv azure --vault-url https://chally-kv.vault.azure.net \
  --tenant <tenant-id> --client-id <app-id> --client-secret

vaultwrap provider test chally/kv
vaultwrap source add kv --prefix '{env}-'     # staging-database-url → DATABASE_URL
```

Anything with a CLI works through the `command` provider:

```bash
vaultwrap provider add acme/op command \
  --command 'op inject -i ~/clients/acme/{project}.{env}.env.tpl'
vaultwrap provider add acme/doppler command \
  --command 'doppler secrets download --no-file --format env --project {project} --config {env}'
vaultwrap provider add acme/aws command --format json \
  --command 'aws secretsmanager get-secret-value --secret-id {project}/{env} --query SecretString --output text'
```

See [USAGE.md](USAGE.md) for all commands, provider options and shell integration details.

## Precedence

Sources apply in the order you attach them; later ones win. Values stored locally with
`vaultwrap set` / `import` always win last, so you can override one remote value on your machine.

## Security notes

- The vault is encrypted with SQLCipher 4 (AES-256) using a random 256-bit key. That key is stored in the
  Keychain and, wrapped with your master password (Argon2id, 64 MiB), in `vault.recovery`.
- Decrypted data lives only in the daemon's memory while unlocked. `vaultwrap lock` drops it.
- The socket accepts connections only from your own user (peer credential check).
- Secrets injected into a process are visible to that process and its children, as with any env var.
  `vaultwrap run` keeps them scoped to one command. The shell hook and `activate` put them in your shell.
- `.vaultwrap.toml` holds only org/project/env names and is safe to commit.

## Layout

```
crates/vaultwrap-core      store (SQLCipher), providers, daemon, socket protocol, client
crates/vaultwrap           CLI: commands, shell integration, service install
crates/vaultwrap-desktop   Tauri menu-bar app (ui/ is plain HTML/JS, no build step)
scripts/                   pkg installer, uninstaller, icon generator
```

The app and the CLI are both clients of the daemon (`crates/vaultwrap-core/src/protocol.rs`).

### Signing

Builds are ad-hoc signed, which is fine on the Mac that built them. To share the installer with
other machines, sign and notarize it with an Apple Developer ID:

```bash
make dist SIGN_IDENTITY="Developer ID Application: …" INSTALLER_IDENTITY="Developer ID Installer: …"
```

## License

MIT
