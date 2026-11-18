use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use adw::prelude::*;
use gtk4::{gdk, gio, glib, EventControllerKey, Orientation, Paned, PropagationPhase, ScrolledWindow};
use sourceview5::prelude::*;
use sourceview5::{Buffer as SourceBuffer, LanguageManager, View as SourceView, VimIMContext};
use webkit6::prelude::*;
use webkit6::{NavigationPolicyDecision, PolicyDecisionType, WebView};

use crate::config::*;
use crate::ui::preview::{build_page, current_theme, render_markdown};
use crate::ui::schemes;
use crate::ui::search::SearchController;
use crate::ui::window::MainWindow;
use crate::utils::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ViewMode {
    Split,
    Editor,
    Preview,
}

impl ViewMode {
    pub fn name(self) -> &'static str {
        match self {
            ViewMode::Split => "split",
            ViewMode::Editor => "editor",
            ViewMode::Preview => "preview",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            ViewMode::Split => "view-dual-symbolic",
            ViewMode::Editor => "document-edit-symbolic",
            ViewMode::Preview => "view-reveal-symbolic",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ViewMode::Split => "Split",
            ViewMode::Editor => "Editor Only",
            ViewMode::Preview => "Preview Only",
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            "editor" | "editor-only" => ViewMode::Editor,
            "preview" | "preview-only" => ViewMode::Preview,
            _ => ViewMode::Split,
        }
    }
}

pub fn md_filters() -> gio::ListStore {
    let md = gtk4::FileFilter::new();
    md.set_name(Some("Markdown"));
    md.add_mime_type("text/markdown");
    md.add_suffix("md");
    md.add_suffix("markdown");
    let text = gtk4::FileFilter::new();
    text.set_name(Some("Plain Text"));
    text.add_mime_type("text/plain");
    let filters = gio::ListStore::new::<gtk4::FileFilter>();
    filters.append(&md);
    filters.append(&text);
    filters
}

fn pdf_filters() -> gio::ListStore {
    let pdf = gtk4::FileFilter::new();
    pdf.set_name(Some("PDF"));
    pdf.add_mime_type("application/pdf");
    let filters = gio::ListStore::new::<gtk4::FileFilter>();
    filters.append(&pdf);
    filters
}

pub fn add_to_recents(file: &gio::File) {
    gtk4::RecentManager::default().add_item(&file.uri());
}

pub struct Document {
    pub root: gtk4::Box,
    pub buffer: SourceBuffer,
    pub view: SourceView,
    pub search: Rc<SearchController>,
    webview: WebView,
    paned: Paned,
    editor_scroll: ScrolledWindow,
    host: Weak<MainWindow>,
    pub page: RefCell<Option<adw::TabPage>>,
    file: RefCell<Option<gio::File>>,
    first_render: Cell<bool>,
    loading: Cell<bool>,
    page_uri: RefCell<String>,
    pending_render: RefCell<Option<glib::SourceId>>,
    vim: RefCell<Option<EventControllerKey>>,
    pane_animation: RefCell<Option<adw::TimedAnimation>>,
    print_op: RefCell<Option<webkit6::PrintOperation>>,
}

impl Document {
    pub fn new(host: &Rc<MainWindow>) -> Rc<Self> {
        let buffer = SourceBuffer::new(None);
        buffer.set_language(LanguageManager::default().language("markdown").as_ref());
        buffer.set_highlight_matching_brackets(true);

        let view = SourceView::builder()
            .buffer(&buffer)
            .monospace(true)
            .auto_indent(true)
            .insert_spaces_instead_of_tabs(true)
            .tab_width(4)
            .smart_home_end(sourceview5::SmartHomeEndType::Before)
            .top_margin(12)
            .bottom_margin(12)
            .left_margin(12)
            .right_margin(12)
            .build();
        let editor_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .child(&view)
            .build();

        let webview = WebView::new();
        if let Some(settings) = WebViewExt::settings(&webview) {
            settings.set_enable_javascript_markup(false);
            settings.set_enable_developer_extras(false);
            settings.set_enable_back_forward_navigation_gestures(false);
        }

        let paned = Paned::builder()
            .orientation(if host.is_narrow() { Orientation::Vertical } else { Orientation::Horizontal })
            .start_child(&editor_scroll)
            .end_child(&webview)
            .shrink_start_child(true)
            .shrink_end_child(true)
            .vexpand(true)
            .build();

        let search = SearchController::new(&view, &buffer);
        let root = gtk4::Box::new(Orientation::Vertical, 0);
        root.append(&search.search_bar);
        root.append(&paned);

        let doc = Rc::new(Self {
            root,
            buffer,
            view,
            search,
            webview,
            paned,
            editor_scroll,
            host: Rc::downgrade(host),
            page: RefCell::new(None),
            file: RefCell::new(None),
            first_render: Cell::new(true),
            loading: Cell::new(false),
            page_uri: RefCell::new(String::new()),
            pending_render: RefCell::new(None),
            vim: RefCell::new(None),
            pane_animation: RefCell::new(None),
            print_op: RefCell::new(None),
        });
        doc.setup_signals();
        doc.apply_settings();
        doc.render(true);
        doc
    }

    fn weak(self: &Rc<Self>) -> Weak<Self> {
        Rc::downgrade(self)
    }

    fn host(&self) -> Option<Rc<MainWindow>> {
        self.host.upgrade()
    }

    fn parent_window(&self) -> Option<adw::ApplicationWindow> {
        self.host().map(|h| h.window.clone())
    }

    fn toast(&self, message: &str) {
        if let Some(host) = self.host() {
            host.toast(adw::Toast::new(message));
        }
    }

    fn setup_signals(self: &Rc<Self>) {
        let weak = self.weak();
        self.buffer.connect_changed(move |_| {
            if let Some(doc) = weak.upgrade() {
                doc.schedule_render();
                if doc.view.has_focus()
                    && let Some(host) = doc.host()
                {
                    host.hide_chrome();
                }
            }
        });

        let weak = self.weak();
        self.buffer.connect_modified_changed(move |_| {
            if let Some(doc) = weak.upgrade() {
                doc.update_title();
            }
        });

        let weak = self.weak();
        self.editor_scroll.hadjustment().connect_page_size_notify(move |_| {
            if let Some(doc) = weak.upgrade() {
                doc.update_margins();
            }
        });

        let weak = self.weak();
        self.editor_scroll.vadjustment().connect_value_changed(move |_| {
            if let Some(doc) = weak.upgrade() {
                doc.sync_scroll();
            }
        });

        let weak = self.weak();
        self.webview.connect_load_changed(move |_, event| {
            if event == webkit6::LoadEvent::Finished
                && let Some(doc) = weak.upgrade()
            {
                doc.sync_scroll();
            }
        });

        let weak = self.weak();
        self.webview.connect_decide_policy(move |_, decision, kind| {
            let Some(doc) = weak.upgrade() else { return false };
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
            let page = doc.page_uri.borrow().clone();
            if uri == "about:blank" || uri.split('#').next() == Some(page.as_str()) {
                return false;
            }
            decision.ignore();
            if uri.starts_with("file:") && is_markdown_uri(&uri) {
                if let Some(host) = doc.host() {
                    host.open_file(gio::File::for_uri(&uri));
                }
            } else {
                gtk4::UriLauncher::new(&uri).launch(
                    doc.parent_window().as_ref(),
                    None::<&gio::Cancellable>,
                    |_| {},
                );
            }
            true
        });

        let drop = gtk4::DropTarget::new(gio::File::static_type(), gdk::DragAction::COPY);
        let weak = self.weak();
        drop.connect_drop(move |_, value, _, _| {
            let (Some(doc), Ok(file)) = (weak.upgrade(), value.get::<gio::File>()) else {
                return false;
            };
            if let Some(host) = doc.host() {
                host.open_file(file);
            }
            true
        });
        self.editor_scroll.add_controller(drop);
    }

    pub fn apply_settings(&self) {
        self.view.set_show_line_numbers(pref_bool(PREF_LINE_NUMBERS, DEFAULT_LINE_NUMBERS == "true"));
        self.view
            .set_highlight_current_line(pref_bool(PREF_HIGHLIGHT_LINE, DEFAULT_HIGHLIGHT_LINE == "true"));
        self.view.set_wrap_mode(if pref_bool(PREF_WORD_WRAP, DEFAULT_WORD_WRAP == "true") {
            gtk4::WrapMode::WordChar
        } else {
            gtk4::WrapMode::None
        });
        self.editor_scroll.set_hscrollbar_policy(if self.view.wrap_mode() == gtk4::WrapMode::None {
            gtk4::PolicyType::Automatic
        } else {
            gtk4::PolicyType::Never
        });

        let want_vim = pref_bool(PREF_VIM_MODE, DEFAULT_VIM_MODE == "true");
        let mut vim = self.vim.borrow_mut();
        if want_vim && vim.is_none() {
            let ctx = VimIMContext::new();
            ctx.set_client_widget(Some(&self.view));
            let keys = EventControllerKey::new();
            keys.set_propagation_phase(PropagationPhase::Capture);
            keys.set_im_context(Some(&ctx));
            self.view.add_controller(keys.clone());
            *vim = Some(keys);
        } else if !want_vim && let Some(keys) = vim.take() {
            self.view.remove_controller(&keys);
        }
        drop(vim);
        self.update_margins();
        self.apply_style_scheme();
    }

    fn update_margins(&self) {
        let width = self.editor_scroll.hadjustment().page_size() as i32;
        let margin = if pref_bool(PREF_READABLE_LINE, DEFAULT_READABLE_LINE == "true") && width > 0 {
            let max = load_pref(PREF_MAX_WIDTH, DEFAULT_MAX_WIDTH)
                .parse::<i32>()
                .unwrap_or(1000)
                .clamp(400, 3000);
            ((width - max) / 2).max(12)
        } else {
            12
        };
        if self.view.left_margin() != margin {
            self.view.set_left_margin(margin);
            self.view.set_right_margin(margin);
        }
    }

    pub fn apply_style_scheme(&self) {
        let dark = adw::StyleManager::default().is_dark();
        self.buffer
            .set_style_scheme(schemes::resolve(&schemes::saved_family(), dark).as_ref());
        self.webview.set_background_color(&current_theme().background());
    }

    pub fn file(&self) -> Option<gio::File> {
        self.file.borrow().clone()
    }

    pub fn name(&self) -> String {
        self.file.borrow().as_ref().map(display_name).unwrap_or_else(|| "New Document".into())
    }

    pub fn folder(&self) -> String {
        self.file.borrow().as_ref().map(display_folder).unwrap_or_default()
    }

    pub fn is_modified(&self) -> bool {
        self.buffer.is_modified()
    }

    pub fn is_empty_and_unmodified(&self) -> bool {
        !self.loading.get()
            && self.file.borrow().is_none()
            && !self.buffer.is_modified()
            && self.buffer.char_count() == 0
    }

    pub fn update_title(&self) {
        let name = self.name();
        if let Some(page) = self.page.borrow().as_ref() {
            page.set_title(&if self.is_modified() { format!("• {name}") } else { name });
            let folder = self.folder();
            page.set_tooltip(&folder);
        }
        if let Some(host) = self.host() {
            host.update_title();
        }
    }

    fn schedule_render(self: &Rc<Self>) {
        if let Some(id) = self.pending_render.borrow_mut().take() {
            id.remove();
        }
        let delay = load_pref(PREF_DEBOUNCE, DEFAULT_DEBOUNCE).parse::<f64>().unwrap_or(150.0) as u64;
        if delay == 0 {
            self.render(false);
            return;
        }
        let weak = self.weak();
        let id = glib::timeout_add_local_once(std::time::Duration::from_millis(delay), move || {
            if let Some(doc) = weak.upgrade() {
                doc.pending_render.borrow_mut().take();
                doc.render(false);
            }
        });
        *self.pending_render.borrow_mut() = Some(id);
    }

    pub fn render(&self, full_reload: bool) {
        let text = self.buffer.text(&self.buffer.start_iter(), &self.buffer.end_iter(), false);
        let body = render_markdown(&text);
        let in_place = pref_bool(PREF_DOM_INJECTION, DEFAULT_DOM_INJECTION == "true");

        if in_place && !full_reload && !self.first_render.get() {
            let args = glib::VariantDict::new(None);
            args.insert("html", body.as_str());
            self.webview.call_async_javascript_function(
                "document.body.innerHTML = html;",
                Some(&args.end()),
                None,
                None,
                None::<&gio::Cancellable>,
                |_| {},
            );
            return;
        }

        let theme = current_theme();
        let base = base_uri_for_preview(self.file.borrow().as_ref());
        *self.page_uri.borrow_mut() = base.clone();
        self.webview.set_background_color(&theme.background());
        self.webview.load_html(&build_page(&body, &theme), Some(&base));
        self.first_render.set(false);
    }

    fn sync_scroll(&self) {
        if !pref_bool(PREF_SYNC_SCROLL, DEFAULT_SYNC_SCROLL == "true") {
            return;
        }
        let adj = self.editor_scroll.vadjustment();
        let range = adj.upper() - adj.page_size();
        if range <= 0.0 {
            return;
        }
        let args = glib::VariantDict::new(None);
        args.insert("fraction", adj.value() / range);
        self.webview.call_async_javascript_function(
            "window.scrollTo(0, (document.documentElement.scrollHeight - window.innerHeight) * fraction);",
            Some(&args.end()),
            None,
            None,
            None::<&gio::Cancellable>,
            |_| {},
        );
    }

    pub fn set_orientation(&self, orientation: Orientation) {
        self.paned.set_orientation(orientation);
    }

    fn pane_extent(&self) -> i32 {
        let horizontal = self.paned.orientation() == Orientation::Horizontal;
        let size = if horizontal { self.paned.width() } else { self.paned.height() };
        if size > 0 {
            return size;
        }
        self.parent_window()
            .map(|w| if horizontal { w.default_width() } else { w.default_height() })
            .unwrap_or(1000)
    }

    pub fn apply_view_mode(&self, mode: ViewMode, animate: bool) {
        if let Some(anim) = self.pane_animation.borrow_mut().take() {
            anim.skip();
        }
        let extent = self.pane_extent();
        let target = match mode {
            ViewMode::Split => extent / 2,
            ViewMode::Editor => extent,
            ViewMode::Preview => 0,
        };
        let editor = self.editor_scroll.clone();
        let preview = self.webview.clone();
        let finish = move || {
            editor.set_visible(mode != ViewMode::Preview);
            preview.set_visible(mode != ViewMode::Editor);
        };

        if !animate || !self.paned.is_mapped() {
            self.paned.set_position(target);
            finish();
            return;
        }

        let from = if !self.editor_scroll.is_visible() {
            0
        } else if !self.webview.is_visible() {
            extent
        } else {
            self.paned.position()
        };
        self.editor_scroll.set_visible(true);
        self.webview.set_visible(true);
        self.paned.set_position(from);

        let anim = adw::TimedAnimation::builder()
            .widget(&self.paned)
            .value_from(from as f64)
            .value_to(target as f64)
            .duration(250)
            .easing(adw::Easing::EaseOutCubic)
            .target(&adw::PropertyAnimationTarget::new(&self.paned, "position"))
            .build();
        anim.connect_done(move |_| finish());
        anim.play();
        *self.pane_animation.borrow_mut() = Some(anim);
    }

    pub async fn confirm_discard(self: &Rc<Self>) -> bool {
        if !self.is_modified() {
            return true;
        }
        let dialog = adw::AlertDialog::new(
            Some("Save Changes?"),
            Some(&format!(
                "“{}” has unsaved changes. Changes which are not saved will be permanently lost.",
                self.name()
            )),
        );
        dialog.add_responses(&[("cancel", "_Cancel"), ("discard", "_Discard"), ("save", "_Save")]);
        dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
        dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("save"));
        dialog.set_close_response("cancel");
        match dialog.choose_future(self.parent_window().as_ref()).await.as_str() {
            "save" => self.save().await,
            "discard" => true,
            _ => false,
        }
    }

    pub fn reserve(&self) {
        self.loading.set(true);
    }

    pub async fn load(self: &Rc<Self>, file: gio::File) -> bool {
        self.loading.set(true);
        let ok = self.load_inner(file).await;
        self.loading.set(false);
        ok
    }

    async fn load_inner(self: &Rc<Self>, file: gio::File) -> bool {
        let bytes = match file.load_contents_future().await {
            Ok((bytes, _)) => bytes,
            Err(e) => {
                self.toast(&format!("Could not open “{}”: {}", display_name(&file), e.message()));
                return false;
            }
        };
        let Ok(text) = std::str::from_utf8(&bytes) else {
            self.toast(&format!("“{}” is not a UTF-8 text file", display_name(&file)));
            return false;
        };
        add_to_recents(&file);
        *self.file.borrow_mut() = Some(file);
        self.first_render.set(true);
        self.buffer.begin_irreversible_action();
        self.buffer.set_text(text);
        self.buffer.end_irreversible_action();
        self.buffer.place_cursor(&self.buffer.start_iter());
        self.buffer.set_modified(false);
        self.update_title();
        self.render(true);
        true
    }

    async fn write_to(self: &Rc<Self>, file: &gio::File) -> bool {
        let text = self
            .buffer
            .text(&self.buffer.start_iter(), &self.buffer.end_iter(), false)
            .to_string();
        match file
            .replace_contents_future(text, None, false, gio::FileCreateFlags::NONE)
            .await
        {
            Ok(_) => {
                let renamed = self.file.borrow().as_ref() != Some(file);
                add_to_recents(file);
                *self.file.borrow_mut() = Some(file.clone());
                self.buffer.set_modified(false);
                self.update_title();
                if renamed {
                    self.render(true);
                }
                true
            }
            Err((_, e)) => {
                self.toast(&format!("Could not save “{}”: {}", display_name(file), e.message()));
                false
            }
        }
    }

    pub async fn save(self: &Rc<Self>) -> bool {
        match self.file() {
            Some(file) => self.write_to(&file).await,
            None => self.save_as().await,
        }
    }

    pub async fn save_as(self: &Rc<Self>) -> bool {
        let dialog = gtk4::FileDialog::builder()
            .title("Save As")
            .filters(&md_filters())
            .build();
        match self.file().as_ref() {
            Some(file) => dialog.set_initial_file(Some(file)),
            None => dialog.set_initial_name(Some("Untitled.md")),
        }
        match dialog.save_future(self.parent_window().as_ref()).await {
            Ok(file) => self.write_to(&file).await,
            Err(_) => false,
        }
    }

    pub async fn export_pdf(self: &Rc<Self>) {
        let name = self.name();
        let stem = name.rsplit_once('.').map(|(s, _)| s.to_string()).unwrap_or(name);
        let dialog = gtk4::FileDialog::builder()
            .title("Export as PDF")
            .initial_name(format!("{stem}.pdf"))
            .filters(&pdf_filters())
            .build();
        let Ok(file) = dialog.save_future(self.parent_window().as_ref()).await else {
            return;
        };

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
        let weak = self.weak();
        op.connect_finished(move |_| {
            let Some(doc) = weak.upgrade() else { return };
            doc.print_op.borrow_mut().take();
            let Some(host) = doc.host() else { return };
            let toast = adw::Toast::builder()
                .title(format!("Exported “{}”", display_name(&file)))
                .button_label("Open")
                .build();
            let file = file.clone();
            let window = host.window.clone();
            toast.connect_button_clicked(move |_| {
                gtk4::FileLauncher::new(Some(&file)).launch(
                    Some(&window),
                    None::<&gio::Cancellable>,
                    |_| {},
                );
            });
            host.toast(toast);
        });
        let weak = self.weak();
        op.connect_failed(move |_, e| {
            if let Some(doc) = weak.upgrade() {
                doc.print_op.borrow_mut().take();
                doc.toast(&format!("Export failed: {}", e.message()));
            }
        });
        op.print();
        *self.print_op.borrow_mut() = Some(op);
    }
}
