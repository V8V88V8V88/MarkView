MarkView 2.1.0-beta.1 is a test build of the first Windows and macOS versions. Please report anything that doesn't work.

- **Windows**: download the `x86_64-pc-windows-gnu.zip`, extract it and run `bin\markview.exe`. The preview uses Microsoft Edge WebView2, which comes with Windows 10 and 11.
- **macOS (Apple Silicon)**: install the libraries with `brew install gtk4 libadwaita gtksourceview5 adwaita-icon-theme`, then extract the `aarch64-apple-darwin.tar.gz` and run `markview`. The build isn't signed, so macOS may block it the first time. The preview uses the system WebKit, and shortcuts use Cmd instead of Ctrl.
- **Linux**: the preview still uses WebKitGTK. MarkView now needs GTK 4.24, libadwaita 1.10 and GtkSourceView 5.22.

Things to check on Windows and macOS: live preview updates while typing, scroll sync, images next to the document, links, PDF export, and that the preview stays lined up with its pane when you resize the window or open dialogs.

Known issues: app shortcuts don't work while the preview has keyboard focus, and the preview shows blank in the tab overview.
