#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# VaultWrap installer
echo -e "${BLUE}🔐 VaultWrap Installer${NC}"
echo "Setting up VaultWrap - Environment variable injection like Python venv"
echo ""

# Detect shell
SHELL_NAME=$(basename "$SHELL")
case "$SHELL_NAME" in
    "zsh")
        SHELL_CONFIG="$HOME/.zshrc"
        ;;
    "bash")
        SHELL_CONFIG="$HOME/.bashrc"
        ;;
    "fish")
        SHELL_CONFIG="$HOME/.config/fish/config.fish"
        ;;
    *)
        echo -e "${YELLOW}Warning: Unsupported shell '$SHELL_NAME'. Defaulting to ~/.bashrc${NC}"
        SHELL_CONFIG="$HOME/.bashrc"
        ;;
esac

echo -e "Detected shell: ${GREEN}$SHELL_NAME${NC}"
echo -e "Shell config: ${GREEN}$SHELL_CONFIG${NC}"
echo ""

# Install VaultWrap binary
echo -e "${BLUE}📦 Installing VaultWrap binary...${NC}"
if command -v cargo >/dev/null 2>&1; then
    cargo install --path . --force
    echo -e "${GREEN}✅ VaultWrap binary installed successfully${NC}"
else
    echo -e "${RED}❌ Error: Cargo not found. Please install Rust first.${NC}"
    echo "Visit: https://rustup.rs/"
    exit 1
fi

# Create shell integration function
VAULTWRAP_FUNCTION='
# VaultWrap shell integration - Environment variable injection like Python venv
vaultwrap() {
    case "$1" in
        set)
            if [ -z "$2" ]; then
                echo "Usage: vaultwrap set <environment>"
                return 1
            fi
            eval "$(command vaultwrap set "$2" --shell-output)"
            ;;
        drop)
            eval "$(command vaultwrap drop --shell-output)"
            ;;
        *)
            command vaultwrap "$@"
            ;;
    esac
}'

# Check if already installed
if grep -q "VaultWrap shell integration" "$SHELL_CONFIG" 2>/dev/null; then
    echo -e "${YELLOW}⚠️  VaultWrap shell integration already exists in $SHELL_CONFIG${NC}"
    echo ""
    read -p "Do you want to update it? (y/N): " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        # Remove old integration
        if [[ "$SHELL_NAME" == "fish" ]]; then
            echo -e "${RED}❌ Fish shell auto-update not supported yet. Please manually update your config.${NC}"
            exit 1
        else
            # Create backup
            cp "$SHELL_CONFIG" "$SHELL_CONFIG.vaultwrap.backup"
            echo -e "${BLUE}📋 Created backup: $SHELL_CONFIG.vaultwrap.backup${NC}"
            
            # Remove old integration (between markers)
            sed -i.tmp '/# VaultWrap shell integration/,/^}$/d' "$SHELL_CONFIG"
            rm -f "$SHELL_CONFIG.tmp"
        fi
    else
        echo -e "${YELLOW}⏭️  Skipping shell integration update${NC}"
        echo -e "${GREEN}✅ VaultWrap installation complete!${NC}"
        exit 0
    fi
fi

# Add shell integration
echo -e "${BLUE}🔧 Setting up shell integration...${NC}"

if [[ "$SHELL_NAME" == "fish" ]]; then
    # Fish shell function
    FISH_FUNCTION='
# VaultWrap shell integration - Environment variable injection like Python venv
function vaultwrap
    switch $argv[1]
        case set
            if test (count $argv) -lt 2
                echo "Usage: vaultwrap set <environment>"
                return 1
            end
            eval (command vaultwrap set $argv[2] --shell-output)
        case drop
            eval (command vaultwrap drop --shell-output)
        case "*"
            command vaultwrap $argv
    end
end'
    echo "$FISH_FUNCTION" >> "$SHELL_CONFIG"
else
    # Bash/Zsh function
    echo "$VAULTWRAP_FUNCTION" >> "$SHELL_CONFIG"
fi

echo -e "${GREEN}✅ Shell integration added to $SHELL_CONFIG${NC}"
echo ""

# Success message
echo -e "${GREEN}🎉 VaultWrap installation complete!${NC}"
echo ""
echo -e "${BLUE}📖 Quick Start:${NC}"
echo "1. Restart your terminal or run: source $SHELL_CONFIG"
echo "2. Connect to a VaultWrap server: vaultwrap connect localhost:4000 --save local --default"
echo "3. Activate an environment: vaultwrap set <environment>"
echo "4. Deactivate: vaultwrap drop"
echo ""
echo -e "${BLUE}📚 Documentation:${NC}"
echo "• Usage guide: cat USAGE.md"
echo "• Demo: ./demo.sh"
echo "• Help: vaultwrap --help"
echo ""

# Ask to reload shell
read -p "Do you want to reload your shell configuration now? (Y/n): " -n 1 -r
echo ""
if [[ $REPLY =~ ^[Nn]$ ]]; then
    echo -e "${YELLOW}⚠️  Remember to restart your terminal or run: source $SHELL_CONFIG${NC}"
else
    echo -e "${BLUE}🔄 Reloading shell configuration...${NC}"
    if [[ "$SHELL_NAME" == "fish" ]]; then
        exec fish
    else
        source "$SHELL_CONFIG"
        echo -e "${GREEN}✅ Shell configuration reloaded!${NC}"
    fi
fi

echo ""
echo -e "${GREEN}🚀 VaultWrap is ready to use!${NC}" 