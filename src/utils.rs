use gtk4::gio;
use gtk4::gio::prelude::*;
use std::path::PathBuf;

pub fn data_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("data")];
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(PathBuf::from)) {
        dirs.push(exe_dir.join("data"));
        if let Some(prefix) = exe_dir.parent() {
            dirs.push(prefix.join("Resources").join("data"));
            dirs.push(prefix.join("share").join("markview"));
        }
    }
    dirs.retain(|d| d.is_dir());
    dirs
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn base_uri_for_preview(current_file: Option<&gio::File>) -> String {
    if let Some(parent) = current_file.and_then(|f| f.parent()) {
        let mut uri = parent.uri().to_string();
        if !uri.ends_with('/') {
            uri.push('/');
        }
        return uri;
    }
    std::env::current_dir()
        .ok()
        .and_then(|path| path.canonicalize().ok())
        .map(|path| gio::File::for_path(path).uri().to_string() + "/")
        .unwrap_or_else(|| "file:///".into())
}

pub fn display_name(file: &gio::File) -> String {
    file.basename()
        .map(|b| b.to_string_lossy().into_owned())
        .unwrap_or_else(|| file.parse_name().to_string())
}

pub fn display_folder(file: &gio::File) -> String {
    let Some(parent) = file.parent() else {
        return String::new();
    };
    match parent.path() {
        Some(path) => {
            let home = gtk4::glib::home_dir();
            match path.strip_prefix(&home) {
                Ok(rest) if rest.as_os_str().is_empty() => "~".into(),
                Ok(rest) => format!("~/{}", rest.display()),
                Err(_) => path.display().to_string(),
            }
        }
        None => parent.parse_name().to_string(),
    }
}

pub fn is_markdown_uri(uri: &str) -> bool {
    let path = uri.split(['#', '?']).next().unwrap_or(uri).to_ascii_lowercase();
    path.ends_with(".md") || path.ends_with(".markdown")
}
