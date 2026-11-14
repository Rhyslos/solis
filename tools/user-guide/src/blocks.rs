//! The manual's Markdown, cut into blocks: headings, paragraphs, lists,
//! tables, quotes and code. Only what the manual uses is read, and the
//! rest stays words: nothing a page holds can stop the guide being built.

/// One block of a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// A heading: its level (1 to 6) and its words.
    Heading(u8, String),
    /// A paragraph's words, its lines joined by newlines.
    Paragraph(String),
    /// Code between fences: the language named after the fence, and the
    /// lines.
    Code(String, String),
    /// A list.
    List(List),
    /// A table.
    Table(Table),
    /// A quote, with blocks of its own.
    Quote(Vec<Block>),
    /// A line across.
    Rule,
}

/// A list: numbered (with the number it starts at) or not, and its items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct List {
    /// The first item's number, for a numbered list.
    pub start: Option<u32>,
    /// Each item's blocks.
    pub items: Vec<Vec<Block>>,
    /// Whether blank lines part its items: then each paragraph is drawn
    /// as one.
    pub loose: bool,
}

/// A table: its heading row and the rows under it, as cells of words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// The heading row.
    pub head: Vec<String>,
    /// The other rows, each as long as the heading row.
    pub rows: Vec<Vec<String>>,
}

/// How deep quotes and lists may go inside each other: deeper is read as
/// words, so a hostile page can't exhaust the stack.
const DEEPEST: usize = 16;

/// Cuts `text` into blocks.
pub fn parse(text: &str) -> Vec<Block> {
    let text = text.replace("\r\n", "\n");
    let lines: Vec<&str> = text.split('\n').collect();
    blocks(&lines, 0)
}

fn blocks(lines: &[&str], depth: usize) -> Vec<Block> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        let (block, used) = block_at(lines, i, depth);
        out.push(block);
        i += used.max(1);
    }
    out
}

/// The block starting at line `i`, and how many lines it used.
fn block_at(lines: &[&str], i: usize, depth: usize) -> (Block, usize) {
    let line = lines[i];
    if let Some(fence) = fence(line) {
        return code(lines, i, &fence);
    }
    if let Some((level, words)) = heading(line) {
        return (Block::Heading(level, words), 1);
    }
    if is_rule(line) {
        return (Block::Rule, 1);
    }
    if depth < DEEPEST {
        if quoted(line).is_some() {
            return quote(lines, i, depth);
        }
        if let Some(marker) = list_marker(line) {
            return list(lines, i, &marker, depth);
        }
    }
    if starts_table(lines, i) {
        return table(lines, i);
    }
    paragraph(lines, i)
}

/// An opening fence: its indent, its character, its length and the
/// language after it.
struct Fence {
    indent: usize,
    mark: char,
    length: usize,
    language: String,
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn fence(line: &str) -> Option<Fence> {
    let indent = indent_of(line);
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let mark = rest.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let length = rest.chars().take_while(|c| *c == mark).count();
    if length < 3 {
        return None;
    }
    let language = rest[length..].trim().to_string();
    if mark == '`' && language.contains('`') {
        return None;
    }
    Some(Fence {
        indent,
        mark,
        length,
        language,
    })
}

fn code(lines: &[&str], i: usize, open: &Fence) -> (Block, usize) {
    let mut body = Vec::new();
    let mut j = i + 1;
    while j < lines.len() {
        let line = lines[j];
        let closes = fence(line).is_some_and(|f| {
            f.mark == open.mark && f.length >= open.length && f.language.is_empty()
        });
        if closes {
            j += 1;
            break;
        }
        let strip = indent_of(line).min(open.indent);
        body.push(&line[strip..]);
        j += 1;
    }
    let words = open.language.split_whitespace().next().unwrap_or("");
    (Block::Code(words.to_string(), body.join("\n")), j - i)
}

fn heading(line: &str) -> Option<(u8, String)> {
    if indent_of(line) > 3 {
        return None;
    }
    let rest = line.trim_start();
    let level = rest.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let after = &rest[level..];
    if !after.is_empty() && !after.starts_with(' ') {
        return None;
    }
    let words = after.trim().trim_end_matches('#').trim_end();
    Some((level as u8, words.to_string()))
}

fn is_rule(line: &str) -> bool {
    if indent_of(line) > 3 {
        return false;
    }
    let marks: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    let Some(first) = marks.chars().next() else {
        return false;
    };
    matches!(first, '-' | '*' | '_') && marks.len() >= 3 && marks.chars().all(|c| c == first)
}

/// The line without its quote mark, if it is quoted.
fn quoted(line: &str) -> Option<&str> {
    if indent_of(line) > 3 {
        return None;
    }
    let rest = line.trim_start().strip_prefix('>')?;
    Some(rest.strip_prefix(' ').unwrap_or(rest))
}

fn quote(lines: &[&str], i: usize, depth: usize) -> (Block, usize) {
    let mut inner = Vec::new();
    let mut j = i;
    while j < lines.len() {
        match quoted(lines[j]) {
            Some(rest) => inner.push(rest),
            // A line straight after quoted words carries their paragraph on.
            None if !lines[j].trim().is_empty()
                && inner.last().is_some_and(|l| !l.trim().is_empty())
                && !interrupts(lines[j]) =>
            {
                inner.push(lines[j]);
            }
            None => break,
        }
        j += 1;
    }
    (Block::Quote(blocks(&inner, depth + 1)), j - i)
}

/// What starts a list item: its indent, whether it is numbered (and its
/// number), its mark's character, and where the item's words begin.
#[derive(Debug, Clone)]
struct Marker {
    indent: usize,
    number: Option<u32>,
    mark: char,
    content: usize,
}

fn list_marker(line: &str) -> Option<Marker> {
    let indent = indent_of(line);
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let (number, mark, width) = if let Some(c) = rest.chars().next().filter(|c| "-*+".contains(*c))
    {
        (None, c, 1)
    } else {
        let digits = rest.chars().take_while(char::is_ascii_digit).count();
        if digits == 0 || digits > 9 {
            return None;
        }
        let mark = rest[digits..]
            .chars()
            .next()
            .filter(|c| *c == '.' || *c == ')')?;
        (rest[..digits].parse().ok(), mark, digits + 1)
    };
    let after = &rest[width..];
    if !after.is_empty() && !after.starts_with(' ') {
        return None;
    }
    let spaces = indent_of(after);
    // More than four spaces would be code inside the item: count one.
    let gap = if after.trim().is_empty() || spaces > 4 {
        1
    } else {
        spaces
    };
    Some(Marker {
        indent,
        number,
        mark,
        content: indent + width + gap,
    })
}

/// Whether `line` starts a block that ends a paragraph above it.
fn interrupts(line: &str) -> bool {
    fence(line).is_some()
        || heading(line).is_some()
        || is_rule(line)
        || quoted(line).is_some()
        || list_marker(line)
            .is_some_and(|m| m.number.is_none_or(|n| n == 1) && !line.trim()[1..].trim().is_empty())
}

fn same_list(a: &Marker, b: &Marker) -> bool {
    a.mark == b.mark && a.number.is_some() == b.number.is_some()
}

fn list(lines: &[&str], i: usize, first: &Marker, depth: usize) -> (Block, usize) {
    let mut items = Vec::new();
    let mut loose = false;
    let mut j = i;
    let mut marker = first.clone();
    loop {
        let (body, used, blank_inside) = item(lines, j, &marker);
        loose |= blank_inside;
        let parsed = blocks(&body, depth + 1);
        loose |= parsed
            .iter()
            .filter(|b| matches!(b, Block::Paragraph(_)))
            .count()
            > 1;
        items.push(parsed);
        j += used;
        // Blank lines, then another item of the same list, keep it going.
        let mut k = j;
        while k < lines.len() && lines[k].trim().is_empty() {
            k += 1;
        }
        match lines.get(k).and_then(|l| list_marker(l)) {
            Some(next) if same_list(first, &next) && next.indent < first.content => {
                loose |= k > j;
                marker = next;
                j = k;
            }
            _ => break,
        }
    }
    let block = Block::List(List {
        start: first.number,
        items,
        loose,
    });
    (block, j - i)
}

/// One item's lines (without its mark and indent), how many lines it
/// used, and whether a blank line parts its blocks.
fn item<'a>(lines: &[&'a str], i: usize, marker: &Marker) -> (Vec<&'a str>, usize, bool) {
    let first = lines[i];
    let mut body = vec![first.get(marker.content..).unwrap_or("").trim_end()];
    if first.trim_end().len() <= marker.content {
        body[0] = "";
    }
    let mut j = i + 1;
    let mut blank_inside = false;
    let mut in_code = fence(body[0]).is_some();
    while j < lines.len() {
        let line = lines[j];
        if line.trim().is_empty() {
            // The item goes on only if an indented line follows.
            let next = lines[j..].iter().position(|l| !l.trim().is_empty());
            match next.map(|n| lines[j + n]) {
                Some(l) if indent_of(l) >= marker.content => {
                    if !in_code {
                        blank_inside = true;
                    }
                    body.push("");
                    j += 1;
                    continue;
                }
                _ => break,
            }
        }
        if indent_of(line) >= marker.content {
            let rest = &line[marker.content..];
            if fence(rest).is_some() {
                in_code = !in_code;
            }
            body.push(rest);
        } else if in_code {
            body.push(line.trim_start());
        } else if list_marker(line).is_some() {
            // The next item, or a list after this one.
            break;
        } else if body.last().is_some_and(|l| !l.trim().is_empty()) && !interrupts(line) {
            // Words straight after words carry the paragraph on.
            body.push(line.trim_start());
        } else {
            break;
        }
        j += 1;
    }
    while body.last().is_some_and(|l| l.trim().is_empty()) {
        body.pop();
    }
    (body, j - i, blank_inside)
}

fn is_delimiter_row(line: &str) -> bool {
    let cells = split_row(line);
    !cells.is_empty()
        && cells.iter().all(|c| {
            let c = c.trim();
            let inner = c.trim_start_matches(':').trim_end_matches(':');
            !inner.is_empty() && inner.chars().all(|ch| ch == '-')
        })
}

fn starts_table(lines: &[&str], i: usize) -> bool {
    lines[i].contains('|')
        && lines
            .get(i + 1)
            .is_some_and(|l| l.contains('-') && is_delimiter_row(l))
}

/// A row's cells: the line cut at each `|` that isn't escaped or inside
/// code, without the outer ones.
fn split_row(line: &str) -> Vec<String> {
    let line = line.trim();
    let line = line.strip_prefix('|').unwrap_or(line);
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut ticks = 0;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                cell.push('|');
                chars.next();
            }
            '`' => {
                ticks = if ticks == 0 { 1 } else { 0 };
                cell.push(c);
            }
            '|' if ticks == 0 => cells.push(std::mem::take(&mut cell)),
            _ => cell.push(c),
        }
    }
    if !cell.trim().is_empty() {
        cells.push(cell);
    }
    cells.into_iter().map(|c| c.trim().to_string()).collect()
}

fn table(lines: &[&str], i: usize) -> (Block, usize) {
    let head = split_row(lines[i]);
    let width = head.len();
    let mut rows = Vec::new();
    let mut j = i + 2;
    while j < lines.len() && lines[j].contains('|') && !lines[j].trim().is_empty() {
        let mut row = split_row(lines[j]);
        row.resize(width, String::new());
        rows.push(row);
        j += 1;
    }
    (Block::Table(Table { head, rows }), j - i)
}

fn paragraph(lines: &[&str], i: usize) -> (Block, usize) {
    let mut words = vec![lines[i].trim()];
    let mut j = i + 1;
    while j < lines.len() {
        let line = lines[j];
        if line.trim().is_empty() || interrupts(line) || starts_table(lines, j) {
            break;
        }
        words.push(line.trim());
        j += 1;
    }
    (Block::Paragraph(words.join("\n")), j - i)
}

#[cfg(test)]
#[path = "blocks_tests.rs"]
mod tests;
