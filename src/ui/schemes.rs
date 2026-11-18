use std::path::PathBuf;

use sourceview5::{StyleScheme, StyleSchemeManager};

use crate::config::*;

pub fn init_search_path() {
    let mgr = StyleSchemeManager::default();
    let mut dirs = vec![PathBuf::from("data/styles")];
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        dirs.push(dir.join("data/styles"));
    }
    for dir in dirs {
        if dir.is_dir()
            && let Some(dir) = dir.to_str()
        {
            mgr.append_search_path(dir);
        }
    }
}

pub fn family_of(id: &str) -> &str {
    id.strip_suffix("-dark")
        .or_else(|| id.strip_suffix("-light"))
        .unwrap_or(id)
}

pub fn is_dark_scheme(scheme: &StyleScheme) -> bool {
    if let Some(variant) = scheme.metadata("variant") {
        return variant == "dark";
    }
    let id = scheme.id();
    id.ends_with("-dark") || matches!(id.as_str(), "oblivion" | "cobalt")
}

pub fn resolve(family: &str, dark: bool) -> Option<StyleScheme> {
    let mgr = StyleSchemeManager::default();
    let (wanted, other) = if dark { ("-dark", "-light") } else { ("-light", "-dark") };
    [format!("{family}{wanted}"), family.to_string(), format!("{family}{other}")]
        .iter()
        .find_map(|id| mgr.scheme(id))
        .or_else(|| mgr.scheme(if dark { "Adwaita-dark" } else { "Adwaita" }))
}

pub fn family_name(scheme: &StyleScheme) -> String {
    let name = scheme.name();
    let trimmed = [" Dark", " Light", " dark", " light", " (Dark)", " (Light)"]
        .iter()
        .find_map(|suffix| name.strip_suffix(suffix))
        .unwrap_or(&name);
    trimmed.trim_end_matches(['-', ' ']).to_string()
}

pub fn saved_family() -> String {
    family_of(&load_pref(PREF_SCHEME, DEFAULT_SCHEME)).to_string()
}

pub fn schemes_for(dark: bool) -> Vec<StyleScheme> {
    let mgr = StyleSchemeManager::default();
    let mut families: Vec<String> = mgr
        .scheme_ids()
        .iter()
        .map(|id| family_of(id).to_string())
        .collect();
    families.sort();
    families.dedup();
    let mut schemes: Vec<StyleScheme> = families
        .iter()
        .filter_map(|f| resolve(f, dark))
        .filter(|s| is_dark_scheme(s) == dark)
        .collect();
    schemes.sort_by_key(|s| s.name().to_lowercase());
    schemes.dedup_by_key(|s| s.id());
    schemes
}
