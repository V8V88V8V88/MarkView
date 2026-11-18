<div align="center">
  <img src="images/markview-logo.png" alt="MarkView" width="150" />
  <h1>MarkView</h1>
  <p>A Markdown editor for GNOME with a live preview.</p>
</div>

![MarkView](screenshot.png)

Write Markdown on the left and see it rendered on the right as you type. Built with Rust, GTK4 and libadwaita.

## Features

- Live preview rendered with WebKit, updated in place as you type
- Tabs with a tab overview, and a searchable list of recently opened files
- Preview follows the system accent color, document font and light/dark style
- Editor, split and preview layouts with animated switching, stacking vertically on narrow windows
- GtkSourceView editor with Markdown highlighting, 27 color schemes that switch light/dark with the system, line numbers, wrapping and optional Vim keybindings
- Find and replace with match count, case and regex options
- GitHub-style alerts (`> [!NOTE]`, `> [!WARNING]`, …), footnotes, task lists, tables, definition lists and sub/superscript
- YAML front matter can be shown, hidden or rendered as text
- Asks before closing or opening over unsaved changes
- Open files by dragging them in; links open in your browser, links to other Markdown files open in MarkView
- Export to PDF
- Every keyboard shortcut can be changed in Preferences

## Install

Download the Linux x86_64 tarball from the [latest release](https://github.com/V8V88V8V88/MarkView/releases/latest), extract it and run `./markview`. It needs GTK 4.12+, libadwaita 1.8+, GtkSourceView 5.4+ and WebKitGTK 6.0 installed on your system.

## Build from source

You need Rust and the development packages for GTK 4.12+, libadwaita 1.8+, GtkSourceView 5.4+ and WebKitGTK 6.0.

```bash
# Fedora
sudo dnf install gtk4-devel libadwaita-devel gtksourceview5-devel webkitgtk6.0-devel

git clone https://github.com/V8V88V8V88/MarkView.git
cd MarkView
cargo run --release
```

## Keyboard shortcuts

| Action | Default |
|---|---|
| New tab | Ctrl+T |
| New window | Ctrl+N |
| Open | Ctrl+O |
| Save / Save As | Ctrl+S / Ctrl+Shift+S |
| Export as PDF | Ctrl+P |
| Find and replace | Ctrl+F |
| Show or hide editor | Ctrl+E |
| Switch layout | Ctrl+L |
| Readable line length | Ctrl+R |
| Preferences | Ctrl+, |
| Keyboard shortcuts | Ctrl+? |
| Close tab / window | Ctrl+W / Ctrl+Shift+W |
| Quit | Ctrl+Q |

## License

GPL-2.0. See [LICENSE](LICENSE).
