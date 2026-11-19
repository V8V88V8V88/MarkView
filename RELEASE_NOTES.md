MarkView 2.0.2 fixes images in the Flatpak preview and changes the app ID.

- Images and other files next to your document (like `images/logo.png`) now show up in the preview. The Flatpak could only read `~/Documents` and `~/Downloads` before, so it now has access to your home folder.
- The app ID is now `com.v8v88v8v88.MarkView` (was `io.github.v8v88v8v88.MarkView`).

Because the app ID changed, this installs as a separate app instead of updating the old one. Remove the old version first:

```
flatpak uninstall io.github.v8v88v8v88.MarkView
```

Your MarkView preferences don't carry over to the new ID, so you'll need to set them again.
