# Vaultwrap — `make help` lists everything.

VERSION   := $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
APP_NAME  := Vaultwrap
APP       := target/release/bundle/macos/$(APP_NAME).app
DIST      := dist
PKG       := $(DIST)/$(APP_NAME)-$(VERSION).pkg
DMG       := $(DIST)/$(APP_NAME)-$(VERSION).dmg

# Code signing. "-" = ad-hoc (fine for your own machine). For distribution set
# SIGN_IDENTITY="Developer ID Application: …" and INSTALLER_IDENTITY="Developer ID Installer: …".
SIGN_IDENTITY      ?= -
INSTALLER_IDENTITY ?=

# Dev runs use a throwaway vault in .dev/ and never touch your real vault,
# Keychain or login items. A JSON file stands in for the Keychain.
DEV_HOME := $(CURDIR)/.dev/vault
DEV_ENV  := VAULTWRAP_HOME=$(DEV_HOME) VAULTWRAP_KEYCHAIN_FILE=$(CURDIR)/.dev/keychain.json VAULTWRAP_SKIP_SETUP=1

.DEFAULT_GOAL := help
.PHONY: help build test lint fmt check dev demo demo-seed cli daemon dev-stop dev-reset \
        release app dmg pkg dist install install-cli uninstall icons tauri-cli clean

help: ## Show this help
	@awk 'BEGIN {FS = ":.*## "} /^##@/ {printf "\n\033[1m%s\033[0m\n", substr($$0, 5)} /^[a-zA-Z_-]+:.*## / {printf "  \033[36m%-13s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

##@ Development

build: ## Debug build: CLI/daemon + desktop app
	cargo build --workspace

test: ## Run all tests
	cargo test --workspace

lint: ## rustfmt check + clippy (warnings are errors)
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	node --check crates/vaultwrap-desktop/ui/app.js
	bash -n scripts/demo.sh scripts/build-pkg.sh scripts/uninstall.sh

fmt: ## Format all Rust code
	cargo fmt --all

check: lint test ## Everything CI should run

dev: build ## Run the desktop app against the dev vault (.dev/)
	$(DEV_ENV) ./target/debug/vaultwrap-desktop

demo: build ## Open the app with mock orgs, projects and secrets (.dev/demo)
	./scripts/demo.sh

demo-seed: build ## Recreate the demo vault from scratch
	./scripts/demo.sh seed

cli: build ## Run the CLI against the dev vault: make cli ARGS="ls"
	@$(DEV_ENV) ./target/debug/vaultwrap $(ARGS)

daemon: build dev-stop ## Run the daemon in the foreground with logs (dev vault)
	$(DEV_ENV) ./target/debug/vaultwrap daemon

dev-stop: ## Stop the dev vault's daemon
	-@$(DEV_ENV) ./target/debug/vaultwrap lock >/dev/null 2>&1; \
	  pkill -f "$(CURDIR)/target/debug/vaultwrap daemon" 2>/dev/null; true

dev-reset: dev-stop ## Delete the dev vault and start fresh
	rm -rf .dev

##@ Packaging (macOS)

release: ## Release build of the CLI/daemon
	cargo build --release -p vaultwrap

tauri-cli:
	@cargo tauri --version >/dev/null 2>&1 || cargo install tauri-cli --version "^2" --locked

app: release tauri-cli ## Build Vaultwrap.app (CLI bundled inside)
	cd crates/vaultwrap-desktop && cargo tauri build --bundles app
	codesign --force --deep --options runtime --sign "$(SIGN_IDENTITY)" "$(APP)"
	codesign --verify --deep --strict "$(APP)"
	@echo "built $(APP)"

pkg: app ## Build the installer: dist/Vaultwrap-<version>.pkg
	./scripts/build-pkg.sh "$(APP)" "$(VERSION)" "$(PKG)" "$(INSTALLER_IDENTITY)"

dmg: app ## Build a drag-to-Applications disk image
	rm -rf $(DIST)/dmg && mkdir -p $(DIST)/dmg
	ditto "$(APP)" "$(DIST)/dmg/$(APP_NAME).app"
	ln -s /Applications "$(DIST)/dmg/Applications"
	hdiutil create -quiet -volname "$(APP_NAME)" -srcfolder $(DIST)/dmg -ov -format UDZO "$(DMG)"
	rm -rf $(DIST)/dmg
	@echo "built $(DMG)"

dist: pkg dmg ## Build both the .pkg and the .dmg

install: pkg ## Build and install on this Mac (asks for your password)
	sudo installer -pkg "$(PKG)" -target /

install-cli: ## Install just the CLI with cargo (no app)
	cargo install --locked --path crates/vaultwrap

uninstall: ## Remove the app, CLI link and services (keeps your vault)
	./scripts/uninstall.sh

icons: tauri-cli ## Regenerate app + menu-bar icons
	python3 scripts/make_icons.py crates/vaultwrap-desktop/icons
	cd crates/vaultwrap-desktop && cargo tauri icon icons/app-icon.png -o icons && rm -rf icons/android icons/ios
	cp crates/vaultwrap-desktop/icons/128x128@2x.png crates/vaultwrap-desktop/ui/logo.png

clean: ## Remove build outputs and the dev vault
	cargo clean
	rm -rf $(DIST) .dev
