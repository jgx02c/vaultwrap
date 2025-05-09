#!/bin/bash
# simple-activate.sh - Load vault environment variables
# Usage: source simple-activate.sh

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

# Set up our environment
echo "Activating vault environment..."

# Modify the prompt
PS1="(vault) $PS1"

# Store fixed environment variables
export DATABASE_URL="$(eval "$VAULTWRAP_BIN" run echo '$DATABASE_URL' | tail -n 1)"
export API_KEY="$(eval "$VAULTWRAP_BIN" run echo '$API_KEY' | tail -n 1)"
export SECRET_KEY="$(eval "$VAULTWRAP_BIN" run echo '$SECRET_KEY' | tail -n 1)"
export REDIS_URL="$(eval "$VAULTWRAP_BIN" run echo '$REDIS_URL' | tail -n 1)"
export ENVIRONMENT="$(eval "$VAULTWRAP_BIN" run echo '$ENVIRONMENT' | tail -n 1)"
export DEBUG="$(eval "$VAULTWRAP_BIN" run echo '$DEBUG' | tail -n 1)"
export MY_CUSTOM_SECRET="$(eval "$VAULTWRAP_BIN" run echo '$MY_CUSTOM_SECRET' | tail -n 1)"
export PROJECT_ID="$(eval "$VAULTWRAP_BIN" run echo '$PROJECT_ID' | tail -n 1)"

# Simple clean deactivation
vault_deactivate() {
    echo "Deactivating vault environment..."
    
    # Restore the prompt
    PS1="$ORIG_PS1"
    
    # Unset variables
    unset DATABASE_URL
    unset API_KEY
    unset SECRET_KEY
    unset REDIS_URL
    unset ENVIRONMENT
    unset DEBUG
    unset MY_CUSTOM_SECRET
    unset PROJECT_ID
    unset ORIG_PS1
    
    # Clean up
    unset -f vault_deactivate
    unalias deactivate 2>/dev/null || true
    
    echo "Vault environment deactivated"
}

# Create the alias
alias deactivate=vault_deactivate

# Show results
echo "Vault environment activated with these variables:"
echo "- DATABASE_URL"
echo "- API_KEY"
echo "- SECRET_KEY"
echo "- REDIS_URL"
echo "- ENVIRONMENT"
echo "- DEBUG"
echo "- MY_CUSTOM_SECRET"
echo "- PROJECT_ID"

echo "Type 'deactivate' to exit the vault environment" 