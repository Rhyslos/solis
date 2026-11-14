//! The manual's contents page, `docs/manual/SUMMARY.md`: the guide's
//! chapters, in order, in their groups.
//!
//! Its shape: a title, words for the guide's welcome, then a `##` heading
//! for each group with a list of links to the group's pages. Each page is
//! listed once; the check (`tests/manual.rs`) fails when a page of the
//! manual is missing from it.

use crate::blocks::{self, Block};

/// The contents: the welcome's words and the groups of chapters.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Contents {
    /// The blocks between the title and the first group: the guide's
    /// welcome.
    pub welcome: Vec<Block>,
    /// The groups, in order.
    pub groups: Vec<Group>,
}

/// A group of chapters, under its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// The group's name, from its heading.
    pub name: String,
    /// The pages, by file name without `.md`, in order.
    pub pages: Vec<String>,
}

impl Contents {
    /// Every page, in the guide's order.
    pub fn pages(&self) -> impl Iterator<Item = &str> {
        self.groups
            .iter()
            .flat_map(|g| g.pages.iter().map(String::as_str))
    }
}

/// Reads the contents page. A problem is said in plain words.
pub fn read(text: &str) -> Result<Contents, Vec<String>> {
    let mut contents = Contents::default();
    let mut problems = Vec::new();
    for block in blocks::parse(text) {
        match block {
            Block::Heading(1, _) if contents.groups.is_empty() && contents.welcome.is_empty() => {}
            Block::Heading(2, name) => contents.groups.push(Group {
                name,
                pages: Vec::new(),
            }),
            Block::List(list) if !contents.groups.is_empty() => {
                for item in &list.items {
                    match item.first().and_then(page_of) {
                        Some(page) => {
                            if let Some(group) = contents.groups.last_mut() {
                                group.pages.push(page);
                            }
                        }
                        None => problems.push(
                            "SUMMARY.md: each item under a group must start with a link to a page \
                             of the manual, such as [Physics](physics.md)"
                                .to_string(),
                        ),
                    }
                }
            }
            block if contents.groups.is_empty() => contents.welcome.push(block),
            _ => problems.push(
                "SUMMARY.md: under a group heading, only a list of pages may follow".to_string(),
            ),
        }
    }
    let mut seen = Vec::new();
    for page in contents.pages() {
        if seen.contains(&page) {
            problems.push(format!("SUMMARY.md lists {page}.md twice"));
        }
        seen.push(page);
    }
    if seen.is_empty() {
        problems.push("SUMMARY.md lists no pages".to_string());
    }
    if problems.is_empty() {
        Ok(contents)
    } else {
        Err(problems)
    }
}

/// The page an item's first link names: `[Words](page.md)` gives `page`.
fn page_of(block: &Block) -> Option<String> {
    let Block::Paragraph(words) = block else {
        return None;
    };
    let rest = words.trim_start().strip_prefix('[')?;
    let target = rest.split_once("](")?.1.split_once(')')?.0;
    let page = target.strip_suffix(".md")?;
    let plain = !page.is_empty()
        && page
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    plain.then(|| page.to_string())
}

#[cfg(test)]
#[path = "contents_tests.rs"]
mod tests;
