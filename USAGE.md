# VaultWrap Usage Guide

VaultWrap allows you to securely manage and inject environment variables into your applications without storing them in local .env files.

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

## Usage Options

VaultWrap provides three different ways to use your environment variables:

### Option 1: Direct Command Execution

Run any command with environment variables injected at runtime:

```bash
./use-vault-env.sh python3 your_script.py
```

This will connect to the vault server, fetch the environment variables, and inject them only into that specific process.

### Option 2: Shell Activation (like venv)

Activate the vault environment in your current shell session:

```bash
source activate-vault.sh
```

Your prompt will change to show `(vault)` and all environment variables will be loaded into your shell. You can then run commands normally:

```bash
python3 your_script.py
node server.js
```

When finished, deactivate the environment:

```bash
deactivate
```

### Option 3: Direct CLI Usage

You can also use the vaultwrap CLI directly:

```bash
cd vaultwrap
cargo run -- run python3 your_script.py
```

## How to Use in Different Projects

1. **Local Development**: Simply copy `use-vault-env.sh` or `activate-vault.sh` to your project directory
2. **CI/CD Pipeline**: Integrate the vaultwrap CLI into your build process
3. **Team Development**: Run vaultd on a central server and configure each developer with the client scripts

## Security Benefits

- Environment variables never exist in local .env files
- Variables are only available in memory, never on disk
- Access control is managed centrally
- Automatic cleanup when processes terminate
- Memory protection prevents sensitive data exposure

## Troubleshooting

- **Connection error**: Make sure vaultd is running
- **Command not authorized**: Check the allowed commands in vaultd's configuration
- **Shell prompt issues**: Run `unalias deactivate` before activating again

For more details, see the main project README.md. 