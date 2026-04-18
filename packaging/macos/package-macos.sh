#!/usr/bin/env bash
set -euo pipefail

APP_NAME="serial-scope"
DISPLAY_NAME="Serial Scope"
ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION="${VERSION:-$(sed -n 's/^version = "\(.*\)"$/\1/p' "$ROOT_DIR/Cargo.toml" | head -n1)}"
DIST_DIR="$ROOT_DIR/dist/${APP_NAME}-macos"
APP_BUNDLE="$DIST_DIR/${DISPLAY_NAME}.app"
CONTENTS_DIR="$APP_BUNDLE/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"
ICONSET_DIR="$ROOT_DIR/dist/${APP_NAME}.iconset"
ICON_FILE="$RESOURCES_DIR/${APP_NAME}.icns"
ZIP_PATH="$ROOT_DIR/dist/${APP_NAME}-macos.app.zip"

rm -rf "$DIST_DIR" "$ICONSET_DIR" "$ZIP_PATH"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR" "$ICONSET_DIR"

cp "$ROOT_DIR/target/release/$APP_NAME" "$MACOS_DIR/$APP_NAME"
chmod +x "$MACOS_DIR/$APP_NAME"

cat > "$CONTENTS_DIR/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>Serial Scope</string>
    <key>CFBundleExecutable</key>
    <string>serial-scope</string>
    <key>CFBundleIconFile</key>
    <string>serial-scope</string>
    <key>CFBundleIdentifier</key>
    <string>io.github.serial-scope</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Serial Scope</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST

for size in 16 32 128 256 512; do
  sips -z "$size" "$size" "$ROOT_DIR/assets/app-icon.png" --out "$ICONSET_DIR/icon_${size}x${size}.png" >/dev/null
done

sips -z 32 32 "$ROOT_DIR/assets/app-icon.png" --out "$ICONSET_DIR/icon_16x16@2x.png" >/dev/null
sips -z 64 64 "$ROOT_DIR/assets/app-icon.png" --out "$ICONSET_DIR/icon_32x32@2x.png" >/dev/null
sips -z 256 256 "$ROOT_DIR/assets/app-icon.png" --out "$ICONSET_DIR/icon_128x128@2x.png" >/dev/null
sips -z 512 512 "$ROOT_DIR/assets/app-icon.png" --out "$ICONSET_DIR/icon_256x256@2x.png" >/dev/null
sips -z 1024 1024 "$ROOT_DIR/assets/app-icon.png" --out "$ICONSET_DIR/icon_512x512@2x.png" >/dev/null

iconutil -c icns "$ICONSET_DIR" -o "$ICON_FILE"

ditto -c -k --sequesterRsrc --keepParent "$APP_BUNDLE" "$ZIP_PATH"

rm -rf "$ICONSET_DIR"

echo "Created $ZIP_PATH"
