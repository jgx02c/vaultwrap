# VaultWrap Usage Guide

VaultWrap allows you to securely manage and inject environment variables into your applications without storing them in local .env files. It works like Python's virtual environments (venv) but for environment variables.

## Setup

### 1. Define Environment Variables

Create or edit the `vaultd/secrets.toml` file with your environment variables:

```toml
# Database configuration
DATABASE_URL = "postgres://user:password@localhost:5432/mydb"

# API credentials
API_KEY = "your_api_key"

# Application settings
ENVIRONMENT = "development"
```

### 2. Start the Vault Server

The vault server (vaultd) securely stores and serves your environment variables:

```bash
cd vaultd
cargo run
```

Keep this server running in a terminal window.

### 3. Connect to the Server

First, connect your CLI to the vault server:

```bash
cd vaultwrap
cargo run -- connect localhost:4000 --save local --default
```

## Core Usage (Python venv-like)

VaultWrap works exactly like Python virtual environments:

### Activate an Environment

Set environment variables in your current shell (like `source venv/bin/activate`):

```bash
eval "$(cargo run -- set dev)"
```

Your shell prompt will change to show `(dev)` and all environment variables from the `dev` environment will be available:

```bash
(dev) joshuagoodman$ echo $DATABASE_URL
postgres://user:password@localhost:5432/mydb
```

### Deactivate the Environment

Remove environment variables from your shell (like `deactivate`):

```bash
eval "$(cargo run -- drop)"
```

Your prompt returns to normal and all vault environment variables are removed.

## Runtime Injection Mode (Optional)

Runtime injection allows you to automatically inject environment variables when specific commands run, without setting them in your shell.

### Configure Commands for Injection

Add commands that should automatically get environment variables:

```bash
cargo run -- add cargo      # Inject vars when running cargo
cargo run -- add python3    # Inject vars when running python3
cargo run -- add node       # Inject vars when running node
```

### Enable Runtime Injection

```bash
cargo run -- enable
```

Now when you activate an environment with runtime injection enabled, variables are NOT set in your shell. Instead, they're automatically injected only when the configured commands run:

```bash
eval "$(cargo run -- set dev)"  # Only sets prompt to (dev), no variables in shell
cargo run                       # Automatically gets dev environment variables
python3 app.py                  # Automatically gets dev environment variables
echo $DATABASE_URL              # Empty - variables not in shell
```

### Manage Runtime Commands

```bash
# Add a command
cargo run -- add python

# Remove a command  
cargo run -- remove cargo

# Check status
cargo run -- status

# Disable runtime injection
cargo run -- disable
```

### Two Modes Comparison

**Normal Mode (runtime injection disabled):**
- `vaultwrap set dev` outputs all environment variables
- Variables are set in your shell session
- All commands in that shell have access to the variables

**Runtime Injection Mode (runtime injection enabled):**
- `vaultwrap set dev` only sets the prompt to `(dev)`
- Variables are NOT set in your shell
- Only configured commands (cargo, python3, etc.) get the variables automatically
- More secure - variables only exist during command execution

## Environment Management

### List Environments

```bash
cargo run -- env list
```

### Create New Environment

```bash
cargo run -- env add production
```

### Add Variables to Environment

```bash
cargo run -- key add production DATABASE_URL "postgres://prod:secret@prod-db:5432/app"
cargo run -- key add production API_KEY "prod-api-key-12345"
```

### Delete Environment

```bash
cargo run -- env delete staging
```

## Connection Management

### Save Multiple Connections

```bash
# Connect to development server
cargo run -- connect dev.company.com:4000 --save dev

# Connect to production server  
cargo run -- connect prod.company.com:4000 --save prod --default
```

### Switch Between Connections

```bash
cargo run -- connections use dev
cargo run -- connections use prod
```

### List Saved Connections

```bash
cargo run -- connections list
```

## Installation (Optional)

To install globally and use `vaultwrap` instead of `cargo run --`:

```bash
cd vaultwrap
cargo install --path .
```

Then you can use:

```bash
eval "$(vaultwrap set dev)"
eval "$(vaultwrap drop)"
vaultwrap add cargo
vaultwrap enable
```

## Security Benefits

- Environment variables never exist in local .env files
- Variables are only available in memory, never on disk
- Access control is managed centrally
- Automatic cleanup when processes terminate
- Memory protection prevents sensitive data exposure
- Works seamlessly with existing development workflows
- Runtime injection mode provides even more security by limiting variable exposure

## Troubleshooting

- **Connection error**: Make sure vaultd is running
- **No default connection**: Run `vaultwrap connect` first
- **Shell prompt issues**: Make sure to use `eval "$(vaultwrap set <env>)"` and `eval "$(vaultwrap drop)"`
- **Variables not set**: Check that the environment exists with `vaultwrap env list`
- **Runtime injection not working**: Check `vaultwrap status` and ensure commands are added with `vaultwrap add <command>`

For more details, see the main project README.md. 