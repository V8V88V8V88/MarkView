#!/usr/bin/env bash
# Build MarkView and bundle it with its GTK runtime into a self-contained
# folder and zip. Run from the repository root in an MSYS2 UCRT64 shell with:
#
#   pacman -S mingw-w64-ucrt-x86_64-{gtk4,libadwaita,gtksourceview5,pkgconf,gcc,rust,ntldd} zip
#
# Layout (GLib looks for data in the "share" folder next to an exe's "bin"):
#   markview/bin/markview.exe + DLLs, WebView2Loader.dll
#   markview/lib/gdk-pixbuf-2.0/   image loaders
#   markview/share/                icons, schemas, GtkSourceView data, MarkView styles
set -euo pipefail

VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
NAME="markview-${VERSION}-x86_64-pc-windows-gnu"
PREFIX="${MINGW_PREFIX:?run this from an MSYS2 UCRT64 shell}"
OUT="dist/$NAME"

cargo build --release --locked

rm -rf "$OUT"
mkdir -p "$OUT/bin" "$OUT/lib" "$OUT/share/glib-2.0" "$OUT/share/icons" "$OUT/share/markview"

cp target/release/gtk-markdown-viewer.exe "$OUT/bin/markview.exe"

# On the GNU target webview2-com links WebView2Loader.dll dynamically; its
# build script copies the DLL into "$OUT_DIR/x64". Where OUT_DIR sits depends
# on the Cargo version's build-dir layout, so search the whole target folder.
# The build script copies it from the crate's own source folder, so fall back
# to that.
loader=$(find target -path '*webview2-com-sys*' -path '*/out/x64/WebView2Loader.dll' | head -1)
if [ -z "$loader" ]; then
    ver=$(grep -A1 '^name = "webview2-com-sys"$' Cargo.lock | sed -n 's/^version = "\(.*\)"/\1/p')
    loader=$(find "${CARGO_HOME:-$HOME/.cargo}/registry/src" -path "*/webview2-com-sys-${ver}/x64/WebView2Loader.dll" 2>/dev/null | head -1)
fi
if [ -z "$loader" ]; then
    echo "WebView2Loader.dll not found under target/" >&2
    exit 1
fi
cp "$loader" "$OUT/bin/"

cp -r "$PREFIX/lib/gdk-pixbuf-2.0" "$OUT/lib/"

# Every DLL the program and the pixbuf loaders need from the MSYS2 prefix.
copy_deps() {
    ntldd -R "$@" | tr -d '\r' | awk '{print $3}' | grep -i '^[a-z]:.*\\ucrt64\\bin\\' | sort -u |
        while read -r dll; do cp -n "$(cygpath -u "$dll")" "$OUT/bin/"; done
}
copy_deps "$OUT/bin/markview.exe"
copy_deps "$OUT"/lib/gdk-pixbuf-2.0/2.10.0/loaders/*.dll

cp -r "$PREFIX/share/icons/Adwaita" "$PREFIX/share/icons/hicolor" "$OUT/share/icons/"
install -Dm644 data/icons/hicolor/256x256/apps/com.v8v88v8v88.MarkView.png \
    "$OUT/share/icons/hicolor/256x256/apps/com.v8v88v8v88.MarkView.png"
gtk4-update-icon-cache -f -t "$OUT/share/icons/hicolor" || true

cp -r "$PREFIX/share/glib-2.0/schemas" "$OUT/share/glib-2.0/"
glib-compile-schemas "$OUT/share/glib-2.0/schemas"
cp -r "$PREFIX/share/gtksourceview-5" "$OUT/share/"
cp -r data/styles "$OUT/share/markview/"

cp LICENSE README.md "$OUT/"
(cd dist && rm -f "$NAME.zip" && zip -qr "$NAME.zip" "$NAME")
echo "Built dist/$NAME.zip"
