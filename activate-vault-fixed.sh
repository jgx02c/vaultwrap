#!/bin/bash
# activate-vault-fixed.sh - Load vault environment variables into your shell
# Usage: source activate-vault-fixed.sh

# Check if being sourced
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    echo "This script must be sourced, not executed."
    echo "Please use: source ${0}"
    exit 1
fi

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

# Save original PS1 prompt
export VAULT_ORIGINAL_PS1="$PS1"

# Set the prompt to show we're in a vault environment
export PS1="(vault) $PS1"

# Create temporary file for environment variables
temp_file=$(mktemp)

# Get the environment variables from vaultwrap
"$VAULTWRAP_PATH" activate > "$temp_file" 2>/dev/null

# Read the temp file line by line
declare -a VAR_NAMES=()

while IFS= read -r line; do
    # Skip stderr output (starts with [vaultwrap])
    if [[ "$line" == \[vaultwrap\]* ]]; then
        continue
    fi
    
    # Extract variable name from export statements
    if [[ "$line" =~ ^export\ ([A-Za-z0-9_]+)= ]]; then
        var_name="${BASH_REMATCH[1]}"
        VAR_NAMES+=("$var_name")
        # Execute the export command
        eval "$line"
    fi
done < "$temp_file"

# Remove temp file
rm -f "$temp_file"

# Store variable names for deactivation
export VAULT_VAR_NAMES=$(IFS=, ; echo "${VAR_NAMES[*]}")

# Define deactivate function
vault_deactivate() {
    # Only proceed if we have variables to unset
    if [ -n "$VAULT_VAR_NAMES" ]; then
        echo "Deactivating vault environment..."
        
        # Restore original prompt
        if [ -n "$VAULT_ORIGINAL_PS1" ]; then
            export PS1="$VAULT_ORIGINAL_PS1"
            unset VAULT_ORIGINAL_PS1
        fi
        
        # Unset all environment variables
        IFS=',' read -ra VARS <<< "$VAULT_VAR_NAMES"
        for var in "${VARS[@]}"; do
            echo "Unsetting $var"
            unset "$var"
        done
        
        # Clean up
        unset VAULT_VAR_NAMES
        unset -f vault_deactivate
        unalias deactivate 2>/dev/null || true
        
        echo "Vault environment deactivated"
    else
        echo "No active vault environment found"
    fi
}

# Create an alias for deactivate
alias deactivate=vault_deactivate

# Display loaded variables
echo "Vault environment activated with these variables:"
if [ ${#VAR_NAMES[@]} -gt 0 ]; then
    for var in "${VAR_NAMES[@]}"; do
        echo "- $var"
    done
else
    echo "No variables were loaded"
fi

echo "Type 'deactivate' to exit the vault environment" 