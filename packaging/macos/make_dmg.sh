#!/bin/sh
# Builds the macOS download (run on a Mac from anywhere):
#   dist/bounce_tales-<version>-macos-universal.dmg
# The game is built for Apple Silicon and Intel and merged into one
# universal binary inside "Bounce Tales.app". Not notarized: the bundle is
# only ad-hoc signed, so the first launch needs right-click > Open.
set -e
cd "$(dirname "$0")/../.."
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)

rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

app="target/macos/Bounce Tales.app"
rm -rf target/macos
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
lipo -create -output "$app/Contents/MacOS/bounce_tales" \
    target/aarch64-apple-darwin/release/bounce_tales \
    target/x86_64-apple-darwin/release/bounce_tales
sed "s/__VERSION__/$version/g" packaging/macos/Info.plist > "$app/Contents/Info.plist"
cp packaging/icon/bounce_tales.icns "$app/Contents/Resources/"
plutil -lint "$app/Contents/Info.plist"
codesign --force --deep --sign - "$app"

# Disk image: the app and a link to drag it into Applications.
stage=target/macos/dmg
mkdir -p "$stage"
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
mkdir -p dist
out="dist/bounce_tales-$version-macos-universal.dmg"
rm -f "$out"
hdiutil create -volname "Bounce Tales" -srcfolder "$stage" -ov -format UDZO "$out"
echo "$out"
