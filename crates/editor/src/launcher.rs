//! The launcher ([decision 0086](../../../docs/decisions/0086-the-launcher.md)),
//! as the project lead approved it (the mock-up of 5 November): a full
//! page with a sidebar, shown when Solis starts with no project open and
//! from File > Projects…. It grew from the start page (E1, decision 0070).
//!
//! - **Projects:** every project Solis knows, pinned ones first; search by
//!   name or folder; sort by last opened or name; ☆ to pin; double-click or
//!   Enter to open; ⋯ for Open and Remove from list. Each row shows the
//!   build of Solis that last opened the project. A folder that's gone
//!   says so.
//! - **Learn:** the three examples, four steps for someone new, and the
//!   manual.
//!
//! The app keeps the list and reads each project's file; the launcher
//! shows it and asks for changes. While it's up, the editor isn't drawn
//! and its keys are off.

use crate::theme;
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Align2, Color32, FontId, RichText, Sense, Ui, vec2};
use std::path::PathBuf;

/// One project in the launcher's list, as the app reads it.
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

/// The launcher's pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Page {
    /// The projects.
    #[default]
    Projects,
    /// The examples and how to begin.
    Learn,
    /// This build, the newest, and updating.
    Updates,
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
    fn name(self) -> &'static str {
        match self {
            Self::LastOpened => "Last opened",
            Self::Name => "Name",
        }
    }
}

/// Whether the launcher is up, and how it's set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Launcher {
    /// It's showing.
    pub(crate) open: bool,
    /// The newer build whose bar was put away with Not now (this time
    /// the launcher is up).
    pub(crate) update_hidden: Option<u64>,
    /// The page shown.
    pub(crate) page: Page,
    /// What's typed in the search box.
    pub(crate) search: String,
    /// The order.
    pub(crate) sort: Sort,
    /// The row picked (Enter opens it).
    pub(crate) selected: Option<PathBuf>,
    /// The search box takes the keys when the launcher comes up.
    focus_search: bool,
    /// A line from the app about what it did or couldn't do (renamed,
    /// copying, found), shown under the heading until closed.
    pub(crate) notice: Option<String>,
    /// The project being renamed, the name typed, and why it can't be.
    pub(crate) renaming: Option<Renaming>,
}

/// The Rename… question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Renaming {
    /// The project's folder.
    pub(crate) root: PathBuf,
    /// The name typed.
    pub(crate) name: String,
    /// Its focus is given once.
    focused: bool,
}

impl Default for Launcher {
    fn default() -> Self {
        Self {
            open: false,
            update_hidden: None,
            page: Page::Projects,
            search: String::new(),
            sort: Sort::LastOpened,
            selected: None,
            focus_search: true,
            notice: None,
            renaming: None,
        }
    }
}

/// The sidebar's width, a row's height, and an example card's width.
const SIDEBAR: f32 = 232.0;
const ROW: f32 = 62.0;
const CARD: f32 = 260.0;

/// An example project, made in a folder the user picks.
struct Example {
    name: &'static str,
    about: &'static str,
    make: WorkbenchAction,
}

fn examples() -> [Example; 4] {
    [
        Example {
            name: "Playground",
            about: "Walk a character around a 3D level: steps, ramps, crates and a moving platform",
            make: WorkbenchAction::MakePlayground,
        },
        Example {
            name: "Turret sample",
            about: "Game code in Lust, Lua and Rust, talking through events",
            make: WorkbenchAction::MakeSampleGame,
        },
        Example {
            name: "UI sample",
            about: "A HUD and a pause menu with the player's settings, drawn on the canvas",
            make: WorkbenchAction::MakeUiSample,
        },
        Example {
            name: "Benchmark",
            about: "Fleets of ships firing lasers: how much this computer can run",
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

    /// Its words, colours (text and fill) and explanation.
    fn look(self) -> (String, Color32, Color32, &'static str) {
        match self {
            Self::ThisOne(b) => (
                format!("build {b} · this one"),
                Color32::from_rgb(0x8f, 0xd4, 0x9a),
                Color32::from_rgb(0x24, 0x40, 0x2a),
                "Last opened with this build",
            ),
            Self::Older(b) => (
                format!("build {b} · older"),
                Color32::from_rgb(0xe8, 0xc0, 0x6a),
                Color32::from_rgb(0x46, 0x38, 0x1a),
                "Last opened with an older build: it opens as usual, and its files are \
                 updated (with a backup first) when they need it",
            ),
            Self::Newer(b) => (
                format!("build {b} · newer"),
                theme::ERROR,
                Color32::from_rgb(0x4a, 0x24, 0x22),
                "Last opened with a newer build: update first, or parts made with the newer \
                 build may not open",
            ),
            Self::Some(b) => (
                format!("build {b}"),
                theme::text(),
                theme::field(),
                "Last opened with this build (this copy was built from source and has no \
                 number to compare)",
            ),
        }
    }
}

/// A colour of its own for a project's letter, from its name.
fn tile_colour(name: &str) -> (Color32, Color32) {
    const TILES: [(Color32, Color32); 6] = [
        (
            Color32::from_rgb(0x2a, 0x35, 0x50),
            Color32::from_rgb(0x9c, 0xc5, 0xf5),
        ),
        (
            Color32::from_rgb(0x3a, 0x2a, 0x2a),
            Color32::from_rgb(0xd9, 0x8a, 0x8a),
        ),
        (
            Color32::from_rgb(0x2a, 0x3d, 0x2c),
            Color32::from_rgb(0x8f, 0xd4, 0x9a),
        ),
        (
            Color32::from_rgb(0x40, 0x34, 0x1c),
            Color32::from_rgb(0xe8, 0xc0, 0x6a),
        ),
        (
            Color32::from_rgb(0x34, 0x2a, 0x45),
            Color32::from_rgb(0xc4, 0xa8, 0xf0),
        ),
        (
            Color32::from_rgb(0x1f, 0x3a, 0x3a),
            Color32::from_rgb(0x7f, 0xd0, 0xc8),
        ),
    ];
    let sum: usize = name.bytes().map(usize::from).sum();
    TILES[sum % TILES.len()]
}

/// Why `name` can't be a project's name, if it can't (the app checks
/// again when it renames).
fn name_problem(name: &str) -> Option<String> {
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

/// The Updates page's words about the newest build: a small heading,
/// a big line and what it means.
fn newest_words(
    update: &crate::updates::UpdateNotice,
    name: &str,
) -> (&'static str, String, String) {
    use crate::updates::UpdateNotice as N;
    match update {
        N::Available(b) => (
            "NEW BUILD",
            format!("Build {b}"),
            "Out now: download it while you work; it installs when you close or restart."
                .to_string(),
        ),
        N::Downloading(b) => (
            "NEW BUILD",
            format!("Build {b}"),
            "Downloading…".to_string(),
        ),
        N::Ready(b) => (
            "NEW BUILD",
            format!("Build {b}"),
            format!(
                "Downloaded and checked. {name} closes (asking about unsaved work first), \
                 updates and opens again. Your projects aren't touched."
            ),
        ),
        N::NotInstalled(b) => (
            "NEW BUILD",
            format!("Build {b}"),
            "This copy wasn't installed with the installer, so it can't update itself: get \
             the new build from the releases page."
                .to_string(),
        ),
        N::UpToDate(_) => (
            "NEWEST BUILD",
            "You have it".to_string(),
            "Nothing to do.".to_string(),
        ),
        N::Checking => (
            "NEWEST BUILD",
            "Checking…".to_string(),
            "Asking GitHub.".to_string(),
        ),
        N::NotChecked => (
            "NEWEST BUILD",
            "Not checked yet".to_string(),
            "Check now asks GitHub.".to_string(),
        ),
        N::FromSource => (
            "NEWEST BUILD",
            "—".to_string(),
            "Copies built from source don't update themselves; installed copies do.".to_string(),
        ),
        N::Failed(why) => ("NEWEST BUILD", "Couldn't check".to_string(), why.clone()),
    }
}

/// Seconds since 1970, now.
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl Workbench {
    /// The bar under the heading while a newer build is out, downloading
    /// or downloaded (the project lead, 2026-11-06): no question when
    /// Solis starts; Update now, See what's new, and Not now.
    fn update_bar(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        use crate::updates::UpdateNotice as N;
        let name = self.info.name.clone();
        let (text, button) = match &self.update {
            N::Available(build) if self.launcher.update_hidden != Some(*build) => (
                format!(
                    "Build {build} is out. Your projects are backed up before an update changes \
                     them."
                ),
                Some(("Update now", WorkbenchAction::DownloadUpdate)),
            ),
            N::Downloading(build) => (format!("Downloading build {build}…"), None),
            N::Ready(build) => (
                format!("Build {build} is downloaded. It installs when {name} closes."),
                Some(("Restart to update", WorkbenchAction::Quit)),
            ),
            _ => return,
        };
        let fill = self.accent.colors().fill;
        ui.add_space(10.0);
        egui::Frame::new()
            .fill(theme::UPDATE)
            .stroke(egui::Stroke::new(1.0, fill))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⬇").color(theme::text_strong()));
                    ui.label(RichText::new(&text).color(theme::text_strong()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some((label, action)) = button {
                            let go = egui::Button::new(
                                RichText::new(label).color(theme::on_accent()).strong(),
                            )
                            .fill(fill);
                            if ui.add(go).clicked() {
                                // Restart to update: the window closes to
                                // install it.
                                self.restarting = action == WorkbenchAction::Quit;
                                asked.push(action);
                            }
                        }
                        if let N::Available(build) = self.update {
                            if ui.link("Not now").clicked() {
                                self.launcher.update_hidden = Some(build);
                            }
                            if ui.link("See what's new").clicked() {
                                self.launcher.page = Page::Updates;
                            }
                        }
                    });
                });
            });
    }

    /// Shows the launcher as the editor starts: Solis always opens on it,
    /// and a project opens from it (the project lead, 2026-11-06).
    pub fn launch_launcher(&mut self) {
        self.open_launcher();
    }

    /// Keeps the launcher's update bar away for `build` (one skipped with
    /// Skip this version).
    pub fn hide_update_bar(&mut self, build: u64) {
        self.launcher.update_hidden = Some(build);
    }

    /// Shows the launcher again (the editor's window was closed with a
    /// project open, or File > Projects…): the project stays open behind
    /// it, and "Back to" returns to it.
    pub fn back_to_launcher(&mut self) {
        self.open_launcher();
    }

    /// Whether the launcher is up.
    pub fn launcher_open(&self) -> bool {
        self.launcher.open
    }

    /// Shows the launcher (File > Projects…), asking the app for the list
    /// read afresh. A game being played stops first.
    pub(crate) fn open_launcher(&mut self) {
        if self.open.playing.is_some() {
            self.stop();
        }
        self.launcher.open = true;
        self.launcher.focus_search = true;
        self.open.outbox.push(WorkbenchAction::ListProjects);
    }

    /// The projects Solis knows, for the launcher (the app reads them).
    pub fn set_known_projects(&mut self, projects: Vec<KnownProject>) {
        self.known_projects = projects;
    }

    /// This copy's build number (none for one built from source), for the
    /// launcher's marks.
    pub fn set_this_build(&mut self, build: Option<u64>) {
        self.this_build = build;
    }

    /// A line about what the app did for the launcher (renamed, copied,
    /// found), shown on the Projects page until closed.
    pub fn launcher_notice(&mut self, notice: impl Into<String>) {
        self.launcher.notice = Some(notice.into());
    }

    /// The open project was renamed: its new name in the title and the
    /// launcher, without opening it again.
    pub fn rename_open_project(&mut self, name: &str) {
        if let Some(project) = &mut self.open.project {
            project.name = name.to_string();
        }
    }

    /// Where the manual is, for the launcher's link (the app knows it).
    pub fn set_manual_url(&mut self, url: String) {
        self.manual_url = url;
    }

    /// Draws the launcher over the whole window while it's up. Returns
    /// whether it was drawn (the editor isn't, then).
    pub(crate) fn launcher(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) -> bool {
        if !self.launcher.open {
            return false;
        }
        let mut asked: Vec<WorkbenchAction> = Vec::new();
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) && self.open.unsaved_prompt.is_none() {
            if self.launcher.renaming.is_some() {
                self.launcher.renaming = None;
            } else {
                self.launcher.open = false;
            }
        }
        egui::Panel::left("launcher sidebar")
            .exact_size(SIDEBAR)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    // The design's File page: its menu column.
                    .fill(crate::tokens::ribbon())
                    .inner_margin(egui::Margin::symmetric(14, 22)),
            )
            .show(ui, |ui| self.launcher_sidebar(ui));
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    // And its page.
                    .fill(crate::tokens::shelf_bar())
                    .inner_margin(egui::Margin::symmetric(36, 28)),
            )
            .show(ui, |ui| match self.launcher.page {
                Page::Projects => self.projects_page(ui, &mut asked),
                Page::Learn => self.learn_page(ui, &mut asked),
                Page::Updates => self.updates_page(ui, &mut asked),
            });
        self.rename_question(ui, &mut asked);
        for action in asked {
            if let WorkbenchAction::OpenRecent(root) = &action
                && self.open.project.as_ref().is_some_and(|p| &p.root == root)
            {
                // Already open: back to it.
                self.launcher.open = false;
                continue;
            }
            self.request(action, actions);
        }
        true
    }

    fn launcher_sidebar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            if let Some(logo) = self.logo {
                ui.add(egui::Image::new((logo, vec2(24.0, 24.0))));
            }
            ui.label(
                RichText::new(&self.info.name)
                    .size(18.0)
                    .strong()
                    .color(theme::text_strong()),
            );
        });
        ui.add_space(22.0);
        let newer = self.update.newer_build();
        for (page, name, icon) in [
            (
                Page::Projects,
                "Projects",
                Some(crate::icons::Icon::Library),
            ),
            (Page::Learn, "Learn", Some(crate::icons::Icon::Info)),
            (Page::Updates, "Updates", None),
        ] {
            let on = self.launcher.page == page;
            let (rect, item) =
                ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
            let painter = ui.painter();
            // The design's File menu: the page in front and the row under
            // the pointer share one quiet grey.
            if on || item.hovered() {
                let radius = f32::from(crate::tokens::size::radius::MENU);
                painter.rect_filled(rect, radius, crate::tokens::group_gap());
            }
            let colour = if on {
                theme::text_strong()
            } else {
                theme::text()
            };
            let icon_colour = if on {
                self.accent.colors().fill
            } else {
                theme::text_weak()
            };
            let icon_at = rect.left_center() + vec2(20.0, 0.0);
            match icon {
                Some(icon) => crate::icons::icon(
                    painter,
                    egui::Rect::from_center_size(icon_at, vec2(14.0, 14.0)),
                    icon,
                    icon_colour,
                ),
                // A downward arrow onto a line: updates.
                None => {
                    let stroke = egui::Stroke::new(1.5, icon_colour);
                    let c = icon_at;
                    painter.line_segment([c + vec2(0.0, -6.0), c + vec2(0.0, 3.0)], stroke);
                    painter.line_segment([c + vec2(-4.0, -1.0), c + vec2(0.0, 3.0)], stroke);
                    painter.line_segment([c + vec2(4.0, -1.0), c + vec2(0.0, 3.0)], stroke);
                    painter.line_segment([c + vec2(-6.0, 6.5), c + vec2(6.0, 6.5)], stroke);
                }
            }
            if page == Page::Updates && newer.is_some() {
                let badge = egui::Rect::from_min_size(
                    egui::pos2(rect.right() - 46.0, rect.center().y - 9.0),
                    vec2(38.0, 18.0),
                );
                painter.rect_filled(badge, 9.0, self.accent.colors().fill);
                painter.text(
                    badge.center(),
                    Align2::CENTER_CENTER,
                    "New",
                    FontId::proportional(11.0),
                    theme::on_accent(),
                );
            }
            painter.text(
                rect.left_center() + vec2(38.0, 0.0),
                Align2::LEFT_CENTER,
                name,
                FontId::proportional(14.0),
                colour,
            );
            item.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, name)
            });
            if item.clicked() {
                self.launcher.page = page;
            }
            ui.add_space(2.0);
        }
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            let build = match self.this_build {
                Some(build) => format!("{} build {build}", self.info.name),
                None => format!(
                    "{} {} · built from source",
                    self.info.name, self.info.version
                ),
            };
            ui.label(RichText::new(build).small().color(theme::text_weak()));
            if let Some(line) = match &self.update {
                crate::updates::UpdateNotice::Ready(b) => {
                    Some(format!("Build {b} is ready to install"))
                }
                crate::updates::UpdateNotice::Available(b)
                | crate::updates::UpdateNotice::NotInstalled(b) => {
                    Some(format!("Build {b} is out"))
                }
                crate::updates::UpdateNotice::Downloading(b) => {
                    Some(format!("Downloading build {b}…"))
                }
                _ => None,
            } {
                ui.label(RichText::new(line).small().color(theme::WARNING));
            }
            ui.add_space(6.0);
            let back = match &self.open.project {
                Some(p) => format!("← Back to {}", p.name),
                None => "Carry on without a project".to_string(),
            };
            let close = ui
                .add(egui::Button::new(RichText::new(back).color(theme::text_weak())).frame(false))
                .on_hover_text("Closes this page (Esc)");
            close.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Close the launcher")
            });
            if close.clicked() {
                self.launcher.open = false;
            }
            ui.separator();
        });
    }

    fn projects_page(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        let fill = self.accent.colors().fill;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Projects")
                    .size(24.0)
                    .strong()
                    .color(theme::text_strong()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let new = egui::Button::new(
                    RichText::new("+ New project")
                        .color(theme::on_accent())
                        .strong(),
                )
                .fill(fill)
                .min_size(vec2(0.0, 32.0));
                if ui
                    .add(new)
                    .on_hover_text("Makes a folder into a project for your game's files")
                    .clicked()
                {
                    asked.push(WorkbenchAction::NewProject);
                }
                if ui
                    .add(egui::Button::new("Open project…").min_size(vec2(0.0, 32.0)))
                    .on_hover_text("Opens a project folder that isn't in the list yet")
                    .clicked()
                {
                    asked.push(WorkbenchAction::OpenProject);
                }
                if ui
                    .add(egui::Button::new("Scan a folder…").min_size(vec2(0.0, 32.0)))
                    .on_hover_text(
                        "Looks through a folder, and the folders in it, for projects to add to \
                         the list",
                    )
                    .clicked()
                {
                    asked.push(WorkbenchAction::ScanForProjects);
                }
            });
        });
        self.update_bar(ui, asked);
        if let Some(notice) = self.launcher.notice.clone() {
            ui.add_space(8.0);
            egui::Frame::new()
                .fill(theme::notice())
                .stroke(egui::Stroke::new(1.0, theme::selected_border()))
                .corner_radius(6.0)
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&notice).color(theme::text_strong()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let close = ui.add(egui::Button::new("×").frame(false));
                            close.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    true,
                                    "Close the note",
                                )
                            });
                            if close.clicked() {
                                self.launcher.notice = None;
                            }
                        });
                    });
                });
        }
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            let sort_width = 190.0;
            let search = ui.add(
                egui::TextEdit::singleline(&mut self.launcher.search)
                    .hint_text("Search projects")
                    .desired_width((ui.available_width() - sort_width).max(120.0))
                    .margin(vec2(10.0, 7.0)),
            );
            if std::mem::take(&mut self.launcher.focus_search) {
                search.request_focus();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::ComboBox::from_id_salt("launcher sort")
                    .selected_text(self.launcher.sort.name())
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        for sort in [Sort::LastOpened, Sort::Name] {
                            ui.selectable_value(&mut self.launcher.sort, sort, sort.name());
                        }
                    });
                ui.label(RichText::new("Sort by").small().color(theme::text_weak()));
            });
        });
        ui.add_space(14.0);
        let (pinned, rest) = shown(
            &self.known_projects,
            &self.launcher.search,
            self.launcher.sort,
        );
        // Arrows move along the list; Enter opens the one picked.
        let order: Vec<usize> = pinned.iter().chain(&rest).copied().collect();
        let at = self.launcher.selected.as_ref().and_then(|s| {
            order
                .iter()
                .position(|&i| &self.known_projects[i].root == s)
        });
        let (down, up, enter) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::Enter),
            )
        });
        let moved = match (down, up, at) {
            (true, _, None) | (_, true, None) => order.first().copied(),
            (true, _, Some(at)) => order.get(at + 1).or(order.last()).copied(),
            (_, true, Some(at)) => order.get(at.saturating_sub(1)).copied(),
            _ => None,
        };
        if let Some(i) = moved {
            self.launcher.selected = Some(self.known_projects[i].root.clone());
        }
        if enter
            && let Some(i) = at.map(|at| order[at])
            && self.known_projects[i].problem.is_none()
        {
            asked.push(WorkbenchAction::OpenRecent(
                self.known_projects[i].root.clone(),
            ));
        }
        if self.known_projects.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new("No projects yet")
                        .size(18.0)
                        .color(theme::text_strong()),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(
                        "Make one with + New project, open a folder with Open project…, or try \
                         an example under Learn.",
                    )
                    .color(theme::text_weak()),
                );
            });
            return;
        }
        if order.is_empty() {
            ui.label(
                RichText::new(format!(
                    "No project matches \u{201c}{}\u{201d}.",
                    self.launcher.search
                ))
                .color(theme::text_weak()),
            );
            return;
        }
        let now = now();
        let mut changes = Vec::new();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (heading, list) in [("PINNED", &pinned), ("ALL PROJECTS", &rest)] {
                    if list.is_empty() {
                        continue;
                    }
                    if !pinned.is_empty() {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(heading)
                                .small()
                                .strong()
                                .color(theme::text_weak()),
                        );
                        ui.add_space(2.0);
                    }
                    for &i in list {
                        self.project_row(ui, i, now, asked, &mut changes);
                        ui.add_space(6.0);
                    }
                }
                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "Double-click a project to open it. Removing one from the list leaves \
                         its files where they are.",
                    )
                    .small()
                    .color(theme::text_weak()),
                );
            });
        for change in changes {
            match &change {
                WorkbenchAction::PinProject(root, pinned) => {
                    if let Some(p) = self.known_projects.iter_mut().find(|p| &p.root == root) {
                        p.pinned = *pinned;
                    }
                }
                WorkbenchAction::ForgetProject(root) => {
                    self.known_projects.retain(|p| &p.root != root);
                }
                _ => {}
            }
            asked.push(change);
        }
    }

    /// One project's row: its letter, name and folder, the build that last
    /// opened it, when, its pin and its ⋯.
    fn project_row(
        &mut self,
        ui: &mut Ui,
        i: usize,
        now: u64,
        asked: &mut Vec<WorkbenchAction>,
        changes: &mut Vec<WorkbenchAction>,
    ) {
        let project = self.known_projects[i].clone();
        let selected = self.launcher.selected.as_ref() == Some(&project.root);
        let (rect, row) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::click());
        // The row's own parts go in a child, so the list's place moves on by
        // the row alone.
        let ui = &mut ui.new_child(egui::UiBuilder::new().max_rect(rect));
        row.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, &project.name)
        });
        let painter = ui.painter().clone();
        let (fill, edge) = if selected {
            (theme::selected(), theme::selected_border())
        } else if row.hovered() {
            (theme::menu_row(), theme::button_border())
        } else {
            (theme::panel(), theme::card_border())
        };
        painter.rect(
            rect,
            8.0,
            fill,
            egui::Stroke::new(1.0, edge),
            egui::StrokeKind::Inside,
        );
        // The letter.
        let tile = egui::Rect::from_min_size(rect.left_top() + vec2(14.0, 9.0), vec2(70.0, 44.0));
        let (back, front) = if project.problem.is_some() {
            (theme::field(), theme::text_weak())
        } else {
            tile_colour(&project.name)
        };
        painter.rect_filled(tile, 5.0, back);
        let letter = project
            .name
            .chars()
            .find(|c| c.is_alphanumeric())
            .map_or_else(|| "?".to_string(), |c| c.to_uppercase().collect());
        painter.text(
            tile.center(),
            Align2::CENTER_CENTER,
            if project.problem.is_some() {
                "?".to_string()
            } else {
                letter
            },
            FontId::proportional(20.0),
            front,
        );
        // The name and the folder (or what's wrong).
        let words = rect.left() + 100.0;
        let right = rect.right();
        let name_colour = if project.problem.is_some() {
            theme::text_weak()
        } else {
            theme::text_strong()
        };
        let clip = egui::Rect::from_min_max(
            egui::pos2(words, rect.top()),
            egui::pos2(right - 380.0, rect.bottom()),
        );
        let painter_clipped = painter.with_clip_rect(clip);
        painter_clipped.text(
            egui::pos2(words, rect.top() + 21.0),
            Align2::LEFT_CENTER,
            &project.name,
            FontId::proportional(15.0),
            name_colour,
        );
        let (under, under_colour, under_font) = match &project.problem {
            Some(problem) => (
                format!("{} · {}", problem, project.root.display()),
                theme::WARNING,
                FontId::proportional(12.0),
            ),
            None => (
                project.root.display().to_string(),
                theme::text_weak(),
                FontId::monospace(12.0),
            ),
        };
        painter_clipped.text(
            egui::pos2(words, rect.top() + 41.0),
            Align2::LEFT_CENTER,
            under,
            under_font,
            under_colour,
        );
        // The build that last opened it.
        let chip_at = egui::pos2(right - 370.0, rect.center().y);
        if project.problem.is_none()
            && let Some(mark) = BuildMark::of(project.saved_by, self.this_build)
        {
            let (text, colour, back, why) = mark.look();
            let galley = painter.layout_no_wrap(text, FontId::proportional(12.0), colour);
            let chip = egui::Rect::from_min_size(
                chip_at - vec2(0.0, 11.0),
                galley.size() + vec2(16.0, 8.0),
            );
            painter.rect_filled(chip, 10.0, back);
            let words = galley.text().to_string();
            painter.galley(chip.min + vec2(8.0, 4.0), galley, colour);
            let chip = ui.interact(chip, row.id.with("build"), Sense::hover());
            chip.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &words));
            chip.on_hover_text(why);
        } else if project.problem.is_none() {
            painter.text(
                chip_at,
                Align2::LEFT_CENTER,
                "—",
                FontId::proportional(13.0),
                theme::text_weak(),
            );
            ui.interact(
                egui::Rect::from_center_size(chip_at + vec2(6.0, 0.0), vec2(20.0, 20.0)),
                row.id.with("build"),
                Sense::hover(),
            )
            .on_hover_text(
                "Not known: last opened before builds were noted, or by a copy built from \
                 source",
            );
        }
        // When.
        if project.problem.is_none() {
            let when = project
                .last_opened
                .map_or_else(|| "—".to_string(), |t| ago(t, now));
            painter.text(
                egui::pos2(right - 200.0, rect.center().y),
                Align2::LEFT_CENTER,
                when,
                FontId::proportional(13.0),
                theme::text(),
            );
        }
        // The pin and ⋯, or Remove for a project that can't be opened.
        let root = project.root.clone();
        if project.problem.is_some() {
            let locate = ui.put(
                egui::Rect::from_center_size(
                    egui::pos2(right - 140.0, rect.center().y),
                    vec2(84.0, 28.0),
                ),
                egui::Button::new("Locate…"),
            );
            locate.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    true,
                    format!("Locate {}", project.name),
                )
            });
            if locate
                .on_hover_text("Moved it? Choose the folder where it is now")
                .clicked()
            {
                asked.push(WorkbenchAction::LocateProject(root.clone()));
            }
            let remove = ui.put(
                egui::Rect::from_center_size(
                    egui::pos2(right - 52.0, rect.center().y),
                    vec2(76.0, 28.0),
                ),
                egui::Button::new("Remove"),
            );
            remove.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    true,
                    format!("Remove {} from the list", project.name),
                )
            });
            if remove
                .on_hover_text("Forgets it here; nothing is deleted")
                .clicked()
            {
                changes.push(WorkbenchAction::ForgetProject(root.clone()));
            }
        } else {
            let star = ui.put(
                egui::Rect::from_center_size(
                    egui::pos2(right - 74.0, rect.center().y),
                    vec2(30.0, 30.0),
                ),
                egui::Button::new(
                    RichText::new(if project.pinned { "★" } else { "☆" })
                        .size(16.0)
                        .color(if project.pinned {
                            theme::WARNING
                        } else {
                            theme::text_weak()
                        }),
                )
                .frame(false),
            );
            let what = if project.pinned { "Unpin" } else { "Pin" };
            star.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    true,
                    format!("{what} {}", project.name),
                )
            });
            if star
                .on_hover_text(if project.pinned {
                    "Unpin: back among the others"
                } else {
                    "Pin: kept at the top of the list"
                })
                .clicked()
            {
                changes.push(WorkbenchAction::PinProject(root.clone(), !project.pinned));
            }
            let more_rect = egui::Rect::from_center_size(
                egui::pos2(right - 34.0, rect.center().y),
                vec2(30.0, 30.0),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(more_rect), |ui| {
                let menu = egui::containers::menu::MenuButton::from_button(
                    egui::Button::new(RichText::new("⋯").size(16.0).color(theme::text()))
                        .min_size(vec2(30.0, 30.0)),
                )
                .ui(ui, |ui| {
                    if ui.button("Open").clicked() {
                        asked.push(WorkbenchAction::OpenRecent(root.clone()));
                        ui.close();
                    }
                    if ui.button("Show in Explorer").clicked() {
                        asked.push(WorkbenchAction::ShowProject(root.clone()));
                        ui.close();
                    }
                    if ui
                        .button("Rename…")
                        .on_hover_text("Changes the project's name; its folder keeps its name")
                        .clicked()
                    {
                        self.launcher.renaming = Some(Renaming {
                            root: root.clone(),
                            name: project.name.clone(),
                            focused: false,
                        });
                        ui.close();
                    }
                    if ui
                        .button("Make a copy…")
                        .on_hover_text(
                            "Copies the whole folder to an empty one you choose (leaving out the \
                             import cache, which is made again)",
                        )
                        .clicked()
                    {
                        asked.push(WorkbenchAction::CopyProject(root.clone()));
                        ui.close();
                    }
                    ui.separator();
                    if ui
                        .button("Remove from list")
                        .on_hover_text("Forgets it here; its files stay where they are")
                        .clicked()
                    {
                        changes.push(WorkbenchAction::ForgetProject(root.clone()));
                        ui.close();
                    }
                });
                menu.0.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        true,
                        format!("More for {}", project.name),
                    )
                });
            });
        }
        if row.clicked() || row.secondary_clicked() {
            self.launcher.selected = Some(root.clone());
        }
        if row.double_clicked() && project.problem.is_none() {
            asked.push(WorkbenchAction::OpenRecent(root));
        }
        if let Some(problem) = &project.problem {
            row.on_hover_text(problem.clone());
        } else {
            row.on_hover_text("Double-click to open");
        }
    }

    /// The Rename… question, while it's asked: Enter or Rename gives the
    /// project its new name, Esc or Cancel leaves it.
    fn rename_question(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        let Some(renaming) = &mut self.launcher.renaming else {
            return;
        };
        let mut done = None;
        egui::Modal::new(egui::Id::new("rename project")).show(ui.ctx(), |ui| {
            ui.set_width(380.0);
            ui.label(
                RichText::new("Rename the project")
                    .size(16.0)
                    .strong()
                    .color(theme::text_strong()),
            );
            ui.label(
                RichText::new("Its name in the title bar and the list. Its folder keeps its name.")
                    .color(theme::text_weak()),
            );
            ui.add_space(8.0);
            let field = ui.add(
                egui::TextEdit::singleline(&mut renaming.name)
                    .desired_width(f32::INFINITY)
                    .margin(vec2(8.0, 6.0)),
            );
            if !renaming.focused {
                field.request_focus();
                renaming.focused = true;
            }
            let why = name_problem(&renaming.name);
            if let Some(why) = &why {
                ui.label(RichText::new(why).small().color(theme::WARNING));
            }
            let enter = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let ok = ui.add_enabled(why.is_none(), egui::Button::new("Rename"));
                if (ok.clicked() || enter) && why.is_none() {
                    done = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    done = Some(false);
                }
            });
        });
        match done {
            Some(true) => {
                if let Some(renaming) = self.launcher.renaming.take() {
                    asked.push(WorkbenchAction::RenameProject(
                        renaming.root,
                        renaming.name.trim().to_string(),
                    ));
                }
            }
            Some(false) => self.launcher.renaming = None,
            None => {}
        }
    }

    fn updates_page(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        use crate::updates::UpdateNotice as N;
        let fill = self.accent.colors().fill;
        let name = self.info.name.clone();
        ui.label(
            RichText::new("Updates")
                .size(24.0)
                .strong()
                .color(theme::text_strong()),
        );
        ui.add_space(16.0);
        let card = |fill: Color32, edge: Color32| {
            egui::Frame::new()
                .fill(fill)
                .stroke(egui::Stroke::new(1.0, edge))
                .corner_radius(10.0)
                .inner_margin(egui::Margin::symmetric(20, 16))
        };
        let small_heading = |ui: &mut Ui, text: &str, colour: Color32| {
            ui.label(RichText::new(text).small().strong().color(colour));
        };
        ui.horizontal_top(|ui| {
            card(theme::panel(), theme::card_border()).show(ui, |ui| {
                ui.set_width(260.0);
                ui.set_min_height(86.0);
                ui.vertical(|ui| {
                    small_heading(ui, "YOU HAVE", theme::text_weak());
                    let (big, under) = match self.this_build {
                        Some(build) => (
                            format!("Build {build}"),
                            format!("{name} {}", self.info.version),
                        ),
                        None => (
                            format!("{name} {}", self.info.version),
                            "Built from source: it doesn't update itself".to_string(),
                        ),
                    };
                    ui.label(
                        RichText::new(big)
                            .size(24.0)
                            .strong()
                            .color(theme::text_strong()),
                    );
                    ui.label(RichText::new(under).color(theme::text_weak()));
                });
            });
            ui.add_space(12.0);
            let newer = self.update.newer_build();
            let (back, edge) = if newer.is_some() {
                (theme::notice(), theme::selected_border())
            } else {
                (theme::panel(), theme::card_border())
            };
            card(back, edge).show(ui, |ui| {
                ui.set_width((ui.available_width() - 40.0).max(300.0));
                ui.set_min_height(86.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        let (heading, big, under) = newest_words(&self.update, &name);
                        let colour = if newer.is_some() {
                            Color32::from_rgb(0x9c, 0xc5, 0xf5)
                        } else {
                            theme::text_weak()
                        };
                        small_heading(ui, heading, colour);
                        let mut line = RichText::new(big)
                            .size(24.0)
                            .strong()
                            .color(theme::text_strong());
                        if newer.is_none() {
                            line = line.size(18.0);
                        }
                        ui.label(line);
                        if let Some(day) = self
                            .release_notes
                            .as_ref()
                            .filter(|n| Some(n.build) == newer)
                            .and_then(|n| n.published.clone())
                        {
                            ui.label(
                                RichText::new(format!("Published {day}"))
                                    .small()
                                    .color(theme::text_weak()),
                            );
                        }
                        ui.add(egui::Label::new(RichText::new(under).color(theme::text())).wrap());
                    });
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let big = |text: &str| {
                        egui::Button::new(RichText::new(text).color(theme::on_accent()).strong())
                            .fill(fill)
                            .min_size(vec2(0.0, 32.0))
                    };
                    let button = match &self.update {
                        N::Available(_) => Some(("Download", WorkbenchAction::DownloadUpdate)),
                        N::Ready(_) => Some(("Restart to update", WorkbenchAction::Quit)),
                        N::NotInstalled(_) => {
                            Some(("Open the releases page ↗", WorkbenchAction::OpenUpdatePage))
                        }
                        _ => None,
                    };
                    if let Some((text, action)) = button
                        && ui.add(big(text)).clicked()
                    {
                        // Restart to update: the window closes to install it.
                        self.restarting = action == WorkbenchAction::Quit;
                        asked.push(action);
                    }
                });
            });
        });
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            let busy = matches!(self.update, N::Checking | N::Downloading(_));
            if ui
                .add_enabled(
                    !busy,
                    egui::Button::new("Check now").min_size(vec2(0.0, 28.0)),
                )
                .on_hover_text("Asks GitHub for the newest build now")
                .clicked()
            {
                asked.push(WorkbenchAction::CheckForUpdates);
            }
            ui.add_space(12.0);
            let mut auto = self.auto_update;
            if ui
                .checkbox(&mut auto, format!("Look for updates when {name} starts"))
                .changed()
            {
                self.auto_update = auto;
                asked.push(WorkbenchAction::SetAutoUpdate(auto));
            }
        });
        ui.add_space(16.0);
        let notes = self
            .release_notes
            .clone()
            .filter(|n| Some(n.build) == self.update.newer_build());
        if let Some(notes) = notes {
            card(theme::panel(), theme::card_border()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new(format!("What's new in build {}", notes.build))
                        .size(17.0)
                        .strong()
                        .color(theme::text_strong()),
                );
                ui.add_space(6.0);
                let text = if notes.notes.is_empty() {
                    "Its release says nothing more: see the changes list.".to_string()
                } else {
                    notes.notes.clone()
                };
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        for line in text.lines() {
                            let line = line.trim_start_matches("> ");
                            ui.add(
                                egui::Label::new(RichText::new(line).color(theme::text())).wrap(),
                            );
                        }
                    });
                ui.add_space(8.0);
                if ui.link("Every change, build by build ↗").clicked() {
                    asked.push(WorkbenchAction::OpenUpdatePage);
                }
            });
            ui.add_space(12.0);
        }
        egui::Frame::new()
            .fill(Color32::from_rgb(0x2e, 0x28, 0x16))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(0x4a, 0x3d, 0x1c)))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new(
                        "Projects opened with an older build open in a newer one. Files laid \
                         out the old way are upgraded when saved, with a backup copy first, so \
                         you can go back.",
                    )
                    .color(Color32::from_rgb(0xe8, 0xc0, 0x6a)),
                );
            });
    }

    fn learn_page(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        ui.label(
            RichText::new("Learn")
                .size(24.0)
                .strong()
                .color(theme::text_strong()),
        );
        ui.label(
            RichText::new(
                "New to making games? Make an example, press Play, then change something and \
                 see what happens.",
            )
            .color(theme::text_weak()),
        );
        ui.add_space(20.0);
        ui.label(
            RichText::new("EXAMPLES TO OPEN AND TAKE APART")
                .small()
                .strong()
                .color(theme::text_weak()),
        );
        ui.add_space(6.0);
        let fill = self.accent.colors().fill;
        // The cards share the room across, up to their usual width.
        let all = examples();
        let gap = ui.spacing().item_spacing.x;
        let count = all.len() as f32;
        let card = ((ui.available_width() - gap * (count - 1.0)) / count - 34.0).clamp(150.0, CARD);
        ui.horizontal_wrapped(|ui| {
            for example in all {
                egui::Frame::new()
                    .fill(theme::panel())
                    .stroke(egui::Stroke::new(1.0, theme::card_border()))
                    .corner_radius(10.0)
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        ui.set_width(card);
                        ui.set_height(if card < CARD { 140.0 } else { 118.0 });
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(example.name)
                                    .size(15.0)
                                    .strong()
                                    .color(theme::text_strong()),
                            );
                            ui.add(
                                egui::Label::new(
                                    RichText::new(example.about).color(theme::text_weak()),
                                )
                                .wrap(),
                            );
                            ui.add_space(8.0);
                            let make = ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Make it in a folder…")
                                            .color(theme::on_accent())
                                            .strong(),
                                    )
                                    .fill(fill),
                                )
                                .on_hover_text("Made as a new project, in a folder you choose");
                            make.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    true,
                                    format!("Make the {}", example.name),
                                )
                            });
                            if make.clicked() {
                                asked.push(example.make);
                            }
                        });
                    });
                ui.add_space(8.0);
            }
        });
        ui.add_space(20.0);
        egui::Frame::new()
            .fill(theme::panel())
            .stroke(egui::Stroke::new(1.0, theme::card_border()))
            .corner_radius(10.0)
            .inner_margin(egui::Margin::symmetric(22, 18))
            .show(ui, |ui| {
                ui.set_width(3.0 * CARD + 2.0 * 40.0);
                ui.label(
                    RichText::new("Your first hour")
                        .size(16.0)
                        .strong()
                        .color(theme::text_strong()),
                );
                ui.add_space(6.0);
                for step in [
                    "1. Make a project: a folder for your game's files.",
                    "2. Put art in its assets folder: it shows in the Library.",
                    "3. Right-click in the Library to make maps, tilesets and code files.",
                    "4. Press ▶ Play to try it.",
                    "5. Stuck? Tap Space and type what you want to do.",
                ] {
                    ui.label(step);
                }
                ui.add_space(10.0);
                if !self.manual_url.is_empty() && ui.link("Open the manual ↗").clicked() {
                    ui.ctx()
                        .open_url(egui::OpenUrl::new_tab(self.manual_url.clone()));
                }
            });
    }
}

#[cfg(test)]
#[path = "launcher_tests.rs"]
mod tests;
