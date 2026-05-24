pub const PREVIEW_CSS_DARK: &str = r#"
    :root { color-scheme: dark; background: #1a1a1a !important; }
    html { background: #1a1a1a !important; min-height: 100%; }
    body { font-family: 'Cantarell','Inter',system-ui,sans-serif; font-size: 15px; line-height: 1.7;
        padding: 16px 24px; margin: 0; min-height: 100%; color: #e0e0e0; background: #1a1a1a !important; word-wrap: break-word; }
    h1,h2,h3,h4,h5,h6 { color: #fff; margin-top: 1.2em; margin-bottom: 0.4em; font-weight: 600; }
    h1 { font-size: 1.8em; border-bottom: 1px solid #444; padding-bottom: 0.3em; }
    h2 { font-size: 1.5em; border-bottom: 1px solid #3a3a3a; padding-bottom: 0.2em; }
    h3 { font-size: 1.25em; }
    p { margin: 0.6em 0; }
    a { color: #78b9f5; text-decoration: none; }
    a:hover { text-decoration: underline; }
    code { font-family: 'JetBrains Mono','Source Code Pro',monospace; background: #1e1e1e; padding: 2px 6px; border-radius: 4px; font-size: 0.9em; }
    pre { background: #1e1e1e; padding: 14px 18px; border-radius: 8px; overflow-x: auto; border: 1px solid #3a3a3a; }
    pre code { background: none; padding: 0; }
    blockquote { border-left: 3px solid #78b9f5; margin: 0.8em 0; padding: 0.4em 1em; color: #b0b0b0; background: #252525; border-radius: 0 6px 6px 0; }
    ul,ol { padding-left: 1.8em; }
    li { margin: 0.25em 0; }
    hr { border: none; border-top: 1px solid #444; margin: 1.5em 0; }
    table { border-collapse: collapse; width: 100%; margin: 1em 0; }
    th,td { border: 1px solid #444; padding: 8px 12px; text-align: left; }
    th { background: #333; font-weight: 600; }
    img { max-width: 100%; border-radius: 6px; }
    strong { color: #f0f0f0; }
    em { color: #d0d0d0; }
    .placeholder { color: #a8a8a8; text-align: center; margin-top: 2em; }
    .metadata { 
        background: #252525; 
        border: 1px dashed #444; 
        border-radius: 8px; 
        padding: 12px; 
        margin-bottom: 20px; 
        font-family: 'JetBrains Mono', monospace; 
        font-size: 0.85em;
        color: #a0a0a0;
        white-space: pre-wrap;
    }
    .metadata::before {
        content: "metadata";
        display: block;
        font-size: 0.7em;
        color: #666;
        margin-bottom: 6px;
        border-bottom: 1px solid #333;
        padding-bottom: 2px;
    }
"#;

pub const PRINT_CSS: &str = r#"
    @media print {
        html, body, :root { margin: 0 !important; padding: 0 !important; border: none !important; outline: none !important; }
        body { padding: 16px 24px !important; }
        img { margin: 0 !important; padding: 0 !important; border: none !important; outline: none !important; box-shadow: none !important; }
    }
"#;

pub const PREVIEW_CSS_LIGHT: &str = r#"
    :root { color-scheme: light; background: #fafafa !important; }
    html { background: #fafafa !important; min-height: 100%; }
    body { font-family: 'Cantarell','Inter',system-ui,sans-serif; font-size: 15px; line-height: 1.7;
        padding: 16px 24px; margin: 0; min-height: 100%; color: #241f31; background: #fafafa !important; word-wrap: break-word; }
    h1,h2,h3,h4,h5,h6 { color: #1c1c1c; margin-top: 1.2em; margin-bottom: 0.4em; font-weight: 600; }
    h1 { font-size: 1.8em; border-bottom: 1px solid #c0bfc4; padding-bottom: 0.3em; }
    h2 { font-size: 1.5em; border-bottom: 1px solid #d1d0d5; padding-bottom: 0.2em; }
    h3 { font-size: 1.25em; }
    p { margin: 0.6em 0; }
    a { color: #1c71d8; text-decoration: none; }
    a:hover { text-decoration: underline; }
    code { font-family: 'JetBrains Mono','Source Code Pro',monospace; background: #ebebeb; padding: 2px 6px; border-radius: 4px; font-size: 0.9em; color: #1c1c1c; }
    pre { background: #ebebeb; padding: 14px 18px; border-radius: 8px; overflow-x: auto; border: 1px solid #d1d0d5; }
    pre code { background: none; padding: 0; }
    blockquote { border-left: 3px solid #1c71d8; margin: 0.8em 0; padding: 0.4em 1em; color: #56565c; background: #f0eff1; border-radius: 0 6px 6px 0; }
    ul,ol { padding-left: 1.8em; }
    li { margin: 0.25em 0; }
    hr { border: none; border-top: 1px solid #c0bfc4; margin: 1.5em 0; }
    table { border-collapse: collapse; width: 100%; margin: 1em 0; }
    th,td { border: 1px solid #c0bfc4; padding: 8px 12px; text-align: left; }
    th { background: #ebe9ed; font-weight: 600; }
    img { max-width: 100%; border-radius: 6px; }
    strong { color: #1c1c1c; }
    em { color: #363536; }
    .placeholder { color: #6b6b6b; text-align: center; margin-top: 2em; }
    .metadata { 
        background: #f0eff1; 
        border: 1px dashed #c0bfc4; 
        border-radius: 8px; 
        padding: 12px; 
        margin-bottom: 20px; 
        font-family: 'JetBrains Mono', monospace; 
        font-size: 0.85em;
        color: #666;
        white-space: pre-wrap;
    }
    .metadata::before {
        content: "metadata";
        display: block;
        font-size: 0.7em;
        color: #999;
        margin-bottom: 6px;
        border-bottom: 1px solid #d1d0d5;
        padding-bottom: 2px;
    }
"#;

pub const SEARCH_BAR_CSS: &str = "
    searchbar, revealer, searchbar > revealer > box {
        background-color: transparent;
        border-style: none;
        box-shadow: none;
    }
    .card {
        border-radius: 12px;
        padding: 8px;
        border: 1px solid alpha(@window_fg_color, 0.1);
        background-color: @window_bg_color;
        box-shadow: 0 4px 16px rgba(0,0,0,0.3);
    }
";
