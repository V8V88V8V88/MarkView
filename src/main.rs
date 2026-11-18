mod config;
mod styles;
mod ui;
mod utils;

use adw::prelude::*;
use adw::{Application, StyleManager};
use gtk4::{gio, glib};

use crate::config::*;
use crate::ui::prefs::{color_scheme_from_pref, show_preferences_dialog};
use crate::ui::window::{refresh_appearance_of_all_windows, window_for, windows, MainWindow, APP_NAME};

const APP_ID: &str = "io.github.v8v88v8v88.MarkView";

pub fn apply_accels(app: &Application) {
    for shortcut in SHORTCUTS {
        let accel = load_pref(shortcut.pref_key, shortcut.default);
        let accels: Vec<&str> = if accel.is_empty() { vec![] } else { vec![accel.as_str()] };
        app.set_accels_for_action(shortcut.action, &accels);
    }
}

fn active_window(app: &Application) -> Option<gtk4::Window> {
    app.active_window()
}

fn add_action(app: &Application, name: &str, f: impl Fn(&Application) + 'static) {
    let action = gio::SimpleAction::new(name, None);
    let app_weak = app.downgrade();
    action.connect_activate(move |_, _| {
        if let Some(app) = app_weak.upgrade() {
            f(&app);
        }
    });
    app.add_action(&action);
}

fn show_shortcuts(app: &Application) {
    let dialog = adw::ShortcutsDialog::new();
    let mut sections: Vec<&str> = SHORTCUTS.iter().map(|s| s.section).collect();
    sections.dedup();
    for name in sections {
        let section = adw::ShortcutsSection::new(Some(name));
        for shortcut in SHORTCUTS.iter().filter(|s| s.section == name) {
            section.add(adw::ShortcutsItem::from_action(shortcut.title, shortcut.action));
        }
        dialog.add(section);
    }
    let editing = adw::ShortcutsSection::new(Some("Find and Replace"));
    editing.add(adw::ShortcutsItem::new("Next Match", "Return"));
    editing.add(adw::ShortcutsItem::new("Previous Match", "<Shift>Return"));
    editing.add(adw::ShortcutsItem::new("Close Search", "Escape"));
    dialog.add(editing);
    dialog.present(active_window(app).as_ref());
}

fn show_about(app: &Application) {
    let dialog = adw::AboutDialog::builder()
        .application_name("MarkView")
        .application_icon(APP_ID)
        .version(env!("CARGO_PKG_VERSION"))
        .comments("A Markdown editor with a live preview")
        .developer_name("Vaibhav Pratap Singh")
        .developers(["Vaibhav Pratap Singh"])
        .copyright("© 2026 Vaibhav Pratap Singh")
        .website("https://github.com/v8v88v8v88/MarkView")
        .issue_url("https://github.com/v8v88v8v88/MarkView/issues")
        .license_type(gtk4::License::Gpl20)
        .build();
    dialog.add_acknowledgement_section(
        Some("Built With"),
        &[
            "GTK https://gtk.org",
            "libadwaita https://gnome.pages.gitlab.gnome.org/libadwaita/",
            "GtkSourceView https://gitlab.gnome.org/GNOME/gtksourceview",
            "WebKitGTK https://webkitgtk.org",
            "pulldown-cmark https://github.com/pulldown-cmark/pulldown-cmark",
            "gtk-rs https://gtk-rs.org",
            "Rust https://www.rust-lang.org",
        ],
    );
    for (name, copyright, license) in [
        ("GTK", "© The GTK Team", gtk4::License::Lgpl21),
        ("libadwaita", "© The GNOME Project", gtk4::License::Lgpl21),
        ("GtkSourceView", "© The GtkSourceView Team", gtk4::License::Lgpl21),
        ("WebKitGTK", "© The WebKitGTK Team", gtk4::License::Lgpl21),
        ("pulldown-cmark", "© Raph Levien and contributors", gtk4::License::MitX11),
        ("gtk-rs", "© The gtk-rs Project Developers", gtk4::License::MitX11),
    ] {
        dialog.add_legal_section(name, Some(copyright), license, None);
    }
    dialog.present(active_window(app).as_ref());
}

fn setup(app: &Application) {
    let style = StyleManager::default();
    style.set_color_scheme(color_scheme_from_pref());
    let css = gtk4::CssProvider::new();
    css.load_from_string(styles::APP_CSS);
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(&display, &css, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    }
    let scheme = gio::SimpleAction::new_stateful(
        "color-scheme",
        Some(glib::VariantTy::STRING),
        &load_pref(PREF_THEME, DEFAULT_THEME).to_variant(),
    );
    scheme.connect_activate(|action, target| {
        let Some(name) = target.and_then(|t| t.str()) else { return };
        action.set_state(&name.to_variant());
        save_pref(PREF_THEME, name);
        StyleManager::default().set_color_scheme(color_scheme_from_pref());
    });
    app.add_action(&scheme);
    let refresh = |_: &StyleManager| refresh_appearance_of_all_windows();
    style.connect_dark_notify(refresh);
    style.connect_accent_color_rgba_notify(refresh);
    style.connect_document_font_name_notify(refresh);
    style.connect_monospace_font_name_notify(refresh);
    ui::schemes::init_search_path();
    if let Some(display) = gtk4::gdk::Display::default() {
        let theme = gtk4::IconTheme::for_display(&display);
        let mut dirs = vec![std::path::PathBuf::from("data/icons")];
        if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("data/icons"))) {
            dirs.push(dir);
        }
        for dir in dirs.into_iter().filter(|d| d.is_dir()) {
            theme.add_search_path(dir);
        }
    }
    gtk4::Window::set_default_icon_name(APP_ID);

    add_action(app, "new-window", |app| MainWindow::new(app).present());
    add_action(app, "preferences", |app| {
        if let Some(win) = active_window(app) {
            show_preferences_dialog(&win, app);
        }
    });
    add_action(app, "shortcuts", show_shortcuts);
    add_action(app, "about", show_about);
    add_action(app, "quit", |_| {
        for win in windows() {
            win.window.close();
        }
    });
    apply_accels(app);
}

fn main() -> glib::ExitCode {
    glib::set_application_name(APP_NAME);
    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_startup(setup);

    app.connect_activate(|app| match app.active_window() {
        Some(win) => win.present(),
        None => MainWindow::new(app).present(),
    });

    app.connect_open(|app, files, _| {
        let win = app
            .active_window()
            .and_then(|w| window_for(&w))
            .unwrap_or_else(|| MainWindow::new(app));
        win.present();
        for file in files {
            win.open_file(file.clone());
        }
    });

    app.run()
}
