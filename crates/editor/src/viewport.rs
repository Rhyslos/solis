//! The viewport panel: the 3D image, the toolbox down its left edge
//! (`toolbox.rs`) and the pills drawn over each view (`viewport_pills.rs`;
//! spec 177-182).
//!
//! The image itself comes from a [`ViewportRenderer`], so this crate never
//! touches the GPU and runs in tests without one.

use crate::area_view::AreaView;
use crate::camera::{Camera, ViewKind};
use crate::icons;
use crate::look::Look;
use crate::theme::{self, Accent};
use crate::tokens::size;
use crate::transforming::Snapping;
use crate::viewport_tools::{self, Tools};
use egui::{Rect, Ui, UiBuilder};
use serde::{Deserialize, Serialize};

/// The tools ([decision 0004](../../../docs/decisions/0004-editor-layout.md)).
/// There is no Place tool: placing things from the library is part of
/// Select.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Tool {
    /// Pick things in the viewport.
    #[default]
    Select,
    /// Move along an axis or freely.
    Move,
    /// Turn around an axis.
    Rotate,
    /// Make bigger or smaller.
    Scale,
    /// Move, rotate and scale at once.
    Transform,
}

impl Tool {
    /// Every tool, in the order the viewport shows them.
    pub const ALL: [Self; 5] = [
        Self::Select,
        Self::Move,
        Self::Rotate,
        Self::Scale,
        Self::Transform,
    ];

    /// The tool's name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Move => "Move",
            Self::Rotate => "Rotate",
            Self::Scale => "Scale",
            Self::Transform => "Transform",
        }
    }

    /// The tool's icon.
    pub fn icon(self) -> icons::Icon {
        match self {
            Self::Select => icons::Icon::Select,
            Self::Move => icons::Icon::Move,
            Self::Rotate => icons::Icon::Rotate,
            Self::Scale => icons::Icon::Scale,
            Self::Transform => icons::Icon::Transform,
        }
    }

    /// The key that picks the tool.
    pub fn key(self) -> egui::Key {
        match self {
            Self::Select => egui::Key::Q,
            Self::Move => egui::Key::W,
            Self::Rotate => egui::Key::R,
            Self::Scale => egui::Key::S,
            Self::Transform => egui::Key::T,
        }
    }

    /// What the tool's button says when pointed at, such as "Move (W)".
    pub fn hint(self) -> String {
        format!("{} ({})", self.name(), self.key().name())
    }
}

/// Which of the viewport's corner controls are shown (the ≡ menu). Saved
/// between sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Overlays {
    /// The tools, bottom centre.
    pub tools: bool,
    /// Grid, angle and camera speed, bottom right.
    pub snapping: bool,
    /// Frame rate and entity count, top right.
    pub stats: bool,
    /// Whether sprites play their clips while editing (Phase 9, D3). Off
    /// by standard, so the editor rests (decision 0026); the game always
    /// animates.
    pub preview_animation: bool,
}

impl Default for Overlays {
    fn default() -> Self {
        Self {
            tools: true,
            snapping: true,
            stats: true,
            preview_animation: false,
        }
    }
}

/// The viewport's state between frames.
#[derive(Debug, Clone)]
pub struct Viewport {
    /// The camera.
    pub camera: Camera,
    /// The chosen tool.
    pub tool: Tool,
    /// Which corner controls are shown.
    pub overlays: Overlays,
    /// The time per frame, smoothed, in seconds.
    frame_time: f32,
    /// Whether nothing is moving, so frames are only drawn when something
    /// changes: the corner says "Idle" instead of a frame rate.
    pub(crate) idle: bool,
    /// How models are shaded.
    pub look: Look,
    /// What the area shows, worked out each frame by the workbench.
    pub area: AreaView,
    /// How many entities the area has, for the corner.
    pub entities: usize,
    /// While looking around with the right mouse button: where the mouse
    /// was when it started, to put it back afterwards.
    flying_from: Option<egui::Pos2>,
    /// Wheel movement not yet a whole notch (trackpads scroll in small
    /// amounts), in notches.
    wheel_carry: f32,
    /// Picking, the gizmo and the selection's outline.
    pub(crate) tools: Tools,
    /// Snapping, axis locks and world or object axes.
    pub snapping: Snapping,
    /// Where the viewport was drawn last frame, on screen.
    pub(crate) rect: Rect,
    /// Where the corner controls were last frame: clicks there are theirs.
    covered: Vec<Rect>,
    /// How the viewport is split into views.
    pub layout: ViewLayout,
    /// The view shown alone, filling the viewport, if one is.
    pub maximized: Option<usize>,
    /// The view in use: its camera, look and tools are the fields above;
    /// the others wait in `panes`.
    active: usize,
    /// Every view's own state; the one in use is a stand-in.
    panes: Vec<Pane>,
    /// Where the whole viewport (every view) was drawn last frame.
    pub(crate) full: Rect,
    /// The project's 2D settings, for the 2D view (the workbench hands
    /// them over each frame).
    pub(crate) pixels: area::pixels::PixelSettings,
    /// The project's sprite sheets, to draw sprites with.
    pub(crate) sheets: std::sync::Arc<crate::sprite_sheets::SpriteSheets>,
    /// The selected tile map's values laid over it, its hidden layers,
    /// and the cell under the pointer (Phase 9, D4).
    pub(crate) tile_overlay: crate::tile_view::TileOverlay,
    /// The hour the viewport shows the area at, from the Environment's
    /// Preview time (decision 0092); never saved. `None`: its start.
    pub(crate) preview_hour: Option<f64>,
    /// The time animated tiles show, in milliseconds (the editor's clock).
    pub(crate) tile_ms: u64,
    /// When Preview animation was turned on (its clips play from there).
    preview_since: std::cell::Cell<Option<std::time::Instant>>,
    /// Whether the editor is placing (the Place shelf tab is in front):
    /// there is no toolbox then (the workbench hands it over each frame).
    pub(crate) placing: bool,
    /// The brush list B opens, and a brush being drawn out (Tools 1a,
    /// B2).
    pub(crate) brushing: crate::brushing::Brushing,
}

/// One view's own camera, look and state, while another view is in use.
#[derive(Debug, Clone)]
struct Pane {
    camera: Camera,
    look: Look,
    tools: Tools,
    rect: Rect,
    covered: Vec<Rect>,
    flying_from: Option<egui::Pos2>,
    wheel_carry: f32,
}

impl Default for Pane {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            look: Look::default(),
            tools: Tools::default(),
            rect: Rect::NOTHING,
            covered: Vec::new(),
            flying_from: None,
            wheel_carry: 0.0,
        }
    }
}

/// What each view starts as: perspective, then top, front and right.
const FIRST_KINDS: [ViewKind; VIEWS] = [
    ViewKind::Perspective,
    ViewKind::Top,
    ViewKind::Front,
    ViewKind::Right,
];

impl Default for Viewport {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            tool: Tool::Select,
            overlays: Overlays::default(),
            frame_time: 1.0 / 60.0,
            idle: false,
            flying_from: None,
            wheel_carry: 0.0,
            look: Look::default(),
            area: AreaView::default(),
            entities: 0,
            tools: Tools::default(),
            snapping: Snapping::default(),
            rect: Rect::NOTHING,
            covered: Vec::new(),
            layout: ViewLayout::One,
            maximized: None,
            active: 0,
            full: Rect::NOTHING,
            pixels: area::pixels::PixelSettings::default(),
            sheets: std::sync::Arc::default(),
            tile_overlay: crate::tile_view::TileOverlay::default(),
            preview_hour: None,
            tile_ms: 0,
            placing: false,
            brushing: crate::brushing::Brushing::default(),
            preview_since: std::cell::Cell::new(None),
            panes: FIRST_KINDS
                .iter()
                .map(|&kind| Pane {
                    camera: Camera::default().as_kind(kind),
                    ..Pane::default()
                })
                .collect(),
        }
    }
}

/// How quickly the frame-rate counter follows changes (0 to 1 per frame).
const FRAME_TIME_SMOOTHING: f32 = 0.05;

/// A gap between frames longer than this means the editor was idle.
const IDLE_GAP: f32 = 0.25;

impl Viewport {
    /// Forgets everything of the project that was open, keeping what is
    /// the user's own. Every field is named, so a new one can't be added
    /// without deciding which it is.
    pub(crate) fn forget_project(&mut self) {
        let Self {
            // The user's own: the views and their cameras and looks, the
            // tool, the corner controls, snapping, and where things were
            // drawn last frame.
            camera: _,
            tool: _,
            overlays: _,
            frame_time: _,
            idle: _,
            look: _,
            flying_from: _,
            wheel_carry: _,
            snapping: _,
            rect: _,
            covered: _,
            layout: _,
            maximized: _,
            active: _,
            full: _,
            tile_ms: _,
            preview_since: _,
            placing: _,
            // The project's: what its area shows, its files, the map
            // selected and the painter's stamp of its cells, and picks and
            // drags under way in any view.
            area,
            entities,
            tools,
            panes,
            pixels,
            sheets,
            tile_overlay,
            preview_hour,
            brushing,
        } = self;
        *area = AreaView::default();
        *entities = 0;
        *tools = Tools::default();
        for pane in panes {
            pane.tools = Tools::default();
        }
        *pixels = area::pixels::PixelSettings::default();
        *sheets = std::sync::Arc::default();
        *tile_overlay = crate::tile_view::TileOverlay::default();
        *preview_hour = None;
        *brushing = crate::brushing::Brushing::default();
    }

    /// Whether the right mouse button is held on the viewport, so W A S D,
    /// Q and E move the camera instead of picking tools.
    pub fn is_flying(&self) -> bool {
        self.flying_from.is_some()
    }

    /// Whether a view needs the next frame straight away: the camera is
    /// flying, a click or a drag is going on, or a pick's answer is on its
    /// way from the graphics card.
    pub fn needs_frames(&self) -> bool {
        self.previewing()
            || self.is_flying()
            || self.tools.pick.is_some()
            || self.tools.pressing != viewport_tools::Pressing::Nothing
            || self
                .panes
                .iter()
                .any(|p| p.tools.pick.is_some() || p.flying_from.is_some())
    }

    /// Whether sprites are playing their clips in the viewport: Preview
    /// animation is on and a sprite has a clip to play.
    pub fn previewing(&self) -> bool {
        self.overlays.preview_animation
            && self.area.sprites.iter().any(|s| {
                s.sprite.playing
                    && s.sprite
                        .sheet
                        .and_then(|sheet| self.sheets.clips(sheet.asset))
                        .is_some_and(|set| set.chosen(&s.sprite.clip).is_some())
            })
    }

    /// How far sprites' clips have played on since Preview animation was
    /// turned on, in seconds (0 when it's off).
    pub fn preview_seconds(&self) -> f64 {
        if self.overlays.preview_animation {
            let since = self
                .preview_since
                .get()
                .unwrap_or_else(std::time::Instant::now);
            self.preview_since.set(Some(since));
            since.elapsed().as_secs_f64()
        } else {
            self.preview_since.set(None);
            0.0
        }
    }

    /// The frame rate shown in the corner.
    pub fn frames_per_second(&self) -> f32 {
        1.0 / self.frame_time.max(1e-4)
    }

    /// The camera in use as a view of `size_px` physical pixels shows it:
    /// itself, or for the 2D view, set to show whole pixels
    /// ([`Camera::seen`]).
    pub fn seen(&self, size_px: [u32; 2]) -> Camera {
        self.camera.seen(size_px, self.pixels.pixels_per_unit)
    }

    /// Draws the viewport in `ui`, filling it: each view of the layout,
    /// with its own camera. A press or the wheel in a view makes it the
    /// one in use (F, adding things and the tool bar follow it).
    pub fn show(&mut self, ui: &mut Ui, renderer: &mut dyn ViewportRenderer, accent: Accent) {
        let dt = ui.input(|i| i.unstable_dt);
        // Frames drawn now and then while idle (or the first after) say
        // nothing about the frame rate.
        if !self.idle && dt.is_finite() && dt > 0.0 && dt < IDLE_GAP {
            self.frame_time += (dt - self.frame_time) * FRAME_TIME_SMOOTHING;
        }
        let mut full = ui.max_rect();
        if self.overlays.tools && !self.placing {
            // The toolbox down the group's left edge, the views beside it.
            let (column, views) = full.split_left_right_at_x(full.left() + size::TOOLBOX_W);
            self.toolbox(ui, column);
            full = views;
        }
        self.full = full;
        let shown = self.shown(full);
        // The view to use from now: the one being dragged or flown in, or
        // else one just pressed or scrolled in.
        let busy = self.is_flying()
            || self.tools.pressing != viewport_tools::Pressing::Nothing
            || self
                .brushing
                .draw
                .as_ref()
                .is_some_and(crate::brushing::draw::Draw::is_under_way);
        let (pointer, poked) = ui.input(|i| {
            (
                i.pointer.latest_pos(),
                i.pointer.any_pressed()
                    || i.events
                        .iter()
                        .any(|e| matches!(e, egui::Event::MouseWheel { .. })),
            )
        });
        // The selected map's pointer and asks are found anew each frame,
        // by the view the pointer is in.
        let overlay = &mut self.tile_overlay;
        overlay.hovered = None;
        overlay.pointer = None;
        overlay.input = crate::tile_paint::PaintInput::default();
        let mut next = self.active;
        if !busy
            && poked
            && let Some(p) = pointer
            && let Some(&(i, _)) = shown.iter().find(|(_, r)| r.contains(p))
        {
            next = i;
        }
        if !shown.iter().any(|&(i, _)| i == next) {
            next = shown.first().map_or(0, |&(i, _)| i);
        }
        // Each view works with its own state, so a click in one never
        // reaches another.
        let gizmo = self.tools.gizmo;
        let outlines = self.tools.outlines.clone();
        let mut events = Vec::new();
        let several = shown.len() > 1;
        if several {
            // The gaps between the views.
            ui.painter().rect_filled(full, 0.0, theme::panel());
        }
        for &(i, rect) in &shown {
            self.activate(i);
            self.tools.gizmo = gizmo;
            self.tools.outlines = outlines.clone();
            let mut pane = ui.new_child(
                UiBuilder::new()
                    .id_salt(("viewport view", i))
                    .max_rect(rect),
            );
            self.show_view(&mut pane, renderer, accent, dt, i == next, several);
            events.append(&mut self.tools.events);
            if several && i == next {
                ui.painter().rect_stroke(
                    rect,
                    0.0,
                    egui::Stroke::new(1.0, accent.colors().fill),
                    egui::StrokeKind::Inside,
                );
            }
        }
        self.activate(next);
        self.tools.events = events;
    }

    /// While a file is dragged over the view at `rect`: where it would
    /// land on the ground, the label beside the pointer with that and
    /// what it snaps to, and the hint strip (spec 244-254).
    fn placing_overlay(
        &self,
        ui: &mut Ui,
        rect: Rect,
        (pointer, at): (egui::Pos2, glam::Vec2),
        seen: &Camera,
        name: &str,
    ) {
        let ctrl = ui.input(|i| i.modifiers.command);
        let ground = seen
            .ray_through(at, glam::Vec2::new(rect.width(), rect.height()))
            .hit_ground(0.0)
            .map(|p| crate::placement::landing(p, &self.snapping, ctrl));
        if let Some(spot) = ground {
            let half = if self.snapping.grid {
                self.snapping.grid_step / 2.0
            } else {
                crate::placement::FOOTPRINT_HALF
            };
            crate::placement::footprint(ui, rect, seen, (spot, half));
        }
        let snap = crate::placement::snap_word(&self.snapping, ctrl);
        crate::placement::drag_label(ui, pointer, name, ground, snap);
        crate::placement::hint_strip(ui, rect);
    }

    /// The toolbox in `column`, and the tool it picks.
    fn toolbox(&mut self, ui: &mut Ui, column: Rect) {
        let paint = &mut self.tile_overlay.paint;
        if let Some(tool) = crate::toolbox::toolbox(ui, column, self.tool, paint) {
            self.tool = tool;
            paint.tool = None;
        }
    }
}

/// Where `ground` (a point on the ground, in metres) is, in the game's
/// pixels, and the map cell under it if a map is selected:
/// `x 192 · y 64 px · cell 12, 4`. In a 2D game, down the screen is down
/// the picture, so y counts downward from the top as pixel-art programs do.
pub(crate) fn cursor_readout(
    ground: glam::Vec3,
    pixels: area::pixels::PixelSettings,
    cell: Option<[u32; 2]>,
) -> String {
    let per = pixels.pixels_per_unit.max(1) as f32;
    // Whole pixels, and never "-0".
    let x = (ground.x * per).floor() + 0.0;
    let y = (-ground.y * per).floor() + 0.0;
    let place = format!("x {x:.0} · y {y:.0} px");
    match cell {
        Some([cx, cy]) => format!("{place} · cell {cx}, {cy}"),
        None => place,
    }
}

#[path = "viewport_frame.rs"]
mod frame;
#[path = "viewport_layout.rs"]
mod layout;
#[path = "viewport_navigate.rs"]
mod navigate;
#[path = "viewport_pills.rs"]
mod pills;
#[path = "viewport_view.rs"]
mod view;

pub use frame::{
    BOLT_WIDTH, DrawReport, GAME_IMAGE, GpuSplit, MAX_IMAGES, NoRenderer, PixelLayer, PixelView,
    VIEWPORT_IMAGE, VIEWS, ViewBolt, ViewportRenderer, ViewportView,
};
pub use layout::{ViewLayout, ViewSettings, Views};
pub(crate) use navigate::POINTS_PER_NOTCH;

#[cfg(test)]
#[path = "viewport_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "views_tests.rs"]
mod views_tests;

#[cfg(test)]
#[path = "two_d_tests.rs"]
mod two_d_tests;
