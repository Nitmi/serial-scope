#!/usr/bin/env bash
set -euo pipefail

APP_NAME="serial-scope"
VERSION="${1:-0.1.1}"
ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
DIST_DIR="$ROOT_DIR/dist/${APP_NAME}-linux-x86_64"
BIN_DIR="$DIST_DIR/usr/bin"
APP_DIR="$DIST_DIR/usr/share/applications"
ICON_DIR="$DIST_DIR/usr/share/icons/hicolor/512x512/apps"

rm -rf "$DIST_DIR"
mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"

cp "$ROOT_DIR/target/release/$APP_NAME" "$BIN_DIR/$APP_NAME"
cp "$ROOT_DIR/packaging/linux/$APP_NAME.desktop" "$APP_DIR/$APP_NAME.desktop"
cp "$ROOT_DIR/assets/app-icon.png" "$ICON_DIR/$APP_NAME.png"
cp "$ROOT_DIR/README.md" "$DIST_DIR/README.md"
cp "$ROOT_DIR/LICENSE" "$DIST_DIR/LICENSE"

python3 - <<'PY' "$APP_DIR/$APP_NAME.desktop" "$VERSION"
import pathlib
import sys

desktop_path = pathlib.Path(sys.argv[1])
version = sys.argv[2]
text = desktop_path.read_text(encoding='utf-8')
if 'X-AppVersion=' not in text:
    text += f'X-AppVersion={version}\n'
desktop_path.write_text(text, encoding='utf-8')
PY

tar -C "$ROOT_DIR/dist" -czf "$ROOT_DIR/dist/${APP_NAME}-linux-x86_64.tar.gz" "${APP_NAME}-linux-x86_64"

echo "Created $ROOT_DIR/dist/${APP_NAME}-linux-x86_64.tar.gz"
