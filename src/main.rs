use adw::prelude::*;
use adw::{Application, ApplicationWindow, HeaderBar};
use adw::AboutDialog;
use gtk4::{Box, MenuButton, Orientation, Paned};
use gtk4::{gio, Settings};
use pulldown_cmark::{html, Options, Parser};
use sourceview5::{prelude::*, Buffer as SourceBuffer, View as SourceView};

fn build_ui(app: &Application) {
    let settings = Settings::default().expect("Failed to get default settings");
    settings.set_gtk_keynav_use_caret(false);
    settings.set_gtk_error_bell(false);

    let header_bar = HeaderBar::new();
    let menu_button = MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .build();
    header_bar.pack_end(&menu_button);

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

    // Combine the content in a box
    // Adwaita's ApplicationWindow does not include a HeaderBar
    let content = Box::new(Orientation::Vertical, 0);
    content.append(&header_bar);
    content.append(&paned);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("MarkView")
        .default_width(800)
        .default_height(600)
        .content(&content)
        .build();

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
        let about_dialog = AboutDialog::builder()
            .application_name("MarkView")
            .version("1.0")
            .website("https://github.com/v8v88v8v88/MarkView")
            .developers(vec!["Vaibhav Pratap Singh"])
            .license_type(gtk4::License::Gpl30)
            .build();
        about_dialog.present(Some(&window_clone));
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