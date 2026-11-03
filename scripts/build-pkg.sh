#!/usr/bin/env bash
# Build a macOS installer package from a built Vaultwrap.app.
# usage: build-pkg.sh <Vaultwrap.app> <version> <out.pkg> [installer signing identity]
set -euo pipefail
APP=$1 VERSION=$2 OUT=$3 SIGN=${4:-}
HERE="$(cd "$(dirname "$0")" && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

mkdir -p "$WORK/root/Applications" "$WORK/scripts" "$WORK/pkgs" "$(dirname "$OUT")"
ditto "$APP" "$WORK/root/Applications/Vaultwrap.app"
cp "$HERE/pkg/preinstall" "$HERE/pkg/postinstall" "$WORK/scripts/"

# Always install to /Applications, even if another copy of the app exists elsewhere.
pkgbuild --analyze --root "$WORK/root" "$WORK/component.plist" >/dev/null
plutil -replace 0.BundleIsRelocatable -bool NO "$WORK/component.plist"

pkgbuild --quiet --root "$WORK/root" --component-plist "$WORK/component.plist" \
  --scripts "$WORK/scripts" --identifier dev.vaultwrap.app --version "$VERSION" \
  --install-location / "$WORK/pkgs/Vaultwrap-component.pkg"

ARCH="$(lipo -archs "$APP/Contents/MacOS/vaultwrap-desktop" | tr ' ' ',')"
sed -e "s/__VERSION__/$VERSION/" -e "s/__ARCH__/$ARCH/" "$HERE/pkg/distribution.xml" > "$WORK/distribution.xml"

productbuild --quiet --distribution "$WORK/distribution.xml" --resources "$HERE/pkg/resources" \
  --package-path "$WORK/pkgs" ${SIGN:+--sign "$SIGN"} "$OUT"
echo "built $OUT"
