MarkView 2.1.0-beta.5 is a test build of the Windows and macOS versions. Please report anything that doesn't work.

New in this build:

- Formatting toolbar above the editor: bold, italic, strikethrough, headings, bulleted, numbered and task lists, quotes, code, links, images, tables and horizontal lines. It can be turned off in Preferences. Shortcuts: Ctrl+B, Ctrl+I, Ctrl+Shift+X, Ctrl+Shift+C, Ctrl+K and Ctrl+1–3 (Cmd on macOS), all changeable in Preferences.
- Pressing Enter continues lists, quotes and tables; Enter on an empty item ends them. The table button adds a row when the cursor is in a table.
- The preview uses libadwaita's own colors.
- New Tab and Open are one button; its menu has recent files and Browse Files.
- The layout button switches layouts with one click.
- The tab overview button only shows with more than one tab.
- Line numbers are off by default.
- macOS: the preview no longer shows up shifted to the right, the divider between editor and preview can be dragged, and switching layouts no longer stutters.

Downloads:

- **Windows**: extract the `x86_64-pc-windows-gnu.zip` and run `bin\markview.exe`. The preview uses Microsoft Edge WebView2, which comes with Windows 10 and 11.
- **macOS (Apple Silicon)**: open the `.dmg` and drag MarkView to Applications. The app isn't signed with an Apple Developer ID, so the first time macOS says it can't verify it: click **Done**, then go to **System Settings > Privacy & Security** and click **Open Anyway**.
- **Linux**: Flatpak or tarball. MarkView needs GTK 4.24, libadwaita 1.10 and GtkSourceView 5.22.

Known issue: on macOS the app closes after exporting to PDF (#2).
