MarkView 2.1.0-beta.6 is a test build of the Windows and macOS versions. Please report anything that doesn't work.

New in this build:

- Saving shows a short "Saved" toast, so you can see the file was written. Save As shows "Saved as" with the new name.

Downloads:

- **Windows**: extract the `x86_64-pc-windows-gnu.zip` and run `bin\markview.exe`. The preview uses Microsoft Edge WebView2, which comes with Windows 10 and 11.
- **macOS (Apple Silicon)**: open the `.dmg` and drag MarkView to Applications. The app isn't signed with an Apple Developer ID, so the first time macOS says it can't verify it: click **Done**, then go to **System Settings > Privacy & Security** and click **Open Anyway**.
- **Linux**: Flatpak or tarball. MarkView needs GTK 4.24, libadwaita 1.10 and GtkSourceView 5.22.

Known issues: on macOS the app closes after exporting to PDF (#2), and on Windows and macOS toasts can be hidden behind the preview (#4).
