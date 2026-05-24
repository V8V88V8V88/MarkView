use crate::config::*;
use crate::styles::*;
use gtk4::gio;
use gio::prelude::*;

pub fn build_html_page(body: &str, dark: bool) -> String {
    let css = if dark { PREVIEW_CSS_DARK } else { PREVIEW_CSS_LIGHT };
    
    let is_readable = load_pref(PREF_READABLE_LINE, DEFAULT_READABLE_LINE) == "true";
    let max_width = load_pref(PREF_MAX_WIDTH, DEFAULT_MAX_WIDTH)
        .parse::<u32>()
        .unwrap_or(1000)
        .clamp(400, 3000);

    let readable_css = if is_readable {
        format!("body {{ max-width: {}px; margin-left: auto !important; margin-right: auto !important; }}", max_width)
    } else {
        String::new()
    };

    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><style>{} {} {}</style></head><body>{}</body></html>",
        css, PRINT_CSS, readable_css, body
    )
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn escape_javascript_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('$', "\\$")
}

pub fn base_uri_for_preview(current_file: Option<&gio::File>) -> Option<String> {
    if let Some(file) = current_file
        && let Some(parent) = file.parent()
    {
        let mut uri = parent.uri().to_string();
        if !uri.ends_with('/') {
            uri.push('/');
        }
        return Some(uri);
    }
    std::env::current_dir()
        .ok()
        .and_then(|path| path.canonicalize().ok())
        .map(|path| format!("file://{}/", path.to_string_lossy()))
}
