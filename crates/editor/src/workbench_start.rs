//! The start page (E1, decision 0070), as the project lead approved it
//! (mock-up e1-start): shown when the editor starts with no project open,
//! and from File > Start page. New project, Open project, the recent
//! projects, the three examples, four steps for someone new, and the
//! manual. Only what's built is offered.

use crate::theme;
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{RichText, Ui, vec2};

/// Whether the start page is up, and whether it comes up at launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StartPage {
    /// It's showing.
    pub(crate) open: bool,
    /// It shows when the editor starts with no project (saved per
    /// computer; "Don't show this at start" turns it off).
    pub(crate) at_launch: bool,
}

impl Default for StartPage {
    fn default() -> Self {
        Self {
            open: false,
            at_launch: true,
        }
    }
}

/// How wide the start page is, and each example's card.
const WIDTH: f32 = 690.0;
const CARD: f32 = 200.0;

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

impl Workbench {
    /// Whether the start page comes up when the editor starts with no
    /// project (for the app to save).
    pub fn start_page_at_launch(&self) -> bool {
        self.start.at_launch
    }

    /// Sets whether the start page comes up at launch (as saved), and
    /// shows it now if it should: no project is open.
    pub fn launch_start_page(&mut self, at_launch: bool) {
        self.start.at_launch = at_launch;
        self.start.open = at_launch && self.project.is_none();
    }

    /// Shows the start page (File > Start page).
    pub(crate) fn open_start_page(&mut self) {
        self.start.open = true;
    }

    /// Where the manual is, for the start page's link (the app knows it).
    pub fn set_manual_url(&mut self, url: String) {
        self.manual_url = url;
    }

    /// Draws the start page while it's open, over everything.
    pub(crate) fn start_page(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        if !self.start.open {
            return;
        }
        let mut asked: Vec<WorkbenchAction> = Vec::new();
        let mut close = false;
        let modal = egui::Modal::new(egui::Id::new("start page"))
            .backdrop_color(egui::Color32::from_black_alpha(220))
            .show(ui.ctx(), |ui| {
                ui.set_min_width(WIDTH);
                ui.set_max_width(WIDTH);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Welcome to {}", self.info.name))
                            .size(24.0)
                            .strong(),
                    );
                    ui.add_space((ui.available_width() - 24.0).max(0.0));
                    {
                        let x = ui
                            .add(egui::Button::new("×").frame(false))
                            .on_hover_text("Close, and carry on without a project (Esc)");
                        x.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                true,
                                "Close the start page",
                            )
                        });
                        close |= x.clicked();
                    }
                });
                ui.label(
                    RichText::new("Make a project to start, or open one you've made.")
                        .color(theme::TEXT_WEAK),
                );
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let fill = self.accent.colors().fill;
                    let new = egui::Button::new(
                        RichText::new("+ New project…")
                            .color(egui::Color32::WHITE)
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
                        .clicked()
                    {
                        asked.push(WorkbenchAction::OpenProject);
                    }
                });
                ui.add_space(14.0);
                ui.label(RichText::new("Recent projects").color(theme::TEXT_WEAK));
                if self.recent.is_empty() {
                    ui.label(
                        RichText::new("None yet: the projects you open are listed here.")
                            .small()
                            .color(theme::TEXT_WEAK),
                    );
                }
                for path in self.recent.iter().take(6) {
                    let name = path.file_name().map_or_else(
                        || path.display().to_string(),
                        |n| n.to_string_lossy().into_owned(),
                    );
                    let row = ui
                        .add(egui::Button::new(&name).frame(false))
                        .on_hover_text(path.display().to_string());
                    row.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            true,
                            format!("Open {name}"),
                        )
                    });
                    if row.clicked() {
                        asked.push(WorkbenchAction::OpenRecent(path.clone()));
                    }
                }
                ui.add_space(14.0);
                ui.label(RichText::new("Start from an example").color(theme::TEXT_WEAK));
                ui.horizontal(|ui| {
                    for example in examples() {
                        let card = egui::Frame::new()
                            .fill(theme::FIELD)
                            .corner_radius(6.0)
                            .inner_margin(10.0)
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.set_width(CARD);
                                    ui.set_height(64.0);
                                    ui.label(RichText::new(example.name).strong());
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(example.about)
                                                .small()
                                                .color(theme::TEXT_WEAK),
                                        )
                                        .wrap(),
                                    );
                                });
                            })
                            .response
                            .interact(egui::Sense::click())
                            .on_hover_text("Made as a new project, in a folder you choose");
                        card.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                true,
                                format!("Make the {}", example.name),
                            )
                        });
                        if card.clicked() {
                            asked.push(example.make);
                        }
                    }
                });
                ui.add_space(14.0);
                ui.label(RichText::new("New here?").color(theme::TEXT_WEAK));
                for step in [
                    "1. Make a project: a folder for your game's files.",
                    "2. Put art in its assets folder: it shows in the Library.",
                    "3. Right-click in the Library to make maps, tilesets and code files.",
                    "4. Press ▶ Play to try it.",
                ] {
                    ui.label(step);
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if !self.manual_url.is_empty() && ui.link("Open the manual ↗").clicked() {
                        ui.ctx()
                            .open_url(egui::OpenUrl::new_tab(self.manual_url.clone()));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut hide = !self.start.at_launch;
                        if ui.checkbox(&mut hide, "Don't show this at start").changed() {
                            self.start.at_launch = !hide;
                        }
                    });
                });
            });
        if modal.should_close() || close || !asked.is_empty() {
            self.start.open = false;
        }
        for action in asked {
            self.request(action, actions);
        }
    }
}

#[cfg(test)]
#[path = "workbench_start_tests.rs"]
mod tests;
