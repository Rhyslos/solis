//! The project's own fonts on the game's screens (stage 1, text layout):
//! every `font "fonts/Lora.ttf"` the screens name, in their code or as
//! they run (several between commas: the first, then its fallbacks), read
//! into one [`ui::FontFiles`] the screens lay their text out with. The app
//! reads each font file asked for ([`Workbench::take_wanted_fonts`]) on a
//! worker and hands it back ([`Workbench::set_font_file`]); a name written
//! in a screen's code that names no font in the project, or a file that
//! isn't a font Solis can read, is in Problems with its line, before the
//! game runs.

use crate::console::LogLevel;
use crate::workbench::Workbench;
use script::CodeProblem;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::Arc;

/// The most font names seen in running screens that are kept.
pub(crate) const MAX_SEEN: usize = 256;

/// The screens' fonts, and what's wrong with those named.
#[derive(Debug, Default)]
pub(crate) struct ScreenFonts {
    /// The fonts read, for every screen to lay text out with.
    pub(crate) files: Option<Arc<ui::FontFiles>>,
    /// Fonts named in screens' code that can't be used, where.
    pub(crate) problems: Vec<CodeProblem>,
    /// Names the running screens' views gave (perhaps worked out as the
    /// game runs).
    pub(crate) seen: BTreeSet<String>,
    /// Each font file the app has read, by its place in the assets.
    read: BTreeMap<String, Result<Arc<Vec<u8>>, String>>,
    /// What the fonts were made from: the code and the Library (a
    /// fingerprint), how many names had been seen, and how many files had
    /// been read.
    from: Option<(u64, usize, u64)>,
    /// Counts the files the app has handed over.
    reads: u64,
    /// The font files last asked for, and whether they changed since the
    /// app last took them.
    wanted: BTreeSet<String>,
    wanted_changed: bool,
    /// Names worked out as the game ran that couldn't be used, said once.
    told: HashSet<String>,
}

/// Whether `name` ends as a font file does.
fn is_font(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".ttf") || lower.ends_with(".otf")
}

/// Every font name in `view`'s styles and what's inside it, into `out`
/// (at most [`MAX_SEEN`] in all).
pub(crate) fn names_in(view: &ui::view::Node, out: &mut BTreeSet<String>) {
    fn walk(node: &ui::view::Node, depth: usize, out: &mut BTreeSet<String>) {
        if out.len() >= MAX_SEEN || depth >= ui::screen::MAX_DEPTH {
            return;
        }
        for (key, value) in &node.styles {
            if let ("font", ui::view::Style::Text(list)) = (key.as_str(), value) {
                for name in list.split(',').map(str::trim).filter(|n| !n.is_empty()) {
                    if out.len() < MAX_SEEN {
                        out.insert(name.chars().take(ui::pictures::MAX_NAME).collect());
                    }
                }
            }
        }
        for child in &node.children {
            walk(child, depth + 1, out);
        }
    }
    walk(view, 0, out);
}

impl ScreenFonts {
    /// Takes in the font file at `path` as the app read it.
    pub(crate) fn set_read(&mut self, path: String, read: Result<Vec<u8>, String>) {
        self.read.insert(path, read.map(Arc::new));
        self.reads += 1;
    }

    /// Makes the fonts again if anything they come from changed: the
    /// names in `uses` (written in screens' code) and those seen running,
    /// and the files read. `in_project` says whether a file is among the
    /// assets; `fingerprint` changes with the code and the Library. Gives
    /// the names worked out as the game ran that can't be used, with why,
    /// each once.
    pub(crate) fn refresh(
        &mut self,
        uses: &[script::screens::FileUse],
        in_project: &dyn Fn(&str) -> bool,
        fingerprint: u64,
    ) -> Vec<String> {
        let now = (fingerprint, self.seen.len(), self.reads);
        if self.from == Some(now) {
            return Vec::new();
        }
        self.from = Some(now);
        let mut names: Vec<String> = Vec::new();
        for name in uses.iter().map(|u| &u.name).chain(&self.seen) {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
        let wanted: BTreeSet<String> = names
            .iter()
            .filter(|n| is_font(n) && in_project(n))
            .cloned()
            .collect();
        if wanted != self.wanted {
            self.wanted = wanted;
            self.wanted_changed = true;
        }
        self.read.retain(|path, _| self.wanted.contains(path));
        let mut files = ui::FontFiles::new();
        let mut why_not = std::collections::HashMap::new();
        for name in &names {
            if let Err(why) = self.add(&mut files, name, in_project) {
                why_not.insert(name.clone(), why);
            }
        }
        self.problems = uses
            .iter()
            .filter_map(|u| {
                why_not.get(&u.name).map(|why| CodeProblem {
                    file: u.file.clone(),
                    line: u.line,
                    column: u.column,
                    message: why.clone(),
                })
            })
            .collect();
        let written: HashSet<&str> = uses.iter().map(|u| u.name.as_str()).collect();
        let mut said = Vec::new();
        for name in &self.seen {
            if let Some(why) = why_not.get(name)
                && !written.contains(name.as_str())
                && self.told.insert(name.clone())
            {
                said.push(format!("A screen uses the font \"{name}\", but {why}."));
            }
        }
        self.files = (!files.is_empty()).then(|| Arc::new(files));
        said
    }

    /// Adds the font `name` to `files`, or says why it can't be. One still
    /// being read is left out for now, with no complaint.
    fn add(
        &self,
        files: &mut ui::FontFiles,
        name: &str,
        in_project: &dyn Fn(&str) -> bool,
    ) -> Result<(), String> {
        if !is_font(name) {
            return Err(format!(
                "{name} isn't a font: font names a TrueType (.ttf) or OpenType (.otf) file in \
                 the project, like font \"fonts/Lora.ttf\""
            ));
        }
        if !in_project(name) {
            return Err(format!(
                "there's no font {name} in the project: font names a font file by its place in \
                 the Library, like font \"fonts/Lora.ttf\""
            ));
        }
        match self.read.get(name) {
            None => Ok(()),
            Some(Err(why)) => Err(format!("{name} couldn't be read: {why}")),
            Some(Ok(bytes)) => files
                .add(name, bytes.to_vec())
                .map_err(|why| format!("{name} can't be used: {why}")),
        }
    }
}

impl Workbench {
    /// Makes the screens' fonts again if what they come from changed, and
    /// puts them on the screens shown.
    pub(crate) fn refresh_screen_fonts(&mut self) {
        let uses = self
            .open
            .program
            .as_ref()
            .and_then(|p| p.screens.as_ref())
            .map_or(&[][..], |s| s.fonts.as_slice());
        let fonts = &mut self.open.screen_fonts;
        if uses.is_empty() && fonts.seen.is_empty() {
            if fonts.files.is_some() || !fonts.problems.is_empty() || !fonts.wanted.is_empty() {
                self.open.screen_fonts = ScreenFonts {
                    wanted_changed: true,
                    ..Default::default()
                };
            }
            return;
        }
        let items = &self.open.library;
        let library: std::cell::OnceCell<HashSet<&str>> = std::cell::OnceCell::new();
        let in_project = |f: &str| {
            library
                .get_or_init(|| items.iter().map(|i| i.path.as_str()).collect())
                .contains(f)
        };
        let said = self
            .open
            .screen_fonts
            .refresh(uses, &in_project, self.sources);
        for line in said {
            self.log(LogLevel::Warning, line);
        }
        let files = self.open.screen_fonts.files.clone();
        for screen in &mut self.open.game_screens.pictures {
            if screen.fonts != files {
                Arc::make_mut(screen).fonts = files.clone();
            }
        }
    }

    /// The font files the game's screens use (paths inside the assets),
    /// when they changed since last taken: the app reads them.
    pub fn take_wanted_fonts(&mut self) -> Option<BTreeSet<String>> {
        let f = &mut self.open.screen_fonts;
        std::mem::take(&mut f.wanted_changed).then(|| f.wanted.clone())
    }

    /// Takes in the font file at `path` (inside the assets) as the app
    /// read it: its bytes, or why it couldn't be read.
    pub fn set_font_file(&mut self, path: String, read: Result<Vec<u8>, String>) {
        self.open.screen_fonts.set_read(path, read);
    }

    /// Fonts named in the screens' code that can't be used, where.
    pub fn font_problems(&self) -> &[CodeProblem] {
        &self.open.screen_fonts.problems
    }
}

#[cfg(test)]
#[path = "screen_fonts_tests.rs"]
mod tests;
