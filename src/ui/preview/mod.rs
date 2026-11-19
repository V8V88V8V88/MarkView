use adw::StyleManager;
use gtk4::pango;
use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag, TagEnd};

use crate::config::*;
use crate::styles::{theme_variables, PreviewTheme, PREVIEW_CSS};
use crate::utils::html_escape;

// Each platform renders the preview with its own system web engine:
// WebKitGTK on Linux, WebView2 on Windows and WKWebView on macOS.
#[cfg(any(windows, target_os = "macos"))]
mod native;
#[cfg(not(any(windows, target_os = "macos")))]
mod webkit;

#[cfg(any(windows, target_os = "macos"))]
pub use native::PreviewView;
#[cfg(not(any(windows, target_os = "macos")))]
pub use webkit::PreviewView;

/// Name of the web engine behind the preview, for the About dialog.
pub const ENGINE: (&str, &str) = if cfg!(windows) {
    ("Microsoft Edge WebView2", "https://developer.microsoft.com/microsoft-edge/webview2/")
} else if cfg!(target_os = "macos") {
    ("WebKit", "https://developer.apple.com/documentation/webkit")
} else {
    ("WebKitGTK", "https://webkitgtk.org")
};

#[derive(Clone, Copy, PartialEq)]
enum Metadata {
    Show,
    Hide,
    Ignore,
}

fn metadata_mode() -> Metadata {
    match load_pref(PREF_SHOW_METADATA, DEFAULT_SHOW_METADATA).as_str() {
        "hide" => Metadata::Hide,
        "ignore" => Metadata::Ignore,
        _ => Metadata::Show,
    }
}

pub fn render_markdown(text: &str) -> String {
    let mode = metadata_mode();

    let mut options = Options::all();
    options.remove(Options::ENABLE_OLD_FOOTNOTES);
    options.insert(Options::ENABLE_FOOTNOTES);
    if mode == Metadata::Ignore {
        options.remove(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
        options.remove(Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS);
    }

    let mut in_metadata = false;
    let events = Parser::new_ext(text, options).filter_map(|event| match event {
        Event::Start(Tag::MetadataBlock(_)) => {
            in_metadata = true;
            (mode == Metadata::Show).then_some(Event::Html(CowStr::Borrowed("<div class=\"metadata\">")))
        }
        Event::End(TagEnd::MetadataBlock(_)) => {
            in_metadata = false;
            (mode == Metadata::Show).then_some(Event::Html(CowStr::Borrowed("</div>")))
        }
        Event::Text(t) if in_metadata => {
            (mode == Metadata::Show).then_some(Event::Html(html_escape(&t).into()))
        }
        _ if in_metadata => None,
        other => Some(other),
    });

    let mut out = String::with_capacity(text.len() * 3 / 2);
    html::push_html(&mut out, events);
    if out.trim().is_empty() {
        out = "<p class=\"placeholder\">Start typing Markdown in the editor\u{2026}</p>".into();
    }
    out
}

fn font_from_pango(name: &str) -> (String, f64) {
    let desc = pango::FontDescription::from_string(name);
    let family = desc
        .family()
        .map(|f| f.to_string())
        .filter(|f| !f.is_empty())
        .unwrap_or_else(|| "Adwaita Sans".into());
    let size = desc.size();
    let points = if size > 0 { size as f64 / pango::SCALE as f64 } else { 11.0 };
    let px = if desc.is_size_absolute() { points } else { points * 96.0 / 72.0 };
    (family, px)
}

pub fn current_theme() -> PreviewTheme {
    let sm = StyleManager::default();
    let accent = sm.accent_color_rgba();
    let (font_family, font_size_px) = font_from_pango(&sm.document_font_name());
    let (mono_family, _) = font_from_pango(&sm.monospace_font_name());
    PreviewTheme {
        dark: sm.is_dark(),
        accent: format!(
            "rgb({} {} {})",
            (accent.red() * 255.0).round(),
            (accent.green() * 255.0).round(),
            (accent.blue() * 255.0).round()
        ),
        font_family,
        font_size_px: font_size_px * 1.05,
        mono_family,
    }
}

pub fn build_page(body: &str, theme: &PreviewTheme) -> String {
    let readable_css = if pref_bool(PREF_READABLE_LINE, DEFAULT_READABLE_LINE == "true") {
        let max_width = load_pref(PREF_MAX_WIDTH, DEFAULT_MAX_WIDTH)
            .parse::<u32>()
            .unwrap_or(1000)
            .clamp(400, 3000);
        format!("body {{ max-width: {max_width}px; margin: 0 auto; }}")
    } else {
        String::new()
    };

    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\">\
         <meta name=\"color-scheme\" content=\"light dark\">\
         <style>{}{}{}</style></head><body>{}</body></html>",
        theme_variables(theme),
        PREVIEW_CSS,
        readable_css,
        body
    )
}
