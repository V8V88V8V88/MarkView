use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;
use gtk4::{Box, Button, Entry, Orientation, Revealer, SearchBar, SearchEntry, ToggleButton};
use sourceview5::prelude::*;
use sourceview5::{Buffer as SourceBuffer, SearchContext, SearchSettings, View as SourceView};

pub struct SearchController {
    pub search_bar: SearchBar,
    entry: SearchEntry,
    replace_entry: Entry,
    matches_label: gtk4::Label,
    context: SearchContext,
    view: SourceView,
    jump_pending: Cell<bool>,
}

impl SearchController {
    pub fn new(view: &SourceView, buffer: &SourceBuffer) -> Rc<Self> {
        let settings = SearchSettings::builder().wrap_around(true).build();
        let context = SearchContext::new(buffer, Some(&settings));
        context.set_highlight(false);

        let entry = SearchEntry::builder()
            .placeholder_text("Find")
            .hexpand(true)
            .build();
        let matches_label = gtk4::Label::builder()
            .css_classes(["dimmed", "numeric", "caption"])
            .margin_start(6)
            .margin_end(6)
            .build();
        let prev = Button::builder()
            .icon_name("go-up-symbolic")
            .tooltip_text("Previous Match (Shift+Enter)")
            .build();
        let next = Button::builder()
            .icon_name("go-down-symbolic")
            .tooltip_text("Next Match (Enter)")
            .build();
        let nav = Box::builder().css_classes(["linked"]).build();
        nav.append(&prev);
        nav.append(&next);

        let case_toggle = ToggleButton::builder()
            .icon_name("uppercase-symbolic")
            .tooltip_text("Match Case")
            .css_classes(["flat"])
            .build();
        let regex_toggle = ToggleButton::builder()
            .icon_name("regex-symbolic")
            .tooltip_text("Regular Expressions")
            .css_classes(["flat"])
            .build();
        for (toggle, label) in [(&case_toggle, "Aa"), (&regex_toggle, ".*")] {
            let theme = gtk4::IconTheme::for_display(&toggle.display());
            if !theme.has_icon(&toggle.icon_name().unwrap_or_default()) {
                toggle.set_label(label);
            }
        }
        let replace_toggle = ToggleButton::builder()
            .icon_name("edit-find-replace-symbolic")
            .tooltip_text("Replace")
            .css_classes(["flat"])
            .build();

        let find_row = Box::new(Orientation::Horizontal, 6);
        let entry_box = Box::builder().css_classes(["linked"]).hexpand(true).build();
        entry_box.append(&entry);
        find_row.append(&entry_box);
        find_row.append(&matches_label);
        find_row.append(&nav);
        find_row.append(&case_toggle);
        find_row.append(&regex_toggle);
        find_row.append(&replace_toggle);

        let replace_entry = Entry::builder()
            .placeholder_text("Replace")
            .hexpand(true)
            .build();
        let replace_btn = Button::with_label("Replace");
        let replace_all_btn = Button::with_label("Replace All");
        let replace_row = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(6)
            .margin_top(6)
            .build();
        replace_row.append(&replace_entry);
        replace_row.append(&replace_btn);
        replace_row.append(&replace_all_btn);
        let replace_revealer = Revealer::builder()
            .child(&replace_row)
            .transition_type(gtk4::RevealerTransitionType::SlideDown)
            .build();
        replace_toggle
            .bind_property("active", &replace_revealer, "reveal-child")
            .sync_create()
            .build();

        let content = Box::new(Orientation::Vertical, 0);
        content.append(&find_row);
        content.append(&replace_revealer);
        let clamp = adw::Clamp::builder()
            .maximum_size(720)
            .child(&content)
            .hexpand(true)
            .build();

        let search_bar = SearchBar::builder()
            .child(&clamp)
            .show_close_button(true)
            .build();
        search_bar.connect_entry(&entry);

        case_toggle
            .bind_property("active", &settings, "case-sensitive")
            .sync_create()
            .build();
        regex_toggle
            .bind_property("active", &settings, "regex-enabled")
            .sync_create()
            .build();

        let this = Rc::new(Self {
            search_bar,
            entry: entry.clone(),
            replace_entry: replace_entry.clone(),
            matches_label,
            context: context.clone(),
            view: view.clone(),
            jump_pending: Cell::new(false),
        });

        let weak = Rc::downgrade(&this);
        let with = move |f: fn(&Self)| {
            let weak = weak.clone();
            move || {
                if let Some(this) = weak.upgrade() {
                    f(&this);
                }
            }
        };

        entry.connect_search_changed({
            let settings = settings.clone();
            let refresh = with(|s| {
                s.jump_pending.set(!s.entry.text().is_empty());
                s.update_matches_label();
            });
            move |e| {
                let text = e.text();
                settings.set_search_text((!text.is_empty()).then_some(text.as_str()));
                refresh();
            }
        });
        context.connect_occurrences_count_notify({
            let f = with(|s| {
                if s.jump_pending.get() && s.context.occurrences_count() >= 0 {
                    s.jump_pending.set(false);
                    s.find(true, false);
                } else {
                    s.update_matches_label();
                }
            });
            move |_| f()
        });
        entry.connect_activate({
            let f = with(|s| s.find(true, true));
            move |_| f()
        });
        entry.connect_next_match({
            let f = with(|s| s.find(true, true));
            move |_| f()
        });
        entry.connect_previous_match({
            let f = with(|s| s.find(false, true));
            move |_| f()
        });
        entry.connect_stop_search({
            let f = with(|s| s.search_bar.set_search_mode(false));
            move |_| f()
        });
        next.connect_clicked({
            let f = with(|s| s.find(true, true));
            move |_| f()
        });
        prev.connect_clicked({
            let f = with(|s| s.find(false, true));
            move |_| f()
        });
        replace_btn.connect_clicked({
            let f = with(Self::replace_one);
            move |_| f()
        });
        replace_entry.connect_activate({
            let f = with(Self::replace_one);
            move |_| f()
        });
        replace_all_btn.connect_clicked({
            let f = with(|s| {
                let text = s.replace_entry.text();
                let _ = s.context.replace_all(&text);
                s.update_matches_label();
            });
            move |_| f()
        });

        let keys = gtk4::EventControllerKey::new();
        keys.connect_key_pressed({
            let f = with(|s| s.find(false, true));
            move |_, key, _, mods| {
                if matches!(key, gtk4::gdk::Key::Return | gtk4::gdk::Key::KP_Enter)
                    && mods.contains(gtk4::gdk::ModifierType::SHIFT_MASK)
                {
                    f();
                    return gtk4::glib::Propagation::Stop;
                }
                gtk4::glib::Propagation::Proceed
            }
        });
        entry.add_controller(keys);

        this.search_bar.connect_search_mode_enabled_notify({
            let f = with(|s| {
                let enabled = s.search_bar.is_search_mode();
                s.context.set_highlight(enabled);
                if enabled {
                    s.prefill_from_selection();
                    s.entry.grab_focus();
                    s.entry.select_region(0, -1);
                } else {
                    s.view.grab_focus();
                }
            });
            move |_| f()
        });

        this
    }

    fn buffer(&self) -> gtk4::TextBuffer {
        self.view.buffer()
    }

    fn prefill_from_selection(&self) {
        let buf = self.buffer();
        if let Some((start, end)) = buf.selection_bounds()
            && start.line() == end.line()
        {
            self.entry.set_text(&buf.text(&start, &end, false));
        }
    }

    fn current_match(&self) -> Option<i32> {
        let buf = self.buffer();
        let (start, end) = buf.selection_bounds()?;
        let pos = self.context.occurrence_position(&start, &end);
        (pos > 0).then_some(pos)
    }

    fn update_matches_label(&self) {
        let count = self.context.occurrences_count();
        let text = if self.entry.text().is_empty() || count < 0 {
            String::new()
        } else if count == 0 {
            "No results".into()
        } else if let Some(pos) = self.current_match() {
            format!("{pos} of {count}")
        } else {
            format!("{count} results")
        };
        self.matches_label.set_label(&text);
        if count == 0 && !self.entry.text().is_empty() {
            self.entry.add_css_class("error");
        } else {
            self.entry.remove_css_class("error");
        }
    }

    fn find(&self, forward: bool, step: bool) {
        let buf = self.buffer();
        let (sel_start, sel_end) = buf
            .selection_bounds()
            .unwrap_or_else(|| {
                let it = buf.iter_at_mark(&buf.get_insert());
                (it, it)
            });
        let found = if forward {
            self.context.forward(if step { &sel_end } else { &sel_start })
        } else {
            self.context.backward(&sel_start)
        };
        if let Some((start, end, _)) = found {
            buf.select_range(&start, &end);
            self.view.scroll_to_mark(&buf.get_insert(), 0.25, false, 0.0, 0.0);
        }
        self.update_matches_label();
    }

    fn replace_one(&self) {
        let buf = self.buffer();
        if self.current_match().is_none() {
            self.find(true, false);
        }
        if let Some((mut start, mut end)) = buf.selection_bounds()
            && self.context.occurrence_position(&start, &end) > 0
        {
            let text = self.replace_entry.text();
            let _ = self.context.replace(&mut start, &mut end, &text);
        }
        self.find(true, true);
    }

    pub fn toggle(&self) {
        self.search_bar.set_search_mode(!self.search_bar.is_search_mode());
    }
}
