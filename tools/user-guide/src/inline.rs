//! The words inside a block: bold, italics, code, links, keys. Everything
//! else is written out as it is, made safe for HTML.
//!
//! **Keys** (the manual's convention, `docs/manual/_template.md`): code
//! whose every part, between ` + `, is a key's name (`Ctrl + K`, `Esc`,
//! `F2`) is drawn as keys, and so is `<kbd>…</kbd>`. A mouse word after a
//! key (`Shift + click`) stays a word.

/// Where links and anchors go: the page being written decides.
pub trait Places {
    /// The address a link to `url`, as the manual writes it, has in the
    /// guide.
    fn href(&mut self, url: &str) -> String;
    /// The guide's id for an anchor the manual names `id` on this page.
    fn anchor(&mut self, id: &str) -> String;
}

/// Keys with a name of more than one letter.
const NAMED_KEYS: &[&str] = &[
    "Ctrl",
    "Shift",
    "Alt",
    "Win",
    "Cmd",
    "Esc",
    "Enter",
    "Tab",
    "Space",
    "Backspace",
    "Delete",
    "Insert",
    "Home",
    "End",
    "Page Up",
    "Page Down",
    "Up",
    "Down",
    "Left",
    "Right",
    "Caps Lock",
    "Print Screen",
    "↑",
    "↓",
    "←",
    "→",
];

/// Mouse words that may follow a key (`Ctrl + click`).
const MOUSE_WORDS: &[&str] = &[
    "click",
    "double-click",
    "right-click",
    "middle-click",
    "drag",
    "right-drag",
    "middle-drag",
    "wheel",
    "mouse wheel",
];

/// Whether `part` names one key.
fn is_key(part: &str, alone: bool) -> bool {
    if NAMED_KEYS.contains(&part) {
        return true;
    }
    if let Some(n) = part.strip_prefix('F')
        && n.parse::<u8>().is_ok_and(|n| (1..=24).contains(&n))
    {
        return true;
    }
    let mut chars = part.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_uppercase() || c.is_ascii_digit() => true,
        // A mark is a key only beside another (`Ctrl + /`): alone it is
        // more often code, such as `.`.
        (Some(c), None) => !alone && c.is_ascii_punctuation(),
        _ => false,
    }
}

/// The keys `code` names, if it is keys: each part, and whether it is a
/// key (or a mouse word after one).
fn as_keys(code: &str) -> Option<Vec<(&str, bool)>> {
    let parts: Vec<&str> = code.split(" + ").map(str::trim).collect();
    let alone = parts.len() == 1;
    let mut out = Vec::new();
    for (n, part) in parts.iter().enumerate() {
        if is_key(part, alone) {
            out.push((*part, true));
        } else if n > 0 && MOUSE_WORDS.contains(&part.to_lowercase().as_str()) {
            out.push((*part, false));
        } else {
            return None;
        }
    }
    Some(out)
}

/// Escapes text for HTML.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Keys drawn as keys, joined by a plus.
fn keys_html(keys: &[(&str, bool)]) -> String {
    let parts: Vec<String> = keys
        .iter()
        .map(|(part, key)| {
            if *key {
                format!("<kbd>{}</kbd>", escape(part))
            } else {
                escape(part)
            }
        })
        .collect();
    format!(
        "<span class=\"keys\">{}</span>",
        parts.join("<span class=\"plus\">+</span>")
    )
}

/// The words of `text` as HTML.
pub fn render(text: &str, places: &mut dyn Places) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    walk(&chars, places, &mut out, 0);
    out
}

/// How deep bold and italics may nest before the rest is read as words.
const DEEPEST: usize = 8;

fn walk(chars: &[char], places: &mut dyn Places, out: &mut String, depth: usize) {
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let used = match c {
            '\\' => escaped(chars, i, out),
            '`' => code_span(chars, i, out),
            '!' if chars.get(i + 1) == Some(&'[') => image(chars, i, places, out),
            '[' if depth < DEEPEST => link(chars, i, places, out, depth),
            '<' => html(chars, i, places, out),
            '*' | '_' if depth < DEEPEST => emphasis(chars, i, places, out, depth),
            '\n' => {
                out.push('\n');
                Some(1)
            }
            _ => None,
        };
        match used {
            Some(n) => i += n,
            None => {
                out.push_str(&escape(&c.to_string()));
                i += 1;
            }
        }
    }
}

fn escaped(chars: &[char], i: usize, out: &mut String) -> Option<usize> {
    let next = *chars.get(i + 1)?;
    if !next.is_ascii_punctuation() {
        return None;
    }
    out.push_str(&escape(&next.to_string()));
    Some(2)
}

fn find(chars: &[char], from: usize, what: &[char]) -> Option<usize> {
    if what.is_empty() || from > chars.len() {
        return None;
    }
    chars[from..]
        .windows(what.len())
        .position(|w| w == what)
        .map(|p| p + from)
}

fn code_span(chars: &[char], i: usize, out: &mut String) -> Option<usize> {
    let run = chars[i..].iter().take_while(|c| **c == '`').count();
    let mut j = i + run;
    while j < chars.len() {
        if chars[j] == '`' {
            let close = chars[j..].iter().take_while(|c| **c == '`').count();
            if close == run {
                let inner: String = chars[i + run..j].iter().collect();
                let inner = inner.replace('\n', " ");
                let trimmed = if inner.len() > 2 && inner.starts_with(' ') && inner.ends_with(' ') {
                    &inner[1..inner.len() - 1]
                } else {
                    inner.as_str()
                };
                match as_keys(trimmed) {
                    Some(keys) => out.push_str(&keys_html(&keys)),
                    None => {
                        out.push_str("<code>");
                        out.push_str(&escape(trimmed));
                        out.push_str("</code>");
                    }
                }
                return Some(j + close - i);
            }
            j += close;
        } else {
            j += 1;
        }
    }
    // No closing run: the backticks are words.
    out.push_str(&"`".repeat(run));
    Some(run)
}

/// The text in brackets from `open` (a `[`), allowing brackets inside,
/// and where the closing one is.
fn bracketed(chars: &[char], open: usize) -> Option<usize> {
    let mut level = 0;
    let mut j = open;
    while j < chars.len() {
        match chars[j] {
            '\\' => j += 1,
            '`' => {
                // Code inside link words keeps its brackets.
                let run = chars[j..].iter().take_while(|c| **c == '`').count();
                j += run;
                while j < chars.len() && chars[j] != '`' {
                    j += 1;
                }
                j += chars[j.min(chars.len())..]
                    .iter()
                    .take_while(|c| **c == '`')
                    .count();
                continue;
            }
            '[' => level += 1,
            ']' => {
                level -= 1;
                if level == 0 {
                    return Some(j);
                }
            }
            _ => {}
        }
        j += 1;
    }
    None
}

/// The address in parentheses at `open` (a `(`): the address without a
/// title, and where the closing parenthesis is.
fn destination(chars: &[char], open: usize) -> Option<(String, usize)> {
    if chars.get(open) != Some(&'(') {
        return None;
    }
    let mut level = 0;
    let mut j = open;
    while j < chars.len() {
        match chars[j] {
            '(' => level += 1,
            ')' => {
                level -= 1;
                if level == 0 {
                    let inner: String = chars[open + 1..j].iter().collect();
                    let url = inner.split_whitespace().next().unwrap_or("").to_string();
                    let url = url
                        .trim_start_matches('<')
                        .trim_end_matches('>')
                        .to_string();
                    return Some((url, j));
                }
            }
            '\n' => return None,
            _ => {}
        }
        j += 1;
    }
    None
}

fn link(
    chars: &[char],
    i: usize,
    places: &mut dyn Places,
    out: &mut String,
    depth: usize,
) -> Option<usize> {
    let close = bracketed(chars, i)?;
    let (url, end) = destination(chars, close + 1)?;
    let href = places.href(&url);
    let outside = href.starts_with("http://") || href.starts_with("https://");
    out.push_str(&format!("<a href=\"{}\"", escape(&href)));
    if outside {
        out.push_str(" target=\"_blank\" rel=\"noopener\"");
    }
    out.push('>');
    walk(&chars[i + 1..close], places, out, depth + 1);
    if outside {
        out.push_str("<span class=\"out\" aria-hidden=\"true\">↗</span>");
    }
    out.push_str("</a>");
    Some(end + 1 - i)
}

fn image(chars: &[char], i: usize, places: &mut dyn Places, out: &mut String) -> Option<usize> {
    let close = bracketed(chars, i + 1)?;
    let (url, end) = destination(chars, close + 1)?;
    let alt: String = chars[i + 2..close].iter().collect();
    let src = places.href(&url);
    out.push_str(&format!(
        "<img src=\"{}\" alt=\"{}\" loading=\"lazy\">",
        escape(&src),
        escape(&alt)
    ));
    Some(end + 1 - i)
}

/// The little HTML the manual may use: an anchor to link to (`<a
/// id="pill"></a>`), keys (`<kbd>`), a line break and web addresses in
/// angle brackets. Anything else is written out as words.
fn html(chars: &[char], i: usize, places: &mut dyn Places, out: &mut String) -> Option<usize> {
    let close = find(chars, i, &['>'])?;
    let tag: String = chars[i..=close].iter().collect();
    if let Some(id) = tag
        .strip_prefix("<a id=\"")
        .and_then(|rest| rest.strip_suffix("\">"))
        .filter(|id| !id.contains('"'))
    {
        let end = close + 1;
        let closing: Vec<char> = "</a>".chars().collect();
        let skip = if chars.get(end..end + 4) == Some(&closing[..]) {
            4
        } else {
            0
        };
        out.push_str(&format!("<a id=\"{}\"></a>", escape(&places.anchor(id))));
        return Some(close + 1 - i + skip);
    }
    if tag == "<kbd>" {
        let end = find(chars, close, &['<', '/', 'k', 'b', 'd', '>'])?;
        let inner: String = chars[close + 1..end].iter().collect();
        out.push_str(&format!("<kbd>{}</kbd>", escape(&inner)));
        return Some(end + 6 - i);
    }
    if tag == "<br>" || tag == "<br/>" || tag == "<br />" {
        out.push_str("<br>");
        return Some(close + 1 - i);
    }
    let inner = &tag[1..tag.len() - 1];
    if (inner.starts_with("https://") || inner.starts_with("http://")) && !inner.contains(' ') {
        out.push_str(&format!(
            "<a href=\"{0}\" target=\"_blank\" rel=\"noopener\">{0}</a>",
            escape(inner)
        ));
        return Some(close + 1 - i);
    }
    None
}

fn emphasis(
    chars: &[char],
    i: usize,
    places: &mut dyn Places,
    out: &mut String,
    depth: usize,
) -> Option<usize> {
    let mark = chars[i];
    let run = chars[i..].iter().take_while(|c| **c == mark).count().min(2);
    let after = *chars.get(i + run)?;
    if after.is_whitespace() {
        return None;
    }
    // An underscore inside a word is part of it (snake_case).
    if mark == '_' && i > 0 && chars[i - 1].is_alphanumeric() {
        return None;
    }
    let delimiter = vec![mark; run];
    let mut j = i + run;
    while let Some(at) = find(chars, j, &delimiter) {
        let before = chars[at - 1];
        let next = chars.get(at + run).copied();
        let word_after = mark == '_' && next.is_some_and(char::is_alphanumeric);
        let longer = run == 1 && next == Some(mark);
        if !before.is_whitespace() && !word_after && !longer && at > i + run {
            let tag = if run == 2 { "strong" } else { "em" };
            out.push_str(&format!("<{tag}>"));
            walk(&chars[i + run..at], places, out, depth + 1);
            out.push_str(&format!("</{tag}>"));
            return Some(at + run - i);
        }
        j = at + if longer { 2 } else { 1 };
    }
    None
}

#[cfg(test)]
#[path = "inline_tests.rs"]
mod tests;
