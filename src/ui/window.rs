use adw::prelude::*;
use adw::{Application, ApplicationWindow, HeaderBar};
use gtk4::{Box as GtkBox, Orientation, Paned, ScrolledWindow, Overlay};
use sourceview5::View as SourceView;
use webkit6::WebView;

pub struct MarkViewWindow {
    pub window: ApplicationWindow,
    pub header_bar: HeaderBar,
    pub paned: Paned,
    pub editor_scroll: ScrolledWindow,
    #[allow(dead_code)]
    preview_scroll: ScrolledWindow,
    pub source_view: SourceView,
    pub webview: WebView,
    pub overlay: Overlay,
}

impl MarkViewWindow {
    pub fn new(app: &Application) -> Self {
        let header_bar = HeaderBar::new();

        let paned = Paned::builder()
            .orientation(Orientation::Horizontal)
            .vexpand(true)
            .hexpand(true)
            .build();

        let editor_scroll = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .build();

        let preview_scroll = ScrolledWindow::builder()
            .build();

        let source_view = SourceView::builder()
            .wrap_mode(gtk4::WrapMode::Word)
            .show_line_numbers(true)
            .insert_spaces_instead_of_tabs(true)
            .tab_width(4)
            .build();
        editor_scroll.set_child(Some(&source_view));

        let webview = WebView::new();
        preview_scroll.set_child(Some(&webview));

        paned.set_start_child(Some(&editor_scroll));
        paned.set_end_child(Some(&preview_scroll));
        paned.set_position(600);

        let overlay = Overlay::builder()
            .child(&paned)
            .build();

        let content = GtkBox::new(Orientation::Vertical, 0);
        content.append(&header_bar);
        content.append(&overlay);

        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(1200)
            .default_height(800)
            .title("MarkView")
            .content(&content)
            .build();

        Self {
            window,
            header_bar,
            paned,
            editor_scroll,
            preview_scroll,
            source_view,
            webview,
            overlay,
        }
    }
}
