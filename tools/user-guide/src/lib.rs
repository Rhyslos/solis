//! Builds the user guide from the user manual (decision 0112): one HTML
//! file, with its style, script, fonts and logo inside it, that opens
//! from the disk without a network.
//!
//! The manual (`docs/manual/*.md`) is the one source. Its contents page,
//! `SUMMARY.md`, gives the chapters' order and groups; the program's name
//! and repository come from `crates/app/src/branding.rs`, read as text;
//! the logo is `branding/logo.svg`; the fonts are the editor's own
//! (`crates/editor/assets/fonts`).
//!
//! [`read`] gathers all of that from the repository, [`build`] turns it
//! into the guide or says what stops it: a page missing from the
//! contents, a link to nothing. The tests in `tests/manual.rs` fail when
//! the copy kept in `docs/user-guide/index.html` isn't what [`build`]
//! makes now.

pub mod blocks;
pub mod contents;
mod guide;
pub mod html;
pub mod inline;
pub mod links;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Where the copy kept in the repository is, from its top folder.
pub const KEPT_COPY: &str = "docs/user-guide/index.html";

/// The pages of the manual folder that aren't chapters: the contents
/// page itself and the template new pages start from.
const NOT_CHAPTERS: [&str; 2] = ["SUMMARY", "_template"];

/// A page bigger than this isn't read: no page comes near it.
const LARGEST_PAGE: u64 = 1 << 20;
/// More pages than this aren't read.
const MOST_PAGES: usize = 1000;

/// The editor's fonts the guide draws with: Inter for words (regular and
/// semibold), JetBrains Mono for code and keys. Their licences are
/// already in `licenses/fonts/`.
const FONTS: [(&str, u16, &str); 3] = [
    ("Inter", 400, "Inter-Regular.ttf"),
    ("Inter", 600, "Inter-SemiBold.ttf"),
    ("JetBrains Mono", 400, "JetBrainsMonoNL-Regular.ttf"),
];

/// A font written into the guide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Font {
    /// Its family's name in the style.
    pub family: &'static str,
    /// Its weight.
    pub weight: u16,
    /// The font file.
    pub data: Vec<u8>,
}

/// Everything the guide is made from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manual {
    /// `SUMMARY.md`.
    pub contents: String,
    /// Every other page of the manual, by file name without `.md`.
    pub pages: BTreeMap<String, String>,
    /// The program's name (`APP_NAME`).
    pub name: String,
    /// Its repository on GitHub (`UPDATE_REPO`), for links out of the
    /// manual.
    pub repo: String,
    /// The logo, as SVG.
    pub logo: Vec<u8>,
    /// The fonts.
    pub fonts: Vec<Font>,
    /// The repository's top folder, where linked files are looked for.
    pub root: PathBuf,
}

/// Reads the manual, the name, the logo and the fonts from the
/// repository at `root`.
pub fn read(root: &Path) -> Result<Manual, String> {
    let manual = root.join("docs/manual");
    let contents = read_text(&manual.join("SUMMARY.md"))?;
    let mut pages = BTreeMap::new();
    let listing = std::fs::read_dir(&manual)
        .map_err(|e| format!("Couldn't read {}: {e}", manual.display()))?;
    for entry in listing {
        let entry = entry.map_err(|e| format!("Couldn't read {}: {e}", manual.display()))?;
        let path = entry.path();
        let Some(page) = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".md"))
        else {
            continue;
        };
        if NOT_CHAPTERS.contains(&page) {
            continue;
        }
        if pages.len() >= MOST_PAGES {
            return Err(format!("The manual has more than {MOST_PAGES} pages"));
        }
        pages.insert(page.to_string(), read_text(&path)?);
    }
    let branding = read_text(&root.join("crates/app/src/branding.rs"))?;
    let fonts_folder = root.join("crates/editor/assets/fonts");
    let fonts = FONTS
        .iter()
        .map(|(family, weight, file)| {
            let path = fonts_folder.join(file);
            std::fs::read(&path)
                .map(|data| Font {
                    family,
                    weight: *weight,
                    data,
                })
                .map_err(|e| format!("Couldn't read {}: {e}", path.display()))
        })
        .collect::<Result<_, _>>()?;
    let logo_path = root.join("branding/logo.svg");
    Ok(Manual {
        contents,
        pages,
        name: constant(&branding, "APP_NAME")?,
        repo: constant(&branding, "UPDATE_REPO")?,
        logo: std::fs::read(&logo_path)
            .map_err(|e| format!("Couldn't read {}: {e}", logo_path.display()))?,
        fonts,
        root: root.to_path_buf(),
    })
}

/// A text file, refused when it is larger than any page should be.
fn read_text(path: &Path) -> Result<String, String> {
    let size = std::fs::metadata(path)
        .map_err(|e| format!("Couldn't read {}: {e}", path.display()))?
        .len();
    if size > LARGEST_PAGE {
        return Err(format!(
            "{} is {size} bytes: more than a page of the manual should be",
            path.display()
        ));
    }
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
    Ok(text.replace("\r\n", "\n"))
}

/// A `pub const NAME: &str = "…";` of `branding.rs`, read as text so the
/// guide needs nothing built. The Windows build reads it the same way.
pub fn constant(source: &str, name: &str) -> Result<String, String> {
    let start = format!("pub const {name}: &str = \"");
    source
        .lines()
        .find_map(|line| line.trim().strip_prefix(&start)?.strip_suffix("\";"))
        .filter(|value| !value.is_empty() && !value.contains(['"', '\\', '<', '>']))
        .map(str::to_string)
        .ok_or_else(|| format!("branding.rs has no `{start}…\";` line"))
}

/// The pages of the manual missing from its contents page.
pub fn unlisted(manual: &Manual) -> Result<Vec<String>, Vec<String>> {
    let contents = contents::read(&manual.contents)?;
    let listed: Vec<&str> = contents.pages().collect();
    Ok(manual
        .pages
        .keys()
        .filter(|page| !listed.contains(&page.as_str()))
        .cloned()
        .collect())
}

/// The guide, or every problem that stops it being built.
pub fn build(manual: &Manual) -> Result<String, Vec<String>> {
    let contents = contents::read(&manual.contents)?;
    let mut problems: Vec<String> = unlisted(manual)?
        .into_iter()
        .map(|page| format!("{page}.md isn't in SUMMARY.md: add it to a group there"))
        .collect();
    match guide::write(manual, &contents) {
        Ok(html) if problems.is_empty() => Ok(html),
        Ok(_) => Err(problems),
        Err(found) => {
            problems.extend(found);
            Err(problems)
        }
    }
}

/// `template` with each `{{key}}` replaced by its value, in one pass, so
/// a value is never itself searched.
fn fill_in(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(at) = rest.find("{{") {
        out.push_str(&rest[..at]);
        let after = &rest[at + 2..];
        let value = after.find("}}").and_then(|end| {
            let key = &after[..end];
            values
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v, end + 2))
        });
        match value {
            Some((value, used)) => {
                out.push_str(value);
                rest = &after[used..];
            }
            None => {
                out.push_str("{{");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Bytes as base64, for the logo and fonts written into the page.
fn base64(bytes: &[u8]) -> String {
    const LETTERS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(char::from(LETTERS[((n >> shift) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
