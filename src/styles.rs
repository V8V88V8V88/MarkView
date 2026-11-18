struct Palette {
    bg: &'static str,
    fg: &'static str,
    heading: &'static str,
    muted: &'static str,
    rule: &'static str,
    surface: &'static str,
    code_bg: &'static str,
    note: &'static str,
    tip: &'static str,
    important: &'static str,
    warning: &'static str,
    caution: &'static str,
}

const DARK: Palette = Palette {
    bg: "#1d1d20",
    fg: "#e0e0e3",
    heading: "#ffffff",
    muted: "#9a9a9e",
    rule: "rgb(255 255 255 / 15%)",
    surface: "rgb(255 255 255 / 6%)",
    code_bg: "rgb(255 255 255 / 8%)",
    note: "#78aeed",
    tip: "#57e389",
    important: "#dc8add",
    warning: "#f8e45c",
    caution: "#ff7b63",
};

const LIGHT: Palette = Palette {
    bg: "#ffffff",
    fg: "rgb(0 0 6 / 80%)",
    heading: "rgb(0 0 6 / 90%)",
    muted: "rgb(0 0 6 / 55%)",
    rule: "rgb(0 0 6 / 15%)",
    surface: "rgb(0 0 6 / 4%)",
    code_bg: "rgb(0 0 6 / 6%)",
    note: "#1c71d8",
    tip: "#26a269",
    important: "#9141ac",
    warning: "#9c6e03",
    caution: "#c01c28",
};

#[derive(Clone, PartialEq)]
pub struct PreviewTheme {
    pub dark: bool,
    pub accent: String,
    pub font_family: String,
    pub font_size_px: f64,
    pub mono_family: String,
}

impl PreviewTheme {
    pub fn background(&self) -> gtk4::gdk::RGBA {
        gtk4::gdk::RGBA::parse(if self.dark { DARK.bg } else { LIGHT.bg })
            .unwrap_or(gtk4::gdk::RGBA::WHITE)
    }
}

fn css_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn palette_vars(p: &Palette, dark: bool) -> String {
    format!(
        "color-scheme: {scheme};
            --bg: {bg}; --fg: {fg}; --heading: {heading}; --muted: {muted};
            --rule: {rule}; --surface: {surface}; --code-bg: {code_bg};
            --note: {note}; --tip: {tip}; --important: {important};
            --warning: {warning}; --caution: {caution};",
        scheme = if dark { "dark" } else { "light" },
        bg = p.bg,
        fg = p.fg,
        heading = p.heading,
        muted = p.muted,
        rule = p.rule,
        surface = p.surface,
        code_bg = p.code_bg,
        note = p.note,
        tip = p.tip,
        important = p.important,
        warning = p.warning,
        caution = p.caution,
    )
}

pub fn theme_variables(t: &PreviewTheme) -> String {
    let p = if t.dark { &DARK } else { &LIGHT };
    format!(
        ":root {{ {vars}
            --accent: {accent};
            --font: {font}, 'Adwaita Sans', 'Cantarell', system-ui, sans-serif;
            --font-size: {size:.2}px;
            --mono: {mono}, 'Adwaita Mono', 'Source Code Pro', monospace; }}
        @media print {{ :root {{ {print_vars} --accent: {print_accent}; --font-size: 11pt; }} }}",
        vars = palette_vars(p, t.dark),
        accent = t.accent,
        font = css_string(&t.font_family),
        size = t.font_size_px,
        mono = css_string(&t.mono_family),
        print_vars = palette_vars(&LIGHT, false),
        print_accent = LIGHT.note,
    )
}

pub const PREVIEW_CSS: &str = r#"
    html { background: var(--bg); min-height: 100%; }
    body { font-family: var(--font); font-size: var(--font-size); line-height: 1.65;
        padding: 18px 28px; margin: 0; color: var(--fg); background: var(--bg);
        overflow-wrap: break-word; }
    ::selection { background: color-mix(in srgb, var(--accent) 35%, transparent); }
    h1,h2,h3,h4,h5,h6 { color: var(--heading); margin: 1.3em 0 0.5em; font-weight: 700; line-height: 1.25; }
    body > :first-child { margin-top: 0.4em; }
    h1 { font-size: 1.85em; border-bottom: 1px solid var(--rule); padding-bottom: 0.3em; }
    h2 { font-size: 1.45em; border-bottom: 1px solid var(--rule); padding-bottom: 0.25em; }
    h3 { font-size: 1.2em; }
    p { margin: 0.65em 0; }
    a { color: var(--accent); text-decoration: none; }
    a:hover { text-decoration: underline; }
    code { font-family: var(--mono); background: var(--code-bg); padding: 0.15em 0.4em; border-radius: 6px; font-size: 0.9em; }
    pre { line-height: 1.45; background: var(--code-bg); padding: 12px 16px; border-radius: 12px; overflow-x: auto; }
    pre code { background: none; padding: 0; font-size: 0.88em; }
    blockquote { border-left: 3px solid var(--accent); margin: 0.9em 0; padding: 0.35em 1em; color: var(--muted);
        background: var(--surface); border-radius: 0 12px 12px 0; }
    ul,ol { padding-left: 1.8em; }
    li { margin: 0.25em 0; }
    hr { border: none; border-top: 1px solid var(--rule); margin: 1.6em 0; }
    table { border-collapse: separate; border-spacing: 0; width: 100%; margin: 1em 0;
        border: 1px solid var(--rule); border-radius: 12px; overflow: hidden; }
    th,td { padding: 8px 12px; text-align: left; border-bottom: 1px solid var(--rule); }
    th + th, td + td { border-left: 1px solid var(--rule); }
    tr:last-child td { border-bottom: none; }
    th { background: var(--surface); font-weight: 700; }
    img { max-width: 100%; border-radius: 8px; }
    kbd { font-family: var(--mono); font-size: 0.85em; padding: 0.1em 0.45em; border-radius: 6px;
        border: 1px solid var(--rule); box-shadow: inset 0 -1px 0 var(--rule); }
    mark { background: color-mix(in srgb, var(--warning) 40%, transparent); color: inherit; border-radius: 3px; }
    .placeholder { color: var(--muted); text-align: center; margin-top: 30vh; font-size: 1.1em; }

    .metadata { background: var(--surface); border: 1px dashed var(--rule); border-radius: 12px;
        padding: 12px; margin-bottom: 20px; font-family: var(--mono); font-size: 0.85em;
        color: var(--muted); white-space: pre-wrap; }
    .metadata::before { content: "Metadata"; display: block; font-family: var(--font); font-size: 0.8em;
        font-weight: 700; margin-bottom: 6px; opacity: 0.8; }
    blockquote[class^="markdown-alert-"] { color: inherit; border-left-width: 4px; }
    blockquote[class^="markdown-alert-"]::before { display: block; font-weight: 700; margin-bottom: 0.2em; }
    blockquote.markdown-alert-note { border-left-color: var(--note); }
    blockquote.markdown-alert-note::before { content: "Note"; color: var(--note); }
    blockquote.markdown-alert-tip { border-left-color: var(--tip); }
    blockquote.markdown-alert-tip::before { content: "Tip"; color: var(--tip); }
    blockquote.markdown-alert-important { border-left-color: var(--important); }
    blockquote.markdown-alert-important::before { content: "Important"; color: var(--important); }
    blockquote.markdown-alert-warning { border-left-color: var(--warning); }
    blockquote.markdown-alert-warning::before { content: "Warning"; color: var(--warning); }
    blockquote.markdown-alert-caution { border-left-color: var(--caution); }
    blockquote.markdown-alert-caution::before { content: "Caution"; color: var(--caution); }
    sup.footnote-reference a { text-decoration: none; }
    .footnote-definition { font-size: 0.9em; color: var(--muted); margin: 0.4em 0; }
    .footnote-definition:first-of-type { border-top: 1px solid var(--rule); padding-top: 1em; margin-top: 2em; }
    .footnote-definition p { display: inline; }
    .footnote-definition-label { margin-right: 0.4em; }
    dt { font-weight: 700; margin-top: 0.6em; }
    dd { margin: 0.2em 0 0.4em 1.8em; }
    li:has(> input[type="checkbox"]) { list-style: none; }
    li > input[type="checkbox"] { margin: 0 0.5em 0 -1.4em; vertical-align: middle; accent-color: var(--accent); }
    sup, sub { line-height: 0; }
    .math { font-family: var(--mono); font-size: 0.95em; }
    .math-display { display: block; text-align: center; margin: 1em 0; overflow-x: auto; }

    @media print {
        html, body { background: #fff !important; min-height: 0; }
        body { padding: 0 !important; max-width: none !important; margin: 0 !important; }
        * { -webkit-print-color-adjust: exact; print-color-adjust: exact; }
        h1, h2, h3, h4, h5, h6 { break-after: avoid; }
        pre, blockquote, tr, img { break-inside: avoid; }
        pre, pre code { white-space: pre-wrap; overflow-wrap: anywhere; overflow: visible; }
        a { color: var(--accent); }
    }
"#;

pub const APP_CSS: &str = r#"
    .style-switcher { margin: 6px 0 9px; }
    .style-switcher checkbutton {
        padding: 1px; min-width: 44px; min-height: 44px;
        border-radius: 9999px; background-clip: content-box;
        box-shadow: inset 0 0 0 1px alpha(currentColor, 0.15);
    }
    .style-switcher checkbutton:checked { box-shadow: inset 0 0 0 2px var(--accent-bg-color); }
    .style-switcher checkbutton.follow { background-image: linear-gradient(to bottom right, #fff 49.99%, #202020 50.01%); }
    .style-switcher checkbutton.light { background-color: #fff; }
    .style-switcher checkbutton.dark { background-color: #202020; }
    .style-switcher checkbutton radio {
        -gtk-icon-source: none; border: none; box-shadow: none; background: none;
        min-width: 12px; min-height: 12px; padding: 2px; margin: 0;
        transform: translate(27px, 14px);
    }
    .style-switcher checkbutton radio:checked {
        -gtk-icon-source: -gtk-icontheme("object-select-symbolic");
        background-color: var(--accent-bg-color); color: var(--accent-fg-color);
        border-radius: 9999px;
    }
"#;
