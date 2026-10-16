#!/bin/sh
# Builds the Linux package: dist/bounce_tales-<version>-linux-amd64.deb
# Needs cargo-deb (`cargo install cargo-deb`). Run from anywhere.
set -e
cd "$(dirname "$0")/../.."
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
cargo build --release
# cargo-deb names pre-releases its own way (1.0.1-rc1 becomes 1.0.1~rc1),
# so build into a clean folder and take whatever it made.
rm -rf target/debian
cargo deb --no-build --output target/debian/
mkdir -p dist
out="dist/bounce_tales-$version-linux-amd64.deb"
cp target/debian/*.deb "$out"
echo "$out"
