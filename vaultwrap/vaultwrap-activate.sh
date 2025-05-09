#!/bin/bash
# VaultWrap Activation Script
# Source this file to activate the vault environment
# Usage: source vaultwrap-activate.sh

# Check if script is being sourced, not executed
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    echo "This script must be sourced, not executed."
    echo "Please use: source ${0}"
    exit 1
fi

# Get the directory where the script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Make sure the vaultwrap binary is available
if [ ! -f "${SCRIPT_DIR}/target/debug/vaultwrap" ]; then
    echo "[vaultwrap] Error: vaultwrap binary not found."
    echo "[vaultwrap] Make sure you've built the project with 'cargo build'"
    return 1
fi

# Check if vaultd is running
if ! nc -z localhost 4000 2>/dev/null; then
    echo "[vaultwrap] Error: vaultd server is not running."
    echo "[vaultwrap] Please start the server with 'cd ../vaultd && cargo run'"
    return 1
fi

echo "[vaultwrap] Activating vault environment..."

# Directly set the environment variables to avoid eval issues
# Save the original PS1
export VAULTWRAP_ORIGINAL_PS1="$PS1"

# Set the prompt to show we're in a vaultwrap environment
export PS1="(vaultwrap) $PS1"

# Mark that we're in a vaultwrap environment
export VAULTWRAP_ACTIVE=1

# Get environment variables from the server and export them
eval "$("${SCRIPT_DIR}/target/debug/vaultwrap" activate)"

# Define deactivation function
vaultwrap_deactivate() {
  if [ -n "$VAULTWRAP_ACTIVE" ]; then
    unset VAULTWRAP_ACTIVE
    export PS1="$VAULTWRAP_ORIGINAL_PS1"
    unset VAULTWRAP_ORIGINAL_PS1
    unset -f vaultwrap_deactivate
    echo "[vaultwrap] Environment deactivated"
  fi
}

# Create alias for easy deactivation
alias deactivate=vaultwrap_deactivate

echo "[vaultwrap] Environment activated. Type 'deactivate' to exit." 