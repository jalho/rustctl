#!/bin/bash

set -e

cd ./server/
cargo build --release --target x86_64-unknown-linux-musl
cd -

cd ./clients/web/
dx bundle --platform web
cd -

#
# TODO: Define dependencies somehow:
#
# - `steamcmd`: Cannot be automatically installed: Installer requires
#               interactivity, and there are other quirks too.
#
# - `wget`: Can be automatically installed.
#

#
# TODO: Define the whole build, including making the .deb package, using the
#       "xtask" pattern: https://github.com/matklad/cargo-xtask
#

PACKAGE_NAME="rustctl"
VERSION="0.1.0-rc9"
ARCH="amd64"
DEB_DIR="target/debian"
PACKAGE_DIR="$DEB_DIR/${PACKAGE_NAME}_${VERSION}_$ARCH"

rm -rf "$DEB_DIR"
mkdir -p "$PACKAGE_DIR/DEBIAN"
mkdir -p "$PACKAGE_DIR/usr/bin"
mkdir -p "$PACKAGE_DIR/var/lib/rustctl/web"

cp ./target/x86_64-unknown-linux-musl/release/rustctl-backend "$PACKAGE_DIR/usr/bin/rustctl-backend"
cp -r ./target/dx/rustctl-web/release/web/public/* "$PACKAGE_DIR/var/lib/rustctl/web/"

cat > "$PACKAGE_DIR/DEBIAN/control" << EOF
Package: $PACKAGE_NAME
Version: $VERSION
Section: base
Priority: optional
Architecture: $ARCH
Maintainer: TODO <todo@todo>
Description: rustctl
 Tooling for running a Rust (the game) server and an integrated web service.
EOF

dpkg-deb --build "$PACKAGE_DIR"

file "$PACKAGE_DIR.deb"

echo "Package built: $PACKAGE_DIR.deb"
