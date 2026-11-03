"""A tiny local stand-in for the Azure Key Vault REST API, used by `make demo`.

Serves GET /secrets (paged) and GET /secrets/<name> for a fixed set of fake
secrets, so the azure-keyvault provider works without an Azure account.
"""
import json
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 18931
BASE = f"http://127.0.0.1:{PORT}"

# name -> (value, enabled, tags)
SECRETS = {
    "staging-database-url": ("postgres://api:s3cr3t@chally-staging.postgres.database.azure.com:5432/api", True, {}),
    "staging-redis-url": ("rediss://:demo@chally-staging.redis.cache.windows.net:6380", True, {}),
    "staging-stripe-secret-key": ("demo-stripe-test-key", True, {}),
    "staging-sendgrid-api-key": ("demo-sendgrid-staging", True, {}),
    "staging-jwt-signing-key": ("staging-7f3c2a9e4b1d", True, {}),
    "staging-feature-flags": ("new-checkout,search-v2", True, {"env_var": "FEATURE_FLAGS"}),
    "staging-old-api-key": ("disabled-value", False, {}),
    "prod-database-url": ("postgres://api:pr0d@chally-prod.postgres.database.azure.com:5432/api", True, {}),
    "prod-redis-url": ("rediss://:demo@chally-prod.redis.cache.windows.net:6380", True, {}),
    "prod-stripe-secret-key": ("demo-stripe-live-key", True, {}),
    "prod-sendgrid-api-key": ("demo-sendgrid-prod", True, {}),
    "prod-jwt-signing-key": ("prod-a81e55c09d2f", True, {}),
    "web-staging-next-public-api-url": ("https://api.staging.chally.example", True, {}),
    "web-staging-sentry-dsn": ("https://demo@o0.ingest.sentry.io/1", True, {}),
    "web-prod-next-public-api-url": ("https://api.chally.example", True, {}),
    "web-prod-sentry-dsn": ("https://demo@o0.ingest.sentry.io/2", True, {}),
}
NAMES = sorted(SECRETS)
PAGE = 8


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_GET(self):
        if self.headers.get("Authorization") != "Bearer demo-token":
            return self.reply(401, {"error": {"code": "Unauthorized", "message": "bad token"}})
        path, _, query = self.path.partition("?")
        if path == "/secrets":
            start = 0
            for part in query.split("&"):
                if part.startswith("skip="):
                    start = int(part[5:])
            chunk = NAMES[start:start + PAGE]
            nxt = start + PAGE
            return self.reply(200, {
                "value": [
                    {"id": f"{BASE}/secrets/{n}", "attributes": {"enabled": SECRETS[n][1]}, "tags": SECRETS[n][2]}
                    for n in chunk
                ],
                "nextLink": f"{BASE}/secrets?api-version=7.4&skip={nxt}" if nxt < len(NAMES) else None,
            })
        name = path.rstrip("/").rsplit("/", 1)[-1]
        if path.startswith("/secrets/") and name in SECRETS:
            return self.reply(200, {"value": SECRETS[name][0], "id": f"{BASE}/secrets/{name}"})
        self.reply(404, {"error": {"code": "SecretNotFound", "message": f"{name} not found"}})

    def reply(self, code, body):
        data = json.dumps(body).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)


ThreadingHTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
