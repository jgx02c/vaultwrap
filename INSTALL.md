# VaultWrap Installation

VaultWrap provides multiple installation methods to get you up and running quickly.

## 🚀 Quick Install (Recommended)

### Automated Installation
```bash
curl -sSL https://raw.githubusercontent.com/yourusername/vaultwrap/main/install-auto.sh | bash
```

This will:
- Install the VaultWrap CLI binary
- Automatically detect your shell (zsh, bash, fish)
- Add shell integration to your config file
- Reload your shell configuration

### Manual Installation
```bash
git clone https://github.com/yourusername/vaultwrap.git
cd vaultwrap
./install.sh
```

This provides an interactive installation with prompts.

## 📦 Homebrew (Coming Soon)

```bash
brew install vaultwrap
```

The Homebrew formula will automatically set up shell integration.

## 🔧 Manual Setup

If you prefer to install manually:

### 1. Install the CLI
```bash
git clone https://github.com/yourusername/vaultwrap.git
cd vaultwrap/vaultwrap
cargo install --path .
```

### 2. Add Shell Integration

Add this to your shell config file:

**For Zsh (~/.zshrc) or Bash (~/.bashrc):**
```bash
# VaultWrap shell integration
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
}
```

**For Fish (~/.config/fish/config.fish):**
```fish
# VaultWrap shell integration
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
end
```

### 3. Reload Your Shell
```bash
source ~/.zshrc    # or ~/.bashrc or ~/.config/fish/config.fish
```

## ✅ Verify Installation

```bash
vaultwrap --help
```

You should see the VaultWrap help message.

## 🎯 Quick Start

1. **Connect to a VaultWrap server:**
   ```bash
   vaultwrap connect localhost:4000 --save local --default
   ```

2. **Activate an environment:**
   ```bash
   vaultwrap set dev
   ```
   Your prompt should change to `(dev) user@host:path $`

3. **Deactivate:**
   ```bash
   vaultwrap drop
   ```
   Your prompt returns to normal.

## 🔍 Troubleshooting

### Shell Integration Not Working
- Make sure you've added the shell function to your config file
- Restart your terminal or run `source ~/.zshrc` (or your shell config)
- Check that `command vaultwrap --help` works

### Command Not Found
- Make sure `~/.cargo/bin` is in your PATH
- Try `cargo install --path . --force` to reinstall

### Permission Issues
- Make sure the installer scripts are executable: `chmod +x install.sh`

## 📚 Next Steps

- Read the [Usage Guide](USAGE.md)
- Run the demo: `./demo.sh`
- Check out the [main README](README.md)

## 🆘 Support

If you encounter issues:
1. Check the troubleshooting section above
2. Open an issue on GitHub
3. Include your shell type and OS version 