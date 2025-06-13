# Setting up VaultWrap Homebrew Tap

## 1. Create GitHub Release

```bash
# Tag your release
git tag v1.0.0
git push origin v1.0.0

# Go to GitHub and create a release from this tag
# This creates: https://github.com/yourusername/vaultwrap/archive/v1.0.0.tar.gz
```

## 2. Get SHA256 Hash

```bash
curl -sL https://github.com/yourusername/vaultwrap/archive/v1.0.0.tar.gz | shasum -a 256
# Copy this hash
```

## 3. Update Formula

Edit `vaultwrap.rb`:
```ruby
class Vaultwrap < Formula
  desc "Environment variable injection like Python venv - secure .env alternative"
  homepage "https://github.com/yourusername/vaultwrap"
  url "https://github.com/yourusername/vaultwrap/archive/v1.0.0.tar.gz"
  sha256 "PUT_ACTUAL_SHA256_HERE"
  license "MIT"
  # ... rest of formula
end
```

## 4. Create Homebrew Tap Repository

```bash
# Create new repo on GitHub: yourusername/homebrew-vaultwrap
# Clone it locally
git clone https://github.com/yourusername/homebrew-vaultwrap.git
cd homebrew-vaultwrap

# Create Formula directory and add formula
mkdir Formula
cp /path/to/vaultwrap.rb Formula/vaultwrap.rb

# Commit and push
git add .
git commit -m "Add vaultwrap formula"
git push origin main
```

## 5. Test Installation

```bash
# Test locally
brew tap yourusername/vaultwrap
brew install --build-from-source vaultwrap
brew test vaultwrap

# Test the shell integration
vaultwrap --help
```

## 6. Users Install With

```bash
brew tap yourusername/vaultwrap
brew install vaultwrap
```

## 7. (Optional) Submit to Homebrew Core

Once your tap is working well:

```bash
# Fork homebrew-core
git clone https://github.com/Homebrew/homebrew-core.git
cd homebrew-core

# Copy your tested formula
cp /path/to/Formula/vaultwrap.rb Formula/vaultwrap.rb

# Test
brew install --build-from-source ./Formula/vaultwrap.rb
brew audit --strict vaultwrap

# Submit PR
git checkout -b vaultwrap
git add Formula/vaultwrap.rb
git commit -m "vaultwrap: new formula"
git push origin vaultwrap
# Create PR on GitHub
```

## Formula Requirements for Homebrew Core

- Must be notable/popular software
- Must have stable releases
- Must pass all tests
- Must follow Homebrew guidelines
- Usually need 75+ GitHub stars or significant usage

## Tips

- Start with your own tap first
- Test thoroughly before submitting to core
- Follow Homebrew formula style guide
- Include good description and homepage
- Make sure all dependencies are correct 