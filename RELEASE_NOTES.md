MarkView 2.0 is a rewrite of the interface on the latest GNOME platform (GTK 4.24, libadwaita 1.10, GtkSourceView 5.22, WebKitGTK 2.54).

## Tabs and files

- Open several documents as tabs, with a tab overview to see them all at once
- The Open button keeps a searchable list of recently opened files
- Asks before closing a tab or window with unsaved changes
- Open files by dragging them in; links to other Markdown files open in a new tab and web links open in your browser
- Files are read and written asynchronously, and errors show up as notifications instead of failing silently

## Editing

- Markdown syntax highlighting in the editor
- Editor colour schemes now pick their light or dark version to match the system style, chosen from previews in Preferences
- With Readable Line Length on, the editor text is centred in wide windows
- The header bar hides while you type and comes back when you move the pointer
- New find and replace bar with a match count, case and regex options, and Enter / Shift+Enter to move between matches

## Preview

- Follows the system accent colour, document font and monospace font
- Styles GitHub alerts (`> [!NOTE]`, `> [!WARNING]`, …), footnotes, task lists, definition lists and sub/superscript
- PDF export is always light and readable, regardless of the app style, with wrapped code blocks and proper page margins

## Interface

- Split, editor and preview layouts in one button, with animated switching; the panes stack vertically on narrow windows
- Light, dark and follow-system style switcher in the main menu
- Save button with Save As and Export as PDF in its menu
- Rebuilt Preferences, with conflict detection when changing keyboard shortcuts
- About dialog lists the open source projects MarkView is built on
- Window size is remembered between sessions

## Shortcuts

- New Tab is Ctrl+T, New Window is Ctrl+N, Close Tab is Ctrl+W and Close Window is Ctrl+Shift+W
