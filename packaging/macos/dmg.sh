#!/usr/bin/env bash
# Builds target/macos/groove_<version>_universal.dmg: Groove.app for Apple Silicon and Intel,
# ad-hoc signed, next to a link to /Applications. Run it from anywhere on a Mac.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

version=$(cargo pkgid -p groove | sed 's/.*[#@]//')
out=target/macos
app=$out/stage/Groove.app
dmg=$out/groove_${version}_universal.dmg

rm -rf "$out"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"

targets=(aarch64-apple-darwin x86_64-apple-darwin)
rustup target add "${targets[@]}"
for target in "${targets[@]}"; do
  cargo build --release --locked -p groove --target "$target"
done
lipo -create -output "$app/Contents/MacOS/groove" \
  "target/${targets[0]}/release/groove" "target/${targets[1]}/release/groove"

# CFBundleVersion takes digits and dots only: 1.0.0-alpha.3 is 1.0.0.
cp packaging/macos/Info.plist "$app/Contents/Info.plist"
plutil -replace CFBundleShortVersionString -string "$version" "$app/Contents/Info.plist"
plutil -replace CFBundleVersion -string "${version%%-*}" "$app/Contents/Info.plist"
plutil -lint "$app/Contents/Info.plist"

iconset=$out/groove.iconset
mkdir "$iconset"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" assets/groove-icon.png --out "$iconset/icon_${size}x${size}.png" > /dev/null
  sips -z $((size * 2)) $((size * 2)) assets/groove-icon.png \
    --out "$iconset/icon_${size}x${size}@2x.png" > /dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/groove.icns"
cp LICENSE "$app/Contents/Resources/LICENSE"

# Apple Silicon refuses an unsigned binary; the ad-hoc signature is not a Developer ID.
codesign --force --sign - "$app"
codesign --verify --strict "$app"

ln -s /Applications "$out/stage/Applications"
hdiutil create -volname Groove -srcfolder "$out/stage" -format UDZO -ov "$dmg"

hdiutil verify "$dmg"
mount=$(mktemp -d)
# The volume can be busy for a moment after a read: retry, then force.
trap 'hdiutil detach "$mount" -quiet || { sleep 2; hdiutil detach "$mount" -force -quiet; }' EXIT
hdiutil attach "$dmg" -nobrowse -readonly -mountpoint "$mount" -quiet
test -x "$mount/Groove.app/Contents/MacOS/groove"
test -L "$mount/Applications"
lipo -archs "$mount/Groove.app/Contents/MacOS/groove"

echo "$dmg"
