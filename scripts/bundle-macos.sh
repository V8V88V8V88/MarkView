#!/usr/bin/env bash
# Build MarkView.app with its GTK runtime inside, and pack it into a .dmg.
# Run from the repository root on an Apple Silicon Mac with:
#
#   brew install pkgconf gtk4 libadwaita gtksourceview5 adwaita-icon-theme dylibbundler
#
# Layout:
#   MarkView.app/Contents/MacOS/markview          the program
#   MarkView.app/Contents/Frameworks/             Homebrew dylibs it links to
#   MarkView.app/Contents/Resources/share/        icons, schemas, GtkSourceView data
#                                                 (found through XDG_DATA_DIRS, see main.rs)
#   MarkView.app/Contents/Resources/data/         MarkView's own styles and icon
#
# The app is ad-hoc signed only, not signed with a Developer ID or notarized,
# so Gatekeeper asks the user to allow it the first time.
set -euo pipefail

VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
# CFBundleShortVersionString must be numbers and dots only.
SHORT_VERSION=${VERSION%%-*}
NAME="markview-v${VERSION}-aarch64-apple-darwin"
BREW=$(brew --prefix)
# Homebrew bottles are built for the macOS version they run on, so the bundle
# needs at least the macOS it was built on.
MIN_MACOS=$(sw_vers -productVersion | cut -d. -f1).0
APP="dist/MarkView.app"
C="$APP/Contents"

cargo build --release --locked

rm -rf dist/MarkView.app "dist/$NAME.dmg" dist/dmg
mkdir -p "$C/MacOS" "$C/Frameworks" "$C/Resources/share/icons" "$C/Resources/share/glib-2.0"

cp target/release/gtk-markdown-viewer "$C/MacOS/markview"

# Copy every Homebrew dylib the program needs and point it at the copies.
dylibbundler -od -b -x "$C/MacOS/markview" -d "$C/Frameworks" \
    -p @executable_path/../Frameworks/ -s "$BREW/lib" </dev/null

# Homebrew's share folders are symlinks into the Cellar, hence -L.
cp -RL "$BREW/share/icons/Adwaita" "$BREW/share/icons/hicolor" "$C/Resources/share/icons/"
cp -RL "$BREW/share/glib-2.0/schemas" "$C/Resources/share/glib-2.0/"
glib-compile-schemas "$C/Resources/share/glib-2.0/schemas"
cp -RL "$BREW/share/gtksourceview-5" "$C/Resources/share/"
gtk4-update-icon-cache -f -t "$C/Resources/share/icons/hicolor" || true

mkdir -p "$C/Resources/data"
cp -R data/styles data/icons "$C/Resources/data/"

# App icon: macOS wants an .icns built from a set of PNG sizes.
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

# install_name_tool invalidates the signatures, and Apple Silicon refuses to
# run unsigned code, so ad-hoc sign every dylib and then the app itself.
find "$C/Frameworks" -name '*.dylib' -exec codesign --force --sign - {} \;
codesign --force --sign - "$APP"
codesign --verify --strict --verbose=2 "$APP"

# Disk image with the app and a shortcut to /Applications to drag it onto.
mkdir -p dist/dmg
cp -R "$APP" dist/dmg/
ln -s /Applications dist/dmg/Applications
hdiutil create -volname MarkView -srcfolder dist/dmg -ov -format UDZO "dist/$NAME.dmg"
rm -rf dist/dmg
echo "Built dist/$NAME.dmg"
