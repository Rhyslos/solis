//! The scoring: how well what is typed matches a name, and which
//! results are shown, best first.

use super::{SearchItem, SearchKind};

/// The most results shown at once.
pub(crate) const MAX_SHOWN: usize = 50;

/// How well `query` matches `text`, or none: each word of the query must
/// be in the text. A word at the start of one of the text's words counts
/// most, anywhere in it less, and its letters in order (`svar` for "Save
/// area") least.
pub(crate) fn score(query: &str, text: &str) -> Option<u32> {
    let text = text.to_lowercase();
    let starts: Vec<usize> = std::iter::once(0)
        .chain(
            text.char_indices()
                .filter(|(_, c)| !c.is_alphanumeric())
                .map(|(i, c)| i + c.len_utf8()),
        )
        .collect();
    let mut total = 0;
    for word in query.to_lowercase().split_whitespace() {
        total += if starts.iter().any(|&s| text[s..].starts_with(word)) {
            3
        } else if text.contains(word) {
            2
        } else if in_order(word, &text) {
            1
        } else {
            return None;
        };
    }
    Some(total)
}

/// Whether `word`'s letters appear in `text` in order.
fn in_order(word: &str, text: &str) -> bool {
    let mut letters = text.chars();
    word.chars().all(|w| letters.any(|t| t == w))
}

/// The results for `query` among `items`: the best first, then actions
/// before panels, entities and files, then the shortest name; at most
/// [`MAX_SHOWN`]. With nothing typed, every action, in menu order.
pub(crate) fn results<'a>(items: &'a [SearchItem], query: &str) -> Vec<&'a SearchItem> {
    if query.trim().is_empty() {
        return items
            .iter()
            .filter(|i| i.kind == SearchKind::Action)
            .take(MAX_SHOWN)
            .collect();
    }
    let mut found: Vec<(u32, &SearchItem)> = items
        .iter()
        .filter_map(|i| {
            let by_name = score(query, &i.name);
            let by_detail = score(query, &format!("{} {}", i.name, i.detail)).map(|s| s / 2);
            by_name.max(by_detail).map(|s| (s, i))
        })
        .collect();
    found.sort_by(|(a, x), (b, y)| {
        b.cmp(a)
            .then(x.kind.cmp(&y.kind))
            .then(x.name.len().cmp(&y.name.len()))
    });
    found.into_iter().take(MAX_SHOWN).map(|(_, i)| i).collect()
}
