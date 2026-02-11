use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use adw::{
    AboutDialog, Application, ApplicationWindow, HeaderBar, PreferencesGroup,
    PreferencesDialog, PreferencesPage, ShortcutsDialog, ShortcutsItem, ShortcutsSection,
    SwitchRow,
};
use gtk4::{
    Box, Button, EventControllerKey, MenuButton, Orientation, Paned, PropagationPhase,
    ScrolledWindow,
};
use gtk4::{gio, Settings};
use pulldown_cmark::{html, Options, Parser};
use sourceview5::{prelude::*, Buffer as SourceBuffer, View as SourceView, VimIMContext};
use webkit6::prelude::*;
use webkit6::WebView;

const PREVIEW_CSS: &str = r#"
    :root { color-scheme: light dark; }
    body {
        font-family: 'Cantarell', 'Inter', system-ui, sans-serif;
        font-size: 15px; line-height: 1.7;
        padding: 16px 24px; margin: 0;
        color: #e0e0e0; background: #2a2a2a;
        word-wrap: break-word;
    }
    h1, h2, h3, h4, h5, h6 { color: #fff; margin-top: 1.2em; margin-bottom: 0.4em; font-weight: 600; }
    h1 { font-size: 1.8em; border-bottom: 1px solid #444; padding-bottom: 0.3em; }
    h2 { font-size: 1.5em; border-bottom: 1px solid #3a3a3a; padding-bottom: 0.2em; }
    h3 { font-size: 1.25em; }
    p { margin: 0.6em 0; }
    a { color: #78b9f5; text-decoration: none; }
    a:hover { text-decoration: underline; }
    code {
        font-family: 'JetBrains Mono', 'Source Code Pro', monospace;
        background: #1e1e1e; padding: 2px 6px; border-radius: 4px; font-size: 0.9em;
    }
    pre { background: #1e1e1e; padding: 14px 18px; border-radius: 8px; overflow-x: auto; border: 1px solid #3a3a3a; }
    pre code { background: none; padding: 0; }
    blockquote {
        border-left: 3px solid #78b9f5; margin: 0.8em 0; padding: 0.4em 1em;
        color: #b0b0b0; background: #252525; border-radius: 0 6px 6px 0;
    }
    ul, ol { padding-left: 1.8em; }
    li { margin: 0.25em 0; }
    hr { border: none; border-top: 1px solid #444; margin: 1.5em 0; }
    table { border-collapse: collapse; width: 100%; margin: 1em 0; }
    th, td { border: 1px solid #444; padding: 8px 12px; text-align: left; }
    th { background: #333; font-weight: 600; }
    img { max-width: 100%; border-radius: 6px; }
    strong { color: #f0f0f0; }
    em { color: #d0d0d0; }
"#;

fn build_html_page(body: &str) -> String {
    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><style>{}</style></head><body>{}</body></html>",
        PREVIEW_CSS, body
    )
}

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

fn build_ui(app: &Application) {
    let settings = Settings::default().expect("Failed to get default settings");
    settings.set_gtk_keynav_use_caret(false);
    settings.set_gtk_error_bell(false);

    let current_file: Rc<RefCell<Option<gio::File>>> = Rc::new(RefCell::new(None));
    let vim_controller: Rc<RefCell<Option<EventControllerKey>>> =
        Rc::new(RefCell::new(None));

    // --- Header Bar ---
    let header_bar = HeaderBar::new();

    let open_button = Button::builder()
        .icon_name("document-open-symbolic")
        .tooltip_text("Open (Ctrl+O)")
        .action_name("app.open")
        .build();

    let save_button = Button::builder()
        .icon_name("media-floppy-symbolic")
        .tooltip_text("Save (Ctrl+S)")
        .action_name("app.save")
        .build();

    let export_pdf_button = Button::builder()
        .icon_name("document-save-symbolic")
        .tooltip_text("Export as PDF")
        .action_name("app.export-pdf")
        .build();

    let menu_button = MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .build();

    header_bar.pack_start(&open_button);
    // pack_end adds right-to-left, so menu first, then pdf, then save
    header_bar.pack_end(&menu_button);
    header_bar.pack_end(&export_pdf_button);
    header_bar.pack_end(&save_button);

    // --- Editor (left) ---
    let paned = Paned::builder()
        .orientation(Orientation::Horizontal)
        .vexpand(true)
        .hexpand(true)
        .build();

    let source_view = SourceView::new();
    let source_buffer: SourceBuffer = source_view.buffer().downcast().unwrap();
    source_buffer.set_language(Some(
        &sourceview5::LanguageManager::default()
            .language("markdown")
            .unwrap(),
    ));
    if let Some(scheme) = sourceview5::StyleSchemeManager::default().scheme("Adwaita-dark") {
        source_buffer.set_style_scheme(Some(&scheme));
    }
    source_buffer.set_highlight_syntax(true);
    source_view.set_show_line_numbers(true);
    source_view.set_monospace(true);
    source_view.set_tab_width(4);
    source_view.set_auto_indent(true);
    source_view.set_indent_on_tab(true);
    source_view.set_smart_backspace(true);
    source_view.set_wrap_mode(gtk4::WrapMode::Word);
    source_view.set_top_margin(8);
    source_view.set_bottom_margin(8);
    source_view.set_left_margin(8);
    source_view.set_right_margin(8);

    let editor_scroll = ScrolledWindow::builder()
        .child(&source_view)
        .vexpand(true)
        .hexpand(true)
        .build();

    // --- Preview (right) ---
    let webview = WebView::new();
    webview.set_vexpand(true);
    webview.set_hexpand(true);
    webview.load_html(
        &build_html_page("<p style='color:#888;text-align:center;margin-top:2em;'>Start typing markdown on the left…</p>"),
        None,
    );
    webview.set_background_color(&gtk4::gdk::RGBA::new(0.165, 0.165, 0.165, 1.0));

    let preview_scroll = ScrolledWindow::builder()
        .child(&webview)
        .vexpand(true)
        .hexpand(true)
        .build();

    paned.set_start_child(Some(&editor_scroll));
    paned.set_end_child(Some(&preview_scroll));
    paned.set_position(400);

    // --- Window ---
    let content = Box::new(Orientation::Vertical, 0);
    content.append(&header_bar);
    content.append(&paned);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("MarkView")
        .default_width(1100)
        .default_height(700)
        .content(&content)
        .build();

    // --- Live Preview ---
    let wv = webview.clone();
    source_buffer.connect_changed(move |buffer| {
        let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
        let parser = Parser::new_ext(&text, Options::all());
        let mut html_out = String::new();
        html::push_html(&mut html_out, parser);
        wv.load_html(&build_html_page(&html_out), None);
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
    app_sec.append(Some("Keyboard Shortcuts"), Some("app.shortcuts"));
    app_sec.append(Some("About"), Some("app.about"));
    app_sec.append(Some("Quit"), Some("app.quit"));
    menu.append_section(None, &app_sec);
    menu_button.set_menu_model(Some(&menu));

    // === Actions ===

    // Open
    let open_action = gio::SimpleAction::new("open", None);
    {
        let w = window.clone();
        let buf = source_buffer.clone();
        let cf = current_file.clone();
        open_action.connect_activate(move |_, _| {
            let dialog = gtk4::FileDialog::builder()
                .title("Open Markdown File")
                .build();
            dialog.set_filters(Some(&create_md_filters()));
            let buf = buf.clone();
            let cf = cf.clone();
            let w = w.clone();
            let w_inner = w.clone();
            dialog.open(Some(&w), None::<&gio::Cancellable>, move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        match std::fs::read_to_string(&path) {
                            Ok(content) => {
                                buf.set_text(&content);
                                if let Some(name) = path.file_name() {
                                    w_inner.set_title(Some(&format!("{} — MarkView", name.to_string_lossy())));
                                }
                                *cf.borrow_mut() = Some(file);
                            }
                            Err(e) => eprintln!("Failed to read file: {e}"),
                        }
                    }
                }
            });
        });
    }
    app.add_action(&open_action);

    // Save
    let save_action = gio::SimpleAction::new("save", None);
    {
        let w = window.clone();
        let buf = source_buffer.clone();
        let cf = current_file.clone();
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
                // No file yet — show Save As dialog
                let dialog = gtk4::FileDialog::builder()
                    .title("Save Markdown File")
                    .initial_name("untitled.md")
                    .build();
                dialog.set_filters(Some(&create_md_filters()));
                let buf = buf.clone();
                let cf = cf.clone();
                let w = w.clone();
                let w_inner = w.clone();
                dialog.save(Some(&w), None::<&gio::Cancellable>, move |result| {
                    if let Ok(file) = result {
                        if let Some(path) = file.path() {
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
                    }
                });
            }
        });
    }
    app.add_action(&save_action);

    // Save As
    let save_as_action = gio::SimpleAction::new("save-as", None);
    {
        let w = window.clone();
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
            let w = w.clone();
            let w_inner = w.clone();
            dialog.save(Some(&w), None::<&gio::Cancellable>, move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
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
                }
            });
        });
    }
    app.add_action(&save_as_action);

    // Export PDF
    let export_pdf_action = gio::SimpleAction::new("export-pdf", None);
    {
        let wv = webview.clone();
        let w = window.clone();
        export_pdf_action.connect_activate(move |_, _| {
            let dialog = gtk4::FileDialog::builder()
                .title("Export as PDF")
                .initial_name("document.pdf")
                .build();
            dialog.set_filters(Some(&create_pdf_filters()));
            let wv = wv.clone();
            let w = w.clone();
            let w_parent = w.clone();
            dialog.save(Some(&w_parent), None::<&gio::Cancellable>, move |result| {
                if let Ok(file) = result {
                    let uri = file.uri().to_string();
                    let settings = gtk4::PrintSettings::new();
                    settings.set(gtk4::PRINT_SETTINGS_OUTPUT_URI.as_str(), Some(uri.as_str()));
                    settings.set(
                        gtk4::PRINT_SETTINGS_OUTPUT_FILE_FORMAT.as_str(),
                        Some("PDF"),
                    );
                    let print_op = webkit6::PrintOperation::new(&wv);
                    print_op.set_print_settings(&settings);
                    print_op.run_dialog(Some(&w));
                }
            });
        });
    }
    app.add_action(&export_pdf_action);

    // Preferences
    let preferences_action = gio::SimpleAction::new("preferences", None);
    {
        let w = window.clone();
        let sv = source_view.clone();
        let vc = vim_controller.clone();
        preferences_action.connect_activate(move |_, _| {
            let vim_row = SwitchRow::builder()
                .title("Vim keybindings")
                .subtitle("Use Vim-style keybindings in the editor")
                .active(false)
                .build();
            let line_numbers_row = SwitchRow::builder()
                .title("Show line numbers")
                .subtitle("Display line numbers in the gutter")
                .active(true)
                .build();
            let word_wrap_row = SwitchRow::builder()
                .title("Word wrap")
                .subtitle("Wrap long lines at word boundaries")
                .active(true)
                .build();
            vim_row.set_active(vc.borrow().is_some());
            line_numbers_row.set_active(sv.shows_line_numbers());
            word_wrap_row.set_active(sv.wrap_mode() == gtk4::WrapMode::Word);
            vim_row.connect_active_notify({
                let sv = sv.clone();
                let vc = vc.clone();
                move |row| {
                    if row.is_active() {
                        let vim_ctx = VimIMContext::new();
                        vim_ctx.set_client_widget(Some(&sv));
                        let key_ctrl = EventControllerKey::new();
                        key_ctrl.set_propagation_phase(PropagationPhase::Capture);
                        key_ctrl.set_im_context(Some(&vim_ctx));
                        let ctrl_clone = key_ctrl.clone();
                        sv.add_controller(ctrl_clone);
                        *vc.borrow_mut() = Some(key_ctrl);
                    } else if let Some(ref ctrl) = *vc.borrow() {
                        sv.remove_controller(ctrl);
                        *vc.borrow_mut() = None;
                    }
                }
            });
            line_numbers_row.connect_active_notify({
                let sv = sv.clone();
                move |row| sv.set_show_line_numbers(row.is_active())
            });
            word_wrap_row.connect_active_notify({
                let sv = sv.clone();
                move |row| {
                    sv.set_wrap_mode(if row.is_active() {
                        gtk4::WrapMode::Word
                    } else {
                        gtk4::WrapMode::None
                    });
                }
            });
            let editor_group = PreferencesGroup::new();
            editor_group.add(&vim_row);
            editor_group.add(&line_numbers_row);
            editor_group.add(&word_wrap_row);
            let editor_page = PreferencesPage::builder()
                .title("Editor")
                .icon_name("accessories-text-editor-symbolic")
                .build();
            editor_page.add(&editor_group);
            let prefs = PreferencesDialog::builder()
                .title("Preferences")
                .build();
            prefs.add(&editor_page);
            prefs.present(Some(&w));
        });
    }
    app.add_action(&preferences_action);

    // About
    let about_action = gio::SimpleAction::new("about", None);
    {
        let w = window.clone();
        about_action.connect_activate(move |_, _| {
            let dlg = AboutDialog::builder()
                .application_name("MarkView")
                .version("1.0")
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
        let w = window.clone();
        shortcuts_action.connect_activate(move |_, _| {
            let file_section = ShortcutsSection::new(Some("File"));
            file_section.add(ShortcutsItem::from_action("Open", "app.open"));
            file_section.add(ShortcutsItem::from_action("Save", "app.save"));
            file_section.add(ShortcutsItem::from_action("Save As", "app.save-as"));
            file_section.add(ShortcutsItem::from_action("Export as PDF", "app.export-pdf"));
                        let app_section = ShortcutsSection::new(Some("Application"));
                        app_section.add(ShortcutsItem::from_action("Preferences", "app.preferences"));
                        app_section.add(ShortcutsItem::from_action("Keyboard Shortcuts", "app.shortcuts"));
                        app_section.add(ShortcutsItem::from_action("Quit", "app.quit"));
            let dlg = ShortcutsDialog::builder()
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
        let w = window.clone();
        quit_action.connect_activate(move |_, _| w.close());
    }
    app.add_action(&quit_action);

    app.set_accels_for_action("app.open", &["<Control>o"]);
    app.set_accels_for_action("app.save", &["<Control>s"]);
    app.set_accels_for_action("app.save-as", &["<Control><Shift>s"]);
    app.set_accels_for_action("app.export-pdf", &["<Control>p"]);
    app.set_accels_for_action("app.preferences", &["<Control>comma"]);
    app.set_accels_for_action("app.shortcuts", &["<Control>question"]);
    app.set_accels_for_action("app.quit", &["<Control>q"]);

    window.present();
}

fn main() {
    let app = Application::builder()
        .application_id("com.example.MarkView")
        .build();
    app.connect_activate(build_ui);
    app.run();
}