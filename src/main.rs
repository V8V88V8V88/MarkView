use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, HeaderBar, MenuButton, Orientation, Paned};
use gtk4::{gio, Settings};
use pulldown_cmark::{html, Options, Parser};
use sourceview5::{prelude::*, View as SourceView, Buffer as SourceBuffer};

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("MarkView")
        .default_width(800)
        .default_height(600)
        .build();

    let settings = Settings::default().expect("Failed to get default settings");
    settings.set_gtk_keynav_use_caret(false);
    settings.set_gtk_error_bell(false);

    let header_bar = HeaderBar::new();
    let menu_button = MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .build();
    header_bar.pack_end(&menu_button);
    window.set_titlebar(Some(&header_bar));

    let paned = Paned::builder()
        .orientation(Orientation::Vertical)
        .build();

    let source_view = SourceView::new();
    let source_buffer: SourceBuffer = source_view.buffer().downcast().unwrap();
    source_buffer.set_language(Some(
        &sourceview5::LanguageManager::default()
            .language("markdown")
            .unwrap(),
    ));
    source_buffer.set_highlight_syntax(true);
    source_view.set_show_line_numbers(true);
    source_view.set_monospace(true);

    let markdown_view = gtk4::TextView::new();
    markdown_view.set_editable(false);
    markdown_view.set_wrap_mode(gtk4::WrapMode::Word);

    paned.set_start_child(Some(&source_view));
    paned.set_end_child(Some(&markdown_view));

    let vbox = Box::new(Orientation::Vertical, 0);
    vbox.append(&paned);
    window.set_child(Some (&vbox));

    let markdown_view_clone = markdown_view.clone();
    source_buffer.connect_changed(move |buffer| {
        let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
        let parser = Parser::new_ext(&text, Options::all());
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);
        markdown_view_clone.buffer().set_text(&html_output);
    });

    let menu = gio::Menu::new();
    menu.append(Some("About"), Some("app.about"));
    menu.append(Some("Quit"), Some("app.quit"));
    menu_button.set_menu_model(Some(&menu));

    let about_action = gio::SimpleAction::new("about", None);
    let window_clone = window.clone();
    about_action.connect_activate(move |_, _| {
        let about_dialog = gtk4::AboutDialog::builder()
            .transient_for(&window_clone)
            .modal(true)
            .program_name("MarkView")
            .version("1.0")
            .authors(vec!["Vaibhav Pratap Singh"]) 
            .website("https://github.com/v8v88v8v88/MarkView")
            .website_label("GitHub Repository")
            .build();
        about_dialog.present();
    });
    app.add_action(&about_action);

    let quit_action = gio::SimpleAction::new("quit", None);
    let window_close_clone = window.clone();
    quit_action.connect_activate(move |_, _| {
        window_close_clone.close();
    });
    app.add_action(&quit_action);

    window.present();
}

fn main() {
    let app = Application::builder()
        .application_id("com.example.MarkView")
        .build();

    app.connect_activate(build_ui);
    app.run();
}