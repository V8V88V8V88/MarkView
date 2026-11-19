use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;
use adw::{
    ActionRow, ButtonRow, ColorScheme, ComboRow, ExpanderRow, PreferencesDialog, PreferencesGroup,
    PreferencesPage, ShortcutLabel, SpinRow, StyleManager, SwitchRow, Toggle, ToggleGroup,
};
use adw::Application;
use gtk4::{gdk, gio, glib, EventControllerKey, FlowBox, PropagationPhase, StringList};
use sourceview5::StyleSchemePreview;

use crate::config::*;
use crate::ui::schemes;
use crate::ui::window::apply_settings_to_all_windows;

pub fn color_scheme_from_pref() -> ColorScheme {
    match load_pref(PREF_THEME, DEFAULT_THEME).as_str() {
        "force-dark" => ColorScheme::ForceDark,
        "force-light" => ColorScheme::ForceLight,
        _ => ColorScheme::Default,
    }
}

fn switch_row(title: &str, subtitle: &str, key: &'static str, default: bool) -> SwitchRow {
    let row = SwitchRow::builder()
        .title(title)
        .subtitle(subtitle)
        .active(pref_bool(key, default))
        .build();
    row.connect_active_notify(move |row| {
        save_pref_bool(key, row.is_active());
        apply_settings_to_all_windows();
    });
    row
}

fn combo_row(
    title: &str,
    subtitle: &str,
    key: &'static str,
    default: &str,
    options: &'static [(&'static str, &'static str)],
) -> ComboRow {
    let labels: Vec<&str> = options.iter().map(|(_, label)| *label).collect();
    let saved = load_pref(key, default);
    let row = ComboRow::builder()
        .title(title)
        .subtitle(subtitle)
        .model(&StringList::new(&labels))
        .selected(options.iter().position(|(v, _)| *v == saved).unwrap_or(0) as u32)
        .build();
    row.connect_selected_notify(move |row| {
        if let Some((value, _)) = options.get(row.selected() as usize) {
            save_pref(key, value);
            apply_settings_to_all_windows();
        }
    });
    row
}

fn spin_row(
    title: &str,
    subtitle: &str,
    key: &'static str,
    default: &str,
    range: (f64, f64, f64),
) -> SpinRow {
    let value = load_pref(key, default).parse::<f64>().unwrap_or(range.0);
    let row = SpinRow::builder()
        .title(title)
        .subtitle(subtitle)
        .adjustment(&gtk4::Adjustment::new(value, range.0, range.1, range.2, range.2 * 10.0, 0.0))
        .build();
    row.connect_value_notify(move |row| {
        save_pref(key, &format!("{:.0}", row.value()));
        apply_settings_to_all_windows();
    });
    row
}

fn appearance_page() -> PreferencesPage {
    let page = PreferencesPage::builder()
        .title("Appearance")
        .icon_name("applications-graphics-symbolic")
        .build();

    let style_group = PreferencesGroup::new();
    let toggles = ToggleGroup::builder().valign(gtk4::Align::Center).build();
    for (name, label) in [("default", "Follow System"), ("force-light", "Light"), ("force-dark", "Dark")] {
        toggles.add(Toggle::builder().name(name).label(label).build());
    }
    toggles.set_active_name(Some(&load_pref(PREF_THEME, DEFAULT_THEME)));
    toggles.connect_active_name_notify(|t| {
        if let (Some(name), Some(app)) = (t.active_name(), gio::Application::default()) {
            app.activate_action("color-scheme", Some(&name.to_variant()));
        }
    });
    let style_row = ActionRow::builder().title("Style").build();
    style_row.add_suffix(&toggles);
    style_group.add(&style_row);
    page.add(&style_group);

    let scheme_group = PreferencesGroup::builder()
        .title("Editor Color Scheme")
        .description("The light or dark version is picked to match the style")
        .build();
    let flow = FlowBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .column_spacing(12)
        .row_spacing(12)
        .min_children_per_line(2)
        .max_children_per_line(4)
        .homogeneous(true)
        .build();
    scheme_group.add(&flow);
    let expanded = Rc::new(Cell::new(false));
    let more = gtk4::Button::builder()
        .label("Show All")
        .halign(gtk4::Align::Center)
        .margin_top(12)
        .css_classes(["pill"])
        .build();
    scheme_group.add(&more);
    let collapse: Rc<dyn Fn()> = Rc::new({
        let flow = flow.clone();
        let more = more.clone();
        let expanded = expanded.clone();
        move || {
            const SHOWN: usize = 6;
            let mut tiles = Vec::new();
            let mut child = flow.first_child();
            while let Some(c) = child {
                child = c.next_sibling();
                tiles.push(c);
            }
            let selected = tiles.iter().position(|c| {
                c.first_child()
                    .and_then(|b| b.first_child())
                    .and_downcast::<StyleSchemePreview>()
                    .is_some_and(|p| p.is_selected())
            });
            let expanded_now = expanded.get();
            flow.set_filter_func(move |child| {
                let i = child.index() as usize;
                expanded_now
                    || match selected {
                        Some(s) if s >= SHOWN => i < SHOWN - 1 || i == s,
                        _ => i < SHOWN,
                    }
            });
            more.set_visible(tiles.len() > SHOWN);
            more.set_label(if expanded.get() { "Show Less" } else { "Show All" });
        }
    });
    more.connect_clicked({
        let collapse = collapse.clone();
        let expanded = expanded.clone();
        move |_| {
            expanded.set(!expanded.get());
            collapse();
        }
    });
    page.add(&scheme_group);

    let fill: Rc<dyn Fn()> = Rc::new({
        let flow = flow.clone();
        let collapse = collapse.clone();
        move || {
            flow.remove_all();
            let selected = schemes::saved_family();
            for scheme in schemes::schemes_for(StyleManager::default().is_dark()) {
                let preview = StyleSchemePreview::new(&scheme);
                preview.set_selected(schemes::family_of(&scheme.id()) == selected);
                let flow_inner = flow.clone();
                preview.connect_activate(move |p| {
                    save_pref(PREF_SCHEME, schemes::family_of(&p.scheme().id()));
                    let mut child = flow_inner.first_child();
                    while let Some(c) = child {
                        if let Some(other) = c
                            .first_child()
                            .and_then(|b| b.first_child())
                            .and_downcast::<StyleSchemePreview>()
                        {
                            other.set_selected(&other == p);
                        }
                        child = c.next_sibling();
                    }
                    apply_settings_to_all_windows();
                });
                let label = gtk4::Label::builder()
                    .label(schemes::family_name(&scheme))
                    .ellipsize(gtk4::pango::EllipsizeMode::End)
                    .css_classes(["caption"])
                    .build();
                let tile = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
                tile.append(&preview);
                tile.append(&label);
                flow.append(&tile);
            }
            collapse();
        }
    });
    fill();
    let handler = StyleManager::default().connect_dark_notify({
        let fill = fill.clone();
        move |_| fill()
    });
    let handler = Cell::new(Some(handler));
    page.connect_destroy(move |_| {
        if let Some(h) = handler.take() {
            StyleManager::default().disconnect(h);
        }
    });

    let preview_group = PreferencesGroup::builder().title("Preview").build();
    let readable = ExpanderRow::builder()
        .title("Readable Line Length")
        .subtitle("Limit how wide the preview text can get")
        .show_enable_switch(true)
        .enable_expansion(pref_bool(PREF_READABLE_LINE, DEFAULT_READABLE_LINE == "true"))
        .build();
    readable.connect_enable_expansion_notify(|row| {
        save_pref_bool(PREF_READABLE_LINE, row.enables_expansion());
        apply_settings_to_all_windows();
    });
    readable.add_row(&spin_row(
        "Maximum Width",
        "In pixels",
        PREF_MAX_WIDTH,
        DEFAULT_MAX_WIDTH,
        (400.0, 3000.0, 50.0),
    ));
    preview_group.add(&readable);
    page.add(&preview_group);

    page
}

fn editor_page() -> PreferencesPage {
    let page = PreferencesPage::builder()
        .title("Editor")
        .icon_name("document-edit-symbolic")
        .build();

    let display = PreferencesGroup::builder().title("Display").build();
    display.add(&switch_row("Line Numbers", "Show line numbers in the gutter", PREF_LINE_NUMBERS, DEFAULT_LINE_NUMBERS == "true"));
    display.add(&switch_row("Highlight Current Line", "Shade the line the cursor is on", PREF_HIGHLIGHT_LINE, DEFAULT_HIGHLIGHT_LINE == "true"));
    display.add(&switch_row("Text Wrapping", "Wrap long lines at word boundaries", PREF_WORD_WRAP, DEFAULT_WORD_WRAP == "true"));
    page.add(&display);

    let behavior = PreferencesGroup::builder().title("Behavior").build();
    behavior.add(&switch_row("Hide Header Bar While Typing", "Move the pointer to bring it back", PREF_AUTOHIDE_HEADER, DEFAULT_AUTOHIDE_HEADER == "true"));
    behavior.add(&switch_row("Vim Keybindings", "Edit with Vim-style modes and motions", PREF_VIM_MODE, DEFAULT_VIM_MODE == "true"));
    page.add(&behavior);

    page
}

fn preview_page() -> PreferencesPage {
    let page = PreferencesPage::builder()
        .title("Preview")
        .icon_name("view-reveal-symbolic")
        .build();

    let layout = PreferencesGroup::builder().title("Layout").build();
    layout.add(&combo_row(
        "Layout on Startup",
        "How new windows are arranged",
        PREF_DEFAULT_VIEW,
        DEFAULT_VIEW,
        &[("dual-pane", "Split"), ("preview-only", "Preview Only"), ("editor-only", "Editor Only")],
    ));
    layout.add(&switch_row("Scroll Together", "The preview follows the editor as you scroll", PREF_SYNC_SCROLL, DEFAULT_SYNC_SCROLL == "true"));
    page.add(&layout);

    let rendering = PreferencesGroup::builder().title("Rendering").build();
    rendering.add(&combo_row(
        "Front Matter",
        "How YAML metadata at the top of a document is treated",
        PREF_SHOW_METADATA,
        DEFAULT_SHOW_METADATA,
        &[("show", "Show"), ("hide", "Hide"), ("ignore", "Render as Text")],
    ));
    rendering.add(&switch_row("Update in Place", "Refresh the preview without reloading the page", PREF_DOM_INJECTION, DEFAULT_DOM_INJECTION == "true"));
    rendering.add(&spin_row(
        "Update Delay",
        "Milliseconds to wait after typing before refreshing",
        PREF_DEBOUNCE,
        DEFAULT_DEBOUNCE,
        (0.0, 5000.0, 10.0),
    ));
    page.add(&rendering);

    page
}

fn is_modifier(key: gdk::Key) -> bool {
    use gdk::Key;
    matches!(
        key,
        Key::Shift_L | Key::Shift_R | Key::Control_L | Key::Control_R | Key::Alt_L | Key::Alt_R
            | Key::Super_L | Key::Super_R | Key::Meta_L | Key::Meta_R | Key::Hyper_L | Key::Hyper_R
            | Key::ISO_Level3_Shift | Key::Caps_Lock
    )
}

fn accel_for(s: &Shortcut) -> String {
    s.accel()
}

fn same_accel(a: &str, b: &str) -> bool {
    match (gtk4::accelerator_parse(a), gtk4::accelerator_parse(b)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

fn capture_shortcut(row: &ActionRow, label: &ShortcutLabel, app: &Application, shortcut: &'static Shortcut) {
    let dialog = adw::Dialog::builder()
        .title("Set Shortcut")
        .content_width(400)
        .build();
    let error = gtk4::Label::builder()
        .css_classes(["error"])
        .visible(false)
        .build();
    let status = adw::StatusPage::builder()
        .icon_name("preferences-desktop-keyboard-shortcuts-symbolic")
        .title(shortcut.title)
        .description("Press the new key combination.\nEsc cancels, Backspace disables the shortcut.")
        .css_classes(["compact"])
        .child(&error)
        .build();
    let reset = gtk4::Button::builder()
        .label("Reset to Default")
        .halign(gtk4::Align::Center)
        .css_classes(["pill"])
        .margin_bottom(24)
        .build();
    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    content.append(&status);
    content.append(&reset);
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&content));
    dialog.set_child(Some(&toolbar));

    let commit: Rc<dyn Fn(&str)> = Rc::new({
        let dialog = dialog.clone();
        let label = label.clone();
        let app = app.clone();
        move |accel: &str| {
            save_pref(shortcut.pref_key, accel);
            label.set_accelerator(accel);
            crate::apply_accels(&app);
            dialog.close();
        }
    });

    reset.connect_clicked({
        let commit = commit.clone();
        move |_| commit(&shortcut.default_accel())
    });

    let keys = EventControllerKey::new();
    keys.set_propagation_phase(PropagationPhase::Capture);
    keys.connect_key_pressed({
        let dialog = dialog.clone();
        move |_, keyval, _, state| {
            let mods = state & gtk4::accelerator_get_default_mod_mask();
            let keyval = keyval.to_lower();
            if mods.is_empty() {
                match keyval {
                    gdk::Key::Escape => {
                        dialog.close();
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::BackSpace => {
                        commit("");
                        return glib::Propagation::Stop;
                    }
                    gdk::Key::Tab | gdk::Key::ISO_Left_Tab => return glib::Propagation::Proceed,
                    _ => {}
                }
            }
            if is_modifier(keyval) || !gtk4::accelerator_valid(keyval, mods) {
                return glib::Propagation::Stop;
            }
            let accel = gtk4::accelerator_name(keyval, mods).to_string();
            if let Some(other) = SHORTCUTS
                .iter()
                .find(|s| s.pref_key != shortcut.pref_key && same_accel(&accel_for(s), &accel))
            {
                error.set_label(&format!("Already used by “{}”", other.title));
                error.set_visible(true);
                return glib::Propagation::Stop;
            }
            commit(&accel);
            glib::Propagation::Stop
        }
    });
    dialog.add_controller(keys);
    dialog.present(Some(row));
}

fn shortcuts_page(app: &Application) -> PreferencesPage {
    let page = PreferencesPage::builder()
        .title("Shortcuts")
        .icon_name("preferences-desktop-keyboard-shortcuts-symbolic")
        .build();

    let mut labels = Vec::new();
    let mut sections: Vec<&str> = SHORTCUTS.iter().map(|s| s.section).collect();
    sections.dedup();
    for section in sections {
        let group = PreferencesGroup::builder().title(section).build();
        for shortcut in SHORTCUTS.iter().filter(|s| s.section == section) {
            let label = ShortcutLabel::new(&accel_for(shortcut));
            label.set_disabled_text("Disabled");
            label.set_valign(gtk4::Align::Center);
            let row = ActionRow::builder()
                .title(shortcut.title)
                .activatable(true)
                .build();
            row.add_suffix(&label);
            row.connect_activated({
                let label = label.clone();
                let app = app.clone();
                move |row| capture_shortcut(row, &label, &app, shortcut)
            });
            group.add(&row);
            labels.push((label, shortcut));
        }
        page.add(&group);
    }

    let reset_group = PreferencesGroup::new();
    let reset = ButtonRow::builder()
        .title("Reset All Shortcuts")
        .start_icon_name("edit-undo-symbolic")
        .build();
    let app = app.clone();
    reset.connect_activated(move |_| {
        for (label, shortcut) in &labels {
            let accel = shortcut.default_accel();
            save_pref(shortcut.pref_key, &accel);
            label.set_accelerator(&accel);
        }
        crate::apply_accels(&app);
    });
    reset_group.add(&reset);
    page.add(&reset_group);

    page
}

pub fn show_preferences_dialog(parent: &impl IsA<gtk4::Widget>, app: &Application) {
    let prefs = PreferencesDialog::builder()
        .title("Preferences")
        .search_enabled(true)
        .build();
    prefs.add(&appearance_page());
    prefs.add(&editor_page());
    prefs.add(&preview_page());
    prefs.add(&shortcuts_page(app));
    prefs.present(Some(parent));
}
