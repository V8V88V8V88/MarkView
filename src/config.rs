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

pub const DEFAULT_THEME: &str = "default";
pub const DEFAULT_SCHEME: &str = "Adwaita-dark";
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

// Keybindings
pub const PREF_KEY_OPEN: &str = "key-open";
pub const PREF_KEY_SAVE: &str = "key-save";
pub const PREF_KEY_SAVE_AS: &str = "key-save-as";
pub const PREF_KEY_EXPORT_PDF: &str = "key-export-pdf";
pub const PREF_KEY_PREFS: &str = "key-preferences";
pub const PREF_KEY_SHORTCUTS: &str = "key-shortcuts-help";
pub const PREF_KEY_SEARCH: &str = "key-search";
pub const PREF_KEY_TOGGLE_EDITOR: &str = "key-toggle-editor";
pub const PREF_KEY_CYCLE_VIEW: &str = "key-cycle-view";
pub const PREF_KEY_TOGGLE_READABLE: &str = "key-toggle-readable";
pub const PREF_KEY_QUIT: &str = "key-quit";

pub const DEFAULT_KEY_OPEN: &str = "<Control>o";
pub const DEFAULT_KEY_SAVE: &str = "<Control>s";
pub const DEFAULT_KEY_SAVE_AS: &str = "<Control><Shift>s";
pub const DEFAULT_KEY_EXPORT_PDF: &str = "<Control>p";
pub const DEFAULT_KEY_PREFS: &str = "<Control>comma";
pub const DEFAULT_KEY_SHORTCUTS: &str = "<Control>question";
pub const DEFAULT_KEY_SEARCH: &str = "<Control>f";
pub const DEFAULT_KEY_TOGGLE_EDITOR: &str = "<Control>e";
pub const DEFAULT_KEY_CYCLE_VIEW: &str = "<Control>l";
pub const DEFAULT_KEY_TOGGLE_READABLE: &str = "<Control>r";
pub const DEFAULT_KEY_QUIT: &str = "<Control>q";

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
