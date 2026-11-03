#!/usr/bin/env bash
# Remove Vaultwrap: app, CLI link, background service and login item.
# Your encrypted vault is kept unless you pass --purge.
set -uo pipefail
echo "Removing Vaultwrap…"
pkill -x vaultwrap-desktop 2>/dev/null
launchctl bootout "gui/$(id -u)/dev.vaultwrap.daemon" 2>/dev/null
rm -f ~/Library/LaunchAgents/dev.vaultwrap.daemon.plist ~/Library/LaunchAgents/dev.vaultwrap.app.plist
pkill -f "vaultwrap daemon" 2>/dev/null
if [ -e /Applications/Vaultwrap.app ] || [ -L /usr/local/bin/vaultwrap ]; then
  sudo rm -rf /Applications/Vaultwrap.app
  [ -L /usr/local/bin/vaultwrap ] && sudo rm -f /usr/local/bin/vaultwrap
  sudo pkgutil --forget dev.vaultwrap.app >/dev/null 2>&1
fi
VAULT="$HOME/Library/Application Support/vaultwrap"
if [ "${1:-}" = "--purge" ]; then
  rm -rf "$VAULT"
  # Vault keys (service dev.vaultwrap) and passwords saved by early builds (service vaultwrap).
  while security delete-generic-password -s dev.vaultwrap >/dev/null 2>&1; do :; done
  while security delete-generic-password -s vaultwrap >/dev/null 2>&1; do :; done
  echo "Deleted the vault and its Keychain entries."
else
  echo "Kept your vault at: $VAULT  (run with --purge to delete it)"
fi
echo "Done."
