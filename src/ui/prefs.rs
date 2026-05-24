use adw::prelude::*;
use adw::{
    ActionRow, Application, ApplicationWindow, ColorScheme, ComboRow, PreferencesDialog,
    PreferencesGroup, PreferencesPage, StyleManager, SwitchRow, SpinRow,
};
use gtk4::{gio, Box, Button, Orientation, PropertyExpression, StringObject, EventControllerKey, PropagationPhase};
use std::cell::RefCell;
use std::rc::Rc;
use sourceview5::prelude::*;
use sourceview5::{Buffer as SourceBuffer, View as SourceView, VimIMContext};

use crate::config::*;
use crate::utils::html_escape;

type PreviewFn = Rc<dyn Fn(&SourceBuffer, bool)>;

pub fn show_preferences_dialog(
    parent_win: &ApplicationWindow,
    source_view: &SourceView,
    source_buffer: &SourceBuffer,
    vim_controller: &Rc<RefCell<Option<EventControllerKey>>>,
    app: &Application,
    refresh_preview: PreviewFn,
) {
    let theme_model = gio::ListStore::new::<StringObject>();
    theme_model.append(&StringObject::new("Auto"));
    theme_model.append(&StringObject::new("Dark"));
    theme_model.append(&StringObject::new("Light"));
    let theme_expr = PropertyExpression::new(StringObject::static_type(), None::<&gtk4::Expression>, "string");
    let theme_row = ComboRow::builder()
        .title("Theme")
        .subtitle("Application appearance")
        .model(&theme_model)
        .expression(&theme_expr)
        .build();
    let saved_theme = load_pref(PREF_THEME, DEFAULT_THEME);
    theme_row.set_selected(match saved_theme.as_str() {
        "force-dark" => 1,
        "force-light" => 2,
        _ => 0,
    });
    let style_mgr = StyleManager::default();
    theme_row.connect_selected_notify({
        let style_mgr = style_mgr.clone();
        move |row| {
            let scheme = match row.selected() {
                1 => ColorScheme::ForceDark,
                2 => ColorScheme::ForceLight,
                _ => ColorScheme::Default,
            };
            style_mgr.set_color_scheme(scheme);
            save_pref(
                PREF_THEME,
                match scheme {
                    ColorScheme::ForceDark => "force-dark",
                    ColorScheme::ForceLight => "force-light",
                    _ => "default",
                },
            );
        }
    });

    let scheme_mgr = sourceview5::StyleSchemeManager::default();
    scheme_mgr.append_search_path("data/styles");
    let mut all_ids: Vec<_> = scheme_mgr.scheme_ids();
    all_ids.sort_by_key(|a| a.to_lowercase());
    let scheme_model = gio::ListStore::new::<StringObject>();
    let mut scheme_ids = Vec::new();
    for id in all_ids.iter() {
        if let Some(scheme) = scheme_mgr.scheme(id.as_str()) {
            scheme_model.append(&StringObject::new(scheme.name().as_ref()));
            scheme_ids.push(id.to_string());
        }
    }
    if scheme_ids.is_empty() {
        for id in all_ids.iter().take(10) {
            if let Some(scheme) = scheme_mgr.scheme(id.as_str()) {
                scheme_model.append(&StringObject::new(scheme.name().as_ref()));
                scheme_ids.push(id.to_string());
            }
        }
    }
    if scheme_ids.is_empty() {
        scheme_model.append(&StringObject::new("Adwaita dark"));
        scheme_ids.push("Adwaita-dark".to_string());
    }
    let scheme_expr = PropertyExpression::new(StringObject::static_type(), None::<&gtk4::Expression>, "string");
    let scheme_row = ComboRow::builder()
        .title("Editor color scheme")
        .subtitle("Syntax highlighting theme")
        .model(&scheme_model)
        .expression(&scheme_expr)
        .build();
    let saved_scheme = load_pref(PREF_SCHEME, DEFAULT_SCHEME);
    let scheme_idx = scheme_ids.iter().position(|s| s == &saved_scheme).unwrap_or(0);
    scheme_row.set_selected(scheme_idx as u32);
    let scheme_ids = Rc::new(scheme_ids);
    scheme_row.connect_selected_notify({
        let sb = source_buffer.clone();
        let scheme_ids = scheme_ids.clone();
        move |row| {
            let idx = row.selected() as usize;
            if let Some(id) = scheme_ids.get(idx) {
                let mgr = sourceview5::StyleSchemeManager::default();
                if let Some(scheme) = mgr.scheme(id) {
                    sb.set_style_scheme(Some(&scheme));
                    save_pref(PREF_SCHEME, id);
                }
            }
        }
    });

    let appearance_group = PreferencesGroup::new();
    appearance_group.set_title("Appearance");
    appearance_group.add(&theme_row);
    appearance_group.add(&scheme_row);
    let appearance_page = PreferencesPage::builder()
        .title("Appearance")
        .icon_name("preferences-desktop-theme-symbolic")
        .build();
    appearance_page.add(&appearance_group);

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
    vim_row.set_active(vim_controller.borrow().is_some());
    line_numbers_row.set_active(source_view.shows_line_numbers());
    word_wrap_row.set_active(source_view.wrap_mode() == gtk4::WrapMode::Word);
    vim_row.connect_active_notify({
        let sv = source_view.clone();
        let vc = vim_controller.clone();
        move |row| {
            save_pref(PREF_VIM_MODE, if row.is_active() { "true" } else { "false" });
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
        let sv = source_view.clone();
        let refresh_preview = refresh_preview.clone();
        let sb = source_buffer.clone();
        move |row| {
            save_pref(PREF_LINE_NUMBERS, if row.is_active() { "true" } else { "false" });
            sv.set_show_line_numbers(row.is_active());
            refresh_preview(&sb, true);
        }
    });
    word_wrap_row.connect_active_notify({
        let sv = source_view.clone();
        move |row| {
            save_pref(PREF_WORD_WRAP, if row.is_active() { "true" } else { "false" });
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

    // Synchronization Page
    let sync_group = PreferencesGroup::new();
    sync_group.set_title("Synchronization");

    let sync_scroll_row = SwitchRow::builder()
        .title("Sync Scroll")
        .subtitle("Preview pane follows editor scroll")
        .active(load_pref(PREF_SYNC_SCROLL, DEFAULT_SYNC_SCROLL) == "true")
        .build();
    sync_scroll_row.connect_active_notify(|row| {
        save_pref(PREF_SYNC_SCROLL, if row.is_active() { "true" } else { "false" });
    });

    let dom_injection_row = SwitchRow::builder()
        .title("DOM Injection")
        .subtitle("Smoothly update preview without reloading")
        .active(load_pref(PREF_DOM_INJECTION, DEFAULT_DOM_INJECTION) == "true")
        .build();
    dom_injection_row.connect_active_notify(|row| {
        save_pref(PREF_DOM_INJECTION, if row.is_active() { "true" } else { "false" });
    });

    let debounce_adjustment = gtk4::Adjustment::new(
        load_pref(PREF_DEBOUNCE, DEFAULT_DEBOUNCE).parse::<f64>().unwrap_or(150.0),
        0.0, 5000.0, 10.0, 100.0, 0.0
    );
    let debounce_row = SpinRow::builder()
        .title("Debounce Duration (ms)")
        .subtitle("Delay before updating preview (0 for live)")
        .adjustment(&debounce_adjustment)
        .build();
    debounce_row.connect_value_notify(|row| {
        save_pref(PREF_DEBOUNCE, &format!("{:.0}", row.value()));
    });

    sync_group.add(&sync_scroll_row);
    sync_group.add(&dom_injection_row);
    sync_group.add(&debounce_row);

    let sync_page = PreferencesPage::builder()
        .title("Synchronization")
        .icon_name("view-refresh-symbolic")
        .build();
    sync_page.add(&sync_group);

    // Interface Page
    let ui_group = PreferencesGroup::new();
    ui_group.set_title("Interface");

    let metadata_model = gio::ListStore::new::<StringObject>();
    metadata_model.append(&StringObject::new("Show Metadata"));
    metadata_model.append(&StringObject::new("Hide Metadata"));
    metadata_model.append(&StringObject::new("Ignore Metadata"));
    let metadata_expr = PropertyExpression::new(StringObject::static_type(), None::<&gtk4::Expression>, "string");
    let metadata_row = ComboRow::builder()
        .title("Metadata Handling")
        .subtitle("How to treat YAML headers")
        .model(&metadata_model)
        .expression(&metadata_expr)
        .build();
    let saved_metadata = load_pref(PREF_SHOW_METADATA, DEFAULT_SHOW_METADATA);
    metadata_row.set_selected(match saved_metadata.as_str() {
        "hide" => 1,
        "ignore" => 2,
        _ => 0,
    });
    metadata_row.connect_selected_notify({
        let refresh_preview = refresh_preview.clone();
        let sb = source_buffer.clone();
        move |row| {
            save_pref(
                PREF_SHOW_METADATA,
                match row.selected() {
                    1 => "hide",
                    2 => "ignore",
                    _ => "show",
                },
            );
            refresh_preview(&sb, true);
        }
    });

    let view_model = gio::ListStore::new::<StringObject>();
    view_model.append(&StringObject::new("Dual Pane"));
    view_model.append(&StringObject::new("Preview Only"));
    let view_expr = PropertyExpression::new(StringObject::static_type(), None::<&gtk4::Expression>, "string");
    let view_row = ComboRow::builder()
        .title("Default View Mode")
        .subtitle("Initial layout when opening a file")
        .model(&view_model)
        .expression(&view_expr)
        .build();
    let saved_view = load_pref(PREF_DEFAULT_VIEW, DEFAULT_VIEW);
    view_row.set_selected(match saved_view.as_str() {
        "preview-only" => 1,
        _ => 0,
    });
    view_row.connect_selected_notify({
        let refresh_preview = refresh_preview.clone();
        let sb = source_buffer.clone();
        move |row| {
            save_pref(
                PREF_DEFAULT_VIEW,
                match row.selected() {
                    1 => "preview-only",
                    _ => "dual-pane",
                },
            );
            refresh_preview(&sb, true);
        }
    });

    let readable_row = SwitchRow::builder()
        .title("Readable Line Length")
        .subtitle("Limit preview content width for better readability")
        .active(load_pref(PREF_READABLE_LINE, DEFAULT_READABLE_LINE) == "true")
        .build();

    let width_adjustment = gtk4::Adjustment::new(
        load_pref(PREF_MAX_WIDTH, DEFAULT_MAX_WIDTH).parse::<f64>().unwrap_or(1000.0),
        400.0, 3000.0, 50.0, 200.0, 0.0
    );
    let width_row = SpinRow::builder()
        .title("Max Content Width (px)")
        .subtitle("Maximum width when Readable Line Length is enabled")
        .adjustment(&width_adjustment)
        .sensitive(readable_row.is_active())
        .build();

    readable_row.connect_active_notify({
        let refresh_preview = refresh_preview.clone();
        let sb = source_buffer.clone();
        let width_row = width_row.clone();
        move |row| {
            let active = row.is_active();
            save_pref(PREF_READABLE_LINE, if active { "true" } else { "false" });
            width_row.set_sensitive(active);
            refresh_preview(&sb, true);
        }
    });
    width_row.connect_value_notify({
        let refresh_preview = refresh_preview.clone();
        let sb = source_buffer.clone();
        move |row| {
            save_pref(PREF_MAX_WIDTH, &format!("{:.0}", row.value()));
            refresh_preview(&sb, true);
        }
    });

    ui_group.add(&metadata_row);
    ui_group.add(&view_row);
    ui_group.add(&readable_row);
    ui_group.add(&width_row);

    let ui_page = PreferencesPage::builder()
        .title("Interface")
        .icon_name("window-new-symbolic")
        .build();
    ui_page.add(&ui_group);

    // Shortcuts Page
    let shortcuts_group = PreferencesGroup::new();
    shortcuts_group.set_title("Keyboard Shortcuts");

    let create_shortcut_row = |title: &str, pref_key: &'static str, default: &str, app: Application, parent_win: &ApplicationWindow| {
        let initial_subtitle = html_escape(&load_pref(pref_key, default));
        let escaped_title = html_escape(title);
        let row = ActionRow::builder()
            .title(&escaped_title)
            .subtitle(&initial_subtitle)
            .use_markup(true)
            .build();
        
        let edit_button = Button::builder()
            .label("Edit")
            .valign(gtk4::Align::Center)
            .build();

        let app_clone = app.clone();
        let parent_win_clone = parent_win.clone();
        let row_clone = row.clone();
        let default_owned = default.to_string();
        let title_owned = title.to_string();

        edit_button.connect_clicked(move |_| {
            let grabber = adw::Window::builder()
                .title("Grab Shortcut")
                .modal(true)
                .transient_for(&parent_win_clone)
                .default_width(300)
                .default_height(150)
                .build();

            let content = Box::new(Orientation::Vertical, 10);
            content.set_margin_top(20);
            content.set_margin_bottom(20);
            content.set_margin_start(20);
            content.set_margin_end(20);
            
            let label = gtk4::Label::builder()
                .label(format!("Press keys for: {}", title_owned))
                .build();
            let current_label = gtk4::Label::builder()
                .label(html_escape(&load_pref(pref_key, &default_owned)))
                .css_classes(vec!["title-2".to_string()])
                .use_markup(true)
                .build();
            
            let hint = gtk4::Label::builder()
                .label("Press Enter to Save, Esc to Reset")
                .build();
            hint.add_css_class("dim-label");

            content.append(&label);
            content.append(&current_label);
            content.append(&hint);
            grabber.set_content(Some(&content));

            let recorded_accel = Rc::new(RefCell::new(load_pref(pref_key, &default_owned)));

            let key_ctrl = EventControllerKey::new();
            let recorded_accel_inner = recorded_accel.clone();
            let current_label_inner = current_label.clone();
            let grabber_inner = grabber.clone();
            let app_inner = app_clone.clone();
            let row_inner = row_clone.clone();

            key_ctrl.connect_key_pressed(move |_, key, _, modifier| {
                let key_name = key.name().unwrap_or_else(|| "unknown".into());

                if key_name == "Return" {
                    let accel = recorded_accel_inner.borrow();
                    save_pref(pref_key, &accel);
                    row_inner.set_subtitle(&html_escape(&accel));
                    crate::apply_accels(&app_inner);
                    grabber_inner.close();
                    return glib::Propagation::Stop;
                }

                if key_name == "Escape" {
                    grabber_inner.close();
                    return glib::Propagation::Stop;
                }

                if key_name.contains("Control") || key_name.contains("Shift") || 
                   key_name.contains("Alt") || key_name.contains("Super") || key_name.contains("Meta") {
                    return glib::Propagation::Stop;
                }

                let mut accel = String::new();
                if modifier.contains(gtk4::gdk::ModifierType::CONTROL_MASK) { accel.push_str("<Control>"); }
                if modifier.contains(gtk4::gdk::ModifierType::SHIFT_MASK) { accel.push_str("<Shift>"); }
                if modifier.contains(gtk4::gdk::ModifierType::ALT_MASK) { accel.push_str("<Alt>"); }
                if modifier.contains(gtk4::gdk::ModifierType::SUPER_MASK) || modifier.contains(gtk4::gdk::ModifierType::META_MASK) { 
                    accel.push_str("<Super>"); 
                }
                
                accel.push_str(&key_name);
                *recorded_accel_inner.borrow_mut() = accel.clone();
                current_label_inner.set_text(&html_escape(&accel));

                glib::Propagation::Stop
            });

            grabber.add_controller(key_ctrl);
            grabber.present();
        });

        row.add_suffix(&edit_button);
        row
    };

    let app_clone = app.clone();
    shortcuts_group.add(&create_shortcut_row("Open File", PREF_KEY_OPEN, DEFAULT_KEY_OPEN, app_clone.clone(), parent_win));
    shortcuts_group.add(&create_shortcut_row("Save File", PREF_KEY_SAVE, DEFAULT_KEY_SAVE, app_clone.clone(), parent_win));
    shortcuts_group.add(&create_shortcut_row("Save As", PREF_KEY_SAVE_AS, DEFAULT_KEY_SAVE_AS, app_clone.clone(), parent_win));
    shortcuts_group.add(&create_shortcut_row("Export PDF", PREF_KEY_EXPORT_PDF, DEFAULT_KEY_EXPORT_PDF, app_clone.clone(), parent_win));
    shortcuts_group.add(&create_shortcut_row("Open Preferences", PREF_KEY_PREFS, DEFAULT_KEY_PREFS, app_clone.clone(), parent_win));
    shortcuts_group.add(&create_shortcut_row("Search & Replace", PREF_KEY_SEARCH, DEFAULT_KEY_SEARCH, app_clone.clone(), parent_win));
    shortcuts_group.add(&create_shortcut_row("Toggle Editor Pane", PREF_KEY_TOGGLE_EDITOR, DEFAULT_KEY_TOGGLE_EDITOR, app_clone.clone(), parent_win));
    shortcuts_group.add(&create_shortcut_row("Cycle View Modes", PREF_KEY_CYCLE_VIEW, DEFAULT_KEY_CYCLE_VIEW, app_clone.clone(), parent_win));
    shortcuts_group.add(&create_shortcut_row("Toggle Readable Width", PREF_KEY_TOGGLE_READABLE, DEFAULT_KEY_TOGGLE_READABLE, app_clone.clone(), parent_win));
    shortcuts_group.add(&create_shortcut_row("Keyboard Shortcuts Dialog", PREF_KEY_SHORTCUTS, DEFAULT_KEY_SHORTCUTS, app_clone.clone(), parent_win));
    shortcuts_group.add(&create_shortcut_row("Quit", PREF_KEY_QUIT, DEFAULT_KEY_QUIT, app_clone.clone(), parent_win));

    let shortcuts_page = PreferencesPage::builder()
        .title("Shortcuts")
        .icon_name("preferences-desktop-keyboard-shortcuts-symbolic")
        .build();
    shortcuts_page.add(&shortcuts_group);

    let prefs = PreferencesDialog::builder()
        .title("Preferences")
        .build();
    prefs.add(&appearance_page);
    prefs.add(&editor_page);
    prefs.add(&sync_page);
    prefs.add(&ui_page);
    prefs.add(&shortcuts_page);
    prefs.present(Some(parent_win));
}
