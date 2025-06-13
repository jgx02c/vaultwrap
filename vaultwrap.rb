class Vaultwrap < Formula
  desc "Environment variable injection like Python venv - secure .env alternative"
  homepage "https://github.com/yourusername/vaultwrap"
  url "https://github.com/yourusername/vaultwrap/archive/v1.0.0.tar.gz"
  sha256 "YOUR_SHA256_HERE"
  license "MIT"

  depends_on "rust" => :build

  def install
    # Build the CLI
    system "cargo", "install", "--path", "vaultwrap", "--root", prefix

    # Install shell integration
    (prefix/"share/vaultwrap").mkpath
    (prefix/"share/vaultwrap/vaultwrap.sh").write shell_integration_script
  end

  def post_install
    puts <<~EOS
      🔐 VaultWrap installed successfully!
      
      To enable shell integration (required for vaultwrap set/drop to work):
      
      For Zsh (default on macOS):
        echo 'source #{prefix}/share/vaultwrap/vaultwrap.sh' >> ~/.zshrc
        source ~/.zshrc
      
      For Bash:
        echo 'source #{prefix}/share/vaultwrap/vaultwrap.sh' >> ~/.bashrc
        source ~/.bashrc
      
      For Fish:
        echo 'source #{prefix}/share/vaultwrap/vaultwrap.sh' >> ~/.config/fish/config.fish
      
      Quick Start:
        1. vaultwrap connect localhost:4000 --save local --default
        2. vaultwrap set <environment>  # Changes prompt to (<environment>)
        3. vaultwrap drop               # Restores original prompt
      
      Documentation: https://github.com/yourusername/vaultwrap
    EOS
  end

  def caveats
    <<~EOS
      VaultWrap requires shell integration to work properly.
      Run the post-install commands above to enable it.
    EOS
  end

  private

  def shell_integration_script
    <<~EOS
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
      }
    EOS
  end

  test do
    # Test that the binary was installed
    assert_match "VaultWrap CLI tool", shell_output("#{bin}/vaultwrap --help")
    
    # Test that shell integration file exists
    assert_predicate prefix/"share/vaultwrap/vaultwrap.sh", :exist?
  end
end 