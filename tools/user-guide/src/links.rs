//! Where the manual's links go in the guide, and whether they go
//! anywhere at all: a link to a page or heading that isn't there is a
//! problem, and the guide isn't built.
//!
//! - `page.md` is that page's chapter, `page.md#heading` the heading in
//!   it, `#heading` one on the same page;
//! - a path out of the manual (`../decisions/0103-….md`) is the file on
//!   the repository's GitHub page, if the file is in the repository;
//! - a web address stays as it is;
//! - a picture in `pictures/` is written into the page as data, and must
//!   be there.

use crate::inline::Places;
use crate::pictures;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

/// What every page's links may reach.
pub struct Reach<'a> {
    /// Each page's anchors (its headings' slugs and the ids it names),
    /// by page.
    pub anchors: &'a BTreeMap<String, Vec<String>>,
    /// The repository's top folder, to look for files the manual links
    /// to outside itself.
    pub root: &'a Path,
    /// The repository on GitHub (`owner/name`).
    pub repo: &'a str,
    /// The manual's pictures, by file name.
    pub pictures: &'a BTreeMap<String, Vec<u8>>,
}

/// The links of one page.
pub struct PageLinks<'a> {
    reach: &'a Reach<'a>,
    page: &'a str,
    /// What is wrong with the links seen so far.
    pub problems: Vec<String>,
    /// The pictures the page shows, by file name.
    pub pictures: BTreeSet<String>,
}

impl<'a> PageLinks<'a> {
    /// The links of `page`.
    pub fn new(reach: &'a Reach<'a>, page: &'a str) -> Self {
        Self {
            reach,
            page,
            problems: Vec::new(),
            pictures: BTreeSet::new(),
        }
    }

    fn broken(&mut self, url: &str, why: &str) {
        self.problems
            .push(format!("{}.md links to `{url}`, but {why}", self.page));
    }

    /// The guide's id of the anchor `fragment` on `page`.
    fn place(&mut self, url: &str, page: &str, fragment: Option<&str>) -> String {
        let Some(anchors) = self.reach.anchors.get(page) else {
            self.broken(url, &format!("{page}.md isn't a page in SUMMARY.md"));
            return format!("#{page}");
        };
        let Some(fragment) = fragment.filter(|f| !f.is_empty()) else {
            return format!("#{page}");
        };
        if !anchors.iter().any(|a| a == fragment) {
            self.broken(
                url,
                &format!("{page}.md has no heading or anchor `{fragment}`"),
            );
        }
        // The first anchor is the page's title: the chapter itself.
        if anchors.first().is_some_and(|title| title == fragment) {
            return format!("#{page}");
        }
        format!("#{}", id(page, fragment))
    }

    /// A path out of the manual, on GitHub.
    fn outside(&mut self, url: &str, path: &str, fragment: Option<&str>) -> String {
        let Some(inside) = within_repository(path) else {
            self.broken(url, "it leads out of the repository");
            return String::new();
        };
        let full = self.reach.root.join(&inside);
        let kind = if full.is_dir() {
            "tree"
        } else if full.is_file() {
            "blob"
        } else {
            self.broken(url, "no such file is in the repository");
            "blob"
        };
        let slashed: Vec<String> = inside
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        let hash = fragment.map(|f| format!("#{f}")).unwrap_or_default();
        format!(
            "https://github.com/{}/{kind}/main/{}{hash}",
            self.reach.repo,
            slashed.join("/")
        )
    }
}

/// The guide's id for the anchor `fragment` of `page`.
pub fn id(page: &str, fragment: &str) -> String {
    format!("{page}--{fragment}")
}

/// A path written from `docs/manual`, as a path from the repository's
/// top, if it stays inside it.
fn within_repository(path: &str) -> Option<PathBuf> {
    let mut parts: Vec<String> = vec!["docs".into(), "manual".into()];
    for part in Path::new(path).components() {
        match part {
            Component::Normal(name) => parts.push(name.to_string_lossy().into_owned()),
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.iter().collect())
}

/// Whether `page` is a plain page name: no folders, nothing odd.
fn is_page_name(page: &str) -> bool {
    !page.is_empty()
        && page
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

impl Places for PageLinks<'_> {
    fn href(&mut self, url: &str) -> String {
        let lower = url.to_ascii_lowercase();
        if lower.starts_with("https://")
            || lower.starts_with("http://")
            || lower.starts_with("mailto:")
        {
            return url.to_string();
        }
        if url.is_empty() {
            self.broken(url, "the address is empty");
            return String::new();
        }
        let (path, fragment) = match url.split_once('#') {
            Some((path, fragment)) => (path, Some(fragment)),
            None => (url, None),
        };
        if path.is_empty() {
            let page = self.page;
            return self.place(url, page, fragment);
        }
        if lower.contains(':') {
            self.broken(
                url,
                "only web addresses and the repository's files can be linked",
            );
            return String::new();
        }
        // The contents page is the guide's welcome.
        if path == "SUMMARY.md" && fragment.is_none() {
            return "#welcome".to_string();
        }
        if let Some(page) = path.strip_suffix(".md").filter(|p| is_page_name(p)) {
            return self.place(url, page, fragment);
        }
        self.outside(url, path, fragment)
    }

    fn anchor(&mut self, id: &str) -> String {
        self::id(self.page, id)
    }

    fn picture(&mut self, url: &str) -> (String, Option<(u32, u32)>) {
        let Some(name) = url
            .strip_prefix(pictures::PREFIX)
            .filter(|n| !n.contains(['/', '\\']))
        else {
            return (self.href(url), None);
        };
        let Some(data) = self.reach.pictures.get(name) else {
            self.broken(
                url,
                &format!(
                    "there's no {name} in {}: take the pictures again with {}",
                    pictures::FOLDER,
                    pictures::RETAKE
                ),
            );
            return (String::new(), None);
        };
        self.pictures.insert(name.to_string());
        (pictures::data_address(data), pictures::size(data))
    }

    fn problem(&mut self, why: String) {
        self.problems.push(format!("{}.md: {why}", self.page));
    }
}

#[cfg(test)]
#[path = "links_tests.rs"]
mod tests;
