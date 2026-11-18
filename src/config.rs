use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

pub const PREF_THEME: &str = "theme";
pub const PREF_SCHEME: &str = "color-scheme";
pub const PREF_SYNC_SCROLL: &str = "sync-scroll";
pub const PREF_DOM_INJECTION: &str = "dom-injection";
pub const PREF_DEBOUNCE: &str = "debounce-duration";
pub const PREF_SHOW_METADATA: &str = "metadata-mode";
pub const PREF_DEFAULT_VIEW: &str = "default-view";
pub const PREF_READABLE_LINE: &str = "readable-line-length";
pub const PREF_MAX_WIDTH: &str = "max-content-width";
pub const PREF_VIM_MODE: &str = "vim-mode";
pub const PREF_LINE_NUMBERS: &str = "line-numbers";
pub const PREF_WORD_WRAP: &str = "word-wrap";
pub const PREF_HIGHLIGHT_LINE: &str = "highlight-current-line";
pub const PREF_AUTOHIDE_HEADER: &str = "autohide-header";

pub const DEFAULT_THEME: &str = "default";
pub const DEFAULT_SCHEME: &str = "Adwaita";
pub const DEFAULT_SYNC_SCROLL: &str = "true";
pub const DEFAULT_DOM_INJECTION: &str = "true";
pub const DEFAULT_DEBOUNCE: &str = "150";
pub const DEFAULT_SHOW_METADATA: &str = "show";
pub const DEFAULT_VIEW: &str = "dual-pane";
pub const DEFAULT_READABLE_LINE: &str = "true";
pub const DEFAULT_MAX_WIDTH: &str = "1000";
pub const DEFAULT_VIM_MODE: &str = "false";
pub const DEFAULT_LINE_NUMBERS: &str = "true";
pub const DEFAULT_WORD_WRAP: &str = "true";
pub const DEFAULT_HIGHLIGHT_LINE: &str = "false";
pub const DEFAULT_AUTOHIDE_HEADER: &str = "true";

pub const PREF_WINDOW_WIDTH: &str = "window-width";
pub const PREF_WINDOW_HEIGHT: &str = "window-height";
pub const PREF_WINDOW_MAXIMIZED: &str = "window-maximized";

pub struct Shortcut {
    pub pref_key: &'static str,
    pub default: &'static str,
    pub action: &'static str,
    pub title: &'static str,
    pub section: &'static str,
}

pub const SHORTCUTS: &[Shortcut] = &[
    Shortcut { pref_key: "key-new-tab", default: "<Control>t", action: "win.new-tab", title: "New Tab", section: "Document" },
    Shortcut { pref_key: "key-new-window", default: "<Control>n", action: "app.new-window", title: "New Window", section: "Document" },
    Shortcut { pref_key: "key-open", default: "<Control>o", action: "win.open", title: "Open", section: "Document" },
    Shortcut { pref_key: "key-save", default: "<Control>s", action: "win.save", title: "Save", section: "Document" },
    Shortcut { pref_key: "key-save-as", default: "<Control><Shift>s", action: "win.save-as", title: "Save As", section: "Document" },
    Shortcut { pref_key: "key-export-pdf", default: "<Control>p", action: "win.export-pdf", title: "Export as PDF", section: "Document" },
    Shortcut { pref_key: "key-search", default: "<Control>f", action: "win.search", title: "Find and Replace", section: "Editing" },
    Shortcut { pref_key: "key-toggle-editor", default: "<Control>e", action: "win.toggle-editor", title: "Show or Hide Editor", section: "View" },
    Shortcut { pref_key: "key-cycle-view", default: "<Control>l", action: "win.cycle-view", title: "Switch Layout", section: "View" },
    Shortcut { pref_key: "key-toggle-readable", default: "<Control>r", action: "win.readable-line", title: "Readable Line Length", section: "View" },
    Shortcut { pref_key: "key-preferences", default: "<Control>comma", action: "app.preferences", title: "Preferences", section: "General" },
    Shortcut { pref_key: "key-shortcuts-help", default: "<Control>question", action: "app.shortcuts", title: "Keyboard Shortcuts", section: "General" },
    Shortcut { pref_key: "key-close-tab", default: "<Control>w", action: "win.close-tab", title: "Close Tab", section: "General" },
    Shortcut { pref_key: "key-close-window", default: "<Control><Shift>w", action: "window.close", title: "Close Window", section: "General" },
    Shortcut { pref_key: "key-quit", default: "<Control>q", action: "app.quit", title: "Quit", section: "General" },
];

pub fn pref_bool(key: &str, default: bool) -> bool {
    load_pref(key, if default { "true" } else { "false" }) == "true"
}

pub fn save_pref_bool(key: &str, value: bool) {
    save_pref(key, if value { "true" } else { "false" });
}

pub fn config_path() -> PathBuf {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    config.join("MarkView").join("preferences.ini")
}

thread_local! {
    static PREF_CACHE: RefCell<Option<HashMap<String, String>>> = const { RefCell::new(None) };
}

fn ensure_cache() {
    PREF_CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.is_some() {
            return;
        }
        let mut prefs = HashMap::new();
        let path = config_path();
        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            for line in content.lines() {
                let line = line.trim();
                if let Some((k, v)) = line.split_once('=') {
                    prefs.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }
        *c = Some(prefs);
    });
}

pub fn load_pref(key: &str, default: &str) -> String {
    ensure_cache();
    PREF_CACHE.with(|c| {
        c.borrow()
            .as_ref()
            .unwrap()
            .get(key)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    })
}

pub fn save_pref(key: &str, value: &str) {
    ensure_cache();
    PREF_CACHE.with(|c| {
        c.borrow_mut()
            .as_mut()
            .unwrap()
            .insert(key.to_string(), value.to_string());
    });

    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    PREF_CACHE.with(|c| {
        let c = c.borrow();
        if let Some(ref prefs) = *c {
            let sorted: BTreeMap<_, _> = prefs.iter().collect();
            let content = sorted
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("\n");
            let final_content = if content.is_empty() {
                content
            } else {
                format!("{}\n", content)
            };
            let _ = std::fs::write(&path, final_content);
        }
    });
}
