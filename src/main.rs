mod config;
mod styles;
mod utils;
mod ui;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Once;

use adw::prelude::*;
use adw::{Application, StyleManager};
use gtk4::{gio, Settings, EventControllerKey, PropagationPhase};
use sourceview5::{prelude::ViewExt, Buffer as SourceBuffer, VimIMContext};
use webkit6::prelude::*;

use crate::styles::SEARCH_BAR_CSS;

use crate::config::*;
use crate::ui::window::MarkViewWindow;
use crate::ui::search::SearchController;
use crate::ui::preview::refresh_preview;
use crate::ui::prefs::show_preferences_dialog;

fn create_md_filters() -> gio::ListStore {
    let md = gtk4::FileFilter::new();
    md.add_pattern("*.md");
    md.add_pattern("*.markdown");
    md.set_name(Some("Markdown Files"));
    let all = gtk4::FileFilter::new();
    all.add_pattern("*");
    all.set_name(Some("All Files"));
    let filters = gio::ListStore::new::<gtk4::FileFilter>();
    filters.append(&md);
    filters.append(&all);
    filters
}

fn create_pdf_filters() -> gio::ListStore {
    let pdf = gtk4::FileFilter::new();
    pdf.add_mime_type("application/pdf");
    pdf.add_pattern("*.pdf");
    pdf.set_name(Some("PDF"));
    let all = gtk4::FileFilter::new();
    all.add_pattern("*");
    all.set_name(Some("All Files"));
    let filters = gio::ListStore::new::<gtk4::FileFilter>();
    filters.append(&pdf);
    filters.append(&all);
    filters
}

pub fn apply_accels(app: &Application) {
    app.set_accels_for_action("app.open", &[&load_pref(PREF_KEY_OPEN, DEFAULT_KEY_OPEN)]);
    app.set_accels_for_action("app.save", &[&load_pref(PREF_KEY_SAVE, DEFAULT_KEY_SAVE)]);
    app.set_accels_for_action("app.save-as", &[&load_pref(PREF_KEY_SAVE_AS, DEFAULT_KEY_SAVE_AS)]);
    app.set_accels_for_action("app.export-pdf", &[&load_pref(PREF_KEY_EXPORT_PDF, DEFAULT_KEY_EXPORT_PDF)]);
    app.set_accels_for_action("app.preferences", &[&load_pref(PREF_KEY_PREFS, DEFAULT_KEY_PREFS)]);
    app.set_accels_for_action("app.shortcuts", &[&load_pref(PREF_KEY_SHORTCUTS, DEFAULT_KEY_SHORTCUTS)]);
    app.set_accels_for_action("app.search", &[&load_pref(PREF_KEY_SEARCH, DEFAULT_KEY_SEARCH)]);
    app.set_accels_for_action("app.toggle-editor", &[&load_pref(PREF_KEY_TOGGLE_EDITOR, DEFAULT_KEY_TOGGLE_EDITOR)]);
    app.set_accels_for_action("app.toggle-readable", &[&load_pref(PREF_KEY_TOGGLE_READABLE, DEFAULT_KEY_TOGGLE_READABLE)]);
    app.set_accels_for_action("app.cycle-view", &[&load_pref(PREF_KEY_CYCLE_VIEW, DEFAULT_KEY_CYCLE_VIEW)]);
    app.set_accels_for_action("app.quit", &[&load_pref(PREF_KEY_QUIT, DEFAULT_KEY_QUIT)]);
}

fn build_ui(app: &Application, initial_file: Option<gio::File>) {
    let settings = Settings::default().expect("Failed to get default settings");
    settings.set_gtk_keynav_use_caret(false);
    settings.set_gtk_error_bell(false);

    let current_file = Rc::new(RefCell::new(initial_file.clone()));
    let vim_controller: Rc<RefCell<Option<EventControllerKey>>> = Rc::new(RefCell::new(None));

    let ui = MarkViewWindow::new(app);
    let source_buffer = ui.source_view.buffer().downcast::<SourceBuffer>().unwrap();

    static CSS_INIT: Once = Once::new();
    CSS_INIT.call_once(|| {
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(SEARCH_BAR_CSS);
        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    if load_pref(PREF_LINE_NUMBERS, DEFAULT_LINE_NUMBERS) == "false" {
        ui.source_view.set_show_line_numbers(false);
    }
    if load_pref(PREF_WORD_WRAP, DEFAULT_WORD_WRAP) == "false" {
        ui.source_view.set_wrap_mode(gtk4::WrapMode::None);
    }
    if load_pref(PREF_VIM_MODE, DEFAULT_VIM_MODE) == "true" {
        let vim_ctx = VimIMContext::new();
        vim_ctx.set_client_widget(Some(&ui.source_view));
        let key_ctrl = EventControllerKey::new();
        key_ctrl.set_propagation_phase(PropagationPhase::Capture);
        key_ctrl.set_im_context(Some(&vim_ctx));
        let ctrl_clone = key_ctrl.clone();
        ui.source_view.add_controller(ctrl_clone);
        *vim_controller.borrow_mut() = Some(key_ctrl);
    }
    if load_pref(PREF_DEFAULT_VIEW, DEFAULT_VIEW) == "preview-only" {
        ui.paned.set_position(0);
    }

    // --- Search & Replace ---
    let search_ctrl = SearchController::new(&ui.source_view, &source_buffer);
    search_ctrl.setup_overlay(&ui.overlay);

    // --- Header Buttons ---
    let open_button = gtk4::Button::builder()
        .icon_name("document-open-symbolic")
        .tooltip_text("Open (Ctrl+O)")
        .action_name("app.open")
        .build();

    let sidebar_toggle = gtk4::Button::builder()
        .icon_name("view-dual-symbolic")
        .tooltip_text("Hide left panel")
        .build();

    let save_button = gtk4::Button::builder()
        .icon_name("media-floppy-symbolic")
        .tooltip_text("Save (Ctrl+S)")
        .action_name("app.save")
        .build();

    let export_pdf_button = gtk4::Button::builder()
        .icon_name("document-save-symbolic")
        .tooltip_text("Export as PDF")
        .action_name("app.export-pdf")
        .build();

    let menu_button = gtk4::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .build();

    ui.header_bar.pack_start(&open_button);
    ui.header_bar.pack_start(&sidebar_toggle);
    ui.header_bar.pack_end(&menu_button);
    ui.header_bar.pack_end(&export_pdf_button);
    ui.header_bar.pack_end(&save_button);

    // --- Sync Scroll ---
    {
        let editor_adj = ui.editor_scroll.vadjustment();
        let wv = ui.webview.clone();

        let sync_scroll: Rc<dyn Fn()> = {
            let editor_adj = editor_adj.clone();
            let wv = wv.clone();
            Rc::new(move || {
                let value = editor_adj.value();
                let upper = editor_adj.upper();
                let page_size = editor_adj.page_size();
                if upper > page_size {
                    let percent = value / (upper - page_size);
                    let script = format!(
                        "window.scrollTo(0, (document.documentElement.scrollHeight - window.innerHeight) * {});",
                        percent
                    );
                    wv.evaluate_javascript(&script, None, None, None::<&gio::Cancellable>, |_| {});
                }
            })
        };

        editor_adj.connect_value_changed({
            let sync_scroll = sync_scroll.clone();
            move |_| {
                if load_pref(PREF_SYNC_SCROLL, DEFAULT_SYNC_SCROLL) == "true" {
                    sync_scroll();
                }
            }
        });

        ui.webview.connect_load_changed({
            let sync_scroll = sync_scroll.clone();
            move |_, event| {
                if event == webkit6::LoadEvent::Finished
                    && load_pref(PREF_SYNC_SCROLL, DEFAULT_SYNC_SCROLL) == "true"
                {
                    sync_scroll();
                }
            }
        });
    }

    // --- Live Preview ---
    let pending_update: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));
    let is_first_render = Rc::new(RefCell::new(true));

    let refresh_preview_fn = {
        let wv = ui.webview.clone();
        let cf = current_file.clone();
        let first = is_first_render.clone();
        Rc::new(move |buffer: &SourceBuffer, force: bool| {
            refresh_preview(&wv, buffer, &cf, &first, force);
        })
    };

    source_buffer.connect_changed({
        let refresh = refresh_preview_fn.clone();
        let pending = pending_update.clone();
        move |buffer| {
            if let Some(source_id) = pending.borrow_mut().take() {
                source_id.remove();
            }

            let debounce_ms = load_pref(PREF_DEBOUNCE, DEFAULT_DEBOUNCE)
                .parse::<f64>()
                .unwrap_or(150.0) as u32;

            if debounce_ms == 0 {
                refresh(buffer, false);
            } else {
                let buffer = buffer.clone();
                let refresh = refresh.clone();
                let pending_inner = pending.clone();

                let source_id = glib::timeout_add_local(
                    std::time::Duration::from_millis(debounce_ms as u64),
                    move || {
                        refresh(&buffer, false);
                        *pending_inner.borrow_mut() = None;
                        glib::ControlFlow::Break
                    },
                );
                *pending.borrow_mut() = Some(source_id);
            }
        }
    });

    StyleManager::default().connect_dark_notify({
        let sb = source_buffer.clone();
        let refresh = refresh_preview_fn.clone();
        move |_| {
            refresh(&sb, true);
        }
    });

    // --- Menu ---
    let menu = gio::Menu::new();
    let file_sec = gio::Menu::new();
    file_sec.append(Some("Open…"), Some("app.open"));
    file_sec.append(Some("Save As…"), Some("app.save-as"));
    file_sec.append(Some("Export as PDF…"), Some("app.export-pdf"));
    menu.append_section(None, &file_sec);
    let app_sec = gio::Menu::new();
    app_sec.append(Some("Preferences"), Some("app.preferences"));
    app_sec.append(Some("Keyboard Shortcuts Dialog"), Some("app.shortcuts"));
    app_sec.append(Some("About"), Some("app.about"));
    app_sec.append(Some("Quit"), Some("app.quit"));
    menu.append_section(None, &app_sec);
    menu_button.set_menu_model(Some(&menu));

    // --- Actions ---
    
    // Open
    let open_action = gio::SimpleAction::new("open", None);
    {
        let w = ui.window.clone();
        let buf = source_buffer.clone();
        let cf = current_file.clone();
        let is_first = is_first_render.clone();
        open_action.connect_activate(move |_, _| {
            let dialog = gtk4::FileDialog::builder()
                .title("Open Markdown File")
                .build();
            dialog.set_filters(Some(&create_md_filters()));
            let buf = buf.clone();
            let cf = cf.clone();
            let is_first = is_first.clone();
            let w = w.clone();
            let w_inner = w.clone();
            dialog.open(Some(&w), None::<&gio::Cancellable>, move |result| {
                if let Ok(file) = result
                    && let Some(path) = file.path()
                {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => {
                            *cf.borrow_mut() = Some(file);
                            *is_first.borrow_mut() = true;
                            buf.set_text(&content);
                            if let Some(name) = path.file_name() {
                                w_inner.set_title(Some(&format!("{} — MarkView", name.to_string_lossy())));
                            }
                        }
                        Err(e) => eprintln!("Failed to read file: {e}"),
                    }
                }
            });
        });
    }
    app.add_action(&open_action);

    // Save
    let save_action = gio::SimpleAction::new("save", None);
    {
        let buf = source_buffer.clone();
        let cf = current_file.clone();
        let app_clone = app.clone();
        save_action.connect_activate(move |_, _| {
            let file_opt = cf.borrow().clone();
            if let Some(file) = file_opt {
                if let Some(path) = file.path() {
                    let text = buf.text(&buf.start_iter(), &buf.end_iter(), false);
                    if let Err(e) = std::fs::write(&path, text.as_str()) {
                        eprintln!("Failed to save: {e}");
                    }
                }
            } else {
                app_clone.activate_action("save-as", None);
            }
        });
    }
    app.add_action(&save_action);

    // Save As
    let save_as_action = gio::SimpleAction::new("save-as", None);
    {
        let w = ui.window.clone();
        let buf = source_buffer.clone();
        let cf = current_file.clone();
        save_as_action.connect_activate(move |_, _| {
            let current = cf.borrow().clone();
            let dialog = if let Some(ref f) = current {
                gtk4::FileDialog::builder()
                    .title("Save Markdown File")
                    .initial_file(f)
                    .build()
            } else {
                gtk4::FileDialog::builder()
                    .title("Save Markdown File")
                    .initial_name("untitled.md")
                    .build()
            };
            dialog.set_filters(Some(&create_md_filters()));
            let buf = buf.clone();
            let cf = cf.clone();
            let w_inner = w.clone();
            dialog.save(Some(&w), None::<&gio::Cancellable>, move |result| {
                if let Ok(file) = result
                    && let Some(path) = file.path()
                {
                    let text = buf.text(&buf.start_iter(), &buf.end_iter(), false);
                    match std::fs::write(&path, text.as_str()) {
                        Ok(_) => {
                            if let Some(name) = path.file_name() {
                                w_inner.set_title(Some(&format!("{} — MarkView", name.to_string_lossy())));
                            }
                            *cf.borrow_mut() = Some(file);
                        }
                        Err(e) => eprintln!("Failed to save: {e}"),
                    }
                }
            });
        });
    }
    app.add_action(&save_as_action);

    // Export PDF
    let export_pdf_action = gio::SimpleAction::new("export-pdf", None);
    {
        let wv = ui.webview.clone();
        let w = ui.window.clone();
        export_pdf_action.connect_activate(move |_, _| {
            let dialog = gtk4::FileDialog::builder()
                .title("Export as PDF")
                .initial_name("document.pdf")
                .build();
            dialog.set_filters(Some(&create_pdf_filters()));
            let wv = wv.clone();
            let w_parent = w.clone();
            let w_parent_inner = w_parent.clone();
            dialog.save(Some(&w_parent), None::<&gio::Cancellable>, move |result| {
                if let Ok(file) = result {
                    let uri = file.uri().to_string();
                    let settings = gtk4::PrintSettings::new();
                    settings.set(gtk4::PRINT_SETTINGS_OUTPUT_URI.as_str(), Some(uri.as_str()));
                    settings.set(
                        gtk4::PRINT_SETTINGS_OUTPUT_FILE_FORMAT.as_str(),
                        Some("PDF"),
                    );
                    let page_setup = gtk4::PageSetup::new();
                    page_setup.set_top_margin(0.0, gtk4::Unit::Mm);
                    page_setup.set_bottom_margin(0.0, gtk4::Unit::Mm);
                    page_setup.set_left_margin(0.0, gtk4::Unit::Mm);
                    page_setup.set_right_margin(0.0, gtk4::Unit::Mm);
                    let print_op = webkit6::PrintOperation::new(&wv);
                    print_op.set_print_settings(&settings);
                    print_op.set_page_setup(&page_setup);
                    print_op.run_dialog(Some(&w_parent_inner));
                }
            });
        });
    }
    app.add_action(&export_pdf_action);

    // Search
    let search_action = gio::SimpleAction::new("search", None);
    {
        let sb = search_ctrl.search_bar.clone();
        search_action.connect_activate(move |_, _| {
            let is_visible = sb.property::<bool>("search-mode-enabled");
            sb.set_property("search-mode-enabled", !is_visible);
        });
    }
    app.add_action(&search_action);

    // Preferences
    let preferences_action = gio::SimpleAction::new("preferences", None);
    {
        let w = ui.window.clone();
        let sv = ui.source_view.clone();
        let sb = source_buffer.clone();
        let vc = vim_controller.clone();
        let app_ptr = app.clone();
        let refresh = refresh_preview_fn.clone();
        preferences_action.connect_activate(move |_, _| {
            show_preferences_dialog(&w, &sv, &sb, &vc, &app_ptr, refresh.clone());
        });
    }
    app.add_action(&preferences_action);

    // Toggle Editor
    let toggle_editor_action = gio::SimpleAction::new("toggle-editor", None);
    {
        let paned = ui.paned.clone();
        let sidebar_btn = sidebar_toggle.clone();
        let saved_pos = Rc::new(RefCell::new(600));
        toggle_editor_action.connect_activate(move |_, _| {
            let current = paned.position();
            if current == 0 {
                paned.set_position(*saved_pos.borrow());
                sidebar_btn.set_icon_name("view-dual-symbolic");
            } else {
                *saved_pos.borrow_mut() = current;
                paned.set_position(0);
                sidebar_btn.set_icon_name("sidebar-show-symbolic");
            }
        });
    }
    app.add_action(&toggle_editor_action);

    {
        let app_ptr = app.clone();
        sidebar_toggle.connect_clicked(move |_| {
            app_ptr.activate_action("toggle-editor", None);
        });
    }

    // Cycle View (3-way: dual → preview-only → editor-only)
    let cycle_view_action = gio::SimpleAction::new("cycle-view", None);
    {
        let paned = ui.paned.clone();
        let initial = if load_pref(PREF_DEFAULT_VIEW, DEFAULT_VIEW) == "preview-only" { 1u8 } else { 0u8 };
        let cycle_state = Rc::new(Cell::new(initial));
        let saved_dual_pos = Rc::new(Cell::new(600i32));
        cycle_view_action.connect_activate(move |_, _| {
            if cycle_state.get() == 0 {
                let pos = paned.position();
                if pos > 0 {
                    saved_dual_pos.set(pos);
                }
            }
            let next = (cycle_state.get() + 1) % 3;
            cycle_state.set(next);
            match next {
                0 => paned.set_position(saved_dual_pos.get()),
                1 => paned.set_position(0),
                _ => paned.set_position(99_999),
            }
        });
    }
    app.add_action(&cycle_view_action);

    // Toggle Readable Line Length
    let toggle_readable_action = gio::SimpleAction::new("toggle-readable", None);
    {
        let refresh = refresh_preview_fn.clone();
        let sb = source_buffer.clone();
        toggle_readable_action.connect_activate(move |_, _| {
            let current = load_pref(PREF_READABLE_LINE, DEFAULT_READABLE_LINE);
            save_pref(PREF_READABLE_LINE, if current == "true" { "false" } else { "true" });
            refresh(&sb, true);
        });
    }
    app.add_action(&toggle_readable_action);

    // About
    let about_action = gio::SimpleAction::new("about", None);
    {
        let w = ui.window.clone();
        about_action.connect_activate(move |_, _| {
            let dlg = adw::AboutDialog::builder()
                .application_name("MarkView")
                .version("1.1.0")
                .developer_name("Vaibhav Pratap Singh")
                .developers(vec!["Vaibhav Pratap Singh"])
                .copyright("© 2026")
                .website("https://github.com/v8v88v8v88/MarkView")
                .license_type(gtk4::License::Gpl30)
                .build();
            dlg.present(Some(&w));
        });
    }
    app.add_action(&about_action);

    // Keyboard Shortcuts
    let shortcuts_action = gio::SimpleAction::new("shortcuts", None);
    {
        let w = ui.window.clone();
        shortcuts_action.connect_activate(move |_, _| {
            let file_section = adw::ShortcutsSection::new(Some("File"));
            file_section.add(adw::ShortcutsItem::from_action("Open", "app.open"));
            file_section.add(adw::ShortcutsItem::from_action("Save", "app.save"));
            file_section.add(adw::ShortcutsItem::from_action("Save As", "app.save-as"));
            file_section.add(adw::ShortcutsItem::from_action("Export as PDF", "app.export-pdf"));
            
            let app_section = adw::ShortcutsSection::new(Some("Application"));
            app_section.add(adw::ShortcutsItem::from_action("Preferences", "app.preferences"));
            app_section.add(adw::ShortcutsItem::from_action("Keyboard Shortcuts", "app.shortcuts"));
            app_section.add(adw::ShortcutsItem::from_action("Quit", "app.quit"));
            
            let dlg = adw::ShortcutsDialog::builder()
                .title("Keyboard Shortcuts")
                .build();
            dlg.add(file_section);
            dlg.add(app_section);
            dlg.present(Some(&w));
        });
    }
    app.add_action(&shortcuts_action);

    // Quit
    let quit_action = gio::SimpleAction::new("quit", None);
    {
        let w = ui.window.clone();
        quit_action.connect_activate(move |_, _| w.close());
    }
    app.add_action(&quit_action);

    apply_accels(app);

    ui.window.present();

    if let Some(file) = initial_file
        && let Some(path) = file.path()
    {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                source_buffer.set_text(&content);
                if let Some(name) = path.file_name() {
                    ui.window.set_title(Some(&format!("{} — MarkView", name.to_string_lossy())));
                }
            }
            Err(e) => eprintln!("Failed to read file: {e}"),
        }
    }
}

fn main() {
    let app = Application::builder()
        .application_id("com.v8v88v8v88.MarkView")
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_activate(|app| {
        build_ui(app, None);
    });

    app.connect_open(|app, files, _| {
        if let Some(file) = files.first() {
            build_ui(app, Some(file.clone()));
        }
    });

    app.run();
}
