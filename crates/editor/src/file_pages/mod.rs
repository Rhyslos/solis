//! The File tab's own pages (specification, Screen 7, and the approved
//! additions, round 2): what belongs to the open project and the editor.
//!
//! Recent projects, Learn and Updates moved into the launcher, a window
//! of its own ([decision 0118](../../../../docs/decisions/0118-the-editor-rethink.md),
//! `crate::launcher`); File keeps Save all, Project settings,
//! Preferences, About, Close project (back to the launcher) and Exit.
//!
//! - `about.rs`: About.
//! - `page_parts.rs`: what the pages (and the launcher's) share.
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
mod page_parts;
mod preferences;
mod project_settings;
mod settings;
mod settings_page;
mod shortcuts;

pub(crate) use input_page::InputPage;
#[cfg(test)]
pub(crate) use input_page::Slot;
pub(crate) use page_parts::{button_row, page_head, plain, section, sentence, with_side};
#[cfg(test)]
pub(crate) use project_settings::fits;
pub(crate) use settings::Tab;

use crate::tokens::{self, size};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Color32, Ui};

/// The File tab's pages that show something of their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Page {
    /// The program's name, version and licences.
    About,
    /// The editor's own settings.
    Preferences,
    /// The open project's settings, its input and its export.
    #[default]
    ProjectSettings,
}

/// How the File pages are set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FilePages {
    /// The newer build whose notice was put away with Not now (the
    /// launcher's Updates).
    pub(crate) update_hidden: Option<u64>,
    /// The page shown.
    pub(crate) page: Page,
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

impl Default for FilePages {
    fn default() -> Self {
        Self {
            update_hidden: None,
            page: Page::ProjectSettings,
            prefs_tab: Tab::General,
            project_tab: Tab::Project,
            settings_search: String::new(),
            name_typed: None,
            shortcuts_search: String::new(),
            shortcuts_device: shortcuts::Device::All,
        }
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
    /// Whether the File tab is in front.
    pub fn file_page_open(&self) -> bool {
        self.shell.tab == crate::shell::RibbonTab::File
    }

    /// Keeps the update notice away for `build` (one skipped with Skip
    /// this version).
    pub fn hide_update_bar(&mut self, build: u64) {
        self.file_pages.update_hidden = Some(build);
    }

    /// The page in front, filling what is left of `ui`.
    pub(crate) fn file_page_body(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        let mut asked: Vec<WorkbenchAction> = Vec::new();
        let [pad_y, pad_x] = size::FILE_PAGE_PAD;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(tokens::shelf_bar())
                    .inner_margin(egui::Margin::symmetric(pad_x as i8, pad_y as i8)),
            )
            .show(ui, |ui| match self.file_pages.page {
                Page::About => self.about_page(ui, &mut asked),
                Page::Preferences => self.settings_page(ui, false, &mut asked),
                Page::ProjectSettings => self.settings_page(ui, true, &mut asked),
            });
        for action in asked {
            self.request(action, actions);
        }
    }
}

#[cfg(test)]
mod tests;
