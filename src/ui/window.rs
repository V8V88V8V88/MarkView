use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use adw::prelude::*;
use adw::{
    Application, ApplicationWindow, Breakpoint, BreakpointCondition, HeaderBar, SplitButton,
    TabBar, TabOverview, TabView, Toast, ToastOverlay, ToolbarView, WindowTitle,
};
use gtk4::{gio, glib, Orientation};

use crate::config::*;
use crate::ui::document::{md_filters, Document, ViewMode};
use crate::utils::*;

pub const APP_NAME: &str = "MarkView";

thread_local! {
    static WINDOWS: RefCell<Vec<Rc<MainWindow>>> = const { RefCell::new(Vec::new()) };
}

pub fn windows() -> Vec<Rc<MainWindow>> {
    WINDOWS.with(|w| w.borrow().clone())
}

pub fn window_for(window: &gtk4::Window) -> Option<Rc<MainWindow>> {
    windows().into_iter().find(|w| w.window.upcast_ref::<gtk4::Window>() == window)
}

fn all_documents() -> Vec<Rc<Document>> {
    windows().iter().flat_map(|w| w.docs.borrow().clone()).collect()
}

pub fn apply_settings_to_all_windows() {
    for win in windows() {
        win.sync_readable_action();
    }
    for doc in all_documents() {
        doc.apply_settings();
        doc.render(true);
    }
}

pub fn refresh_appearance_of_all_windows() {
    for doc in all_documents() {
        doc.apply_style_scheme();
        doc.render(true);
    }
}

fn style_switcher() -> gtk4::Box {
    let switcher = gtk4::Box::builder()
        .spacing(18)
        .halign(gtk4::Align::Center)
        .css_classes(["style-switcher"])
        .build();
    let style = adw::StyleManager::default();
    for (name, class, tooltip) in [
        ("default", "follow", "Follow System Style"),
        ("force-light", "light", "Light Style"),
        ("force-dark", "dark", "Dark Style"),
    ] {
        let button = gtk4::CheckButton::builder()
            .css_classes([class])
            .tooltip_text(tooltip)
            .action_name("app.color-scheme")
            .action_target(&name.to_variant())
            .halign(gtk4::Align::Center)
            .focus_on_click(false)
            .build();
        if name == "default" {
            style
                .bind_property("system-supports-color-schemes", &button, "visible")
                .sync_create()
                .build();
        }
        switcher.append(&button);
    }
    switcher
}

fn menu_row(icon: &str, label: &str) -> (gtk4::Box, gtk4::Label) {
    let row = gtk4::Box::new(Orientation::Horizontal, 12);
    row.append(&gtk4::Image::from_icon_name(icon));
    row.append(&gtk4::Label::builder().label(label).xalign(0.0).hexpand(true).build());
    let suffix = gtk4::Label::builder().css_classes(["dimmed"]).build();
    row.append(&suffix);
    (row, suffix)
}

fn menu_popover() -> (gtk4::Popover, gtk4::Box) {
    let list = gtk4::Box::new(Orientation::Vertical, 0);
    let popover = gtk4::Popover::builder()
        .child(&list)
        .css_classes(["menu"])
        .has_arrow(false)
        .build();
    (popover, list)
}

fn menu_item(popover: &gtk4::Popover, row: &gtk4::Box, action: &str, target: Option<&glib::Variant>) -> gtk4::Button {
    let item = gtk4::Button::builder()
        .child(row)
        .css_classes(["flat"])
        .action_name(action)
        .build();
    if let Some(target) = target {
        item.set_action_target_value(Some(target));
    }
    let popover = popover.clone();
    item.connect_clicked(move |_| popover.popdown());
    item
}

struct RecentEntry {
    row: gtk4::ListBoxRow,
    uri: String,
    haystack: String,
}

pub struct MainWindow {
    pub window: ApplicationWindow,
    toolbar: ToolbarView,
    pointer: Cell<Option<(f64, f64)>>,
    toasts: ToastOverlay,
    tab_view: TabView,
    title: WindowTitle,
    view_button: gtk4::Button,
    recent_list: gtk4::ListBox,
    recent_search: gtk4::SearchEntry,
    recent_popover: gtk4::Popover,
    recent_entries: RefCell<Vec<RecentEntry>>,
    docs: RefCell<Vec<Rc<Document>>>,
    mode: Cell<ViewMode>,
    narrow: Cell<bool>,
    force_close: Cell<bool>,
}

impl MainWindow {
    pub fn new(app: &Application) -> Rc<Self> {
        let title = WindowTitle::new(APP_NAME, "");
        let header = HeaderBar::builder().title_widget(&title).build();

        let recent_search = gtk4::SearchEntry::builder()
            .placeholder_text("Search recent files")
            .hexpand(true)
            .build();
        let browse = gtk4::Button::builder()
            .icon_name("folder-open-symbolic")
            .tooltip_text("Browse Files…")
            .action_name("win.open")
            .build();
        let search_row = gtk4::Box::new(Orientation::Horizontal, 6);
        search_row.append(&recent_search);
        search_row.append(&browse);
        let recent_list = gtk4::ListBox::builder()
            .css_classes(["navigation-sidebar"])
            .selection_mode(gtk4::SelectionMode::None)
            .build();
        recent_list.set_placeholder(Some(
            &gtk4::Label::builder()
                .label("No Recent Files")
                .css_classes(["dimmed"])
                .margin_top(18)
                .margin_bottom(18)
                .build(),
        ));
        let recent_scroll = gtk4::ScrolledWindow::builder()
            .child(&recent_list)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .propagate_natural_height(true)
            .max_content_height(420)
            .min_content_width(340)
            .build();
        let recent_box = gtk4::Box::new(Orientation::Vertical, 6);
        recent_box.append(&search_row);
        recent_box.append(&recent_scroll);
        let recent_popover = gtk4::Popover::builder().child(&recent_box).has_arrow(false).build();
        browse.connect_clicked({
            let popover = recent_popover.clone();
            move |_| popover.popdown()
        });
        let open_button = SplitButton::builder()
            .icon_name("tab-new-symbolic")
            .tooltip_text("New Tab")
            .dropdown_tooltip("Open File")
            .action_name("win.new-tab")
            .popover(&recent_popover)
            .build();

        header.pack_start(&open_button);

        let view_button = gtk4::Button::builder()
            .tooltip_text("Switch Layout")
            .action_name("win.cycle-view")
            .build();

        let (save_popover, save_list) = menu_popover();
        let mut accel_labels = Vec::new();
        for (icon, label, action) in [
            ("document-save-as-symbolic", "Save As…", "win.save-as"),
            ("x-office-document-symbolic", "Export as PDF…", "win.export-pdf"),
        ] {
            let (row, accel) = menu_row(icon, label);
            save_list.append(&menu_item(&save_popover, &row, action, None));
            accel_labels.push((action, accel));
        }
        save_popover.connect_show(move |popover| {
            let Some(app) = popover.root().and_downcast::<gtk4::Window>().and_then(|w| w.application())
            else {
                return;
            };
            for (action, label) in &accel_labels {
                let text = app
                    .accels_for_action(action)
                    .first()
                    .and_then(|a| gtk4::accelerator_parse(a.as_str()))
                    .map(|(key, mods)| gtk4::accelerator_get_label(key, mods).to_string())
                    .unwrap_or_default();
                label.set_label(&text);
            }
        });
        let save_button = SplitButton::builder()
            .icon_name("document-save-symbolic")
            .tooltip_text("Save")
            .dropdown_tooltip("More Save Options")
            .action_name("win.save")
            .popover(&save_popover)
            .build();

        let menu = gio::Menu::new();
        let style_item = gio::MenuItem::new(None, None);
        style_item.set_attribute_value("custom", Some(&"style-switcher".to_variant()));
        let style_section = gio::Menu::new();
        style_section.append_item(&style_item);
        menu.append_section(None, &style_section);
        let doc_section = gio::Menu::new();
        doc_section.append(Some("New Window"), Some("app.new-window"));
        menu.append_section(None, &doc_section);
        let view_section = gio::Menu::new();
        view_section.append(Some("Find and Replace…"), Some("win.search"));
        view_section.append(Some("Readable Line Length"), Some("win.readable-line"));
        menu.append_section(None, &view_section);
        let app_section = gio::Menu::new();
        app_section.append(Some("Preferences"), Some("app.preferences"));
        app_section.append(Some("Keyboard Shortcuts"), Some("app.shortcuts"));
        app_section.append(Some("About MarkView"), Some("app.about"));
        menu.append_section(None, &app_section);
        let menu_button = gtk4::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Main Menu")
            .menu_model(&menu)
            .primary(true)
            .build();
        if let Some(popover) = menu_button.popover().and_downcast::<gtk4::PopoverMenu>() {
            popover.add_child(&style_switcher(), "style-switcher");
        }

        let tab_view = TabView::new();
        let tab_button = gtk4::Button::builder()
            .icon_name("view-grid-symbolic")
            .action_name("overview.open")
            .tooltip_text("View Open Tabs")
            .build();
        tab_view
            .bind_property("n-pages", &tab_button, "visible")
            .transform_to(|_, n: i32| Some(n > 1))
            .sync_create()
            .build();

        header.pack_end(&menu_button);
        header.pack_end(&view_button);
        header.pack_end(&save_button);
        header.pack_end(&tab_button);

        let tab_bar = TabBar::builder().view(&tab_view).autohide(true).build();

        let toolbar = ToolbarView::builder()
            .content(&tab_view)
            .top_bar_style(adw::ToolbarStyle::Flat)
            .build();
        toolbar.add_top_bar(&header);
        toolbar.add_top_bar(&tab_bar);
        let overview = TabOverview::builder()
            .view(&tab_view)
            .child(&toolbar)
            .enable_new_tab(true)
            .build();
        let toasts = ToastOverlay::new();
        toasts.set_child(Some(&overview));

        let width = load_pref(PREF_WINDOW_WIDTH, "1200").parse().unwrap_or(1200);
        let height = load_pref(PREF_WINDOW_HEIGHT, "800").parse().unwrap_or(800);
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(width)
            .default_height(height)
            .maximized(pref_bool(PREF_WINDOW_MAXIMIZED, false))
            .width_request(360)
            .height_request(294)
            .title(APP_NAME)
            .content(&toasts)
            .build();

        let narrow = Breakpoint::new(BreakpointCondition::parse("max-width: 700sp").unwrap());
        window.add_breakpoint(narrow.clone());

        let win = Rc::new(Self {
            window,
            toolbar: toolbar.clone(),
            pointer: Cell::new(None),
            toasts,
            tab_view,
            title,
            view_button,
            recent_list,
            recent_search,
            recent_popover,
            recent_entries: RefCell::new(Vec::new()),
            docs: RefCell::new(Vec::new()),
            mode: Cell::new(ViewMode::from_name(&load_pref(PREF_DEFAULT_VIEW, DEFAULT_VIEW))),
            narrow: Cell::new(false),
            force_close: Cell::new(false),
        });
        WINDOWS.with(|w| w.borrow_mut().push(win.clone()));

        let weak = win.weak();
        overview.connect_create_tab(move |_| {
            let win = weak.upgrade().expect("window alive while overview exists");
            let doc = win.new_tab();
            let page = doc.page.borrow().clone();
            page.expect("new tab has a page")
        });

        win.setup_actions();
        win.setup_signals(&narrow);
        win.sync_view_controls();
        win.new_tab();
        win
    }

    fn weak(self: &Rc<Self>) -> Weak<Self> {
        Rc::downgrade(self)
    }

    pub fn is_narrow(&self) -> bool {
        self.narrow.get()
    }

    pub fn hide_chrome(&self) {
        if pref_bool(PREF_AUTOHIDE_HEADER, DEFAULT_AUTOHIDE_HEADER == "true") && self.toolbar.reveals_top_bars() {
            self.pointer.set(None);
            self.toolbar.set_reveal_top_bars(false);
        }
    }

    fn show_chrome(&self) {
        if !self.toolbar.reveals_top_bars() {
            self.toolbar.set_reveal_top_bars(true);
        }
    }

    pub fn toast(&self, toast: Toast) {
        self.toasts.add_toast(toast);
    }

    pub fn present(&self) {
        self.window.present();
    }

    fn doc_for_page(&self, page: &adw::TabPage) -> Option<Rc<Document>> {
        let child = page.child();
        self.docs.borrow().iter().find(|d| d.root.upcast_ref::<gtk4::Widget>() == &child).cloned()
    }

    pub fn current(&self) -> Option<Rc<Document>> {
        self.tab_view.selected_page().and_then(|p| self.doc_for_page(&p))
    }

    pub fn new_tab(self: &Rc<Self>) -> Rc<Document> {
        let doc = Document::new(self);
        let page = self.tab_view.append(&doc.root);
        *doc.page.borrow_mut() = Some(page.clone());
        self.docs.borrow_mut().push(doc.clone());
        doc.update_title();
        self.tab_view.set_selected_page(&page);
        let mode = self.mode.get();
        let weak = Rc::downgrade(&doc);
        glib::idle_add_local_once(move || {
            if let Some(doc) = weak.upgrade() {
                doc.apply_view_mode(mode, false);
                doc.view.grab_focus();
            }
        });
        doc.apply_view_mode(mode, false);
        doc
    }

    pub fn open_file(self: &Rc<Self>, file: gio::File) {
        let existing = self
            .docs
            .borrow()
            .iter()
            .find(|d| d.file().is_some_and(|f| f.equal(&file)))
            .cloned();
        if let Some(doc) = existing {
            if let Some(page) = doc.page.borrow().as_ref() {
                self.tab_view.set_selected_page(page);
            }
            return;
        }
        let doc = match self.current() {
            Some(doc) if doc.is_empty_and_unmodified() => doc,
            _ => self.new_tab(),
        };
        let placeholder = doc.is_empty_and_unmodified();
        let win = self.clone();
        doc.reserve();
        glib::spawn_future_local(async move {
            let loaded = doc.load(file).await;
            let several = win.docs.borrow().len() > 1;
            let page = doc.page.borrow().clone();
            if !loaded && placeholder && several && let Some(page) = page {
                win.tab_view.close_page(&page);
            }
        });
    }

    pub fn update_title(&self) {
        let Some(doc) = self.current() else {
            self.title.set_title(APP_NAME);
            self.title.set_subtitle("");
            return;
        };
        let name = doc.name();
        self.title
            .set_title(&if doc.is_modified() { format!("• {name}") } else { name.clone() });
        self.title.set_subtitle(&doc.folder());
        self.window.set_title(Some(&format!("{name} — {APP_NAME}")));
    }

    fn sync_view_controls(&self) {
        let mode = self.mode.get();
        self.view_button.set_icon_name(mode.icon());
        self.view_button.set_tooltip_text(Some(&format!("Switch Layout ({})", mode.label())));
        if let Some(action) = self.window.lookup_action("view-mode") {
            action.change_state(&mode.name().to_variant());
        }
    }

    fn set_view_mode(&self, mode: ViewMode) {
        if self.mode.get() == mode {
            return;
        }
        self.mode.set(mode);
        self.sync_view_controls();
        let current = self.current();
        for doc in self.docs.borrow().iter() {
            let is_current = current.as_ref().is_some_and(|c| Rc::ptr_eq(c, doc));
            doc.apply_view_mode(mode, is_current);
        }
        if mode != ViewMode::Preview
            && let Some(doc) = current
        {
            doc.view.grab_focus();
        }
    }

    pub fn sync_readable_action(&self) {
        if let Some(action) = self.window.lookup_action("readable-line") {
            action.change_state(&pref_bool(PREF_READABLE_LINE, DEFAULT_READABLE_LINE == "true").to_variant());
        }
    }

    fn action(self: &Rc<Self>, name: &str, f: impl Fn(&Rc<Self>) + 'static) {
        let action = gio::SimpleAction::new(name, None);
        let weak = self.weak();
        action.connect_activate(move |_, _| {
            if let Some(win) = weak.upgrade() {
                f(&win);
            }
        });
        self.window.add_action(&action);
    }

    fn doc_action<F, Fut>(self: &Rc<Self>, name: &str, f: F)
    where
        F: Fn(Rc<Document>) -> Fut + 'static,
        Fut: std::future::Future<Output = ()> + 'static,
    {
        self.action(name, move |win| {
            if let Some(doc) = win.current() {
                glib::spawn_future_local(f(doc));
            }
        });
    }

    fn setup_actions(self: &Rc<Self>) {
        self.action("new-tab", |win| {
            win.new_tab();
        });
        self.action("close-tab", |win| {
            if let Some(page) = win.tab_view.selected_page() {
                win.tab_view.close_page(&page);
            }
        });
        self.action("open", |win| {
            let win = win.clone();
            glib::spawn_future_local(async move { win.open_with_dialog().await });
        });
        self.doc_action("save", |doc| async move {
            doc.save().await;
        });
        self.doc_action("save-as", |doc| async move {
            doc.save_as().await;
        });
        self.doc_action("export-pdf", |doc| async move { doc.export_pdf().await });
        let format = gio::SimpleAction::new("format", Some(glib::VariantTy::STRING));
        let weak = self.weak();
        format.connect_activate(move |_, target| {
            if let (Some(win), Some(name)) = (weak.upgrade(), target.and_then(|t| t.str()))
                && let Some(doc) = win.current()
            {
                doc.format(name);
            }
        });
        self.window.add_action(&format);
        self.action("search", |win| {
            if let Some(doc) = win.current() {
                doc.search.toggle();
            }
        });
        self.action("toggle-editor", |win| {
            let next = if win.mode.get() == ViewMode::Preview { ViewMode::Split } else { ViewMode::Preview };
            win.set_view_mode(next);
        });
        self.action("cycle-view", |win| {
            win.set_view_mode(match win.mode.get() {
                ViewMode::Split => ViewMode::Preview,
                ViewMode::Preview => ViewMode::Editor,
                ViewMode::Editor => ViewMode::Split,
            });
        });

        let view_mode = gio::SimpleAction::new_stateful(
            "view-mode",
            Some(glib::VariantTy::STRING),
            &self.mode.get().name().to_variant(),
        );
        let weak = self.weak();
        view_mode.connect_activate(move |_, target| {
            if let (Some(win), Some(name)) = (weak.upgrade(), target.and_then(|t| t.str())) {
                win.set_view_mode(ViewMode::from_name(name));
            }
        });
        self.window.add_action(&view_mode);

        let readable = gio::SimpleAction::new_stateful(
            "readable-line",
            None,
            &pref_bool(PREF_READABLE_LINE, DEFAULT_READABLE_LINE == "true").to_variant(),
        );
        readable.connect_activate(|action, _| {
            let enabled = !action.state().and_then(|s| s.get::<bool>()).unwrap_or(false);
            save_pref_bool(PREF_READABLE_LINE, enabled);
            apply_settings_to_all_windows();
        });
        self.window.add_action(&readable);
    }

    fn setup_signals(self: &Rc<Self>, narrow: &Breakpoint) {
        let relayout = |narrow_now: bool| {
            let weak = self.weak();
            move |_: &Breakpoint| {
                let Some(win) = weak.upgrade() else { return };
                win.narrow.set(narrow_now);
                let orientation = if narrow_now { Orientation::Vertical } else { Orientation::Horizontal };
                for doc in win.docs.borrow().iter() {
                    doc.set_orientation(orientation);
                }
                let weak = Rc::downgrade(&win);
                glib::idle_add_local_once(move || {
                    if let Some(win) = weak.upgrade() {
                        for doc in win.docs.borrow().iter() {
                            doc.apply_view_mode(win.mode.get(), false);
                        }
                    }
                });
            }
        };
        narrow.connect_apply(relayout(true));
        narrow.connect_unapply(relayout(false));

        let motion = gtk4::EventControllerMotion::new();
        let weak = self.weak();
        motion.connect_motion(move |_, x, y| {
            let Some(win) = weak.upgrade() else { return };
            match win.pointer.get() {
                Some((px, py)) if (x - px).hypot(y - py) > 8.0 => win.show_chrome(),
                None => win.pointer.set(Some((x, y))),
                _ => {}
            }
        });
        self.window.add_controller(motion);

        let weak = self.weak();
        self.tab_view.connect_selected_page_notify(move |_| {
            if let Some(win) = weak.upgrade() {
                win.update_title();
                if let Some(doc) = win.current() {
                    doc.apply_view_mode(win.mode.get(), false);
                }
            }
        });

        let weak = self.weak();
        self.tab_view.connect_close_page(move |view, page| {
            let Some(win) = weak.upgrade() else { return glib::Propagation::Proceed };
            let Some(doc) = win.doc_for_page(page) else { return glib::Propagation::Proceed };
            let finish = {
                let win = win.clone();
                let doc = doc.clone();
                let view = view.clone();
                let page = page.clone();
                move |confirmed: bool| {
                    view.close_page_finish(&page, confirmed);
                    if confirmed {
                        win.docs.borrow_mut().retain(|d| !Rc::ptr_eq(d, &doc));
                        if win.docs.borrow().is_empty() && !win.force_close.get() {
                            win.new_tab();
                        }
                    }
                }
            };
            if win.force_close.get() || !doc.is_modified() {
                finish(true);
            } else {
                glib::spawn_future_local(async move {
                    let confirmed = doc.confirm_discard().await;
                    finish(confirmed);
                });
            }
            glib::Propagation::Stop
        });

        let weak = self.weak();
        self.window.connect_close_request(move |window| {
            let Some(win) = weak.upgrade() else { return glib::Propagation::Proceed };
            let modified: Vec<Rc<Document>> =
                win.docs.borrow().iter().filter(|d| d.is_modified()).cloned().collect();
            if win.force_close.get() || modified.is_empty() {
                win.save_window_state();
                WINDOWS.with(|w| w.borrow_mut().retain(|o| !Rc::ptr_eq(o, &win)));
                return glib::Propagation::Proceed;
            }
            let window = window.clone();
            glib::spawn_future_local(async move {
                for doc in modified {
                    if let Some(page) = doc.page.borrow().as_ref() {
                        win.tab_view.set_selected_page(page);
                    }
                    if !doc.confirm_discard().await {
                        return;
                    }
                }
                win.force_close.set(true);
                window.close();
            });
            glib::Propagation::Stop
        });

        let weak = self.weak();
        self.recent_popover.connect_show(move |_| {
            if let Some(win) = weak.upgrade() {
                win.recent_search.set_text("");
                win.fill_recents();
                win.recent_search.grab_focus();
            }
        });

        let weak = self.weak();
        self.recent_list.set_filter_func(move |row| {
            let Some(win) = weak.upgrade() else { return true };
            let query = win.recent_search.text().to_lowercase();
            if query.is_empty() {
                return true;
            }
            win.recent_entries
                .borrow()
                .iter()
                .find(|e| &e.row == row)
                .is_none_or(|e| e.haystack.contains(&query))
        });

        let list = self.recent_list.clone();
        self.recent_search.connect_search_changed(move |_| list.invalidate_filter());

        let weak = self.weak();
        self.recent_search.connect_activate(move |_| {
            let Some(win) = weak.upgrade() else { return };
            let first = win
                .recent_entries
                .borrow()
                .iter()
                .find(|e| e.row.is_child_visible())
                .map(|e| e.uri.clone());
            if let Some(uri) = first {
                win.recent_popover.popdown();
                win.open_file(gio::File::for_uri(&uri));
            }
        });

        let weak = self.weak();
        self.recent_list.connect_row_activated(move |_, row| {
            let Some(win) = weak.upgrade() else { return };
            let uri = win.recent_entries.borrow().iter().find(|e| &e.row == row).map(|e| e.uri.clone());
            if let Some(uri) = uri {
                win.recent_popover.popdown();
                win.open_file(gio::File::for_uri(&uri));
            }
        });
    }

    fn fill_recents(self: &Rc<Self>) {
        self.recent_list.remove_all();
        self.recent_entries.borrow_mut().clear();

        let mut items: Vec<gtk4::RecentInfo> = gtk4::RecentManager::default()
            .items()
            .into_iter()
            .filter(|i| i.has_application(APP_NAME) && is_markdown_uri(&i.uri()) && i.exists())
            .collect();
        items.sort_by_key(|i| std::cmp::Reverse(i.modified().to_unix()));

        for info in items.into_iter().take(30) {
            let uri = info.uri().to_string();
            let file = gio::File::for_uri(&uri);
            let name = display_name(&file);
            let folder = display_folder(&file);

            let labels = gtk4::Box::new(Orientation::Vertical, 2);
            labels.set_hexpand(true);
            labels.append(
                &gtk4::Label::builder()
                    .label(&name)
                    .xalign(0.0)
                    .ellipsize(gtk4::pango::EllipsizeMode::End)
                    .build(),
            );
            labels.append(
                &gtk4::Label::builder()
                    .label(&folder)
                    .xalign(0.0)
                    .ellipsize(gtk4::pango::EllipsizeMode::Start)
                    .css_classes(["dimmed", "caption"])
                    .build(),
            );
            let remove = gtk4::Button::builder()
                .icon_name("window-close-symbolic")
                .tooltip_text("Remove From Recent Files")
                .valign(gtk4::Align::Center)
                .css_classes(["flat", "circular"])
                .build();
            let content = gtk4::Box::new(Orientation::Horizontal, 12);
            content.set_margin_top(6);
            content.set_margin_bottom(6);
            content.set_margin_start(6);
            content.append(&labels);
            content.append(&remove);
            let row = gtk4::ListBoxRow::builder().child(&content).build();
            self.recent_list.append(&row);

            let weak = self.weak();
            let row_ref = row.clone();
            let uri_ref = uri.clone();
            remove.connect_clicked(move |_| {
                let _ = gtk4::RecentManager::default().remove_item(&uri_ref);
                if let Some(win) = weak.upgrade() {
                    win.recent_list.remove(&row_ref);
                    win.recent_entries.borrow_mut().retain(|e| e.row != row_ref);
                }
            });

            self.recent_entries.borrow_mut().push(RecentEntry {
                row,
                uri,
                haystack: format!("{name} {folder}").to_lowercase(),
            });
        }
    }

    async fn open_with_dialog(self: &Rc<Self>) {
        let dialog = gtk4::FileDialog::builder()
            .title("Open Markdown File")
            .filters(&md_filters())
            .build();
        if let Some(folder) = self.current().and_then(|d| d.file()).and_then(|f| f.parent()) {
            dialog.set_initial_folder(Some(&folder));
        }
        if let Ok(file) = dialog.open_future(Some(&self.window)).await {
            self.open_file(file);
        }
    }

    fn save_window_state(&self) {
        let (width, height) = self.window.default_size();
        save_pref(PREF_WINDOW_WIDTH, &width.to_string());
        save_pref(PREF_WINDOW_HEIGHT, &height.to_string());
        save_pref_bool(PREF_WINDOW_MAXIMIZED, self.window.is_maximized());
    }
}
