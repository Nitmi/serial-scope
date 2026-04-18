#!/usr/bin/env bash
set -euo pipefail

APP_NAME="serial-scope"
ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
DIST_DIR="$ROOT_DIR/dist/${APP_NAME}-linux-x86_64"

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

cp "$ROOT_DIR/target/release/$APP_NAME" "$DIST_DIR/$APP_NAME"

tar -C "$ROOT_DIR/dist" -czf "$ROOT_DIR/dist/${APP_NAME}-linux-x86_64.tar.gz" "${APP_NAME}-linux-x86_64"

echo "Created $ROOT_DIR/dist/${APP_NAME}-linux-x86_64.tar.gz"
