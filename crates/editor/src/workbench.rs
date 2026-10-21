//! The whole editor window: the top bar and the docked panels
//! ([decision 0004](../../../docs/decisions/0004-editor-layout.md)).
//!
//! The workbench never touches the window, the GPU or files itself: it
//! returns [`WorkbenchAction`]s for the app to carry out, and draws the
//! viewport through a [`ViewportRenderer`] it is handed each frame.

use crate::camera::Camera;
use crate::layout::{self, Preset};
use crate::panel::Panel;
use crate::theme::{self, Accent};
use crate::viewport::{Overlays, Tool, Viewport, ViewportRenderer};
use egui::{Align, Color32, Id, Layout, RichText, Ui, WidgetText};
use egui_dock::{DockArea, DockState, TabViewer};
use std::collections::HashSet;

/// Facts about the program, shown in the window title and the About box.
/// The app passes these in, so this crate never names the program itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    /// The program's name, such as "Solis".
    pub name: String,
    /// The version, such as "0.0.1".
    pub version: String,
}

/// Something the app must do, asked for by the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkbenchAction {
    /// Close the window.
    Quit,
}

/// How serious a console message is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    /// Something happened.
    Info,
    /// Something works, but not quite as asked.
    Warning,
    /// Something failed.
    Error,
}

impl LogLevel {
    /// Every level, in the console's filter order.
    pub const ALL: [Self; 3] = [Self::Info, Self::Warning, Self::Error];

    fn mark(self) -> &'static str {
        match self {
            Self::Info => "●",
            Self::Warning => "⚠",
            Self::Error => "✖",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Warning => "Warnings",
            Self::Error => "Errors",
        }
    }

    fn color(self) -> Color32 {
        match self {
            Self::Info => theme::TEXT,
            Self::Warning => theme::WARNING,
            Self::Error => theme::ERROR,
        }
    }
}

/// One console message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    /// How serious it is.
    pub level: LogLevel,
    /// What it says.
    pub text: String,
}

/// The console's messages and which levels are shown.
#[derive(Debug, Clone)]
struct Console {
    lines: Vec<LogLine>,
    hidden: HashSet<LogLevel>,
    search: String,
}

/// The most messages the console keeps; older ones are dropped.
const CONSOLE_LIMIT: usize = 2_000;

impl Console {
    fn push(&mut self, level: LogLevel, text: String) {
        self.lines.push(LogLine { level, text });
        if self.lines.len() > CONSOLE_LIMIT {
            let extra = self.lines.len() - CONSOLE_LIMIT;
            self.lines.drain(..extra);
        }
    }

    fn count(&self, level: LogLevel) -> usize {
        self.lines.iter().filter(|l| l.level == level).count()
    }

    fn shown(&self) -> impl Iterator<Item = &LogLine> {
        let search = self.search.to_lowercase();
        self.lines.iter().filter(move |l| {
            !self.hidden.contains(&l.level)
                && (search.is_empty() || l.text.to_lowercase().contains(&search))
        })
    }
}

/// The editor's state, kept between frames.
pub struct Workbench {
    info: AppInfo,
    dock: DockState<Panel>,
    accent: Accent,
    viewport: Viewport,
    console: Console,
    about_open: bool,
    /// The tabs that were in front last frame, to underline them in the
    /// accent colour.
    in_front: HashSet<Panel>,
}

impl Workbench {
    /// A workbench with a saved panel layout, if it is usable (otherwise the
    /// default one).
    pub fn new(info: AppInfo, saved_layout: Option<DockState<Panel>>) -> Self {
        Self {
            info,
            dock: layout::restore(saved_layout),
            accent: Accent::default(),
            viewport: Viewport::default(),
            console: Console {
                lines: Vec::new(),
                hidden: HashSet::new(),
                search: String::new(),
            },
            about_open: false,
            in_front: HashSet::new(),
        }
    }

    /// Uses `accent` (as saved last time).
    #[must_use]
    pub fn with_accent(mut self, accent: Accent) -> Self {
        self.accent = accent;
        self
    }

    /// Uses `overlays` (as saved last time).
    #[must_use]
    pub fn with_overlays(mut self, overlays: Overlays) -> Self {
        self.viewport.overlays = overlays;
        self
    }

    /// The panel layout, for saving.
    pub fn layout(&self) -> &DockState<Panel> {
        &self.dock
    }

    /// The accent colour, for saving.
    pub fn accent(&self) -> Accent {
        self.accent
    }

    /// Uses `camera` (as saved last time).
    #[must_use]
    pub fn with_camera(mut self, camera: Camera) -> Self {
        self.viewport.camera = camera;
        self
    }

    /// The viewport camera, for saving.
    pub fn camera(&self) -> Camera {
        self.viewport.camera
    }

    /// Which viewport overlays are shown, for saving.
    pub fn overlays(&self) -> Overlays {
        self.viewport.overlays
    }

    /// The chosen tool.
    pub fn tool(&self) -> Tool {
        self.viewport.tool
    }

    /// The console's messages, oldest first.
    pub fn log_lines(&self) -> &[LogLine] {
        &self.console.lines
    }

    /// Adds a message to the console.
    pub fn log(&mut self, level: LogLevel, text: impl Into<String>) {
        self.console.push(level, text.into());
    }

    /// Arranges the panels as `preset`.
    pub fn apply_preset(&mut self, preset: Preset) {
        self.dock = preset.layout();
    }

    /// The window title: the program, its version and the open scene.
    pub fn title(&self) -> String {
        format!("{} {} · Untitled scene", self.info.name, self.info.version)
    }

    /// Draws the editor filling `ui`, and returns what the app must do.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        renderer: &mut dyn ViewportRenderer,
    ) -> Vec<WorkbenchAction> {
        let mut actions = Vec::new();
        self.tool_keys(ui);
        egui::Panel::top("top_bar")
            .frame(theme::top_bar_frame())
            .exact_size(34.0)
            .show(ui, |ui| self.top_bar(ui, &mut actions));
        let style = theme::dock_style(ui.style().as_ref(), self.accent);
        let mut shown = HashSet::new();
        theme::dock_frame().show(ui, |ui| {
            DockArea::new(&mut self.dock)
                .style(style)
                .tab_context_menus(false)
                .show_close_buttons(false)
                .show_add_buttons(false)
                .show_leaf_close_all_buttons(false)
                .show_leaf_collapse_buttons(true)
                .show_inside(
                    ui,
                    &mut PanelViewer {
                        viewport: &mut self.viewport,
                        renderer,
                        console: &mut self.console,
                        accent: self.accent,
                        in_front: &self.in_front,
                        shown: &mut shown,
                    },
                );
        });
        self.in_front = shown;
        self.about(ui);
        actions
    }

    /// Q, W, R, S and T pick the tools, unless typing in a field.
    fn tool_keys(&mut self, ui: &Ui) {
        // While flying, W A S D, Q and E move the camera instead.
        if ui.ctx().egui_wants_keyboard_input() || self.viewport.is_flying() {
            return;
        }
        let pressed = ui.input(|i| {
            Tool::ALL
                .into_iter()
                .find(|tool| i.key_pressed(tool.key()) && i.modifiers.is_none())
        });
        if let Some(tool) = pressed {
            self.viewport.tool = tool;
        }
    }

    fn top_bar(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        // The right-hand buttons go first, so the menu bar (which takes all
        // the width it is given) only gets what is left.
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            self.play_and_build(ui);
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    self.menus(ui, actions);
                    ui.add_space(8.0);
                    document_tabs(ui, self.accent);
                });
            });
        });
    }

    fn menus(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        ui.menu_button("File", |ui| {
            for item in ["New scene", "Open…", "Save"] {
                ui.add_enabled(false, egui::Button::new(item))
                    .on_disabled_hover_text("Scenes arrive in Phase 2");
            }
            ui.separator();
            if ui.button("Quit").clicked() {
                actions.push(WorkbenchAction::Quit);
            }
        });
        ui.menu_button("Edit", |ui| {
            for item in ["Undo", "Redo"] {
                ui.add_enabled(false, egui::Button::new(item))
                    .on_disabled_hover_text("Undo arrives with editing in Phase 2");
            }
        });
        ui.menu_button("View", |ui| {
            let overlays = &mut self.viewport.overlays;
            ui.checkbox(&mut overlays.tools, "Viewport tools");
            ui.checkbox(&mut overlays.snapping, "Grid, angle and camera speed");
            ui.checkbox(&mut overlays.stats, "Frame rate and entity count");
        });
        ui.menu_button("Entity", |ui| {
            for item in ["Add entity", "Add light", "Add camera"] {
                ui.add_enabled(false, egui::Button::new(item))
                    .on_disabled_hover_text("Entities arrive in Phase 2");
            }
        });
        ui.menu_button("Build", |ui| {
            for item in ["Build", "Build & Play"] {
                ui.add_enabled(false, egui::Button::new(item))
                    .on_disabled_hover_text("Building games arrives in Phase 10");
            }
        });
        ui.menu_button("Window", |ui| {
            ui.menu_button("Layouts", |ui| {
                for preset in Preset::ALL {
                    if ui.button(preset.name()).clicked() {
                        self.apply_preset(preset);
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button("Reset layout").clicked() {
                    self.apply_preset(Preset::Default);
                    ui.close();
                }
            });
            ui.menu_button("Accent colour", |ui| {
                for accent in Accent::ALL {
                    if ui
                        .add(egui::Button::new(accent.name()).selected(self.accent == accent))
                        .clicked()
                    {
                        self.accent = accent;
                        theme::set_accent(ui.ctx(), accent);
                        ui.close();
                    }
                }
            });
        });
        ui.menu_button("Help", |ui| {
            if ui.button(format!("About {}", self.info.name)).clicked() {
                self.about_open = true;
                ui.close();
            }
        });
    }

    /// Play, Stop, Build and Build & Play, right to left.
    fn play_and_build(&mut self, ui: &mut Ui) {
        let fill = self.accent.colors().fill;
        if ui
            .add(egui::Button::new(RichText::new("Build & Play").color(Color32::WHITE)).fill(fill))
            .clicked()
        {
            self.log(
                LogLevel::Info,
                "Building games arrives in Phase 10 (ROADMAP.md).",
            );
        }
        if ui.button("Build ▾").clicked() {
            self.log(
                LogLevel::Info,
                "Building games arrives in Phase 10 (ROADMAP.md).",
            );
        }
        ui.add_space(6.0);
        if ui.button("■").on_hover_text("Stop").clicked() {
            self.log(LogLevel::Info, "Nothing is running.");
        }
        if ui
            .add(egui::Button::new(RichText::new("▶").color(Color32::WHITE)).fill(fill))
            .on_hover_text("Play")
            .clicked()
        {
            self.log(
                LogLevel::Info,
                "Playing the scene arrives in Phase 5 (ROADMAP.md).",
            );
        }
    }

    fn about(&mut self, ui: &mut Ui) {
        let mut open = self.about_open;
        egui::Window::new(format!("About {}", self.info.name))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(&self.info.name);
                    ui.label(format!("Version {}", self.info.version));
                    ui.add_space(8.0);
                    ui.label("A free, open-source 3D game engine.");
                    ui.label("Free software under the MIT licence.");
                });
            });
        self.about_open = open;
    }
}

/// The open documents as tabs, the front one underlined in the accent.
/// For now there is only the one scene; opening more arrives in Phase 2.
fn document_tabs(ui: &mut Ui, accent: Accent) {
    let response = ui.add(
        egui::Label::new(RichText::new("Untitled scene").color(theme::TEXT_STRONG))
            .selectable(false),
    );
    let rect = response.rect.expand2(egui::vec2(10.0, 0.0));
    ui.painter().hline(
        rect.x_range(),
        ui.max_rect().bottom() - 1.0,
        egui::Stroke::new(2.0, accent.colors().light),
    );
    ui.add_enabled(false, egui::Button::new("+").frame(false))
        .on_disabled_hover_text("Opening more scenes, prefabs and code arrives in Phase 2");
}

/// Draws each docked panel.
struct PanelViewer<'a> {
    viewport: &'a mut Viewport,
    renderer: &'a mut dyn ViewportRenderer,
    console: &'a mut Console,
    accent: Accent,
    in_front: &'a HashSet<Panel>,
    shown: &'a mut HashSet<Panel>,
}

impl TabViewer for PanelViewer<'_> {
    type Tab = Panel;

    fn id(&mut self, tab: &mut Panel) -> Id {
        Id::new(("panel", *tab))
    }

    fn title(&mut self, tab: &mut Panel) -> WidgetText {
        tab.title().into()
    }

    fn on_tab_button(&mut self, tab: &mut Panel, response: &egui::Response) {
        // Name the tab for screen readers (and tests).
        let in_front = self.in_front.contains(tab);
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, in_front, tab.title())
        });
        // The tab in front gets Aventail's underline in the accent colour.
        if self.in_front.contains(tab) {
            let painter = response.ctx.layer_painter(response.layer_id);
            painter.hline(
                response.rect.x_range(),
                response.rect.bottom() - 1.0,
                egui::Stroke::new(2.0, self.accent.colors().light),
            );
        }
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Panel) {
        self.shown.insert(*tab);
        if *tab == Panel::Viewport {
            let full = ui.max_rect();
            let mut view_ui = ui.new_child(egui::UiBuilder::new().max_rect(full));
            self.viewport.show(&mut view_ui, self.renderer, self.accent);
            ui.advance_cursor_after_rect(full);
            return;
        }
        match tab {
            Panel::Scene => {
                menu_row(ui, &["Edit", "View", "Entity"]);
                placeholder(ui, "No entities yet. They arrive in Phase 2.");
            }
            Panel::Layers => {
                menu_row(ui, &["Edit", "View"]);
                placeholder(ui, "Layers arrive with scenes in Phase 2.");
            }
            Panel::Properties => {
                menu_row(ui, &["Mode", "Edit", "Units"]);
                placeholder(ui, "Nothing selected.");
            }
            Panel::Library => {
                menu_row(ui, &["Import", "View"]);
                placeholder(
                    ui,
                    "The library arrives with the asset database in Phase 2.",
                );
            }
            Panel::Game => placeholder(ui, "Press Play to run the game here (Phase 5)."),
            Panel::Console => console(ui, self.console, self.accent),
            Panel::Viewport => {}
        }
    }
}

/// The small menu row under a panel's title: "≡ Edit View …".
fn menu_row(ui: &mut Ui, items: &[&str]) {
    theme::menu_row_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            ui.label("≡");
            for item in items {
                ui.label(*item);
            }
        });
    });
}

/// A quiet note in a panel that has nothing to show yet.
fn placeholder(ui: &mut Ui, text: &str) {
    egui::Frame::new()
        .inner_margin(theme::panel_margin())
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(theme::TEXT_WEAK));
        });
}

/// The console: filters by level, a search box, and the messages.
fn console(ui: &mut Ui, console: &mut Console, accent: Accent) {
    theme::menu_row_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            for level in LogLevel::ALL {
                let on = !console.hidden.contains(&level);
                let text = format!("{} {} {}", level.mark(), level.name(), console.count(level));
                let button = if on {
                    egui::Button::new(RichText::new(text).color(Color32::WHITE))
                        .fill(accent.colors().fill)
                } else {
                    egui::Button::new(text)
                };
                if ui.add(button).clicked() {
                    if on {
                        console.hidden.insert(level);
                    } else {
                        console.hidden.remove(&level);
                    }
                }
            }
            // Clear at the right edge, and the search box shrinks to fit
            // whatever width is left.
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Clear").clicked() {
                    console.lines.clear();
                }
                ui.add(
                    egui::TextEdit::singleline(&mut console.search)
                        .hint_text("Search")
                        .desired_width(ui.available_width().clamp(40.0, 200.0)),
                );
            });
        });
    });
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .stick_to_bottom(true)
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(theme::panel_margin())
                .show(ui, |ui| {
                    for line in console.shown() {
                        ui.label(
                            RichText::new(format!("{} {}", line.level.mark(), line.text))
                                .monospace()
                                .color(line.level.color()),
                        );
                    }
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewport::NoRenderer;
    use egui_kittest::kittest::Queryable;

    fn info() -> AppInfo {
        AppInfo {
            name: "Test Engine".into(),
            version: "9.9.9".into(),
        }
    }

    #[test]
    fn the_title_names_program_version_and_scene() {
        let w = Workbench::new(info(), None);
        assert_eq!(w.title(), "Test Engine 9.9.9 · Untitled scene");
    }

    #[test]
    fn a_new_workbench_uses_the_default_layout() {
        let w = Workbench::new(info(), None);
        assert!(layout::is_complete(w.layout()));
        assert_eq!(w.tool(), Tool::Select);
        assert_eq!(w.accent(), Accent::PetrolBlue);
    }

    #[test]
    fn the_console_keeps_a_limited_number_of_messages() {
        let mut w = Workbench::new(info(), None);
        for i in 0..(CONSOLE_LIMIT + 10) {
            w.log(LogLevel::Info, format!("line {i}"));
        }
        assert_eq!(w.log_lines().len(), CONSOLE_LIMIT);
        assert_eq!(w.log_lines()[0].text, "line 10");
    }

    #[test]
    fn the_console_filters_by_level_and_search() {
        let mut c = Console {
            lines: Vec::new(),
            hidden: HashSet::new(),
            search: String::new(),
        };
        c.push(LogLevel::Info, "Graphics: Vulkan".into());
        c.push(LogLevel::Warning, "pairs() order".into());
        c.push(LogLevel::Error, "Shader failed".into());
        assert_eq!(c.shown().count(), 3);
        c.hidden.insert(LogLevel::Info);
        assert_eq!(c.shown().count(), 2);
        c.search = "SHADER".into();
        assert_eq!(c.shown().count(), 1);
        assert_eq!(c.count(LogLevel::Warning), 1);
    }

    #[test]
    fn presets_rearrange_the_panels() {
        let mut w = Workbench::new(info(), None);
        w.apply_preset(Preset::SmallScreen);
        let path = w.layout().find_tab(&Panel::Console).unwrap();
        let group = w.layout()[path.surface][path.node].tabs().unwrap().to_vec();
        assert_eq!(group, [Panel::Library, Panel::Console]);
    }

    /// The whole editor in a test window, with no GPU behind the viewport.
    fn harness() -> egui_kittest::Harness<'static, Workbench> {
        let mut renderer = NoRenderer {
            reason: "testing".into(),
        };
        let mut themed = false;
        egui_kittest::Harness::builder()
            .with_size(egui::vec2(1440.0, 900.0))
            .build_ui_state(
                move |ui, workbench: &mut Workbench| {
                    if !themed {
                        theme::apply(ui.ctx(), workbench.accent());
                        themed = true;
                    }
                    let _ = workbench.show(ui, &mut renderer);
                },
                Workbench::new(info(), None),
            )
    }

    #[test]
    fn the_editor_draws_every_panel_without_a_gpu() {
        let mut h = harness();
        h.run();
        for label in ["Viewport", "Scene", "Properties", "Console", "Library"] {
            let _ = h.get_by_label(label);
        }
        let _ = h.get_by_label("Untitled scene");
        let _ = h.get_by_label("No 3D view: testing");
    }

    #[test]
    fn tool_keys_pick_tools() {
        let mut h = harness();
        h.run();
        h.key_press(egui::Key::W);
        h.run();
        assert_eq!(h.state().tool(), Tool::Move);
        h.key_press(egui::Key::T);
        h.run();
        assert_eq!(h.state().tool(), Tool::Transform);
    }

    #[test]
    fn clicking_a_tool_button_picks_it() {
        let mut h = harness();
        h.run();
        h.get_by_label("Rotate (R)").click();
        h.run();
        assert_eq!(h.state().tool(), Tool::Rotate);
    }

    #[test]
    fn the_viewport_menu_hides_the_tool_bar() {
        let mut h = harness();
        h.run();
        h.get_by_role_and_label(egui::accesskit::Role::Button, "≡")
            .click();
        h.run();
        h.get_by_label("Tools").click();
        h.run();
        assert!(!h.state().overlays().tools);
        assert!(h.query_by_label("Move (W)").is_none());
    }

    #[test]
    fn play_says_when_it_arrives() {
        let mut h = harness();
        h.run();
        h.get_by_label("▶").click();
        h.run();
        let last = h.state().log_lines().last().map(|l| l.text.clone());
        assert_eq!(
            last.as_deref(),
            Some("Playing the scene arrives in Phase 5 (ROADMAP.md).")
        );
    }

    /// A point in the middle of the viewport, clear of its corner controls.
    const IN_VIEW: egui::Pos2 = egui::pos2(500.0, 300.0);

    fn button(
        h: &egui_kittest::Harness<'_, Workbench>,
        button: egui::PointerButton,
        pressed: bool,
        at: egui::Pos2,
    ) {
        h.event(egui::Event::PointerButton {
            pos: at,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }

    #[test]
    fn holding_right_mouse_and_w_flies_forward_without_changing_tool() {
        let mut h = harness();
        h.hover_at(IN_VIEW);
        h.run();
        let start = h.state().camera();
        button(&h, egui::PointerButton::Secondary, true, IN_VIEW);
        h.run();
        assert!(h.state().viewport.is_flying());
        h.key_down(egui::Key::W);
        h.run_steps(5);
        h.key_up(egui::Key::W);
        button(&h, egui::PointerButton::Secondary, false, IN_VIEW);
        h.run();
        assert!(!h.state().viewport.is_flying());
        let moved = h.state().camera().position - start.position;
        assert!(moved.dot(start.forward()) > 0.0, "moved {moved}");
        assert_eq!(
            h.state().tool(),
            Tool::Select,
            "W flew instead of picking Move"
        );
    }

    #[test]
    fn right_dragging_looks_around() {
        let mut h = harness();
        h.hover_at(IN_VIEW);
        h.run();
        let start = h.state().camera();
        button(&h, egui::PointerButton::Secondary, true, IN_VIEW);
        h.run();
        h.event(egui::Event::PointerMoved(IN_VIEW + egui::vec2(80.0, 0.0)));
        h.run();
        button(&h, egui::PointerButton::Secondary, false, IN_VIEW);
        h.run();
        assert!(h.state().camera().heading > start.heading);
        assert_eq!(h.state().camera().position, start.position);
    }

    #[test]
    fn the_wheel_changes_speed_while_flying_and_moves_otherwise() {
        let wheel = |h: &egui_kittest::Harness<'_, Workbench>| {
            h.event(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, 1.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            });
        };
        let mut h = harness();
        h.hover_at(IN_VIEW);
        h.run();
        let start = h.state().camera();
        wheel(&h);
        h.run();
        let after_wheel = h.state().camera();
        assert!((after_wheel.position - start.position).dot(start.forward()) > 0.0);
        assert_eq!(after_wheel.speed, start.speed);

        button(&h, egui::PointerButton::Secondary, true, IN_VIEW);
        h.run();
        wheel(&h);
        h.run();
        button(&h, egui::PointerButton::Secondary, false, IN_VIEW);
        h.run();
        assert_eq!(h.state().camera().speed, start.speed + 1);
        assert_eq!(h.state().camera().position, after_wheel.position);
        let _ = h.get_by_label(&(start.speed + 1).to_string());
    }

    #[test]
    fn middle_dragging_slides_the_camera() {
        let mut h = harness();
        h.hover_at(IN_VIEW);
        h.run();
        let start = h.state().camera();
        button(&h, egui::PointerButton::Middle, true, IN_VIEW);
        h.run();
        for step in 1..=5 {
            h.event(egui::Event::PointerMoved(
                IN_VIEW + egui::vec2(step as f32 * 20.0, 0.0),
            ));
            h.run();
        }
        button(
            &h,
            egui::PointerButton::Middle,
            false,
            IN_VIEW + egui::vec2(100.0, 0.0),
        );
        h.run();
        let moved = h.state().camera().position - start.position;
        assert!(moved.dot(start.right()) < 0.0, "moved {moved}");
        assert_eq!(h.state().camera().heading, start.heading);
    }

    #[test]
    fn tool_keys_still_work_after_flying() {
        let mut h = harness();
        h.hover_at(IN_VIEW);
        h.run();
        button(&h, egui::PointerButton::Secondary, true, IN_VIEW);
        h.run();
        button(&h, egui::PointerButton::Secondary, false, IN_VIEW);
        h.run();
        h.key_press(egui::Key::R);
        h.run();
        assert_eq!(h.state().tool(), Tool::Rotate);
    }
}
