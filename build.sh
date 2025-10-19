#!/bin/bash

set -e

cd ./server/
cargo build --release
cd -

cd ./clients/web/
dx bundle --platform web
cd -

#
# TODO: Build a `.deb` package
#
mv ./target/dx/rustctl-web/release/web/public /var/lib/rustctl/web
