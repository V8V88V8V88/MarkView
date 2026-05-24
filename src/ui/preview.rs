use std::cell::RefCell;
use std::rc::Rc;
use adw::StyleManager;
use gtk4::gio;
use pulldown_cmark::{html, Options, Parser};
use sourceview5::Buffer as SourceBuffer;
use webkit6::prelude::*;
use webkit6::WebView;

use crate::config::*;
use crate::utils::*;

pub fn refresh_preview(
    webview: &WebView,
    buffer: &SourceBuffer,
    current_file: &Rc<RefCell<Option<gio::File>>>,
    is_first_render: &Rc<RefCell<bool>>,
    force_full_reload: bool,
) {
    let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
    
    let metadata_mode = load_pref(PREF_SHOW_METADATA, DEFAULT_SHOW_METADATA);
    let mut options = Options::all();
    
    if metadata_mode == "ignore" {
        options.remove(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
    }

    let parser = Parser::new_ext(&text, options);
    let mut html_out = String::new();
    
    if metadata_mode == "show" || metadata_mode == "hide" {
        let mut events = Vec::new();
        let mut in_metadata = false;
        for event in parser {
            match event {
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::MetadataBlock(_)) => {
                    in_metadata = true;
                    if metadata_mode == "show" {
                        events.push(pulldown_cmark::Event::Html("<div class=\"metadata\">".into()));
                    }
                }
                pulldown_cmark::Event::End(pulldown_cmark::TagEnd::MetadataBlock(_)) => {
                    in_metadata = false;
                    if metadata_mode == "show" {
                        events.push(pulldown_cmark::Event::Html("</div>".into()));
                    }
                }
                pulldown_cmark::Event::Text(t) => {
                    if in_metadata {
                        if metadata_mode == "show" {
                            events.push(pulldown_cmark::Event::Html(html_escape(&t).into()));
                        }
                    } else {
                        events.push(pulldown_cmark::Event::Text(t));
                    }
                }
                _ => {
                    if !in_metadata {
                        events.push(event);
                    }
                }
            }
        }
        html::push_html(&mut html_out, events.into_iter());
    } else {
        html::push_html(&mut html_out, parser);
    }

    let dom_injection = load_pref(PREF_DOM_INJECTION, DEFAULT_DOM_INJECTION) == "true";
    let dark = StyleManager::default().is_dark();
    let first = *is_first_render.borrow();

    let body = if html_out.is_empty() {
        "<p class='placeholder'>Start typing markdown on the left\u{2026}</p>".to_string()
    } else {
        html_out
    };

    if dom_injection && !first && !force_full_reload {
        let escaped_html = escape_javascript_string(&body);
        let js = format!("document.body.innerHTML = `{}`;", escaped_html);
        webview.evaluate_javascript(&js, None, None, None::<&gio::Cancellable>, |_| {});
    } else {
        let base_uri = base_uri_for_preview(current_file.borrow().as_ref());
        webview.load_html(&build_html_page(&body, dark), base_uri.as_deref());
        let (r, g, b) = if dark { (0.102, 0.102, 0.102) } else { (0.98, 0.98, 0.98) };
        webview.set_background_color(&gtk4::gdk::RGBA::new(r, g, b, 1.0));
        *is_first_render.borrow_mut() = false;
    }
}
