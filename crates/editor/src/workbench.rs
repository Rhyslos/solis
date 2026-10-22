//! The whole editor window: the top bar and the docked panels
//! ([decision 0004](../../../docs/decisions/0004-editor-layout.md)).
//!
//! The workbench never touches the window, the GPU or files itself: it
//! returns [`WorkbenchAction`]s for the app to carry out, and draws the
//! viewport through a [`ViewportRenderer`] it is handed each frame.

use crate::camera::Camera;
#[cfg(test)]
use crate::console::CONSOLE_LIMIT;
use crate::console::{Console, LogLevel, LogLine};
use crate::environment::EnvironmentSettings;
use crate::layout::{self, Preset};
use crate::lighting::{Lamp, LampFromFile, LampId, Selection, SunSettings};
use crate::models::{ModelId, ModelInfo};
use crate::panel::Panel;
use crate::panels::{PanelViewer, drop_hint};
use crate::theme::{self, Accent};
use crate::viewport::{Overlays, Tool, Viewport, ViewportRenderer};
use egui::Ui;
use egui_dock::{DockArea, DockState};
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
    /// Ask which model file to import (File > Import model…).
    ImportModel,
    /// Bring in the built-in sample scene (Help > Add the sample scene).
    AddSampleScene,
    /// Take an imported model out of the viewport and free it.
    RemoveModel(ModelId),
}

/// The editor's state, kept between frames.
pub struct Workbench {
    pub(crate) info: AppInfo,
    pub(crate) dock: DockState<Panel>,
    pub(crate) accent: Accent,
    pub(crate) viewport: Viewport,
    pub(crate) console: Console,
    /// The imported models, in the order they arrived.
    pub(crate) models: Vec<ModelInfo>,
    /// What's selected in the Scene panel.
    pub(crate) selection: Selection,
    /// The number the next lamp gets.
    pub(crate) next_lamp: u64,
    /// Whether files are being dragged over the window.
    pub(crate) files_hovering: bool,
    pub(crate) about_open: bool,
    /// The tabs that were in front last frame, to underline them in the
    /// accent colour.
    pub(crate) in_front: HashSet<Panel>,
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
            console: Console::default(),
            models: Vec::new(),
            selection: Selection::Nothing,
            next_lamp: 1,
            files_hovering: false,
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

    /// Adds an imported model to the viewport, frames the camera on it,
    /// and says so in the console with its size and any `warnings` (parts
    /// of the file that were left out).
    pub fn add_model(&mut self, info: ModelInfo, warnings: &[String], seconds: f32) {
        self.log(
            LogLevel::Info,
            format!(
                "Imported {}: {} triangles, {}, in {seconds:.1} s.",
                info.name,
                crate::models::grouped(info.triangles),
                info.size_text()
            ),
        );
        for warning in warnings {
            self.log(LogLevel::Warning, format!("{}: {warning}", info.name));
        }
        self.viewport.camera.frame(info.min, info.max);
        self.models.retain(|m| m.id != info.id);
        self.models.push(info);
        self.sync_models();
    }

    /// Tells the workbench whether files are being dragged over the window,
    /// so it can show where to drop them.
    pub fn set_files_hovering(&mut self, hovering: bool) {
        self.files_hovering = hovering;
    }

    /// The imported models, in the order they arrived.
    pub fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    /// Takes a model out of the list, with any lamps that came in with it
    /// (the app frees it on the GPU).
    pub(crate) fn remove_model(&mut self, id: ModelId) {
        if let Some(i) = self.models.iter().position(|m| m.id == id) {
            let gone = self.models.remove(i);
            self.log(LogLevel::Info, format!("Removed {}.", gone.name));
        }
        self.viewport.lamps.retain(|l| l.from_model != Some(id));
        if self.selection == Selection::Model(id) || !self.selection_exists() {
            self.selection = Selection::Nothing;
        }
        self.sync_models();
    }

    /// Whether what's selected is still there.
    pub(crate) fn selection_exists(&self) -> bool {
        match self.selection {
            Selection::Nothing | Selection::Sun | Selection::Environment => true,
            Selection::Lamp(id) => self.viewport.lamps.iter().any(|l| l.id == id),
            Selection::Model(id) => self.models.iter().any(|m| m.id == id),
        }
    }

    /// Adds the lamps a model's file carries, removed along with the model.
    pub fn add_model_lamps(&mut self, model: ModelId, lamps: &[LampFromFile]) {
        for lamp in lamps {
            let id = self.new_lamp_id();
            self.viewport.lamps.push(Lamp::from_file(id, lamp, model));
        }
        if !lamps.is_empty() {
            let s = if lamps.len() == 1 { "" } else { "s" };
            self.log(
                LogLevel::Info,
                format!("The file brought {} light{s} with it.", lamps.len()),
            );
        }
    }

    pub(crate) fn new_lamp_id(&mut self) -> LampId {
        let id = LampId(self.next_lamp);
        self.next_lamp += 1;
        id
    }

    /// Adds a point light above where the camera looks, and selects it.
    pub fn add_point_light(&mut self) {
        let id = self.new_lamp_id();
        let lamp = Lamp::point(id, self.viewport.camera.focus());
        self.log(LogLevel::Info, format!("Added {}.", lamp.name));
        self.viewport.lamps.push(lamp);
        self.selection = Selection::Lamp(id);
    }

    /// Adds a spotlight at the camera, shining where it looks, and selects
    /// it.
    pub fn add_spotlight(&mut self) {
        let id = self.new_lamp_id();
        let camera = &self.viewport.camera;
        let lamp = Lamp::spot(id, camera.position, camera.forward());
        self.log(LogLevel::Info, format!("Added {}.", lamp.name));
        self.viewport.lamps.push(lamp);
        self.selection = Selection::Lamp(id);
    }

    /// The lamps, in the order they were added.
    pub fn lamps(&self) -> &[Lamp] {
        &self.viewport.lamps
    }

    /// What's selected in the Scene panel.
    pub fn selection(&self) -> Selection {
        self.selection
    }

    /// Selects `selection` (as clicking it in the Scene panel does).
    pub fn select(&mut self, selection: Selection) {
        self.selection = selection;
        if !self.selection_exists() {
            self.selection = Selection::Nothing;
        }
    }

    /// Uses `sun` (as saved last time), with anything out of range put
    /// back in range.
    #[must_use]
    pub fn with_sun(mut self, sun: SunSettings) -> Self {
        self.viewport.sun = sun.sanitized();
        self
    }

    /// Sets the sun from one a model file carries: the way its light
    /// travels, its linear colour and its strength in W/m².
    pub fn use_sun_from_file(
        &mut self,
        name: &str,
        direction: glam::Vec3,
        color: [f32; 3],
        strength: f32,
    ) {
        self.viewport.sun = SunSettings::from_file(direction, color, strength, self.viewport.sun);
        self.log(
            LogLevel::Info,
            format!("The file's sun ({name}) now lights the scene; change it under Sun."),
        );
    }

    /// Uses `environment` (as saved last time), with anything out of range
    /// put back in range.
    #[must_use]
    pub fn with_environment(mut self, environment: EnvironmentSettings) -> Self {
        self.viewport.environment = environment.sanitized();
        self
    }

    /// The sky, fog and exposure, for saving.
    pub fn environment(&self) -> EnvironmentSettings {
        self.viewport.environment
    }

    /// The sun's settings, for saving.
    pub fn sun(&self) -> SunSettings {
        self.viewport.sun
    }

    /// Tells the viewport which models to draw.
    pub(crate) fn sync_models(&mut self) {
        self.viewport.models = self.models.iter().map(|m| m.id).collect();
        self.viewport.triangles = self.models.iter().map(|m| m.triangles).sum();
    }

    /// The panel layout, for saving.
    pub fn layout(&self) -> &DockState<Panel> {
        &self.dock
    }

    /// The accent colour, for saving.
    pub fn accent(&self) -> Accent {
        self.accent
    }

    /// Uses `camera` (as saved last time), with anything unusable in it
    /// replaced (see [`Camera::sanitized`]).
    #[must_use]
    pub fn with_camera(mut self, camera: Camera) -> Self {
        self.viewport.camera = camera.sanitized();
        self
    }

    /// Uses `look` (as saved last time), with anything out of range put
    /// back in range.
    #[must_use]
    pub fn with_look(mut self, look: crate::look::Look) -> Self {
        self.viewport.look = look.sanitized();
        self
    }

    /// How models are shaded, for saving.
    pub fn look(&self) -> crate::look::Look {
        self.viewport.look
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
        self.console.lines()
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
        let mut removed = Vec::new();
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
                        models: &self.models,
                        selection: &mut self.selection,
                        removed: &mut removed,
                        accent: self.accent,
                        in_front: &self.in_front,
                        shown: &mut shown,
                    },
                );
        });
        self.in_front = shown;
        for id in removed {
            self.remove_model(id);
            actions.push(WorkbenchAction::RemoveModel(id));
        }
        if self.files_hovering {
            drop_hint(ui, self.accent);
        }
        self.about(ui);
        actions
    }

    /// Q, W, R, S and T pick the tools, unless typing in a field.
    pub(crate) fn tool_keys(&mut self, ui: &Ui) {
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

#[cfg(test)]
#[path = "workbench_tests.rs"]
mod tests;
