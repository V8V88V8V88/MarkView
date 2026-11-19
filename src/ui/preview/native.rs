//! Preview backed by the system web engine on Windows (WebView2) and macOS
//! (WKWebView), driven through wry.
//!
//! GTK 4 draws a whole window into a single native surface, so the web view
//! cannot be a GTK widget. Instead an empty GTK widget holds the place in the
//! layout and the web view is kept on top of that widget's area:
//!
//! - Windows: as a child HWND of the GTK window.
//! - macOS: in a borderless child NSWindow attached to the GTK window. GDK's
//!   content view is layer-hosting (it sets its own layer before
//!   `setWantsLayer:`), and AppKit does not support subviews in such a view,
//!   so the web view cannot simply be added to it.
//!
//! The web view draws over everything GTK renders in the window, so it is
//! hidden whenever GTK shows something in-window on top of the preview: an
//! AdwDialog or the tab overview.

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};

use adw::prelude::*;
use gtk4::{gdk, gio, glib};
use raw_window_handle::{HandleError, HasWindowHandle, RawWindowHandle, WindowHandle};
use wry::http::{header, Request, Response, StatusCode};
use wry::{dpi, NewWindowResponse, PageLoadEvent, Rect, WebView, WebViewBuilder};

/// Custom URI scheme the preview page and the files it references are served
/// from. A page loaded from a string has no base URL, so relative image and
/// link paths in the Markdown would not resolve without it.
const SCHEME: &str = "markview";

/// WebView2 cannot register real custom schemes, so wry serves them from
/// `http://<scheme>.localhost`. Unlike the initial URL, URLs passed to
/// `WebView::load_url` are not rewritten by wry, so the page URL is built in
/// the form the engine actually sees.
#[cfg(windows)]
const ORIGIN: &str = "http://markview.localhost";
#[cfg(target_os = "macos")]
const ORIGIN: &str = "markview://localhost";

/// The page lives at this name inside the document's folder, so relative
/// paths resolve against that folder.
const PAGE_NAME: &str = ".markview-preview.html";

/// Raw HTML in a Markdown file must not run scripts: they could read local
/// files through the scheme handler. Scripts evaluated by the app itself are
/// not subject to the page's CSP.
const CSP: &str = "<meta http-equiv=\"Content-Security-Policy\" \
                   content=\"script-src 'none'; object-src 'none'; base-uri 'none'\">";

type Done = Box<dyn FnOnce(Result<(), String>)>;

pub struct PreviewView {
    inner: Rc<Inner>,
}

struct Inner {
    host: gtk4::Box,
    webview: RefCell<Option<WebView>>,
    /// The full page, served to the engine by the scheme handler.
    html: Arc<Mutex<String>>,
    page_url: RefCell<String>,
    background: Cell<wry::RGBA>,
    bounds: Cell<Option<(i32, i32, u32, u32)>>,
    shown: Cell<bool>,
    handlers: RefCell<Vec<(glib::Object, glib::SignalHandlerId)>>,
    on_link: Box<dyn Fn(String)>,
    on_loaded: Box<dyn Fn()>,
    #[cfg(windows)]
    focus_gesture: RefCell<Option<(gtk4::Widget, gtk4::GestureClick)>>,
    #[cfg(target_os = "macos")]
    overlay: RefCell<Option<mac::Overlay>>,
    #[cfg(target_os = "macos")]
    print_delegate: RefCell<Option<objc2::rc::Retained<mac::PrintDelegate>>>,
}

impl PreviewView {
    /// `on_link` gets every URI the page tries to navigate away to;
    /// `on_loaded` runs whenever a full page load finishes.
    pub fn new(on_link: impl Fn(String) + 'static, on_loaded: impl Fn() + 'static) -> Self {
        let host = gtk4::Box::builder().hexpand(true).vexpand(true).build();
        let inner = Rc::new(Inner {
            host: host.clone(),
            webview: RefCell::new(None),
            html: Arc::new(Mutex::new(String::new())),
            page_url: RefCell::new(String::new()),
            background: Cell::new((255, 255, 255, 255)),
            bounds: Cell::new(None),
            shown: Cell::new(false),
            handlers: RefCell::new(Vec::new()),
            on_link: Box::new(on_link),
            on_loaded: Box::new(on_loaded),
            #[cfg(windows)]
            focus_gesture: RefCell::new(None),
            #[cfg(target_os = "macos")]
            overlay: RefCell::new(None),
            #[cfg(target_os = "macos")]
            print_delegate: RefCell::new(None),
        });

        // The web view needs the window's native surface, which exists only
        // while the widget is realized. Moving a tab to another window
        // unrealizes and realizes it again, which recreates the web view in
        // the new window.
        let weak = Rc::downgrade(&inner);
        host.connect_realize(move |_| {
            let weak = weak.clone();
            // Creating a WebView2 controller runs a nested message loop, so
            // keep it out of GTK's realize.
            glib::idle_add_local_once(move || {
                if let Some(inner) = weak.upgrade() {
                    inner.attach();
                }
            });
        });
        let weak = Rc::downgrade(&inner);
        host.connect_unrealize(move |_| {
            if let Some(inner) = weak.upgrade() {
                inner.detach();
            }
        });
        let weak = Rc::downgrade(&inner);
        host.connect_map(move |_| {
            if let Some(inner) = weak.upgrade() {
                inner.update_geometry();
            }
        });
        let weak = Rc::downgrade(&inner);
        host.connect_unmap(move |_| {
            if let Some(inner) = weak.upgrade() {
                inner.update_geometry();
            }
        });

        Self { inner }
    }

    pub fn widget(&self) -> gtk4::Widget {
        self.inner.host.clone().upcast()
    }

    pub fn load_page(&self, html: &str, base_uri: &str) {
        let html = html.replacen("<head>", &format!("<head>{CSP}"), 1);
        *self.inner.html.lock().unwrap() = html;
        let url = page_url(base_uri);
        *self.inner.page_url.borrow_mut() = url.clone();
        if let Some(webview) = self.inner.webview.borrow().as_ref() {
            let _ = webview.load_url(&url);
        }
    }

    pub fn set_body(&self, body: &str) {
        // Keep the served page current, so a reload or a recreated web view
        // shows the latest text.
        {
            let mut html = self.inner.html.lock().unwrap();
            if let Some(start) = html.find("<body>").map(|i| i + "<body>".len()) {
                html.truncate(start);
                html.push_str(body);
                html.push_str("</body></html>");
            }
        }
        let Ok(literal) = serde_json::to_string(body) else { return };
        self.inner.eval(&format!("document.body.innerHTML = {literal};"));
    }

    pub fn scroll_to(&self, fraction: f64) {
        if fraction.is_finite() {
            self.inner.eval(&format!(
                "window.scrollTo(0, (document.documentElement.scrollHeight - window.innerHeight) * {fraction});"
            ));
        }
    }

    pub fn set_background(&self, color: &gdk::RGBA) {
        let channel = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        let rgba = (channel(color.red()), channel(color.green()), channel(color.blue()), channel(color.alpha()));
        self.inner.background.set(rgba);
        if let Some(webview) = self.inner.webview.borrow().as_ref() {
            let _ = webview.set_background_color(rgba);
        }
    }

    pub fn export_pdf(&self, file: &gio::File, done: impl FnOnce(Result<(), String>) + 'static) {
        let Some(path) = file.path() else {
            done(Err("PDF export needs a local file".into()));
            return;
        };
        let webview = self.inner.webview.borrow();
        let Some(webview) = webview.as_ref() else {
            done(Err("The preview is not ready yet".into()));
            return;
        };
        #[cfg(windows)]
        let result = win::export_pdf(webview, &path, Box::new(done));
        #[cfg(target_os = "macos")]
        let result = mac::export_pdf(&self.inner, webview, &path, Box::new(done));
        if let Err((message, done)) = result {
            done(Err(message));
        }
    }
}

impl Drop for PreviewView {
    fn drop(&mut self) {
        self.inner.detach();
    }
}

impl Inner {
    fn eval(&self, js: &str) {
        if let Some(webview) = self.webview.borrow().as_ref() {
            let _ = webview.evaluate_script(js);
        }
    }

    fn attach(self: &Rc<Self>) {
        if self.webview.borrow().is_some() || !self.host.is_realized() {
            return;
        }
        #[cfg(windows)]
        let Some(parent) = ParentHandle::for_widget(&self.host) else { return };
        #[cfg(target_os = "macos")]
        let Some(overlay) = mac::Overlay::for_widget(&self.host) else { return };
        #[cfg(target_os = "macos")]
        let parent = overlay.handle();

        let html = self.html.clone();
        let weak = Rc::downgrade(self);
        let on_nav = weak.clone();
        let on_load = weak.clone();
        let on_new_window = weak.clone();

        let url = {
            let url = self.page_url.borrow();
            if url.is_empty() { "about:blank".to_string() } else { url.clone() }
        };
        let bounds = self.bounds_rect().unwrap_or(Rect {
            position: dpi::LogicalPosition::new(0, 0).into(),
            size: dpi::LogicalSize::new(1, 1).into(),
        });

        #[cfg(windows)]
        let mut context = wry::WebContext::new(Some(glib::user_data_dir().join("MarkView").join("WebView2")));
        #[cfg(windows)]
        let builder = WebViewBuilder::new_with_web_context(&mut context);
        #[cfg(not(windows))]
        let builder = WebViewBuilder::new();

        let builder = builder
            .with_url(url)
            .with_bounds(bounds)
            .with_visible(false)
            .with_focused(false)
            .with_background_color(self.background.get())
            .with_accept_first_mouse(true)
            .with_custom_protocol(SCHEME.to_string(), move |_, request| serve(&html, &request))
            .with_navigation_handler(move |url| {
                let Some(inner) = on_nav.upgrade() else { return false };
                if url == "about:blank" || same_page(&url, &inner.page_url.borrow()) {
                    return true;
                }
                // History navigation (mouse back button, context menu) to an
                // earlier preview page: nothing to follow.
                if local_path(&url).is_some_and(|p| p.ends_with(PAGE_NAME)) {
                    return false;
                }
                inner.follow_link(url);
                false
            })
            .with_new_window_req_handler(move |url, _| {
                if let Some(inner) = on_new_window.upgrade() {
                    inner.follow_link(url);
                }
                NewWindowResponse::Deny
            })
            .with_on_page_load_handler(move |event, _| {
                if matches!(event, PageLoadEvent::Finished) {
                    let weak = on_load.clone();
                    glib::idle_add_local_once(move || {
                        if let Some(inner) = weak.upgrade() {
                            (inner.on_loaded)();
                        }
                    });
                }
            });

        #[cfg(windows)]
        let builder = {
            use wry::WebViewBuilderExtWindows;
            // Keep Edge's own shortcuts (print, find, reload…) out of the preview.
            builder.with_browser_accelerator_keys(false)
        };

        // On macOS the web view fills the overlay window and resizes with it.
        #[cfg(windows)]
        let built = builder.build_as_child(&parent);
        #[cfg(target_os = "macos")]
        let built = builder.build(&parent);

        match built {
            Ok(webview) => {
                #[cfg(target_os = "macos")]
                {
                    let _ = webview.set_visible(true);
                    *self.overlay.borrow_mut() = Some(overlay);
                }
                *self.webview.borrow_mut() = Some(webview);
                #[cfg(windows)]
                self.return_focus_on_click();
                self.connect_occluders();
                self.bounds.set(None);
                self.shown.set(false);
                self.update_geometry();
            }
            Err(e) => glib::g_warning!("markview", "Could not create the preview: {e}"),
        }
    }

    /// Win32 keyboard focus stays in the WebView2 child window after it was
    /// clicked, and GDK does not take it back when GTK's part of the window
    /// is clicked. GTK only sees presses outside the web view, so any press
    /// it sees hands focus back to the GTK window.
    #[cfg(windows)]
    fn return_focus_on_click(self: &Rc<Self>) {
        let Some(root) = self.host.root() else { return };
        let gesture = gtk4::GestureClick::builder()
            .button(0)
            .propagation_phase(gtk4::PropagationPhase::Capture)
            .build();
        let weak = Rc::downgrade(self);
        gesture.connect_pressed(move |_, _, _, _| {
            if let Some(inner) = weak.upgrade()
                && let Some(webview) = inner.webview.borrow().as_ref()
            {
                let _ = webview.focus_parent();
            }
        });
        let root: gtk4::Widget = root.upcast();
        root.add_controller(gesture.clone());
        *self.focus_gesture.borrow_mut() = Some((root, gesture));
    }

    fn detach(&self) {
        #[cfg(windows)]
        if let Some((root, gesture)) = self.focus_gesture.borrow_mut().take() {
            root.remove_controller(&gesture);
        }
        for (object, id) in self.handlers.borrow_mut().drain(..) {
            object.disconnect(id);
        }
        self.webview.borrow_mut().take();
        #[cfg(target_os = "macos")]
        self.overlay.borrow_mut().take();
        self.bounds.set(None);
        self.shown.set(false);
    }

    /// Track everything that moves the placeholder or covers it.
    fn connect_occluders(self: &Rc<Self>) {
        let mut handlers = self.handlers.borrow_mut();

        // Any change to the window's layout happens in the frame clock's
        // layout phase, after GTK has allocated the widgets.
        if let Some(clock) = self.host.frame_clock() {
            let weak = Rc::downgrade(self);
            let id = clock.connect_layout(move |_| {
                if let Some(inner) = weak.upgrade() {
                    inner.update_geometry();
                }
            });
            handlers.push((clock.upcast(), id));
        }
        if let Some(window) = self.host.root().and_downcast::<adw::ApplicationWindow>() {
            let weak = Rc::downgrade(self);
            let id = window.connect_visible_dialog_notify(move |_| {
                if let Some(inner) = weak.upgrade() {
                    inner.update_geometry();
                }
            });
            handlers.push((window.upcast(), id));
        }
        if let Some(overview) = self.host.ancestor(adw::TabOverview::static_type()).and_downcast::<adw::TabOverview>() {
            let weak = Rc::downgrade(self);
            let id = overview.connect_open_notify(move |_| {
                if let Some(inner) = weak.upgrade() {
                    inner.update_geometry();
                }
            });
            handlers.push((overview.upcast(), id));
        }
    }

    fn occluded(&self) -> bool {
        let dialog = self
            .host
            .root()
            .and_downcast::<adw::ApplicationWindow>()
            .is_some_and(|w| w.visible_dialog().is_some());
        let overview = self
            .host
            .ancestor(adw::TabOverview::static_type())
            .and_downcast::<adw::TabOverview>()
            .is_some_and(|o| o.is_open());
        dialog || overview
    }

    /// The placeholder's area in the coordinates of the window's native
    /// surface: physical pixels on Windows, points on macOS.
    fn surface_bounds(&self) -> Option<(i32, i32, u32, u32)> {
        let native = self.host.native()?;
        let point = self.host.compute_point(&native, &gtk4::graphene::Point::zero())?;
        // The surface can be larger than the window, e.g. for CSD shadows.
        let (dx, dy) = native.surface_transform();
        let (x, y) = (point.x() as f64 + dx, point.y() as f64 + dy);
        let (w, h) = (self.host.width() as f64, self.host.height() as f64);
        if w <= 0.0 || h <= 0.0 {
            return None;
        }
        // GTK lays out in its own units; on Windows convert with the scale
        // GTK itself uses for the surface, so a DPI mismatch between GTK and
        // the HWND cannot misplace the view.
        #[cfg(windows)]
        let scale = native.surface()?.scale();
        #[cfg(not(windows))]
        let scale = 1.0;
        let x0 = (x * scale).round();
        let y0 = (y * scale).round();
        let x1 = ((x + w) * scale).round();
        let y1 = ((y + h) * scale).round();
        Some((x0 as i32, y0 as i32, (x1 - x0) as u32, (y1 - y0) as u32))
    }

    fn bounds_rect(&self) -> Option<Rect> {
        let (x, y, w, h) = self.surface_bounds()?;
        Some(to_rect(x, y, w, h))
    }

    fn update_geometry(&self) {
        let webview = self.webview.borrow();
        let Some(webview) = webview.as_ref() else { return };
        let bounds = if self.host.is_mapped() && !self.occluded() { self.surface_bounds() } else { None };
        // On macOS the overlay window is moved and shown instead of the view.
        #[cfg(target_os = "macos")]
        let (overlay, _) = (self.overlay.borrow(), webview);
        if let Some(bounds) = bounds
            && self.bounds.get() != Some(bounds)
        {
            let (x, y, w, h) = bounds;
            #[cfg(windows)]
            let _ = webview.set_bounds(to_rect(x, y, w, h));
            #[cfg(target_os = "macos")]
            if let Some(overlay) = overlay.as_ref() {
                overlay.place(x, y, w, h);
            }
            self.bounds.set(Some(bounds));
        }
        let show = bounds.is_some();
        if show != self.shown.get() {
            #[cfg(windows)]
            let _ = webview.set_visible(show);
            #[cfg(target_os = "macos")]
            if let Some(overlay) = overlay.as_ref() {
                overlay.set_shown(show);
            }
            self.shown.set(show);
        }
    }

    /// Hand a navigation away from the preview to the document, outside the
    /// engine's callback.
    fn follow_link(self: &Rc<Self>, url: String) {
        let target = local_file_uri(&url).unwrap_or(url);
        let weak: Weak<Self> = Rc::downgrade(self);
        glib::idle_add_local_once(move || {
            if let Some(inner) = weak.upgrade() {
                (inner.on_link)(target);
            }
        });
    }
}

fn to_rect(x: i32, y: i32, w: u32, h: u32) -> Rect {
    #[cfg(windows)]
    return Rect {
        position: dpi::PhysicalPosition::new(x, y).into(),
        size: dpi::PhysicalSize::new(w, h).into(),
    };
    #[cfg(not(windows))]
    return Rect {
        position: dpi::LogicalPosition::new(x, y).into(),
        size: dpi::LogicalSize::new(w, h).into(),
    };
}

/// URL of the preview page for a document whose folder is `base_uri`.
fn page_url(base_uri: &str) -> String {
    match base_uri.strip_prefix("file://") {
        Some(path) => {
            let path = if path.ends_with('/') { path.to_string() } else { format!("{path}/") };
            format!("{ORIGIN}{path}{PAGE_NAME}")
        }
        None => format!("{ORIGIN}/{PAGE_NAME}"),
    }
}

/// Map a URL served by the scheme handler back to the `file://` URI it
/// stands for.
fn local_file_uri(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix(ORIGIN)
        .or_else(|| url.strip_prefix("markview://localhost"))?;
    let path = rest.split(['?', '#']).next().unwrap_or(rest);
    Some(format!("file://{path}"))
}

fn local_path(url: &str) -> Option<PathBuf> {
    gio::File::for_uri(&local_file_uri(url)?).path()
}

fn same_page(url: &str, page: &str) -> bool {
    match (local_path(url), local_path(page)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

fn serve(html: &Mutex<String>, request: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let path = request.uri().path();
    let response = Response::builder().header(header::CACHE_CONTROL, "no-store");
    if path.ends_with(PAGE_NAME) {
        let page = html.lock().unwrap().clone();
        return response
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Cow::Owned(page.into_bytes()))
            .unwrap();
    }
    let file = gio::File::for_uri(&format!("file://{path}")).path();
    match file.as_ref().map(std::fs::read) {
        Some(Ok(bytes)) => {
            let (content_type, _) = gio::content_type_guess(file.as_deref(), bytes.as_slice());
            let mime = gio::content_type_get_mime_type(&content_type)
                .map(|m| m.to_string())
                .unwrap_or_else(|| "application/octet-stream".into());
            response
                .header(header::CONTENT_TYPE, mime)
                .body(Cow::Owned(bytes))
                .unwrap()
        }
        _ => response
            .status(StatusCode::NOT_FOUND)
            .body(Cow::Borrowed(&[][..]))
            .unwrap(),
    }
}

/// The window handle wry attaches the child web view to.
struct ParentHandle(RawWindowHandle);

impl ParentHandle {
    #[cfg(windows)]
    fn for_widget(widget: &impl IsA<gtk4::Widget>) -> Option<Self> {
        use std::num::NonZeroIsize;
        let surface = widget.native()?.surface()?.downcast::<gdk4_win32::Win32Surface>().ok()?;
        let hwnd = NonZeroIsize::new(surface.handle().0 as isize)?;
        // Keep GTK's drawing of the window out of the area of the WebView2
        // child window.
        unsafe {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, SetWindowLongPtrW, GWL_STYLE, WS_CLIPCHILDREN};
            let hwnd = HWND(hwnd.get() as *mut std::ffi::c_void);
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
            if style & WS_CLIPCHILDREN.0 as isize == 0 {
                SetWindowLongPtrW(hwnd, GWL_STYLE, style | WS_CLIPCHILDREN.0 as isize);
            }
        }
        Some(Self(raw_window_handle::Win32WindowHandle::new(hwnd).into()))
    }
}

impl HasWindowHandle for ParentHandle {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // SAFETY: the handle comes from a live GDK surface and is only used
        // while that surface is realized.
        Ok(unsafe { WindowHandle::borrow_raw(self.0) })
    }
}

#[cfg(windows)]
mod win {
    use std::cell::RefCell;
    use std::path::Path;
    use std::rc::Rc;

    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2Environment6, ICoreWebView2_7};
    use webview2_com::PrintToPdfCompletedHandler;
    use windows::core::{Interface, HSTRING};
    use wry::{WebView, WebViewExtWindows};

    use super::Done;

    /// Print the page straight to a PDF file with WebView2's `PrintToPdf`.
    pub fn export_pdf(webview: &WebView, path: &Path, done: Done) -> Result<(), (String, Done)> {
        let start = || -> windows::core::Result<(ICoreWebView2_7, webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2PrintSettings)> {
            let core = webview.webview().cast::<ICoreWebView2_7>()?;
            let environment = webview.environment().cast::<ICoreWebView2Environment6>()?;
            unsafe {
                let settings = environment.CreatePrintSettings()?;
                // WebView2 takes margins in inches.
                settings.SetMarginTop(15.0 / 25.4)?;
                settings.SetMarginBottom(15.0 / 25.4)?;
                settings.SetMarginLeft(18.0 / 25.4)?;
                settings.SetMarginRight(18.0 / 25.4)?;
                settings.SetShouldPrintBackgrounds(true)?;
                settings.SetShouldPrintHeaderAndFooter(false)?;
                Ok((core, settings))
            }
        };
        let (core, settings) = match start() {
            Ok(v) => v,
            Err(e) => return Err((e.message(), done)),
        };
        // Shared so a synchronous failure can still report through `done`.
        let slot = Rc::new(RefCell::new(Some(done)));
        let pending = slot.clone();
        let handler = PrintToPdfCompletedHandler::create(Box::new(move |result, ok| {
            if let Some(done) = pending.borrow_mut().take() {
                done(match (result, ok) {
                    (Ok(()), true) => Ok(()),
                    (Err(e), _) => Err(e.message()),
                    (Ok(()), false) => Err("WebView2 could not write the PDF".into()),
                });
            }
            Ok(())
        }));
        let path = HSTRING::from(path.as_os_str());
        match unsafe { core.PrintToPdf(&path, &settings, &handler) } {
            Ok(()) => Ok(()),
            Err(e) => match slot.borrow_mut().take() {
                Some(done) => Err((e.message(), done)),
                None => Ok(()),
            },
        }
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::cell::RefCell;
    use std::ffi::c_void;
    use std::path::Path;
    use std::ptr::NonNull;
    use std::rc::Rc;

    use gtk4::glib;
    use gtk4::prelude::*;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, Bool, NSObject};
    use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly, Message};
    use objc2_app_kit::{
        NSBackingStoreType, NSPrintInfo, NSPrintJobSavingURL, NSPrintOperation, NSPrintSaveJob, NSWindow,
        NSWindowOrderingMode, NSWindowStyleMask,
    };
    use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect, NSSize, NSURL};
    use wry::{WebView, WebViewExtMacOS};

    use super::{Done, Inner, ParentHandle};

    /// A borderless child window of the GTK window that holds the web view.
    /// AppKit keeps child windows above their parent and moves them with it.
    pub struct Overlay {
        parent: Retained<NSWindow>,
        child: Retained<NSWindow>,
    }

    impl Overlay {
        pub fn for_widget(widget: &impl IsA<gtk4::Widget>) -> Option<Self> {
            let mtm = MainThreadMarker::new()?;
            let surface = widget.native()?.surface()?.downcast::<gdk4_macos::MacosSurface>().ok()?;
            // SAFETY: GDK's NSWindow for this surface; retaining it keeps it
            // valid for as long as the overlay is attached.
            let parent = unsafe { Retained::retain(surface.native() as *mut NSWindow) }?;
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0));
            let child = unsafe {
                NSWindow::initWithContentRect_styleMask_backing_defer(
                    NSWindow::alloc(mtm),
                    frame,
                    NSWindowStyleMask::Borderless,
                    NSBackingStoreType::Buffered,
                    false,
                )
            };
            // Owned through `Retained`, not released by AppKit on close.
            unsafe { child.setReleasedWhenClosed(false) };
            child.setHasShadow(false);
            let overlay = Self { parent, child };
            overlay.set_shown(false);
            // SAFETY: both windows are alive and owned by this overlay.
            unsafe { overlay.parent.addChildWindow_ordered(&overlay.child, NSWindowOrderingMode::Above) };
            Some(overlay)
        }

        pub fn parent(&self) -> &NSWindow {
            &self.parent
        }

        pub fn handle(&self) -> ParentHandle {
            let view = self.child.contentView().expect("a new window has a content view");
            let view = NonNull::new(Retained::as_ptr(&view) as *mut c_void).expect("non-null view");
            ParentHandle(raw_window_handle::AppKitWindowHandle::new(view).into())
        }

        /// Cover the given area, in the coordinates of GDK's content view
        /// (points, flipped so y grows downwards).
        pub fn place(&self, x: i32, y: i32, w: u32, h: u32) {
            let Some(view) = self.parent.contentView() else { return };
            let local = NSRect::new(NSPoint::new(x as f64, y as f64), NSSize::new(w as f64, h as f64));
            let in_window = view.convertRect_toView(local, None);
            let on_screen = self.parent.convertRectToScreen(in_window);
            self.child.setFrame_display(on_screen, true);
        }

        /// Hide without ordering the window out, which could detach it from
        /// its parent's ordering.
        pub fn set_shown(&self, shown: bool) {
            self.child.setAlphaValue(if shown { 1.0 } else { 0.0 });
            self.child.setIgnoresMouseEvents(!shown);
        }
    }

    impl Drop for Overlay {
        fn drop(&mut self) {
            self.parent.removeChildWindow(&self.child);
            self.child.close();
        }
    }

    define_class!(
        // SAFETY: NSObject has no subclassing requirements and this class
        // does not implement Drop.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "MarkViewPrintDelegate"]
        #[ivars = RefCell<Option<Done>>]
        pub struct PrintDelegate;

        impl PrintDelegate {
            // -[NSObject printOperationDidRun:success:contextInfo:], the
            // didRunSelector of runOperationModalForWindow:….
            #[unsafe(method(printOperationDidRun:success:contextInfo:))]
            fn did_run(&self, _operation: &NSPrintOperation, success: Bool, _context: *mut c_void) {
                if let Some(done) = self.ivars().borrow_mut().take() {
                    let result = if success.as_bool() { Ok(()) } else { Err("The PDF could not be written".into()) };
                    glib::idle_add_local_once(move || done(result));
                }
            }
        }

        unsafe impl NSObjectProtocol for PrintDelegate {}
    );

    impl PrintDelegate {
        fn new(mtm: MainThreadMarker, done: Done) -> Retained<Self> {
            let this = Self::alloc(mtm).set_ivars(RefCell::new(Some(done)));
            unsafe { msg_send![super(this), init] }
        }
    }

    /// Save the page as a paginated PDF through WKWebView's print operation,
    /// with the job's disposition set to save to `path` and no panels shown.
    pub fn export_pdf(inner: &Rc<Inner>, webview: &WebView, path: &Path, done: Done) -> Result<(), (String, Done)> {
        let Some(mtm) = MainThreadMarker::new() else {
            return Err(("PDF export must run on the main thread".into(), done));
        };
        let wk = webview.webview();
        let Some(window) = inner.overlay.borrow().as_ref().map(|o| o.parent().retain()) else {
            return Err(("The preview is not in a window".into(), done));
        };
        let Some(url) = path.to_str().map(|p| NSURL::fileURLWithPath(&objc2_foundation::NSString::from_str(p))) else {
            return Err(("The file name is not valid UTF-8".into(), done));
        };

        let info = NSPrintInfo::new();
        unsafe {
            info.setJobDisposition(NSPrintSaveJob);
            info.dictionary().insert(NSPrintJobSavingURL, AsRef::<AnyObject>::as_ref(&*url));
        }
        // NSPrintInfo margins are in points.
        let mm = 72.0 / 25.4;
        info.setTopMargin(15.0 * mm);
        info.setBottomMargin(15.0 * mm);
        info.setLeftMargin(18.0 * mm);
        info.setRightMargin(18.0 * mm);

        let operation = unsafe { wk.printOperationWithPrintInfo(&info) };
        operation.setShowsPrintPanel(false);
        operation.setShowsProgressPanel(false);
        // WKWebView's print view starts with an empty frame, which prints
        // blank pages.
        if let Some(view) = operation.view() {
            view.setFrame(wk.bounds());
        }

        let delegate = PrintDelegate::new(mtm, done);
        unsafe {
            operation.runOperationModalForWindow_delegate_didRunSelector_contextInfo(
                &window,
                Some(&delegate),
                Some(sel!(printOperationDidRun:success:contextInfo:)),
                std::ptr::null_mut(),
            );
        }
        // The operation does not retain its delegate.
        *inner.print_delegate.borrow_mut() = Some(delegate);
        Ok(())
    }
}
