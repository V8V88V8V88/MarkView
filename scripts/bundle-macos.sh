#!/usr/bin/env bash
# Needs: brew install pkgconf gtk4 libadwaita gtksourceview5 adwaita-icon-theme dylibbundler
set -euo pipefail

VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
SHORT_VERSION=${VERSION%%-*}
NAME="markview-v${VERSION}-aarch64-apple-darwin"
BREW=$(brew --prefix)
# Homebrew bottles target the build machine's macOS.
MIN_MACOS=$(sw_vers -productVersion | cut -d. -f1).0
APP="dist/MarkView.app"
C="$APP/Contents"

cargo build --release --locked

rm -rf dist/MarkView.app "dist/$NAME.dmg" dist/dmg
mkdir -p "$C/MacOS" "$C/Frameworks" "$C/Resources/share/icons" "$C/Resources/share/glib-2.0"

cp target/release/gtk-markdown-viewer "$C/MacOS/markview"

dylibbundler -od -b -x "$C/MacOS/markview" -d "$C/Frameworks" \
    -p @executable_path/../Frameworks/ -s "$BREW/lib" </dev/null

cp -RL "$BREW/share/icons/Adwaita" "$BREW/share/icons/hicolor" "$C/Resources/share/icons/"
cp -RL "$BREW/share/glib-2.0/schemas" "$C/Resources/share/glib-2.0/"
glib-compile-schemas "$C/Resources/share/glib-2.0/schemas"
cp -RL "$BREW/share/gtksourceview-5" "$C/Resources/share/"
gtk4-update-icon-cache -f -t "$C/Resources/share/icons/hicolor" || true

mkdir -p "$C/Resources/data"
cp -R data/styles data/icons "$C/Resources/data/"

ICONSET=dist/MarkView.iconset
rm -rf "$ICONSET" && mkdir -p "$ICONSET"
SRC=data/icons/hicolor/256x256/apps/com.v8v88v8v88.MarkView.png
for size in 16 32 128 256; do
    sips -z $size $size "$SRC" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
done
for size in 16 32 128; do
    sips -z $((size * 2)) $((size * 2)) "$SRC" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$C/Resources/MarkView.icns"
rm -rf "$ICONSET"

cat > "$C/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>MarkView</string>
    <key>CFBundleExecutable</key>
    <string>markview</string>
    <key>CFBundleIconFile</key>
    <string>MarkView</string>
    <key>CFBundleIdentifier</key>
    <string>com.v8v88v8v88.MarkView</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>MarkView</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${SHORT_VERSION}</string>
    <key>CFBundleVersion</key>
    <string>${SHORT_VERSION}</string>
    <key>LSMinimumSystemVersion</key>
    <string>${MIN_MACOS}</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF
plutil -lint "$C/Info.plist"

# install_name_tool breaks the signatures.
find "$C/Frameworks" -name '*.dylib' -exec codesign --force --sign - {} \;
codesign --force --sign - "$APP"
codesign --verify --strict --verbose=2 "$APP"

mkdir -p dist/dmg
cp -R "$APP" dist/dmg/
ln -s /Applications dist/dmg/Applications
hdiutil create -volname MarkView -srcfolder dist/dmg -ov -format UDZO "dist/$NAME.dmg"
rm -rf dist/dmg
echo "Built dist/$NAME.dmg"
