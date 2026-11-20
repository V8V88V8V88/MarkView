use gtk4::prelude::*;
use gtk4::TextIter;
use sourceview5::Buffer;

#[derive(Clone, Copy, PartialEq)]
enum Inline {
    Bold,
    Italic,
    Strike,
    Code,
}

impl Inline {
    fn marker(self) -> &'static str {
        match self {
            Inline::Bold => "**",
            Inline::Italic => "*",
            Inline::Strike => "~~",
            Inline::Code => "`",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Block {
    Bullet,
    Numbered,
    Task,
    Quote,
}

/// Applies a toolbar action by name. Returns false for an unknown name.
pub fn apply(buffer: &Buffer, name: &str) -> bool {
    buffer.begin_user_action();
    let known = match name {
        "bold" => toggle_inline(buffer, Inline::Bold),
        "italic" => toggle_inline(buffer, Inline::Italic),
        "strikethrough" => toggle_inline(buffer, Inline::Strike),
        "code" => toggle_inline(buffer, Inline::Code),
        "bullet-list" => toggle_block(buffer, Block::Bullet),
        "numbered-list" => toggle_block(buffer, Block::Numbered),
        "task-list" => toggle_block(buffer, Block::Task),
        "quote" => toggle_block(buffer, Block::Quote),
        "link" => insert_link(buffer),
        "table" if table_columns(&cursor_line(buffer)).is_some() => add_table_row(buffer),
        "table" => insert_on_own_line(
            buffer,
            "| Column 1 | Column 2 |\n| -------- | -------- |\n|          |          |",
            Some((0, 2, 10)),
        ),
        "rule" => insert_on_own_line(buffer, "---", None),
        _ => match name.strip_prefix("heading-").and_then(|l| l.parse::<usize>().ok()) {
            Some(level @ 1..=6) => set_heading(buffer, level),
            _ => false,
        },
    };
    buffer.end_user_action();
    known
}

pub fn insert_image(buffer: &Buffer, alt: &str, destination: &str) {
    buffer.begin_user_action();
    let (mut start, mut end) = selection(buffer);
    let selected = buffer.text(&start, &end, false);
    let alt = if selected.is_empty() { alt.to_string() } else { selected.to_string() };
    buffer.delete(&mut start, &mut end);
    let destination = if destination.contains([' ', '(', ')']) {
        format!("<{destination}>")
    } else {
        destination.to_string()
    };
    buffer.insert(&mut start, &format!("![{alt}]({destination})"));
    buffer.place_cursor(&start);
    buffer.end_user_action();
}

/// Continues a list, quote or table on Enter. Returns true if it handled the key.
pub fn handle_return(buffer: &Buffer) -> bool {
    if buffer.has_selection() {
        return false;
    }
    let cursor = buffer.iter_at_mark(&buffer.get_insert());
    if !cursor.ends_line() {
        return false;
    }
    let line = cursor_line(buffer);
    let continuation = if let Some(cells) = table_columns(&line) {
        if cells.iter().all(|c| c.trim().is_empty()) { None } else { Some(empty_row(cells.len())) }
    } else if let Some(rest) = line.strip_prefix('>') {
        if rest.trim().is_empty() { None } else { Some("> ".to_string()) }
    } else {
        let indent = &line[..line.len() - line.trim_start().len()];
        let rest = &line[indent.len()..];
        match list_marker(rest) {
            Some((kind, len)) => {
                if rest[len..].trim().is_empty() {
                    None
                } else {
                    let marker = match kind {
                        Block::Task => format!("{} [ ] ", &rest[..1]),
                        Block::Numbered => {
                            let digits = rest.chars().take_while(char::is_ascii_digit).count();
                            let n: u64 = rest[..digits].parse().unwrap_or(0);
                            format!("{}{} ", n + 1, &rest[digits..digits + 1])
                        }
                        _ => rest[..2].to_string(),
                    };
                    Some(format!("{indent}{marker}"))
                }
            }
            None => return false,
        }
    };

    buffer.begin_user_action();
    match continuation {
        Some(next) => {
            let mut at = cursor;
            let offset = at.offset();
            buffer.insert(&mut at, &format!("\n{next}"));
            let cell = if next.starts_with('|') { 3 } else { 1 + next.chars().count() as i32 };
            buffer.place_cursor(&iter(buffer, offset + cell));
        }
        None => {
            let (mut start, mut end, _) = line_text(buffer, cursor.line());
            buffer.delete(&mut start, &mut end);
        }
    }
    buffer.end_user_action();
    true
}

fn cursor_line(buffer: &Buffer) -> String {
    line_text(buffer, buffer.iter_at_mark(&buffer.get_insert()).line()).2
}

/// The cells of a Markdown table row, or None if `line` isn't one.
fn table_columns(line: &str) -> Option<Vec<String>> {
    let t = line.trim();
    if t.len() < 2 || !t.starts_with('|') || !t.ends_with('|') {
        return None;
    }
    Some(t[1..t.len() - 1].split('|').map(str::to_string).collect())
}

fn empty_row(columns: usize) -> String {
    format!("|{}", "   |".repeat(columns.max(1)))
}

fn add_table_row(buffer: &Buffer) -> bool {
    let mut line = buffer.iter_at_mark(&buffer.get_insert()).line();
    let Some(columns) = table_columns(&line_text(buffer, line).2).map(|c| c.len()) else { return false };
    let next = line_text(buffer, line + 1).2;
    let is_separator = |l: &str| {
        table_columns(l).is_some_and(|c| c.iter().all(|c| !c.trim().is_empty() && c.trim().chars().all(|ch| matches!(ch, '-' | ':'))))
    };
    if line + 1 < buffer.line_count() && is_separator(&next) {
        line += 1;
    }
    let mut end = line_text(buffer, line).1;
    let offset = end.offset();
    buffer.insert(&mut end, &format!("\n{}", empty_row(columns)));
    buffer.place_cursor(&iter(buffer, offset + 3));
    true
}

fn selection(buffer: &Buffer) -> (TextIter, TextIter) {
    buffer.selection_bounds().unwrap_or_else(|| {
        let it = buffer.iter_at_mark(&buffer.get_insert());
        (it, it)
    })
}

fn iter(buffer: &Buffer, offset: i32) -> TextIter {
    buffer.iter_at_offset(offset)
}

fn select(buffer: &Buffer, start: i32, end: i32) {
    buffer.select_range(&iter(buffer, start), &iter(buffer, end));
}

fn delete(buffer: &Buffer, start: i32, end: i32) {
    buffer.delete(&mut iter(buffer, start), &mut iter(buffer, end));
}

fn run(buffer: &Buffer, offset: i32, c: char, forward: bool) -> i32 {
    let mut n = 0;
    let mut it = iter(buffer, offset);
    while n < 3 {
        if forward {
            if it.char() != c {
                break;
            }
            it.forward_char();
        } else {
            if !it.backward_char() || it.char() != c {
                break;
            }
        }
        n += 1;
    }
    n
}

fn toggle_inline(buffer: &Buffer, kind: Inline) -> bool {
    let (mut start, mut end) = selection(buffer);
    if start == end && (start.inside_word() || start.ends_word()) && !start.starts_line() {
        if !start.starts_word() {
            start.backward_word_start();
        }
        if !end.ends_word() {
            end.forward_word_end();
        }
    }
    if kind == Inline::Code && start.line() != end.line() {
        return wrap_code_block(buffer, &start, &end);
    }

    let marker = kind.marker();
    let k = marker.len() as i32;
    let c = marker.chars().next().unwrap_or('*');
    let (so, eo) = (start.offset(), end.offset());
    let matches = |n: i32| n >= k && !(kind == Inline::Italic && n == 2);
    let selected = buffer.text(&start, &end, false);

    if so != eo && matches(run(buffer, so, c, false)) && matches(run(buffer, eo, c, true)) {
        delete(buffer, eo, eo + k);
        delete(buffer, so - k, so);
        select(buffer, so - k, eo - k);
    } else if selected.chars().count() as i32 >= 2 * k
        && matches(run(buffer, so, c, true))
        && matches(run(buffer, eo, c, false))
    {
        delete(buffer, eo - k, eo);
        delete(buffer, so, so + k);
        select(buffer, so, eo - 2 * k);
    } else {
        buffer.insert(&mut iter(buffer, eo), marker);
        buffer.insert(&mut iter(buffer, so), marker);
        select(buffer, so + k, eo + k);
    }
    true
}

fn wrap_code_block(buffer: &Buffer, start: &TextIter, end: &TextIter) -> bool {
    let mut first = buffer.iter_at_line(start.line()).unwrap_or(*start);
    let mut last = *end;
    if last.starts_line() && last.line() > first.line() {
        last.backward_char();
    } else if !last.ends_line() {
        last.forward_to_line_end();
    }
    let text = buffer.text(&first, &last, false).to_string();
    let fenced = format!("```\n{text}\n```");
    let so = first.offset();
    buffer.delete(&mut first, &mut last);
    buffer.insert(&mut iter(buffer, so), &fenced);
    select(buffer, so + 4, so + 4 + text.chars().count() as i32);
    true
}

fn selected_lines(buffer: &Buffer) -> (i32, i32) {
    let (start, end) = selection(buffer);
    let mut last = end.line();
    if end.starts_line() && last > start.line() {
        last -= 1;
    }
    (start.line(), last)
}

fn line_text(buffer: &Buffer, line: i32) -> (TextIter, TextIter, String) {
    let start = buffer.iter_at_line(line).unwrap_or_else(|| buffer.end_iter());
    let mut end = start;
    if !end.ends_line() {
        end.forward_to_line_end();
    }
    let text = buffer.text(&start, &end, false).to_string();
    (start, end, text)
}

/// The list marker at the start of `rest` (after indentation), and its length.
fn list_marker(rest: &str) -> Option<(Block, usize)> {
    for bullet in ["- ", "* ", "+ "] {
        if let Some(after) = rest.strip_prefix(bullet) {
            for check in ["[ ] ", "[x] ", "[X] "] {
                if after.starts_with(check) {
                    return Some((Block::Task, 6));
                }
            }
            return Some((Block::Bullet, 2));
        }
    }
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    let tail = &rest[digits..];
    if digits > 0 && (tail.starts_with(". ") || tail.starts_with(") ")) {
        return Some((Block::Numbered, digits + 2));
    }
    None
}

fn has_block(line: &str, kind: Block) -> bool {
    let rest = line.trim_start();
    match kind {
        Block::Quote => line.starts_with('>'),
        _ => list_marker(rest).is_some_and(|(k, _)| k == kind),
    }
}

fn without_block(line: &str, kind: Block) -> String {
    if kind == Block::Quote {
        let rest = line.strip_prefix('>').unwrap_or(line);
        return rest.strip_prefix(' ').unwrap_or(rest).to_string();
    }
    let indent = line.len() - line.trim_start().len();
    let rest = &line[indent..];
    match list_marker(rest) {
        Some((_, len)) => format!("{}{}", &line[..indent], &rest[len..]),
        None => line.to_string(),
    }
}

fn toggle_block(buffer: &Buffer, kind: Block) -> bool {
    let (first, last) = selected_lines(buffer);
    let lines: Vec<String> = (first..=last).map(|l| line_text(buffer, l).2).collect();
    let content = |l: &String| !l.trim().is_empty();
    let remove = lines.iter().filter(|l| content(l)).all(|l| has_block(l, kind)) && lines.iter().any(content);

    let mut number = 0;
    let new: Vec<String> = lines
        .iter()
        .map(|line| {
            if remove {
                return without_block(line, kind);
            }
            if kind == Block::Quote {
                return if line.is_empty() { ">".into() } else { format!("> {line}") };
            }
            if !content(line) {
                return line.clone();
            }
            let stripped = without_block(line, Block::Bullet);
            let indent = stripped.len() - stripped.trim_start().len();
            number += 1;
            let marker = match kind {
                Block::Bullet => "- ".to_string(),
                Block::Numbered => format!("{number}. "),
                _ => "- [ ] ".to_string(),
            };
            format!("{}{marker}{}", &stripped[..indent], &stripped[indent..])
        })
        .collect();

    replace_lines(buffer, first, last, &new.join("\n"));
    true
}

fn replace_lines(buffer: &Buffer, first: i32, last: i32, text: &str) {
    let mut start = line_text(buffer, first).0;
    let mut end = line_text(buffer, last).1;
    let so = start.offset();
    buffer.delete(&mut start, &mut end);
    buffer.insert(&mut iter(buffer, so), text);
    if first == last {
        let line_end = line_text(buffer, first).1;
        buffer.place_cursor(&line_end);
    } else {
        select(buffer, so, so + text.chars().count() as i32);
    }
}

fn set_heading(buffer: &Buffer, level: usize) -> bool {
    let (first, last) = selected_lines(buffer);
    let new: Vec<String> = (first..=last)
        .map(|l| {
            let line = line_text(buffer, l).2;
            let hashes = line.chars().take_while(|&c| c == '#').count();
            let current = if (1..=6).contains(&hashes) && line[hashes..].starts_with(' ') { hashes } else { 0 };
            let text = if current > 0 { line[current + 1..].to_string() } else { line };
            if current == level { text } else { format!("{} {text}", "#".repeat(level)) }
        })
        .collect();
    replace_lines(buffer, first, last, &new.join("\n"));
    true
}

fn insert_link(buffer: &Buffer) -> bool {
    let (mut start, mut end) = selection(buffer);
    let selected = buffer.text(&start, &end, false).to_string();
    let so = start.offset();
    buffer.delete(&mut start, &mut end);
    let is_url = selected.contains("://") || selected.starts_with("www.");
    let (text, cursor) = if is_url {
        (format!("[]({selected})"), so + 1)
    } else {
        let n = selected.chars().count() as i32;
        (format!("[{selected}]()"), if selected.is_empty() { so + 1 } else { so + n + 3 })
    };
    buffer.insert(&mut iter(buffer, so), &text);
    buffer.place_cursor(&iter(buffer, cursor));
    true
}

/// Puts `block` on its own lines with blank lines around it; `select_range` is (line, from, to) within it.
fn insert_on_own_line(buffer: &Buffer, block: &str, select_range: Option<(i32, i32, i32)>) -> bool {
    let cursor = buffer.iter_at_mark(&buffer.get_insert());
    let (_, line_end, text) = line_text(buffer, cursor.line());
    let (at, before) = if text.trim().is_empty() {
        let line = cursor.line();
        let previous_blank = line == 0 || line_text(buffer, line - 1).2.trim().is_empty();
        (line_text(buffer, line).0, if previous_blank { "" } else { "\n" })
    } else {
        (line_end, "\n\n")
    };
    let mut after_it = at;
    after_it.forward_line();
    let after = if at.is_end() || !line_text(buffer, after_it.line()).2.trim().is_empty() { "\n" } else { "" };
    let start = at.offset() + before.chars().count() as i32;
    buffer.insert(&mut iter(buffer, at.offset()), &format!("{before}{block}{after}"));
    match select_range {
        Some((line, from, to)) => {
            let line_start = iter(buffer, start).line() + line;
            let base = line_text(buffer, line_start).0.offset();
            select(buffer, base + from, base + to);
        }
        None => {
            let end = start + block.chars().count() as i32;
            buffer.place_cursor(&iter(buffer, end));
        }
    }
    true
}

pub fn toolbar() -> gtk4::Box {
    let buttons = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    let button = |icon: Option<&str>, label: &str, tooltip: &str, name: &str| {
        let b = gtk4::Button::builder()
            .tooltip_text(tooltip)
            .action_name("win.format")
            .action_target(&name.to_variant())
            .css_classes(["flat"])
            .build();
        match icon {
            Some(icon) => b.set_icon_name(icon),
            None => b.set_label(label),
        }
        b
    };

    let headings = gtk4::gio::Menu::new();
    for level in 1..=6 {
        headings.append(Some(&format!("Heading {level}")), Some(&format!("win.format::heading-{level}")));
    }
    let heading = gtk4::MenuButton::builder()
        .label("H")
        .tooltip_text("Heading")
        .menu_model(&headings)
        .css_classes(["flat"])
        .build();

    buttons.append(&button(Some("format-text-bold-symbolic"), "", "Bold", "bold"));
    buttons.append(&button(Some("format-text-italic-symbolic"), "", "Italic", "italic"));
    buttons.append(&button(Some("format-text-strikethrough-symbolic"), "", "Strikethrough", "strikethrough"));
    buttons.append(&heading);
    buttons.append(&button(Some("view-list-bullet-symbolic"), "", "Bulleted List", "bullet-list"));
    buttons.append(&button(Some("view-list-ordered-symbolic"), "", "Numbered List", "numbered-list"));
    buttons.append(&button(Some("checkbox-checked-symbolic"), "", "Task List", "task-list"));
    buttons.append(&button(None, "“", "Quote", "quote"));
    buttons.append(&button(None, "</>", "Code", "code"));
    buttons.append(&button(Some("insert-link-symbolic"), "", "Link", "link"));
    buttons.append(&button(Some("insert-image-symbolic"), "", "Image", "image"));
    buttons.append(&button(Some("x-office-spreadsheet-symbolic"), "", "Table", "table"));
    buttons.append(&button(None, "—", "Horizontal Line", "rule"));

    buttons.set_hexpand(true);
    buttons.set_halign(gtk4::Align::Center);
    let bar = gtk4::Box::builder().css_classes(["toolbar", "view"]).build();
    bar.append(&buttons);
    bar
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buffer(text: &str, sel: (i32, i32)) -> Buffer {
        let b = Buffer::new(None);
        b.set_text(text);
        b.select_range(&b.iter_at_offset(sel.0), &b.iter_at_offset(sel.1));
        b
    }

    fn run(text: &str, sel: (i32, i32), name: &str) -> (String, String) {
        let b = buffer(text, sel);
        assert!(apply(&b, name));
        let (s, e) = selection(&b);
        (b.text(&b.start_iter(), &b.end_iter(), false).into(), b.text(&s, &e, false).into())
    }

    #[test]
    fn formatting() {
        gtk4::init().unwrap();
        assert_eq!(run("hello world", (0, 5), "bold"), ("**hello** world".into(), "hello".into()));
        assert_eq!(run("**hello** world", (2, 7), "bold"), ("hello world".into(), "hello".into()));
        assert_eq!(run("**hello** world", (0, 9), "bold"), ("hello world".into(), "hello".into()));
        assert_eq!(run("hello world", (2, 2), "italic").0, "*hello* world");
        assert_eq!(run("**hello**", (2, 7), "italic").0, "***hello***");
        assert_eq!(run("***hello***", (3, 8), "italic").0, "**hello**");
        assert_eq!(run("a  b", (2, 2), "code").0, "a `` b");
        assert_eq!(run("x\ny", (0, 3), "code").0, "```\nx\ny\n```");
        assert_eq!(run("a\nb\n\nc", (0, 6), "bullet-list").0, "- a\n- b\n\n- c");
        assert_eq!(run("- a\n- b", (0, 7), "bullet-list").0, "a\nb");
        assert_eq!(run("- a\n- b", (0, 7), "numbered-list").0, "1. a\n2. b");
        assert_eq!(run("a", (0, 0), "task-list").0, "- [ ] a");
        assert_eq!(run("- [x] a", (0, 0), "task-list").0, "a");
        assert_eq!(run("a\nb", (0, 3), "quote").0, "> a\n> b");
        assert_eq!(run("> a", (0, 0), "quote").0, "a");
        assert_eq!(run("Title", (0, 0), "heading-2").0, "## Title");
        assert_eq!(run("## Title", (0, 0), "heading-2").0, "Title");
        assert_eq!(run("## Title", (0, 0), "heading-1").0, "# Title");
        assert_eq!(run("see docs", (4, 8), "link").0, "see [docs]()");
        assert_eq!(run("https://x.org", (0, 13), "link").0, "[](https://x.org)");
        assert_eq!(run("text", (4, 4), "rule").0, "text\n\n---\n");
        assert_eq!(run("text\n", (5, 5), "rule").0, "text\n\n---\n");
        let (t, sel) = run("", (0, 0), "table");
        assert_eq!(t, "| Column 1 | Column 2 |\n| -------- | -------- |\n|          |          |\n");
        assert_eq!(sel, "Column 1");
        let b = buffer("x", (1, 1));
        insert_image(&b, "my pic", "img/a b.png");
        assert_eq!(b.text(&b.start_iter(), &b.end_iter(), false), "x![my pic](<img/a b.png>)");
    }
}
