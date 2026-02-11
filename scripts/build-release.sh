#!/usr/bin/env bash
set -e

VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
ARCH=$(uname -m)
NAME="markview-${VERSION}-${ARCH}-unknown-linux-gnu"

mkdir -p "$NAME"
cargo build --release
cp target/release/gtk-markdown-viewer "$NAME/markview"
cp LICENSE README.md "$NAME/"
tar czvf "${NAME}.tar.gz" "$NAME"
echo "Built ${NAME}.tar.gz"
