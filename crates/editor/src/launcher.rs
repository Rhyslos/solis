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
    /// It shows when the editor starts with no project (saved per
    /// computer).
    pub(crate) at_launch: bool,
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
}

impl Default for Launcher {
    fn default() -> Self {
        Self {
            open: false,
            at_launch: true,
            page: Page::Projects,
            search: String::new(),
            sort: Sort::LastOpened,
            selected: None,
            focus_search: true,
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

fn examples() -> [Example; 3] {
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
                theme::TEXT,
                theme::FIELD,
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

/// Seconds since 1970, now.
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl Workbench {
    /// Whether the launcher comes up when the editor starts with no
    /// project (for the app to save).
    pub fn launcher_at_launch(&self) -> bool {
        self.launcher.at_launch
    }

    /// Sets whether the launcher comes up at launch (as saved), and shows
    /// it now if it should: no project is open.
    pub fn launch_launcher(&mut self, at_launch: bool) {
        self.launcher.at_launch = at_launch;
        if at_launch && self.project.is_none() {
            self.open_launcher();
        }
    }

    /// Whether the launcher is up.
    pub fn launcher_open(&self) -> bool {
        self.launcher.open
    }

    /// Shows the launcher (File > Projects…), asking the app for the list
    /// read afresh. A game being played stops first.
    pub(crate) fn open_launcher(&mut self) {
        if self.playing.is_some() {
            self.stop();
        }
        self.launcher.open = true;
        self.launcher.focus_search = true;
        self.outbox.push(WorkbenchAction::ListProjects);
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
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) && self.unsaved_prompt.is_none() {
            self.launcher.open = false;
        }
        egui::Panel::left("launcher sidebar")
            .exact_size(SIDEBAR)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::HEADER)
                    .inner_margin(egui::Margin::symmetric(14, 22)),
            )
            .show(ui, |ui| self.launcher_sidebar(ui));
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BACKGROUND)
                    .inner_margin(egui::Margin::symmetric(36, 28)),
            )
            .show(ui, |ui| match self.launcher.page {
                Page::Projects => self.projects_page(ui, &mut asked),
                Page::Learn => self.learn_page(ui, &mut asked),
            });
        for action in asked {
            if let WorkbenchAction::OpenRecent(root) = &action
                && self.project.as_ref().is_some_and(|p| &p.root == root)
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
                    .color(theme::TEXT_STRONG),
            );
        });
        ui.add_space(22.0);
        for (page, name, icon) in [
            (Page::Projects, "Projects", crate::icons::Icon::Library),
            (Page::Learn, "Learn", crate::icons::Icon::Info),
        ] {
            let on = self.launcher.page == page;
            let (rect, item) =
                ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
            let painter = ui.painter();
            if on {
                painter.rect_filled(rect, 6.0, Color32::from_rgb(0x2c, 0x3a, 0x4d));
            } else if item.hovered() {
                painter.rect_filled(rect, 6.0, theme::MENU_ROW);
            }
            let colour = if on { theme::TEXT_STRONG } else { theme::TEXT };
            crate::icons::icon(
                painter,
                egui::Rect::from_center_size(
                    rect.left_center() + vec2(20.0, 0.0),
                    vec2(14.0, 14.0),
                ),
                icon,
                if on {
                    self.accent.colors().fill
                } else {
                    theme::TEXT_WEAK
                },
            );
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
            ui.label(RichText::new(build).small().color(theme::TEXT_WEAK));
            ui.add_space(6.0);
            let back = match &self.project {
                Some(p) => format!("← Back to {}", p.name),
                None => "Carry on without a project".to_string(),
            };
            let close = ui
                .add(egui::Button::new(RichText::new(back).color(theme::TEXT_WEAK)).frame(false))
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
                    .color(theme::TEXT_STRONG),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let new = egui::Button::new(
                    RichText::new("+ New project")
                        .color(Color32::WHITE)
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
            });
        });
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
                ui.label(RichText::new("Sort by").small().color(theme::TEXT_WEAK));
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
                        .color(theme::TEXT_STRONG),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(
                        "Make one with + New project, open a folder with Open project…, or try \
                         an example under Learn.",
                    )
                    .color(theme::TEXT_WEAK),
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
                .color(theme::TEXT_WEAK),
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
                                .color(theme::TEXT_WEAK),
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
                    .color(theme::TEXT_WEAK),
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
            (
                Color32::from_rgb(0x2c, 0x3a, 0x4d),
                Color32::from_rgb(0x3f, 0x5a, 0x7a),
            )
        } else if row.hovered() {
            (theme::MENU_ROW, theme::BUTTON_BORDER)
        } else {
            (theme::PANEL, Color32::from_rgb(0x30, 0x30, 0x30))
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
            (theme::FIELD, theme::TEXT_WEAK)
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
            theme::TEXT_WEAK
        } else {
            theme::TEXT_STRONG
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
                theme::TEXT_WEAK,
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
                theme::TEXT_WEAK,
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
                theme::TEXT,
            );
        }
        // The pin and ⋯, or Remove for a project that can't be opened.
        let root = project.root.clone();
        if project.problem.is_some() {
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
                            theme::TEXT_WEAK
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
                    egui::Button::new(RichText::new("⋯").size(16.0).color(theme::TEXT))
                        .min_size(vec2(30.0, 30.0)),
                )
                .ui(ui, |ui| {
                    if ui.button("Open").clicked() {
                        asked.push(WorkbenchAction::OpenRecent(root.clone()));
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

    fn learn_page(&mut self, ui: &mut Ui, asked: &mut Vec<WorkbenchAction>) {
        ui.label(
            RichText::new("Learn")
                .size(24.0)
                .strong()
                .color(theme::TEXT_STRONG),
        );
        ui.label(
            RichText::new(
                "New to making games? Make an example, press Play, then change something and \
                 see what happens.",
            )
            .color(theme::TEXT_WEAK),
        );
        ui.add_space(20.0);
        ui.label(
            RichText::new("EXAMPLES TO OPEN AND TAKE APART")
                .small()
                .strong()
                .color(theme::TEXT_WEAK),
        );
        ui.add_space(6.0);
        let fill = self.accent.colors().fill;
        ui.horizontal(|ui| {
            for example in examples() {
                egui::Frame::new()
                    .fill(theme::PANEL)
                    .stroke(egui::Stroke::new(1.0, Color32::from_rgb(0x30, 0x30, 0x30)))
                    .corner_radius(10.0)
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        ui.set_width(CARD);
                        ui.set_height(118.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(example.name)
                                    .size(15.0)
                                    .strong()
                                    .color(theme::TEXT_STRONG),
                            );
                            ui.add(
                                egui::Label::new(
                                    RichText::new(example.about).color(theme::TEXT_WEAK),
                                )
                                .wrap(),
                            );
                            ui.add_space(8.0);
                            let make = ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Make it in a folder…")
                                            .color(Color32::WHITE)
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
            .fill(theme::PANEL)
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(0x30, 0x30, 0x30)))
            .corner_radius(10.0)
            .inner_margin(egui::Margin::symmetric(22, 18))
            .show(ui, |ui| {
                ui.set_width(3.0 * CARD + 2.0 * 40.0);
                ui.label(
                    RichText::new("Your first hour")
                        .size(16.0)
                        .strong()
                        .color(theme::TEXT_STRONG),
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
        ui.add_space(16.0);
        let mut show = self.launcher.at_launch;
        if ui
            .checkbox(
                &mut show,
                format!(
                    "Show this page when {} starts without a project",
                    self.info.name
                ),
            )
            .changed()
        {
            self.launcher.at_launch = show;
        }
    }
}

#[cfg(test)]
#[path = "launcher_tests.rs"]
mod tests;
