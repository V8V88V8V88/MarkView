use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{gdk, gio, glib};
use webkit6::prelude::*;
use webkit6::{NavigationPolicyDecision, PolicyDecisionType, WebView};

type Done = Box<dyn FnOnce(Result<(), String>)>;

pub struct PreviewView {
    webview: WebView,
    page_uri: Rc<RefCell<String>>,
    print_op: Rc<RefCell<Option<webkit6::PrintOperation>>>,
}

impl PreviewView {
    pub fn new(on_link: impl Fn(String) + 'static, on_loaded: impl Fn() + 'static) -> Self {
        let webview = WebView::new();
        if let Some(settings) = WebViewExt::settings(&webview) {
            settings.set_enable_javascript_markup(false);
            settings.set_enable_developer_extras(false);
            settings.set_enable_back_forward_navigation_gestures(false);
        }

        webview.connect_load_changed(move |_, event| {
            if event == webkit6::LoadEvent::Finished {
                on_loaded();
            }
        });

        let page_uri = Rc::new(RefCell::new(String::new()));
        let page = page_uri.clone();
        webview.connect_decide_policy(move |_, decision, kind| {
            if kind != PolicyDecisionType::NavigationAction {
                return false;
            }
            let Some(uri) = decision
                .downcast_ref::<NavigationPolicyDecision>()
                .and_then(|d| d.navigation_action())
                .and_then(|a| a.request())
                .and_then(|r| r.uri())
            else {
                return false;
            };
            if uri == "about:blank" || uri.split('#').next() == Some(page.borrow().as_str()) {
                return false;
            }
            decision.ignore();
            on_link(uri.to_string());
            true
        });

        Self { webview, page_uri, print_op: Rc::default() }
    }

    pub fn widget(&self) -> gtk4::Widget {
        self.webview.clone().upcast()
    }

    pub fn load_page(&self, html: &str, base_uri: &str) {
        *self.page_uri.borrow_mut() = base_uri.to_string();
        self.webview.load_html(html, Some(base_uri));
    }

    pub fn set_body(&self, body: &str) {
        let args = glib::VariantDict::new(None);
        args.insert("html", body);
        self.webview.call_async_javascript_function(
            "document.body.innerHTML = html;",
            Some(&args.end()),
            None,
            None,
            None::<&gio::Cancellable>,
            |_| {},
        );
    }

    pub fn scroll_to(&self, fraction: f64) {
        let args = glib::VariantDict::new(None);
        args.insert("fraction", fraction);
        self.webview.call_async_javascript_function(
            "window.scrollTo(0, (document.documentElement.scrollHeight - window.innerHeight) * fraction);",
            Some(&args.end()),
            None,
            None,
            None::<&gio::Cancellable>,
            |_| {},
        );
    }

    pub fn suspender(&self) -> impl Fn(bool) + 'static {
        |_| {}
    }

    pub fn set_background(&self, color: &gdk::RGBA) {
        self.webview.set_background_color(color);
    }

    pub fn export_pdf(&self, file: &gio::File, done: impl FnOnce(Result<(), String>) + 'static) {
        let settings = gtk4::PrintSettings::new();
        settings.set(gtk4::PRINT_SETTINGS_OUTPUT_URI, Some(file.uri().as_str()));
        settings.set(gtk4::PRINT_SETTINGS_OUTPUT_FILE_FORMAT, Some("pdf"));
        settings.set_printer("Print to File");
        let page_setup = gtk4::PageSetup::new();
        page_setup.set_top_margin(15.0, gtk4::Unit::Mm);
        page_setup.set_bottom_margin(15.0, gtk4::Unit::Mm);
        page_setup.set_left_margin(18.0, gtk4::Unit::Mm);
        page_setup.set_right_margin(18.0, gtk4::Unit::Mm);

        let op = webkit6::PrintOperation::new(&self.webview);
        op.set_print_settings(&settings);
        op.set_page_setup(&page_setup);

        // "failed" comes before "finished".
        let done: Rc<RefCell<Option<Done>>> = Rc::new(RefCell::new(Some(Box::new(done))));
        let (slot, ops) = (done.clone(), self.print_op.clone());
        op.connect_failed(move |_, e| {
            ops.borrow_mut().take();
            if let Some(done) = slot.borrow_mut().take() {
                done(Err(e.message().to_string()));
            }
        });
        let ops = self.print_op.clone();
        op.connect_finished(move |_| {
            ops.borrow_mut().take();
            if let Some(done) = done.borrow_mut().take() {
                done(Ok(()));
            }
        });
        op.print();
        *self.print_op.borrow_mut() = Some(op);
    }
}
