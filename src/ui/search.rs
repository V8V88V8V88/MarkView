use gtk4::prelude::*;
use gtk4::{Box, Button, Orientation, SearchBar, SearchEntry, Entry, ToggleButton, Align, Overlay};
use sourceview5::prelude::*;
use sourceview5::{SearchSettings, SearchContext, Buffer as SourceBuffer, View as SourceView};

pub struct SearchController {
    pub search_bar: SearchBar,
    #[allow(dead_code)]
    search_context: SearchContext,
}

impl SearchController {
    pub fn new(source_view: &SourceView, source_buffer: &SourceBuffer) -> Self {
        let search_bar = SearchBar::builder()
            .key_capture_widget(source_view)
            .build();
        
        let search_group = Box::new(Orientation::Horizontal, 10);
        search_group.add_css_class("card");
        
        // CSS is handled globally in main for now, but we could move it here
        
        search_group.set_margin_bottom(6);
        search_group.set_margin_start(6);
        search_group.set_margin_end(6);
        search_group.set_halign(Align::Center);
        
        let entries_vbox = Box::new(Orientation::Vertical, 4);
        let search_entry = SearchEntry::builder()
            .placeholder_text("Find...")
            .width_request(250)
            .build();
        let replace_entry = Entry::builder()
            .placeholder_text("Replace with...")
            .width_request(250)
            .build();
        entries_vbox.append(&search_entry);
        entries_vbox.append(&replace_entry);

        let options_hbox = Box::new(Orientation::Horizontal, 4);
        options_hbox.set_valign(Align::Center);
        
        let regex_toggle = ToggleButton::builder()
            .label(".*")
            .tooltip_text("Use Regular Expressions")
            .build();
        let prev_button = Button::builder().icon_name("go-up-symbolic").tooltip_text("Previous").build();
        let next_button = Button::builder().icon_name("go-down-symbolic").tooltip_text("Next").build();
        let replace_button = Button::builder().label("Replace").build();
        let replace_all_button = Button::builder().label("Replace All").build();
        let close_button = Button::builder().icon_name("window-close-symbolic").tooltip_text("Close").build();

        options_hbox.append(&regex_toggle);
        options_hbox.append(&prev_button);
        options_hbox.append(&next_button);
        options_hbox.append(&replace_button);
        options_hbox.append(&replace_all_button);
        options_hbox.append(&close_button);

        search_group.append(&entries_vbox);
        search_group.append(&options_hbox);
        
        search_bar.set_child(Some(&search_group));
        search_bar.connect_entry(&search_entry);

        {
            let sb = search_bar.clone();
            close_button.connect_clicked(move |_| {
                sb.set_property("search-mode-enabled", false);
            });
        }

        let search_settings = SearchSettings::new();
        let search_context = SearchContext::new(source_buffer, Some(&search_settings));
        search_context.set_highlight(true);

        {
            let active_tag = gtk4::TextTag::new(Some("active-match"));
            active_tag.set_background(Some("#e5a50a"));
            active_tag.set_foreground(Some("#1a1a1a"));
            source_buffer.tag_table().add(&active_tag);
        }

        {
            let settings = search_settings.clone();
            let buf = source_buffer.clone();
            search_entry.connect_search_changed(move |entry| {
                settings.set_search_text(Some(&entry.text()));
                let (start, end) = buf.bounds();
                buf.remove_tag_by_name("active-match", &start, &end);
            });
        }

        {
            let settings = search_settings.clone();
            regex_toggle.connect_toggled(move |btn| {
                settings.set_regex_enabled(btn.is_active());
            });
        }

        {
            let context = search_context.clone();
            let sv = source_view.clone();
            next_button.connect_clicked(move |_| {
                let buf = sv.buffer();
                let mark = buf.mark("insert").unwrap();
                let iter = buf.iter_at_mark(&mark);

                if let Some((start, end, _)) = context.forward(&iter) {
                    let (b_start, b_end) = buf.bounds();
                    buf.remove_tag_by_name("active-match", &b_start, &b_end);
                    buf.apply_tag_by_name("active-match", &start, &end);

                    buf.select_range(&end, &start);
                    sv.grab_focus();

                    let mut fresh_iter = buf.iter_at_offset(start.offset());
                    sv.scroll_to_iter(&mut fresh_iter, 0.0, true, 0.5, 0.5);
                }
            });
        }

        {
            let context = search_context.clone();
            let sv = source_view.clone();
            prev_button.connect_clicked(move |_| {
                let buf = sv.buffer();
                let mark = buf.mark("insert").unwrap();
                let iter = buf.iter_at_mark(&mark);

                if let Some((start, end, _)) = context.backward(&iter) {
                    let (b_start, b_end) = buf.bounds();
                    buf.remove_tag_by_name("active-match", &b_start, &b_end);
                    buf.apply_tag_by_name("active-match", &start, &end);

                    buf.select_range(&start, &end);
                    sv.grab_focus();

                    let mut fresh_iter = buf.iter_at_offset(start.offset());
                    sv.scroll_to_iter(&mut fresh_iter, 0.0, true, 0.5, 0.5);
                }
            });
        }

        {
            let context = search_context.clone();
            let re = replace_entry.clone();
            replace_button.connect_clicked(move |_| {
                let text = re.text().to_string();
                let buf = context.buffer().downcast::<SourceBuffer>().unwrap();
                if let Some((start, end)) = buf.selection_bounds() {
                    let mut start_mut = start;
                    let mut end_mut = end;
                    let _ = context.replace(&mut start_mut, &mut end_mut, &text);
                }
            });
        }

        {
            let context = search_context.clone();
            let re = replace_entry.clone();
            replace_all_button.connect_clicked(move |_| {
                let text = re.text().to_string();
                let _ = context.replace_all(&text);
            });
        }

        Self {
            search_bar,
            search_context,
        }
    }

    pub fn setup_overlay(&self, overlay: &Overlay) {
        overlay.add_overlay(&self.search_bar);
        self.search_bar.set_valign(Align::Start);
        self.search_bar.set_halign(Align::Center);
    }
}
