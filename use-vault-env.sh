#!/bin/bash
# use-vault-env.sh - Use environment variables from vaultd
# Copy this script to any project and run it with your command
# Example: ./use-vault-env.sh python3 app.py

# Path to the vaultwrap binary
VAULTWRAP_PATH="/Users/joshuagoodman/Documents/GitHub/vaultwrap/vaultwrap/target/debug/vaultwrap"

# Check if vaultwrap binary exists
if [ ! -f "$VAULTWRAP_PATH" ]; then
    echo "Error: vaultwrap binary not found at $VAULTWRAP_PATH"
    echo "Please build the vaultwrap project first with 'cd vaultwrap && cargo build'"
    exit 1
fi

# Check if vaultd server is running
if ! nc -z localhost 4000 2>/dev/null; then
    echo "Error: vaultd server is not running"
    echo "Please start the server with 'cd vaultd && cargo run'"
    exit 1
fi

# If no arguments provided, show usage
if [ $# -eq 0 ]; then
    echo "Usage: $0 <command> [args...]"
    echo "Example: $0 python3 app.py"
    exit 1
fi

# Get the command to run (everything passed to this script)
COMMAND="$@"

# Use vaultwrap to run the command with environment variables
"$VAULTWRAP_PATH" run $COMMAND 