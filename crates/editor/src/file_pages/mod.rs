//! The File tab's own pages (specification, Screen 7, and the approved
//! additions, round 2): Recent projects, Learn, Updates and About.
//!
//! Solis starts on Recent projects, in the editor's own window, every
//! time ([decision 0105](../../../../docs/decisions/0105-solis-starts-on-the-file-page.md),
//! which keeps decision 0090's two promises): the page is the first thing
//! shown, and no project is reopened by itself. Until a project is open
//! the other ribbon tabs are dimmed, and "Carry on without a project"
//! stands where Back to editor does.
//!
//! The app keeps the list of projects and reads each project's file; the
//! pages show it and ask for changes, never reading a file themselves.
//!
//! - `recent.rs`: the projects as cards, the header's search, order and
//!   Open…, and the template strip.
//! - `project_card.rs`: one project's card and its right-click menu.
//! - `learn.rs`, `updates_page.rs`, `about.rs`: the other pages.
//! - Preferences and Project settings (Screens 8, 12 and 13): `settings.rs`
//!   is the table of their sub-tabs and settings, `settings_page.rs` the
//!   page they share, `preferences.rs`, `project_settings.rs` and
//!   `build.rs` the controls, `shortcuts.rs` and `input_page.rs` the two
//!   sub-tabs that are tables.

mod about;
mod build;
mod input_card;
mod input_page;
mod input_table;
mod learn;
mod page_parts;
mod preferences;
mod project_card;
mod project_settings;
mod recent;
mod settings;
mod settings_page;
mod shortcuts;
mod updates_page;

pub(crate) use input_page::InputPage;
#[cfg(test)]
pub(crate) use input_page::Slot;
use page_parts::{button_row, page_head, plain, section, sentence, with_side};
#[cfg(test)]
pub(crate) use project_settings::fits;
pub(crate) use settings::Tab;

use crate::icons::Icon;
use crate::tokens::{self, size};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Color32, Ui};
use std::path::PathBuf;

/// One project in the list, as the app reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownProject {
    /// Its folder.
    pub root: PathBuf,
    /// Its name (from its project file, or as last read).
    pub name: String,
    /// What's wrong, when it can't be opened as it is: its folder is gone,
    /// or its project file can't be read.
    pub problem: Option<String>,
    /// Kept at the top.
    pub pinned: bool,
    /// When it was last opened (seconds since 1970), if known.
    pub last_opened: Option<u64>,
    /// The build of Solis that last opened it, if recorded.
    pub saved_by: Option<u64>,
}

/// The File tab's pages that show something of their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Page {
    /// The projects.
    #[default]
    Projects,
    /// The examples and how to begin.
    Learn,
    /// This build, the newest, and updating.
    Updates,
    /// The program's name, version and licences.
    About,
    /// The editor's own settings.
    Preferences,
    /// The open project's settings, its input and its export.
    ProjectSettings,
}

/// How the projects are ordered (pinned ones always first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Sort {
    /// The most recently opened first.
    #[default]
    LastOpened,
    /// A to Z.
    Name,
}

impl Sort {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::LastOpened => "Last opened",
            Self::Name => "Name",
        }
    }
}

/// How the File pages are set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FilePages {
    /// The newer build whose notice was put away with Not now.
    pub(crate) update_hidden: Option<u64>,
    /// The page shown.
    pub(crate) page: Page,
    /// What's typed in the projects' search.
    pub(crate) search: String,
    /// The order.
    pub(crate) sort: Sort,
    /// The card picked (Enter opens it).
    pub(crate) selected: Option<PathBuf>,
    /// The search takes the keys when the page comes up.
    pub(crate) focus_search: bool,
    /// A line from the app about what it did or couldn't do (renamed,
    /// copying, found), shown under the heading until closed.
    pub(crate) notice: Option<String>,
    /// The project being renamed, the name typed, and why it can't be.
    pub(crate) renaming: Option<Renaming>,
    /// Preferences' sub-tab in front.
    pub(crate) prefs_tab: Tab,
    /// Project settings' sub-tab in front.
    pub(crate) project_tab: Tab,
    /// What's typed in Preferences' or Project settings' search.
    pub(crate) settings_search: String,
    /// The project's name as typed on Project settings, until the field
    /// is left.
    pub(crate) name_typed: Option<String>,
    /// What's typed in the Shortcuts table's search.
    pub(crate) shortcuts_search: String,
    /// Which of the shortcuts are shown.
    pub(crate) shortcuts_device: shortcuts::Device,
}

/// The Rename… question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Renaming {
    /// The project's folder.
    pub(crate) root: PathBuf,
    /// The name typed.
    pub(crate) name: String,
    /// Its focus is given once.
    pub(crate) focused: bool,
}

impl Default for FilePages {
    fn default() -> Self {
        Self {
            update_hidden: None,
            page: Page::Projects,
            search: String::new(),
            sort: Sort::LastOpened,
            selected: None,
            focus_search: true,
            notice: None,
            renaming: None,
            prefs_tab: Tab::General,
            project_tab: Tab::Project,
            settings_search: String::new(),
            name_typed: None,
            shortcuts_search: String::new(),
            shortcuts_device: shortcuts::Device::All,
        }
    }
}

/// An example project, made in a folder the user picks.
pub(crate) struct Example {
    pub(crate) name: &'static str,
    pub(crate) about: &'static str,
    pub(crate) icon: Icon,
    pub(crate) make: WorkbenchAction,
}

/// Solis's four examples: on Learn, and in the template strip.
pub(crate) fn examples() -> [Example; 4] {
    [
        Example {
            name: "Playground",
            about: "Walk a character around a 3D level: steps, ramps, crates and a moving platform",
            icon: Icon::Gamepad,
            make: WorkbenchAction::MakePlayground,
        },
        Example {
            name: "Turret sample",
            about: "Game code in Lust, Lua and Rust, talking through events",
            icon: Icon::Code,
            make: WorkbenchAction::MakeSampleGame,
        },
        Example {
            name: "UI sample",
            about: "A HUD and a pause menu with the player's settings, drawn on the canvas",
            icon: Icon::Screen,
            make: WorkbenchAction::MakeUiSample,
        },
        Example {
            name: "Benchmark",
            about: "Fleets of ships firing lasers: how much this computer can run",
            icon: Icon::Profiler,
            make: WorkbenchAction::MakeBenchmark,
        },
    ]
}

/// The projects shown, as places in `projects`: the pinned ones, then the
/// rest, each in `sort`'s order, keeping those whose name or folder holds
/// every word of `search` (any case).
pub(crate) fn shown(
    projects: &[KnownProject],
    search: &str,
    sort: Sort,
) -> (Vec<usize>, Vec<usize>) {
    let words: Vec<String> = search.split_whitespace().map(str::to_lowercase).collect();
    let mut kept: Vec<usize> = (0..projects.len())
        .filter(|&i| {
            let p = &projects[i];
            let text = format!("{} {}", p.name, p.root.display()).to_lowercase();
            words.iter().all(|w| text.contains(w.as_str()))
        })
        .collect();
    match sort {
        // Newest first; those never timed keep the list's order, last.
        Sort::LastOpened => kept.sort_by_key(|&i| std::cmp::Reverse(projects[i].last_opened)),
        Sort::Name => kept.sort_by_key(|&i| projects[i].name.to_lowercase()),
    }
    kept.into_iter().partition(|&i| projects[i].pinned)
}

/// How long ago `then` was, at `now` (both seconds since 1970), in words.
pub(crate) fn ago(then: u64, now: u64) -> String {
    let seconds = now.saturating_sub(then);
    let (minute, hour, day) = (60, 60 * 60, 24 * 60 * 60);
    let plural = |n: u64, one: &str| {
        if n == 1 {
            format!("1 {one} ago")
        } else {
            format!("{n} {one}s ago")
        }
    };
    match seconds {
        s if s < minute => "Just now".to_string(),
        s if s < hour => plural(s / minute, "minute"),
        s if s < day => plural(s / hour, "hour"),
        s if s < 2 * day => "Yesterday".to_string(),
        s if s < 14 * day => plural(s / day, "day"),
        s if s < 60 * day => plural(s / (7 * day), "week"),
        s if s < 365 * day => plural(s / (30 * day), "month"),
        s => plural(s / (365 * day), "year"),
    }
}

/// How a project's build compares with this one's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BuildMark {
    /// Opened last by this build.
    ThisOne(u64),
    /// By an older build: it's opened and updated as usual.
    Older(u64),
    /// By a newer build: this one may not open all of it.
    Newer(u64),
    /// By a build, while this copy has no number (built from source).
    Some(u64),
}

impl BuildMark {
    /// The mark for a project last opened by `saved_by`, seen from `this`.
    pub(crate) fn of(saved_by: Option<u64>, this: Option<u64>) -> Option<Self> {
        let saved = saved_by?;
        Some(match this {
            None => Self::Some(saved),
            Some(this) if saved == this => Self::ThisOne(saved),
            Some(this) if saved < this => Self::Older(saved),
            Some(_) => Self::Newer(saved),
        })
    }

    /// Its words, their colour (quiet for this build, amber for one to
    /// mind, as the design's "upgrade") and what it means.
    pub(crate) fn look(self) -> (String, Color32, &'static str) {
        match self {
            Self::ThisOne(b) => (
                format!("build {b} · this one"),
                tokens::text_meta(),
                "Last opened with this build",
            ),
            Self::Older(b) => (
                format!("build {b} · older"),
                tokens::WARNING,
                "Last opened with an older build: it opens as usual, and its files are \
                 updated (with a backup first) when they need it",
            ),
            Self::Newer(b) => (
                format!("build {b} · newer"),
                tokens::WARNING,
                "Last opened with a newer build: update first, or parts made with the newer \
                 build may not open",
            ),
            Self::Some(b) => (
                format!("build {b}"),
                tokens::text_meta(),
                "Last opened with this build (this copy was built from source and has no \
                 number to compare)",
            ),
        }
    }
}

/// Why `name` can't be a project's name, if it can't (the app checks
/// again when it renames).
pub(crate) fn name_problem(name: &str) -> Option<String> {
    let name = name.trim();
    if name.is_empty() {
        Some("A project needs a name.".into())
    } else if name.chars().count() > 100 {
        Some("Names are at most 100 letters.".into())
    } else if name.chars().any(char::is_control) {
        Some("A name can't hold line breaks.".into())
    } else {
        None
    }
}

/// Seconds since 1970, now.
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// A file the program carries beside itself, opened from About and
/// Updates (the installed copy's own, or the repository's online).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramFile {
    /// The program's licence.
    Licence,
    /// The libraries' and fonts' notices and licences.
    Notices,
    /// Every change, build by build.
    Changes,
}

impl Workbench {
    /// Shows Recent projects as the editor starts: Solis always opens on
    /// it, and a project opens from it (decision 0105). Called once the
    /// saved frame is put back, so the tab it left in front is the one
    /// Back to editor (or Esc) returns to. Search projects isn't given
    /// the keys at the start: the arrows walk the cards and Esc carries on
    /// into the editor straight away.
    pub fn start_on_recent_projects(&mut self) {
        self.show_recent_projects();
        self.file_pages.focus_search = false;
    }

    /// Shows Recent projects (the editor's window was closed with a
    /// project open, or File › Recent projects): the project stays open
    /// behind it, and Back to editor returns to it. A game being played
    /// stops first; the list is read afresh.
    pub fn show_recent_projects(&mut self) {
        if self.open.playing.is_some() {
            self.stop();
        }
        self.show_file_page(Page::Projects);
        self.file_pages.focus_search = true;
    }

    /// Whether the File tab is in front.
    pub fn file_page_open(&self) -> bool {
        self.shell.tab == crate::shell::RibbonTab::File
    }

    /// Keeps the update notice away for `build` (one skipped with Skip
    /// this version).
    pub fn hide_update_bar(&mut self, build: u64) {
        self.file_pages.update_hidden = Some(build);
    }

    /// The projects Solis knows (the app reads them).
    pub fn set_known_projects(&mut self, projects: Vec<KnownProject>) {
        self.known_projects = projects;
    }

    /// This copy's build number (none for one built from source), for the
    /// cards' marks and About.
    pub fn set_this_build(&mut self, build: Option<u64>) {
        self.this_build = build;
    }

    /// A line about what the app did for Recent projects (renamed,
    /// copied, found), shown there until closed.
    pub fn projects_notice(&mut self, notice: impl Into<String>) {
        self.file_pages.notice = Some(notice.into());
    }

    /// The open project was renamed: its new name in the title and the
    /// list, without opening it again.
    pub fn rename_open_project(&mut self, name: &str) {
        if let Some(project) = &mut self.open.project {
            project.name = name.to_string();
        }
    }

    /// Where the manual is, for Learn and About (the app knows it).
    pub fn set_manual_url(&mut self, url: String) {
        self.manual_url = url;
    }

    /// Opens the manual in the browser.
    pub(crate) fn open_manual(&self, ui: &Ui) {
        if !self.manual_url.is_empty() {
            ui.ctx()
                .open_url(egui::OpenUrl::new_tab(self.manual_url.clone()));
        }
    }

    /// The page in front, filling what is left of `ui`. Returns whether
    /// the project already open was chosen: the caller goes back to it.
    pub(crate) fn file_page_body(
        &mut self,
        ui: &mut Ui,
        actions: &mut Vec<WorkbenchAction>,
    ) -> bool {
        let mut asked: Vec<WorkbenchAction> = Vec::new();
        let [pad_y, pad_x] = size::FILE_PAGE_PAD;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(tokens::shelf_bar())
                    .inner_margin(egui::Margin::symmetric(pad_x as i8, pad_y as i8)),
            )
            .show(ui, |ui| match self.file_pages.page {
                Page::Projects => self.projects_page(ui, &mut asked),
                Page::Learn => self.learn_page(ui, &mut asked),
                Page::Updates => self.updates_page(ui, &mut asked),
                Page::About => self.about_page(ui, &mut asked),
                Page::Preferences => self.settings_page(ui, false, &mut asked),
                Page::ProjectSettings => self.settings_page(ui, true, &mut asked),
            });
        self.rename_question(ui, &mut asked);
        let mut back = false;
        for action in asked {
            if let WorkbenchAction::OpenRecent(root) = &action
                && self.open.project.as_ref().is_some_and(|p| &p.root == root)
            {
                // Already open: back to it.
                back = true;
                continue;
            }
            self.request(action, actions);
        }
        back
    }
}

#[cfg(test)]
mod tests;
