//! The launcher (decision 0118, board 01 of `docs/design/ui-rethink/`): a
//! small window of its own, where the program starts. Projects as cards,
//! Learn and Updates down its side, Preferences and the user guide at its
//! foot. Opening a project closes it and the editor fills the window;
//! File › Close project comes back to it.
//!
//! The launcher is drawn by the workbench, in the one window the app
//! keeps (`Workbench::show` draws it in place of the editor while
//! [`Launcher::shown`]); the app gives the window the launcher's own size
//! and place while it shows, and the editor's while the editor does.
//! Nothing here reads a file: the app hands over the projects and answers
//! New project's questions about folders.
//!
//! - `side.rs`: the column down the left.
//! - `projects.rs`, `card.rs`: the Projects page and one project's card.
//! - `pictures.rs`: the plain pictures a project, a kind and a starter
//!   show until a project has a picture of its own.
//! - `new_project.rs`, `names.rs`: the New project dialog (board 02) and
//!   the rules its name keeps.
//! - `learn.rs`, `updates_page.rs`: the Learn and Updates pages.

mod card;
mod learn;
mod names;
mod new_project;
mod pictures;
mod projects;
mod side;
mod updates_page;

pub use names::{FolderState, name_problem};
pub(crate) use new_project::NewProject;
pub use new_project::{NewProjectPlan, Starter};

use crate::workbench::{Workbench, WorkbenchAction};
use egui::Ui;
use std::path::PathBuf;

/// What a project makes: 2D, 3D or both (decision 0118). The project file
/// keeps it (`asset_db::ProjectKind`); R6 hides a 2D project's 3D-only
/// parts. Projects from before it was asked are both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum ProjectKind {
    /// Sprites and tile maps.
    TwoD,
    /// Models, lights and cameras.
    ThreeD,
    /// Both: everything.
    #[default]
    Both,
}

impl ProjectKind {
    /// Every kind, in the order they're offered.
    pub const ALL: [Self; 3] = [Self::TwoD, Self::ThreeD, Self::Both];

    /// The word on a card's corner and in the filter.
    pub fn badge(self) -> &'static str {
        match self {
            Self::TwoD => "2D",
            Self::ThreeD => "3D",
            Self::Both => "2D + 3D",
        }
    }

    /// Its name where it's chosen (New project, Project settings).
    pub fn name(self) -> &'static str {
        match self {
            Self::TwoD => "2D",
            Self::ThreeD => "3D",
            Self::Both => "Both",
        }
    }

    /// What it means, in a sentence.
    pub fn about(self) -> &'static str {
        match self {
            Self::TwoD => "Sprites and tile maps, seen from the side or from above.",
            Self::ThreeD => "Models, lights and cameras in a world you walk round.",
            Self::Both => "Everything: 3D worlds with 2D parts, or the other way.",
        }
    }
}

/// The launcher's pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Page {
    /// The projects, as cards.
    #[default]
    Projects,
    /// The examples and how to begin.
    Learn,
    /// This build, the newest, and updating.
    Updates,
    /// The editor's own settings, as File › Preferences has them.
    Preferences,
}

/// Which projects the cards show, by kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Filter {
    /// Every project.
    #[default]
    All,
    /// Only those of this kind.
    Kind(ProjectKind),
}

impl Filter {
    /// Every filter, with its word, in the order drawn.
    pub(crate) fn choices() -> [(Self, &'static str); 4] {
        let kind = |k: ProjectKind| (Self::Kind(k), k.badge());
        [
            (Self::All, "All"),
            kind(ProjectKind::TwoD),
            kind(ProjectKind::ThreeD),
            kind(ProjectKind::Both),
        ]
    }

    fn keeps(self, kind: ProjectKind) -> bool {
        match self {
            Self::All => true,
            Self::Kind(k) => k == kind,
        }
    }
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
    /// 2D, 3D or both.
    pub kind: ProjectKind,
}

/// The launcher's state: whether it shows, the page in front, how the
/// projects are searched, filtered and ordered, and New project.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Launcher {
    /// Whether the launcher shows, in place of the editor.
    pub(crate) shown: bool,
    /// The page in front.
    pub(crate) page: Page,
    /// What's typed in Search projects.
    pub(crate) search: String,
    /// Which kinds are shown.
    pub(crate) filter: Filter,
    /// The order.
    pub(crate) sort: Sort,
    /// The card picked (Enter opens it).
    pub(crate) selected: Option<PathBuf>,
    /// A line from the app about what it did or couldn't do (opened,
    /// copied, found, refused), shown under the heading until closed.
    pub(crate) notice: Option<String>,
    /// The project being renamed, and the name typed.
    pub(crate) renaming: Option<Renaming>,
    /// New project, while it's open.
    pub(crate) new_project: Option<NewProject>,
    /// The folder new projects are made in, as last used (the app keeps
    /// it with the settings).
    pub(crate) create_in: Option<PathBuf>,
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

/// The projects shown, as places in `projects`: the pinned ones, then the
/// rest, each in `sort`'s order, keeping those of `filter`'s kind whose
/// name or folder holds every word of `search` (any case).
pub(crate) fn shown(
    projects: &[KnownProject],
    search: &str,
    filter: Filter,
    sort: Sort,
) -> Vec<usize> {
    let words: Vec<String> = search.split_whitespace().map(str::to_lowercase).collect();
    let mut kept: Vec<usize> = (0..projects.len())
        .filter(|&i| {
            let p = &projects[i];
            let text = format!("{} {}", p.name, p.root.display()).to_lowercase();
            filter.keeps(p.kind) && words.iter().all(|w| text.contains(w.as_str()))
        })
        .collect();
    match sort {
        // Newest first; those never timed keep the list's order, last.
        Sort::LastOpened => kept.sort_by_key(|&i| std::cmp::Reverse(projects[i].last_opened)),
        Sort::Name => kept.sort_by_key(|&i| projects[i].name.to_lowercase()),
    }
    let (pinned, rest): (Vec<usize>, Vec<usize>) =
        kept.into_iter().partition(|&i| projects[i].pinned);
    pinned.into_iter().chain(rest).collect()
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

/// Seconds since 1970, now.
pub(crate) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl Workbench {
    /// Shows the launcher in place of the editor, on Projects, with the
    /// list read afresh (the program starts here, and File › Close
    /// project comes back here). A game being played stops first.
    pub fn open_launcher(&mut self) {
        if self.open.playing.is_some() {
            self.stop();
        }
        self.launcher.shown = true;
        self.launcher.page = Page::Projects;
        self.launcher.new_project = None;
        self.open.outbox.push(WorkbenchAction::ListProjects);
    }

    /// The editor's window was closed with a project open: the project
    /// closes (asking about unsaved work first) and the launcher comes
    /// back, as File › Close project does.
    pub fn close_project_asked(&mut self) {
        let mut actions = Vec::new();
        self.request(WorkbenchAction::CloseProject, &mut actions);
        self.open.outbox.extend(actions);
    }

    /// Whether the launcher shows, in place of the editor: the app gives
    /// the window the launcher's size and place while it does.
    pub fn launcher_shown(&self) -> bool {
        self.launcher.shown
    }

    /// The projects Solis knows (the app reads them).
    pub fn set_known_projects(&mut self, projects: Vec<KnownProject>) {
        self.known_projects = projects;
    }

    /// This copy's build number (none for one built from source), for
    /// About and Updates.
    pub fn set_this_build(&mut self, build: Option<u64>) {
        self.this_build = build;
    }

    /// A line about what the app did for the launcher (renamed, copied,
    /// found, couldn't open), shown on Projects until closed.
    pub fn projects_notice(&mut self, notice: impl Into<String>) {
        self.launcher.notice = Some(notice.into());
    }

    /// The folder New project makes projects in, as last used (the app
    /// remembers it), or the first one: a folder in Documents named from
    /// the program's name.
    pub fn set_create_in(&mut self, folder: PathBuf) {
        self.launcher.create_in = Some(folder.clone());
        if let Some(dialog) = &mut self.launcher.new_project {
            dialog.create_in = folder;
        }
    }

    /// What the app found at a folder New project asked about.
    pub fn set_folder_state(&mut self, folder: PathBuf, state: FolderState) {
        if let Some(dialog) = &mut self.launcher.new_project {
            dialog.found(folder, state);
        }
    }

    /// The app couldn't make the project New project asked for: why,
    /// shown in the dialog, which stays open.
    pub fn new_project_failed(&mut self, why: impl Into<String>) {
        match &mut self.launcher.new_project {
            Some(dialog) => dialog.failed = Some(why.into()),
            None => self.launcher.notice = Some(why.into()),
        }
    }

    /// The open project's kind, as its project file says (the app reads
    /// it as the project opens).
    pub fn set_project_kind(&mut self, kind: ProjectKind) {
        self.open.kind = kind;
    }

    /// The open project's kind.
    pub fn project_kind(&self) -> ProjectKind {
        self.open.kind
    }

    /// The open project was renamed: its new name in the title and the
    /// list, without opening it again.
    pub fn rename_open_project(&mut self, name: &str) {
        if let Some(project) = &mut self.open.project {
            project.name = name.to_string();
        }
    }

    /// The launcher, filling `ui`: its column and the page in front, and
    /// New project over them while it's open. Returns what the app must
    /// do.
    pub(crate) fn show_launcher(&mut self, ui: &mut Ui) -> Vec<WorkbenchAction> {
        let mut actions = std::mem::take(&mut self.open.outbox);
        let mut asked = Vec::new();
        self.launcher_side(ui, &mut asked);
        self.launcher_page(ui, &mut asked);
        self.rename_question(ui, &mut asked);
        self.new_project_dialog(ui, &mut asked);
        self.update_question(ui.ctx(), &mut asked);
        for action in asked {
            self.request(action, &mut actions);
        }
        actions
    }

    /// The page in front, beside the column.
    fn launcher_page(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        let [pad_y, pad_x] = crate::tokens::launcher::PAGE_PAD;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(crate::tokens::shelf_bar())
                    .inner_margin(egui::Margin::symmetric(pad_x as i8, pad_y as i8)),
            )
            .show(ui, |ui| match self.launcher.page {
                Page::Projects => self.projects_page(ui, asked),
                Page::Learn => self.learn_page(ui, asked),
                Page::Updates => self.updates_page(ui, asked),
                Page::Preferences => self.settings_page(ui, false, asked),
            });
    }
}

#[cfg(test)]
mod tests;
