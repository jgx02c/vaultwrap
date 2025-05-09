#!/bin/bash
# dynamic-activate.sh - Dynamically load vault environment variables
# Usage: source dynamic-activate.sh

# Check if sourced
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    echo "This script must be sourced, not executed."
    echo "Please use: source ${0}"
    exit 1
fi

# Path to vaultwrap binary
VAULTWRAP_BIN="/Users/joshuagoodman/Documents/GitHub/vaultwrap/vaultwrap/target/debug/vaultwrap"

# Check dependencies
if [ ! -f "$VAULTWRAP_BIN" ]; then
    echo "Error: vaultwrap binary not found at $VAULTWRAP_BIN"
    return 1
fi

if ! nc -z localhost 4000 2>/dev/null; then
    echo "Error: vaultd server is not running"
    return 1
fi

# Save original prompt
ORIG_PS1="$PS1"

# Modify the prompt
PS1="(vault) $PS1"

# Create function to wrap execution in vault environment
vault_exec() {
    "$VAULTWRAP_BIN" run "$@"
}

# Register vault command hook for common commands
for cmd in python python3 node npm npx ruby php go java mvn gradle cargo ./manage.py; do
    alias $cmd="vault_exec $cmd"
done

# Simple clean deactivation
vault_deactivate() {
    echo "Deactivating vault environment..."
    
    # Restore the prompt
    PS1="$ORIG_PS1"
    
    # Remove command aliases
    for cmd in python python3 node npm npx ruby php go java mvn gradle cargo ./manage.py; do
        unalias $cmd 2>/dev/null || true
    done
    
    # Unset functions
    unset -f vault_exec
    unset -f vault_deactivate
    unset ORIG_PS1
    
    # Remove aliases
    unalias deactivate 2>/dev/null || true
    
    echo "Vault environment deactivated"
}

# Create alias
alias deactivate=vault_deactivate

echo "Vault environment activated!"
echo "Common commands (python, node, npm, etc.) will automatically use vault variables"
echo "For other commands, use: vault_exec <command>"
echo "Type 'deactivate' to exit the vault environment" 