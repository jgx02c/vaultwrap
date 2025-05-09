#!/bin/bash
# activate-vault.sh - Load vault environment variables into your shell
# Usage: source activate-vault.sh

# Check if being sourced
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    echo "This script must be sourced, not executed."
    echo "Please use: source ${0}"
    exit 1
fi

# First, undefine deactivate function if it exists
unset -f deactivate 2>/dev/null
unset -f vault_deactivate 2>/dev/null

# Path to the vaultwrap binary
VAULTWRAP_PATH="/Users/joshuagoodman/Documents/GitHub/vaultwrap/vaultwrap/target/debug/vaultwrap"

# Check if vaultwrap binary exists
if [ ! -f "$VAULTWRAP_PATH" ]; then
    echo "Error: vaultwrap binary not found at $VAULTWRAP_PATH"
    echo "Please build the vaultwrap project first with 'cd vaultwrap && cargo build'"
    return 1
fi

# Check if vaultd server is running
if ! nc -z localhost 4000 2>/dev/null; then
    echo "Error: vaultd server is not running"
    echo "Please start the server with 'cd vaultd && cargo run'"
    return 1
fi

echo "Activating vault environment..."

# Save original PS1
export VAULTWRAP_ORIGINAL_PS1="$PS1"

# Set the prompt to show we're in a vault environment
export PS1="(vault) $PS1"

# Get environment variables from the vault server
output=$("$VAULTWRAP_PATH" activate)
if [ $? -ne 0 ]; then
    echo "Error getting environment variables from vault server"
    return 1
fi

# An array to collect the variable names
declare -a LOADED_VARS=()

# Load environment variables and track which ones we set
while read -r line; do
    if [[ "$line" =~ ^export\ ([A-Za-z0-9_]+)=.* ]]; then
        var_name="${BASH_REMATCH[1]}"
        LOADED_VARS+=("$var_name")
        # Evaluate the export command to set the variable
        eval "$line"
    fi
done <<< "$output"

# Join array elements with space to create a string
VAULT_VAR_LIST=""
for var in "${LOADED_VARS[@]}"; do
    VAULT_VAR_LIST="$VAULT_VAR_LIST $var"
done
# Remove leading space
VAULT_VAR_LIST="${VAULT_VAR_LIST# }"

# Export variable list for deactivation
export VAULT_VAR_LIST

# Define a function to deactivate the vault environment
vault_deactivate() {
    # Restore the original prompt
    if [ -n "$VAULTWRAP_ORIGINAL_PS1" ]; then
        PS1="$VAULTWRAP_ORIGINAL_PS1"
        unset VAULTWRAP_ORIGINAL_PS1
    fi

    # Unset all environment variables from the vault
    if [ -n "$VAULT_VAR_LIST" ]; then
        for var in $VAULT_VAR_LIST; do
            echo "Unsetting $var"
            unset "$var"
        done
    fi
    
    # Clean up
    unset VAULT_VAR_LIST
    unset -f vault_deactivate
    unalias deactivate 2>/dev/null || true
    
    echo "Vault environment deactivated"
}

# Create an alias for the deactivate function
alias deactivate=vault_deactivate

# Show which variables were loaded
echo "Vault environment activated with these variables:"
if [ -n "$VAULT_VAR_LIST" ]; then
    for var in $VAULT_VAR_LIST; do
        echo "- $var"
    done
else
    echo "No variables were loaded"
fi

echo "Type 'deactivate' to exit the vault environment" 