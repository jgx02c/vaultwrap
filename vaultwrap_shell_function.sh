# Add this to your ~/.zshrc or ~/.bashrc

vaultwrap() {
    case "$1" in
        set)
            if [ -z "$2" ]; then
                echo "Usage: vaultwrap set <environment>"
                return 1
            fi
            
            # Get the export commands from the CLI
            local output
            output=$(command vaultwrap set "$2" --shell-output 2>&1)
            local exit_code=$?
            
            if [ $exit_code -eq 0 ]; then
                # If successful, execute the export commands
                eval "$output"
                echo "Environment '$2' activated."
            else
                echo "$output"
                return $exit_code
            fi
            ;;
        drop)
            # Get the unset commands from the CLI
            local output
            output=$(command vaultwrap drop --shell-output 2>&1)
            local exit_code=$?
            
            if [ $exit_code -eq 0 ]; then
                # If successful, execute the unset commands
                eval "$output"
                echo "Environment deactivated."
            else
                echo "$output"
                return $exit_code
            fi
            ;;
        *)
            # For all other commands, just pass through to the CLI
            command vaultwrap "$@"
            ;;
    esac
} 