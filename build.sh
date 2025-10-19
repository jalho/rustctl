#!/bin/bash

set -e

cd ./server/
cargo build --release --target x86_64-unknown-linux-musl
cd -

cd ./clients/web/
dx bundle --platform web
cd -

#
# TODO: Build a `.deb` package
#
mv ./target/dx/rustctl-web/release/web/public /var/lib/rustctl/web
