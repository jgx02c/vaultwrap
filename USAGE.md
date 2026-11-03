# vaultwrap usage

## Choosing the environment

Most commands act on one environment, picked in this order:

1. `-e org/project/env`: fully explicit.
2. `-e staging`: an environment name, combined with the nearest `.vaultwrap.toml`.
3. `VAULTWRAP_ENV=staging`: same as `-e`.
4. The `env` default in the nearest `.vaultwrap.toml`.

```toml
# .vaultwrap.toml (written by `vaultwrap link chally/api -e staging`)
org = "chally"
project = "api"
env = "staging"
# auto = false    # don't auto-load in hooked shells
```

## Commands

| Command | What it does |
|---|---|
| `init` | Create the encrypted vault. Its key goes in the Keychain; you choose a recovery (master) password |
| `unlock` / `lock` | Unlock with the master password (saves the key to the Keychain again) / lock until the next unlock |
| `unlock --keychain` | Unlock with the key in the Keychain, e.g. after `lock` |
| `forget` | Remove the vault key from the Keychain. The next unlock needs the master password |
| `passwd` | Change the master password (asks for the current one) |
| `passwd --reset` | Forgot it? Set a new one after confirming your Mac password (needs the key in the Keychain) |
| `reset-vault` | Last resort: move the vault to `backups/` and start over with an empty one |
| `status` | Daemon, vault, service and linked-project status |
| `tree` | Everything: orgs, providers, projects, environments, sources |
| `org add\|rm\|ls` | Manage orgs |
| `project add\|rm\|ls <org>/<project>` | Manage projects |
| `env add <org>/<project>/<env> [--protect] [--from <other env>]` | Add an environment. Creates the org and project if needed. `--from` copies sources and local values |
| `env protect <target> [--off]` | Mark an environment protected |
| `provider add <org>/<name> <kind> ...` | Add a provider (see below) |
| `provider test <org>/<name> [--prefix p]` | Fetch and list the variable names a provider returns |
| `provider ls\|rm` | List (credentials redacted) / remove |
| `source add <provider> [--prefix p] [--opt k=v]` | Attach a provider to the current environment |
| `source ls\|rm` | Show the layering / detach |
| `set KEY=VALUE ...` / `set KEY` | Store values locally. A bare `KEY` prompts (or reads stdin) |
| `unset KEY ...` | Remove local values |
| `get KEY` | Print one resolved value |
| `ls [--show] [--refresh]` | Resolved variables with their source; values masked unless `--show` |
| `import <file>` | Import a `.env` file as local values |
| `export [--format dotenv\|json\|shell]` | Print resolved variables |
| `link <org>/<project> [-e env]` | Write `.vaultwrap.toml` here |
| `run [-y] -- <cmd> ...` | Run a command with secrets injected. Exit codes and signals pass through |
| `shell [-y]` | Subshell with secrets injected |
| `activate` | `eval "$(vaultwrap activate)"`: load into this shell. `vaultwrap_deactivate` undoes it |
| `hook zsh\|bash\|fish` | Shell hook that auto-loads secrets on `cd` |
| `refresh` | Drop cached remote values |
| `service install\|uninstall\|status` | Background service (launchd on macOS, systemd --user on Linux) |
| `daemon` | Run the daemon in the foreground |

## Providers

A provider belongs to an org and holds the connection. A *source* attaches a provider to an
environment, with optional per-environment options. String settings in both can use
`{org}`, `{project}` and `{env}` templates. Per-source `--opt key=value` overrides provider config.
For example, one Key Vault provider can serve several environments, each with a different
`--opt vault_url=...`.

### `azure-keyvault`

| Setting | |
|---|---|
| `vault_url` | `https://<name>.vault.azure.net` |
| `auth` | `cli` (default: uses `az login`) or `client-secret` |
| `tenant_id`, `client_id`, `client_secret` | Service principal. `--client-secret` prompts for the secret |
| `prefix` (source) | Only secrets starting with this; stripped from the name. E.g. `'{env}-'` |
| `keep_prefix`, `uppercase` | Name mapping toggles (defaults: false, true) |

Key Vault names can't contain `_`, so `staging-database-url` maps to `DATABASE_URL`. To set a name
exactly, add a tag `env_var=DATABASE_URL` to the secret. Disabled secrets are skipped.

### `dotenv`

`--path ~/clients/{org}/{project}/.env.{env}`. Reads a file on every refresh. Good for a shared
drive or a synced folder.

### `command`

`--command '<shell command>' [--format dotenv|json]`. Runs with `sh -c`, with `VAULTWRAP_ORG`,
`VAULTWRAP_PROJECT` and `VAULTWRAP_ENV` set. It must print dotenv lines or a JSON object.
The daemon uses the `PATH` captured at `service install`, so re-run that if you install new CLIs elsewhere.

### Caching

Each provider's values are cached in daemon memory for `cache_ttl` seconds (default 300, `--ttl`).
The last successful fetch is also saved encrypted in the vault. If a provider is unreachable, that
copy is used with a warning. `vaultwrap refresh` or `ls --refresh` refetches.

## Shell integration

**Per command:** `vaultwrap run -- <cmd>`. Secrets exist only in that process tree.

**Per shell (like a virtualenv):**

```bash
eval "$(vaultwrap activate)"      # prompt shows (org/project/env)
vaultwrap_deactivate              # restores previous values
```

**Automatic (like direnv):** add to your shell rc:

```bash
eval "$(vaultwrap hook zsh)"      # or bash
vaultwrap hook fish | source      # fish
```

Entering a linked directory loads its environment. Leaving restores what was there before.
Switch with `export VAULTWRAP_ENV=dev`. After changing secrets, run `vaultwrap_reload`. Protected
environments are never auto-loaded. If the vault is locked, the hook says so once and loads after
`vaultwrap unlock`.

## Files

| Path | |
|---|---|
| `~/Library/Application Support/vaultwrap/` (macOS), `~/.local/share/vaultwrap/` (Linux) | Vault home. Override with `VAULTWRAP_HOME` |
| `vault.db` | The encrypted vault (SQLCipher) |
| `vault.recovery` | The vault key, encrypted with your master password |
| `backups/` | Vaults moved aside by `reset-vault` |
| `vaultd.sock` | Daemon socket |
| `vaultd.log` | Daemon log (never contains values) |
| `~/Library/LaunchAgents/dev.vaultwrap.daemon.plist` | macOS login service |
| `~/.config/systemd/user/vaultwrap.service` | Linux login service |

Back up `vault.db` and `vault.recovery` together. On another Mac they open with the master password.

## Keys and passwords

- The vault is encrypted with a random key. It's kept in the macOS Keychain (service `dev.vaultwrap`,
  account `vault-key:<vault id>`), so the vault unlocks when you log in. No password prompt day to day.
- The same key is also stored in `vault.recovery`, encrypted with your master password (Argon2id +
  XChaCha20-Poly1305). That's how you get in if the Keychain entry is gone (new Mac, deleted entry,
  `vaultwrap forget`). Unlocking with the master password puts the key back in the Keychain.
- Forgot the master password? While the key is in the Keychain, `vaultwrap passwd --reset` (or
  "Reset with Mac password" in the app) sets a new one after you confirm your Mac password.
- Lost both? The data can't be decrypted. `vaultwrap reset-vault` moves it to `backups/` and starts fresh.
- `vaultwrap lock` keeps the vault locked, even across daemon restarts, until you unlock it again.
