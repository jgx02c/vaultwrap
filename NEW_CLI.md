# VaultWrap CLI Tool: Planned Commands & Features

This document outlines the planned commands and features for the improved VaultWrap CLI tool.

## Connection Management

### 1. Connect to a Server
```
vaultwrap connect <host:port> [--username <user>] [--password <pass>] [--save <name>] [--default]
```
- Connects to a VaultWrap server (local or remote).
- Optionally saves the connection under a name for future use.
- Optionally sets as default.

### 2. List Saved Connections
```
vaultwrap connections list
```
- Lists all saved connections.

### 3. Use a Saved Connection
```
vaultwrap connections use <name>
```
- Sets a saved connection as the current/default.

### 4. Remove a Saved Connection
```
vaultwrap connections remove <name>
```
- Removes a saved connection.

### 5. Show Current Connection
```
vaultwrap connections show
```
- Shows the current/active connection.

**How it works:**
- Connections are stored in a config file (e.g., `~/.vaultwrap/config.json`).
- All CLI commands use the current connection unless overridden with `--host`/`--port`/etc.
- You can easily switch between companies/servers.

---

## Core Environment & Key Commands

### 1. List Environments
```
vaultwrap env list
```
- Lists all available environments.

### 2. List Keys in an Environment
```
vaultwrap env keys <env>
```
- Lists all keys/variables in the specified environment.

### 3. Set Environment (was 'activate')
```
vaultwrap set <env>
```
- Sets (injects) the specified environment into the shell/session.

### 4. Drop Environment (was 'deactivate')
```
vaultwrap drop
```
- Drops (removes) the current environment from the shell/session.

### 5. Add Environment
```
vaultwrap env add <env>
```
- Creates a new environment.

### 6. Delete Environment
```
vaultwrap env delete <env>
```
- Deletes the specified environment.

### 7. Add Key to Environment
```
vaultwrap key add <env> <key> <value>
```
- Adds a new key/value pair to the specified environment.

### 8. Delete Key from Environment
```
vaultwrap key delete <env> <key>
```
- Deletes the specified key from the environment.

### 9. Update Key Name or Value
```
vaultwrap key update <env> <old_key> <new_key> <new_value>
```
- Updates the name and/or value of a key in the environment.

---

## Example Usage
```sh
# Connect to a remote server and save as 'acme', set as default
vaultwrap connect acme.example.com:4000 --username alice --password secret --save acme --default

# List all saved connections
vaultwrap connections list

# Switch to a different company/server
vaultwrap connections use acme

# List environments on the current server
vaultwrap env list

# Set and drop environments
vaultwrap set dev
vaultwrap drop
```

---

## Notes
- All commands should provide clear success/error output.
- Consider supporting both interactive and non-interactive (scriptable) usage.
- Future: Support for importing/exporting environments, or bulk operations.

---

**Next Steps:**
- Update the CLI tool to implement these commands.
- Ensure all commands interact with the backend/server as needed. 