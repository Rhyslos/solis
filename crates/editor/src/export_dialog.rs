//! Export for playtesting ([decision 0093](../../../docs/decisions/0093-the-playtest-export.md)):
//! what File › Project settings › Build asks (the game's name, the area it
//! starts in, the window and the folder; `file_pages/build.rs` draws it),
//! and [`Playtest`], what an exported game reads to know them (and what a
//! project remembers for next time).
//!
//! Nothing here looks at the disk. A project's files come from other
//! people, and so may the folder remembered from last time if the saved
//! settings were changed: looking inside a folder on another computer
//! makes Windows sign in to it, so the folder is only ever text here, and
//! the app looks at it once Export is pressed (a test checks this file
//! for calls that would).

use crate::workbench::Workbench;
use std::path::{Component, Path, PathBuf};

/// The first line of a playtest file.
const HEADER: &str = "playtest 1";
/// The longest game name kept.
pub const MAX_NAME: usize = 64;
/// The biggest playtest file read.
pub const MAX_PLAYTEST_BYTES: u64 = 16 << 10;
/// The longest remembered folder kept.
pub const MAX_FOLDER: usize = 1024;

/// An exported game: its name, the area it starts in, and its window as
/// it first opens (players change it in the game's own menu, or with
/// Alt+Enter, and their choice is kept).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playtest {
    /// The game's name: the program's, its window's title and its saves
    /// folder's.
    pub name: String,
    /// The area it starts in, inside the assets, with `/` between folders.
    pub area: String,
    /// Whether it starts filling the screen.
    pub fullscreen: bool,
    /// Its window's size in pixels, when it isn't fullscreen.
    pub size: [u32; 2],
}

impl Default for Playtest {
    fn default() -> Self {
        Self {
            name: "Game".into(),
            area: String::new(),
            fullscreen: false,
            size: [1280, 720],
        }
    }
}

/// `name` made usable as a game's name and a file's: no characters
/// Windows refuses in file names, no spaces or dots at the ends, and at
/// most [`MAX_NAME`] characters; "Game" if nothing is left.
pub fn game_name(name: &str) -> String {
    let kept: String = name
        .chars()
        .filter(|c| !c.is_control() && !r#"<>:"/\|?*"#.contains(*c))
        .take(MAX_NAME)
        .collect();
    let kept = kept.trim_matches(|c: char| c.is_whitespace() || c == '.');
    if kept.is_empty() || device_name(kept) {
        "Game".into()
    } else {
        kept.to_owned()
    }
}

/// Whether Windows keeps `name` for a device (`CON`, `COM1`, `LPT3`,
/// `CONIN$`…), alone or before a dot (`con.txt`), so no file can have it.
fn device_name(name: &str) -> bool {
    let base = name.split('.').next().unwrap_or_default().trim_end();
    let upper = base.to_uppercase();
    if ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].contains(&upper.as_str()) {
        return true;
    }
    let mut chars = upper.chars();
    let prefix: String = chars.by_ref().take(3).collect();
    let rest: Vec<char> = chars.collect();
    (prefix == "COM" || prefix == "LPT")
        && rest.len() == 1
        && matches!(rest[0], '1'..='9' | '¹' | '²' | '³')
}

/// Whether `area` is an area's path inside the assets: relative, with `/`
/// between folders, never going up, ending `.solarea`.
pub fn usable_area(area: &str) -> bool {
    !area.is_empty()
        && area.len() < 1024
        && area.ends_with(".solarea")
        && !area.starts_with('/')
        && !area.contains('\\')
        && !area.contains(':')
        && !area.chars().any(char::is_control)
        && area
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

/// Whether `folder` may be remembered as where a game was last exported:
/// a whole path on one of this computer's own drives. Not a folder on
/// another computer (`\\server\share`, however its slashes lean) or a
/// device, not a path that goes up (`..`), and nothing too long or with
/// control characters in. Decided from the text alone.
pub fn local_folder(folder: &Path) -> bool {
    let Some(text) = folder.to_str() else {
        return false;
    };
    let slash = |c: Option<char>| matches!(c, Some('/' | '\\'));
    let mut start = text.chars();
    let elsewhere = slash(start.next()) && slash(start.next());
    !elsewhere
        && text.len() <= MAX_FOLDER
        && !text.chars().any(char::is_control)
        && folder.is_absolute()
        && !folder.components().any(|c| c == Component::ParentDir)
}

impl Playtest {
    /// As written in a playtest file.
    pub fn text(&self) -> String {
        format!(
            "{HEADER}\nname {}\narea {}\nwindow {} {} {}\n",
            game_name(&self.name),
            self.area,
            if self.fullscreen {
                "fullscreen"
            } else {
                "windowed"
            },
            self.size[0],
            self.size[1],
        )
    }

    /// Reads a playtest file. Anything odd is put right (a name made
    /// usable, a window size kept within what Solis opens); only a file
    /// that isn't one, or names no usable area, is refused. Nothing in it
    /// is a folder on this computer: a `folder` line, which a project's
    /// own copy held before, is passed over like any line not known.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        if lines.next().map(str::trim) != Some(HEADER) {
            return Err("it isn't a playtest file".into());
        }
        let mut found = Playtest::default();
        for line in lines {
            let (key, value) = line.split_once(' ').unwrap_or((line, ""));
            match key {
                "name" => found.name = game_name(value),
                "area" => found.area = value.trim().to_owned(),
                "window" => {
                    let mut words = value.split_whitespace();
                    found.fullscreen = words.next() == Some("fullscreen");
                    let mut side = || words.next().and_then(|w| w.parse::<u32>().ok());
                    if let (Some(w), Some(h)) = (side(), side()) {
                        found.size = script::settings::PlayerSettings::default()
                            .with(script::settings::Setting::Resolution(w, h))
                            .resolution;
                    }
                }
                _ => {}
            }
        }
        if !usable_area(&found.area) {
            return Err("it doesn't name an area to start in".into());
        }
        Ok(found)
    }
}

/// The folder an export goes in by default: `<Game> exports/<Game>
/// playtest <date>` in `documents`.
pub fn default_folder(documents: &Path, name: &str, date: &str) -> PathBuf {
    let name = game_name(name);
    documents
        .join(format!("{name} exports"))
        .join(format!("{name} playtest {date}"))
}

/// The date and time now, as logs write it: `2026-11-06 19:30:05`.
pub fn now_for_logs() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// What the export window asks the app to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportRequest {
    /// The game as it's exported.
    pub playtest: Playtest,
    /// The folder it goes in (made; it must be empty if it's there).
    pub folder: PathBuf,
    /// Whether the window filled the folder in itself (its dated
    /// suggestion, or where the last export went) and it wasn't changed.
    /// Such a folder with files in already (an export earlier the same
    /// day) isn't refused: the export goes in the first of `folder (2)`,
    /// `folder (3)`… without any. The app finds that out, on the
    /// export's own thread, since the window never looks at the disk.
    pub suggested: bool,
}

/// Export for playtesting as File › Project settings › Build shows it,
/// from the first time it is shown for the open project.
#[derive(Debug, Clone, Default)]
pub(crate) struct ExportForm {
    pub(crate) playtest: Playtest,
    pub(crate) folder: String,
    /// The folder as the page filled it in itself: while `folder` still
    /// says this, the export may go beside it ([`ExportRequest::suggested`]).
    pub(crate) suggested: String,
    /// What happened to the last export: where it went, or why it didn't.
    pub(crate) done: Option<Result<PathBuf, String>>,
    /// Exporting now.
    pub(crate) busy: bool,
}

/// The window sizes offered, as the game's Resolution steps through them.
pub(crate) fn sizes() -> &'static [[u32; 2]] {
    &script::settings::RESOLUTIONS
}

impl Workbench {
    /// Remembers how the project was last exported (the app reads it with
    /// the project), for the window to start from.
    pub fn set_last_export(&mut self, last: Option<Playtest>) {
        self.open.last_export = last;
    }

    /// Remembers where the project was last exported on this computer,
    /// for the window to suggest a folder beside it. The app keeps it in
    /// its own saved settings, never in the project. Anything but a whole
    /// path on this computer's own drives ([`local_folder`]) is dropped.
    pub fn set_last_export_folder(&mut self, folder: Option<PathBuf>) {
        self.open.last_export_folder = folder.filter(|f| local_folder(f));
    }

    /// The Documents folder, for exports to go in by default.
    pub fn set_documents_folder(&mut self, folder: Option<PathBuf>) {
        self.documents = folder;
    }

    /// Opens Export for playtesting: the Build tab's page (also File ›
    /// Project settings › Build), filled in afresh.
    pub fn open_export(&mut self) {
        self.prepare_export();
        self.show_tab(crate::shell::RibbonTab::Build);
    }

    /// Fills in Export for playtesting: the game named after the project,
    /// starting in the open area, unless it was exported before.
    pub(crate) fn prepare_export(&mut self) {
        let Some(project) = &self.open.project else {
            self.log(
                crate::LogLevel::Warning,
                "Export for playtesting needs a project: open or make one first.",
            );
            return;
        };
        let assets = project.assets.clone();
        let open_area = self
            .open
            .document
            .file()
            .and_then(|f| f.strip_prefix(&assets).ok())
            .and_then(|f| f.to_str())
            .map(|f| f.replace('\\', "/"))
            .filter(|f| usable_area(f));
        let mut playtest = self.open.last_export.clone().unwrap_or_else(|| Playtest {
            name: game_name(&project.name),
            ..Playtest::default()
        });
        if !usable_area(&playtest.area) || !self.areas().contains(&playtest.area) {
            playtest.area = open_area
                .or_else(|| self.areas().into_iter().next())
                .unwrap_or_default();
        }
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        // Only named here, never looked at: whether it's there, or has
        // files in, is the app's to find out once Export is pressed.
        let folder = match (&self.open.last_export_folder, &self.documents) {
            // A new dated folder beside the last, so old builds stay.
            (Some(last), _) if last.parent().is_some() => last
                .parent()
                .unwrap_or(last)
                .join(format!("{} playtest {date}", game_name(&playtest.name))),
            (_, Some(documents)) => default_folder(documents, &playtest.name, &date),
            _ => project
                .root
                .parent()
                .map(|p| default_folder(p, &playtest.name, &date))
                .unwrap_or_default(),
        };
        let folder = folder.display().to_string();
        self.open.export = Some(ExportForm {
            playtest,
            suggested: folder.clone(),
            folder,
            done: None,
            busy: false,
        });
    }

    /// The project's areas, inside the assets.
    pub(crate) fn areas(&self) -> Vec<String> {
        let mut areas: Vec<String> = self
            .open
            .library
            .iter()
            .filter(|i| i.kind == crate::library::LibraryKind::Area && usable_area(&i.path))
            .map(|i| i.path.clone())
            .collect();
        areas.sort();
        areas
    }

    /// The folder chosen with the Build page's "…" button.
    pub fn set_export_folder(&mut self, folder: PathBuf) {
        if let Some(window) = &mut self.open.export {
            window.folder = folder.display().to_string();
        }
    }

    /// The app finished exporting: where it went, or why it didn't.
    pub fn export_finished(&mut self, outcome: Result<PathBuf, String>) {
        match &outcome {
            Ok(folder) => self.log(
                crate::LogLevel::Info,
                format!(
                    "Exported for playtesting to {}. Zip that folder and send it: the game runs \
                     from the program inside it.",
                    folder.display()
                ),
            ),
            Err(why) => self.log(
                crate::LogLevel::Error,
                format!("The export didn't finish: {why}."),
            ),
        }
        if let Some(window) = &mut self.open.export {
            window.busy = false;
            // Exporting again goes in a folder of its own, beside this
            // one.
            if let Ok(folder) = &outcome {
                window.folder = folder.display().to_string();
                window.suggested = window.folder.clone();
            }
            window.done = Some(outcome);
        }
    }
}

#[cfg(test)]
#[path = "export_dialog_tests.rs"]
mod tests;
