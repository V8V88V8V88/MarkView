# MarkView (Enhanced)

This repository contains an enhanced version of the [original MarkVue project](https://github.com/V8V88V8V88/MarkVue). While maintaining the core goal of a sleek GTK-based Markdown viewer, this version introduces significant architectural improvements, professional features, and a modern UI/UX.

## 🚀 Key Enhancements

Following features and improvements have been implemented over the original project:

### 🎨 Modernized UI/UX
- **Hideable Side Panel**: Added the ability to toggle the editor pane for focused reading.

### ⚡ Performance & Rendering
- **Hybrid Rendering (DOM Injection)**: Replaced full-page reloads with JavaScript-based DOM injection. The preview now updates instantly as you type without flickering or losing scroll position.
- **Sync Scroll**: Implemented scroll synchronization, ensuring the preview follows the editor's position accurately.

### 🛠️ Advanced Editor Features
- **Advanced Search & Replace**: Implemented a non-intrusive search and replace functionality with support for Regular Expressions (Regex).

### 📄 Workflow & Export
- **Metadata Handling**: Support for front-matter blocks, with user settings to show, hide, or ignore metadata during rendering.
- **Customizable Shortcuts**: Implemented a comprehensive keyboard shortcut system where common actions can be remapped via the preferences.

## 📚 Documentation
For the original project description, installation basics, and core dependencies, please refer to:
- The [Original Repository](https://github.com/V8V88V8V88/MarkVue)