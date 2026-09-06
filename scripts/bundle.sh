#!/bin/sh
set -eu

if [ "$(uname -s)" != "Darwin" ]; then
  echo "macOS application bundles can only be built on macOS" >&2
  exit 1
fi

profile="${1:-debug}"
case "$profile" in
  debug)
    app_name="Locus Debug"
    bundle_identifier="me.oneo.locus.dev"
    cargo_flags=""
    ;;
  release)
    app_name="Locus"
    bundle_identifier="me.oneo.locus"
    cargo_flags="--release"
    ;;
  *)
    echo "usage: scripts/bundle.sh [debug|release]" >&2
    exit 2
    ;;
esac

cargo_target_dir="${CARGO_TARGET_DIR:-target}"
version="$(awk -F ' *= *' '$1 == "version" { gsub(/"/, "", $2); print $2; exit }' Cargo.toml)"

cargo build -p locus $cargo_flags

bundle="$cargo_target_dir/$profile/$app_name.app"
contents="$bundle/Contents"
resources="$contents/Resources"
partial_info="$cargo_target_dir/$profile/locus-app-icon.plist"
actool_log="$cargo_target_dir/$profile/locus-app-icon-result.plist"

rm -rf "$bundle"
mkdir -p "$contents/MacOS" "$resources"
cp "$cargo_target_dir/$profile/locus" "$contents/MacOS/$app_name"
cp resources/Info.plist "$contents/Info.plist"

plutil -replace CFBundleDisplayName -string "$app_name" "$contents/Info.plist"
plutil -replace CFBundleExecutable -string "$app_name" "$contents/Info.plist"
plutil -replace CFBundleIdentifier -string "$bundle_identifier" "$contents/Info.plist"
plutil -replace CFBundleName -string "$app_name" "$contents/Info.plist"
plutil -replace CFBundleShortVersionString -string "$version" "$contents/Info.plist"
plutil -replace CFBundleVersion -string "$version" "$contents/Info.plist"

if ! xcrun actool \
  --compile "$resources" \
  --platform macosx \
  --minimum-deployment-target 13.0 \
  --app-icon AppIcon \
  --output-partial-info-plist "$partial_info" \
  resources/AppIcon.icon >"$actool_log"; then
  cat "$actool_log" >&2
  exit 1
fi
/usr/libexec/PlistBuddy -c "Merge $partial_info" "$contents/Info.plist"

xattr -cr "$bundle"
codesign_identity="${LOCUS_CODESIGN_IDENTITY:--}"
if [ "$codesign_identity" = "-" ]; then
  codesign --force --identifier "$bundle_identifier" --sign - "$bundle"
else
  codesign --force --options runtime --timestamp --sign "$codesign_identity" "$bundle"
fi
codesign --verify --deep --strict --verbose=2 "$bundle"

echo "$bundle"
