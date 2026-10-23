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
use crate::document::Document;
use crate::layout::{self, Preset};
use crate::models::{FileLight, FileLightKind, ModelInfo};
use crate::panel::Panel;
use crate::panels::{PanelViewer, drop_hint};
use crate::properties::PropertyRequest;
use crate::scene_view::SceneView;
use crate::theme::{self, Accent};
use crate::viewport::{Overlays, Tool, Viewport, ViewportRenderer};
use egui::Ui;
use egui_dock::{DockArea, DockState};
use glam::Vec3;
use reflect::{AssetId, AssetRef, EntityId};
use scene::EntityData;
use scene::builtin::{Light, LightKind, Model, Transform};
use std::collections::{BTreeMap, HashSet};

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
}

/// How far above the camera's focus point a new point light goes, in
/// metres.
const POINT_LIGHT_HEIGHT: f32 = 2.0;

/// The editor's state, kept between frames.
pub struct Workbench {
    pub(crate) info: AppInfo,
    pub(crate) dock: DockState<Panel>,
    pub(crate) accent: Accent,
    pub(crate) viewport: Viewport,
    pub(crate) console: Console,
    /// The open scene and what's selected in it.
    pub(crate) document: Document,
    /// What the editor knows about the imported models, by asset.
    pub(crate) models: BTreeMap<AssetId, ModelInfo>,
    /// Whether files are being dragged over the window.
    pub(crate) files_hovering: bool,
    pub(crate) about_open: bool,
    /// The tabs that were in front last frame, to underline them in the
    /// accent colour.
    pub(crate) in_front: HashSet<Panel>,
}

impl Workbench {
    /// A workbench with a saved panel layout, if it is usable (otherwise the
    /// default one), and a new scene.
    pub fn new(info: AppInfo, saved_layout: Option<DockState<Panel>>) -> Self {
        Self {
            info,
            dock: layout::restore(saved_layout),
            accent: Accent::default(),
            viewport: Viewport::default(),
            console: Console::default(),
            document: Document::default(),
            models: BTreeMap::new(),
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

    /// Adds an imported model to the scene as an entity (with any lights its
    /// file carries as entities under it; a sun in the file sets the scene's
    /// sun), frames the camera on it, and says so in the console with its
    /// size and any `warnings` (parts of the file that were left out).
    pub fn add_model(
        &mut self,
        info: ModelInfo,
        warnings: &[String],
        seconds: f32,
        lights: &[FileLight],
    ) -> EntityId {
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
        let entity = self.document.scene.spawn(
            EntityData::new(&info.name)
                .with(&Transform::default())
                .with(&Model {
                    asset: Some(AssetRef {
                        asset: info.asset,
                        part: None,
                    }),
                }),
        );
        self.models.insert(info.asset, info);
        self.add_file_lights(entity, lights);
        entity
    }

    /// The lights a model's file carries: lamps become entities under the
    /// model; a sun sets the scene's sun.
    fn add_file_lights(&mut self, model: EntityId, lights: &[FileLight]) {
        let mut lamps = 0;
        for (i, light) in lights.iter().enumerate() {
            let color = light.color.map(crate::models::linear_to_srgb);
            let rotation = scene::builtin::aim(light.direction);
            let strength = if light.strength.is_finite() {
                light.strength.max(0.0)
            } else {
                0.0
            };
            if light.kind == FileLightKind::Sun {
                self.use_file_sun(&light.name, rotation, color, strength);
                continue;
            }
            let (kind, cone, soft_edge) = match light.kind {
                FileLightKind::Spot { inner, outer } => {
                    let outer = outer.to_degrees().clamp(1.0, 89.0);
                    let inner = inner.to_degrees().clamp(0.0, outer);
                    (LightKind::Spot, outer, outer - inner)
                }
                _ => (LightKind::Point, 30.0, 10.0),
            };
            let name = if light.name.trim().is_empty() {
                format!("Light {}", i + 1)
            } else {
                light.name.clone()
            };
            let position = if light.position.is_finite() {
                light.position
            } else {
                Vec3::ZERO
            };
            let mut data = EntityData::new(&name)
                .with(&Transform {
                    position,
                    rotation,
                    ..Transform::default()
                })
                .with(&Light {
                    kind,
                    color,
                    strength,
                    range: light
                        .range
                        .filter(|r| r.is_finite() && *r > 0.0)
                        .unwrap_or(0.0),
                    cone,
                    soft_edge,
                    ..Light::point()
                });
            data.parent = Some(model);
            data.order = format!("a{i:04}");
            self.document.scene.spawn(data);
            lamps += 1;
        }
        if lamps > 0 {
            let s = if lamps == 1 { "" } else { "s" };
            self.log(
                LogLevel::Info,
                format!("The file brought {lamps} light{s} with it."),
            );
        }
    }

    /// Sets the scene's sun (or adds one) from a sun in a model's file.
    fn use_file_sun(&mut self, name: &str, rotation: glam::Quat, color: [f32; 3], strength: f32) {
        let scene = &mut self.document.scene;
        let sun = scene
            .with::<Light>()
            .find(|(_, l)| l.kind == LightKind::Sun)
            .map(|(id, _)| id);
        let id = sun.unwrap_or_else(|| scene.spawn(EntityData::new("Sun").with(&Light::sun())));
        if let Some(entity) = scene.get_mut(id) {
            let mut light = entity.get::<Light>().unwrap_or_else(Light::sun);
            light.color = color;
            light.strength = strength;
            let mut transform = entity.get::<Transform>().unwrap_or_default();
            transform.rotation = rotation;
            entity.set(&light);
            entity.set(&transform);
        }
        self.log(
            LogLevel::Info,
            format!("The file's sun ({name}) now lights the scene; change it under Sun."),
        );
    }

    /// Tells the workbench whether files are being dragged over the window,
    /// so it can show where to drop them.
    pub fn set_files_hovering(&mut self, hovering: bool) {
        self.files_hovering = hovering;
    }

    /// What the editor knows about the imported models, by asset.
    pub fn models(&self) -> &BTreeMap<AssetId, ModelInfo> {
        &self.models
    }

    /// The open scene and its selection.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// The entity shown in Properties, if any.
    pub fn selection(&self) -> Option<EntityId> {
        self.document.selection.primary()
    }

    /// Selects only `id` (as clicking it in the Scene panel does); nothing
    /// if it isn't in the scene.
    pub fn select(&mut self, id: EntityId) {
        if self.document.scene.contains(id) {
            self.document.selection.select_only(id);
        } else {
            self.document.selection.clear();
        }
    }

    /// Adds an entity with only a transform, at the point the camera looks
    /// at, and selects it.
    pub fn add_entity(&mut self) -> EntityId {
        let focus = self.viewport.camera.focus();
        self.spawn_selected(EntityData::new("Entity").with(&Transform {
            position: focus,
            ..Transform::default()
        }))
    }

    /// Adds a point light above where the camera looks, and selects it.
    pub fn add_point_light(&mut self) -> EntityId {
        let n = self.count_lights(LightKind::Point) + 1;
        let position = self.viewport.camera.focus() + Vec3::Z * POINT_LIGHT_HEIGHT;
        self.spawn_selected(
            EntityData::new(&format!("Point light {n}"))
                .with(&Transform {
                    position,
                    ..Transform::default()
                })
                .with(&Light::point()),
        )
    }

    /// Adds a spotlight at the camera, shining where it looks, and selects
    /// it.
    pub fn add_spotlight(&mut self) -> EntityId {
        let n = self.count_lights(LightKind::Spot) + 1;
        let camera = &self.viewport.camera;
        self.spawn_selected(
            EntityData::new(&format!("Spotlight {n}"))
                .with(&Transform {
                    position: camera.position,
                    rotation: scene::builtin::aim(camera.forward()),
                    ..Transform::default()
                })
                .with(&Light::spot()),
        )
    }

    fn count_lights(&self, kind: LightKind) -> usize {
        self.document
            .scene
            .with::<Light>()
            .filter(|(_, l)| l.kind == kind)
            .count()
    }

    fn spawn_selected(&mut self, data: EntityData) -> EntityId {
        self.log(LogLevel::Info, format!("Added {}.", data.name));
        let id = self.document.scene.spawn(data);
        self.document.selection.select_only(id);
        id
    }

    /// Carries out what the user asked for in Properties.
    pub(crate) fn apply_property(&mut self, request: PropertyRequest) {
        let scene = &mut self.document.scene;
        match request {
            PropertyRequest::SetField {
                entity,
                component,
                field,
                value,
            } => {
                if let Some(data) = scene
                    .get_mut(entity)
                    .and_then(|e| e.components.get_mut(&component))
                {
                    data.set(&field, value);
                }
            }
            PropertyRequest::Rename { entity, name } => {
                if let Some(e) = scene.get_mut(entity) {
                    e.name = name;
                }
            }
            PropertyRequest::RemoveComponent { entity, component } => {
                if let Some(e) = scene.get_mut(entity) {
                    e.components.remove(&component);
                }
            }
            PropertyRequest::Delete(entity) => self.delete(entity),
            PropertyRequest::MoveHere(entity) => self.move_here(entity),
        }
    }

    /// Deletes `id` and everything below it.
    pub fn delete(&mut self, id: EntityId) {
        let gone = self.document.scene.remove_tree(id);
        if let Some((_, first)) = gone.first() {
            let more = gone.len() - 1;
            let extra = match more {
                0 => String::new(),
                1 => " and 1 entity under it".into(),
                n => format!(" and {n} entities under it"),
            };
            self.log(LogLevel::Info, format!("Deleted {}{extra}.", first.name));
        }
        self.document
            .selection
            .retain_existing(&self.document.scene);
    }

    /// Moves `id` to where the camera looks: a point light above that
    /// point, a spotlight to the camera, shining the same way, anything
    /// else to the point itself.
    pub fn move_here(&mut self, id: EntityId) {
        let camera = self.viewport.camera;
        let Some(entity) = self.document.scene.get_mut(id) else {
            return;
        };
        let mut transform = entity.get::<Transform>().unwrap_or_default();
        match entity.get::<Light>().map(|l| l.kind) {
            Some(LightKind::Spot) => {
                transform.position = camera.position;
                transform.rotation = scene::builtin::aim(camera.forward());
            }
            Some(LightKind::Point) => {
                transform.position = camera.focus() + Vec3::Z * POINT_LIGHT_HEIGHT;
            }
            _ => transform.position = camera.focus(),
        }
        entity.set(&transform);
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
        let mut requests = Vec::new();
        self.viewport.scene = SceneView::of(&self.document.scene, &self.models);
        self.viewport.entities = self.document.scene.len();
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
                        document: &mut self.document,
                        models: &self.models,
                        removed: &mut removed,
                        requests: &mut requests,
                        accent: self.accent,
                        in_front: &self.in_front,
                        shown: &mut shown,
                    },
                );
        });
        self.in_front = shown;
        for request in requests {
            self.apply_property(request);
        }
        for id in removed {
            self.delete(id);
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
