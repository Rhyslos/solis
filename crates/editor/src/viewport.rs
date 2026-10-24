//! The viewport panel: the 3D image, and the controls drawn over it in its
//! corners ([decision 0004](../../../docs/decisions/0004-editor-layout.md)):
//!
//! - top right: frame rate and entity count;
//! - bottom left: ≡ (what to show), Perspective, Lit and Show;
//! - bottom centre: the tools;
//! - bottom right: grid, angle and camera speed.
//!
//! The image itself comes from a [`ViewportRenderer`], so this crate never
//! touches the GPU and runs in tests without one.

use crate::area_view::{AreaView, ViewLight, ViewModel};
use crate::camera::{Camera, FlyInput, ViewKind};
use crate::icons;
use crate::look::{Look, LookKind};
use crate::theme::{self, Accent};
use crate::transforming::Snapping;
use crate::viewport_tools::{self, PickAnswer, ToolInput, Tools};
use egui::{Align, Color32, Layout, Rect, RichText, Sense, Ui, UiBuilder, Vec2, vec2};
use glam::{Mat4, Vec3};
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
}

impl Default for Overlays {
    fn default() -> Self {
        Self {
            tools: true,
            snapping: true,
            stats: true,
        }
    }
}

/// What one viewport frame needs from the renderer.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewportView {
    /// The image size in physical pixels.
    pub size_px: [u32; 2],
    /// World-to-screen transform.
    pub view_projection: Mat4,
    /// Where the camera is.
    pub camera_position: Vec3,
    /// The point on the ground the grid is centred on.
    pub focus: Vec3,
    /// The sky straight up, sRGB 0 to 1.
    pub sky_top: [f32; 3],
    /// The sky at the horizon, sRGB 0 to 1.
    pub sky_horizon: [f32; 3],
    /// The ground below the horizon, sRGB 0 to 1.
    pub ground: [f32; 3],
    /// The models to draw, with their pick numbers.
    pub models: Vec<ViewModel>,
    /// How models are shaded.
    pub look: Look,
    /// The stylized look's outline width in pixels; 0 for none.
    pub outline_px: f32,
    /// The sun, if the area has one.
    pub sun: Option<ViewLight>,
    /// The other lights.
    pub lamps: Vec<ViewLight>,
    /// The sky, fog and exposure.
    pub environment: area::builtin::Environment,
    /// A rectangle of the image (x, y, width, height in pixels) to find
    /// out what's under, after drawing this frame: the answer comes from
    /// [`ViewportRenderer::picked`] on a later frame.
    pub pick: Option<[u32; 4]>,
    /// Which image this is drawn into ([`VIEWPORT_IMAGE`],
    /// [`GAME_IMAGE`]): each keeps its own, so views never overwrite each
    /// other's.
    pub image: usize,
    /// Whether the editor's lines are drawn: the grid, the ground axes and
    /// lamp markers. Off in the Game panel.
    pub editor_lines: bool,
    /// Perspective or a flat view (the grid lies in a flat view's plane).
    pub kind: ViewKind,
}

/// The viewport's first view's image; its other views use the next ones
/// (up to [`VIEWS`]).
pub const VIEWPORT_IMAGE: usize = 0;
/// How many views the viewport can show at once.
pub const VIEWS: usize = 4;
/// The Game panel's image.
pub const GAME_IMAGE: usize = VIEWS;
/// How many images a renderer keeps at most.
pub const MAX_IMAGES: usize = 8;

/// Draws viewport images. The app implements it on the window's GPU; tests
/// use one that draws nothing.
pub trait ViewportRenderer {
    /// Draws a frame and returns the egui texture showing it, or a
    /// plain-language reason why it could not.
    fn render(&mut self, view: &ViewportView) -> Result<egui::TextureId, String>;

    /// The answer to the last [`ViewportView::pick`] for image `image`,
    /// once it has arrived
    /// (usually the frame after). Never waits. A renderer that can't pick
    /// never answers.
    fn picked(&mut self, image: usize) -> Option<PickAnswer> {
        let _ = image;
        None
    }
}

/// A renderer that never draws, with a reason to show instead.
#[derive(Debug, Clone)]
pub struct NoRenderer {
    /// Why there is no 3D view.
    pub reason: String,
}

impl ViewportRenderer for NoRenderer {
    fn render(&mut self, _view: &ViewportView) -> Result<egui::TextureId, String> {
        Err(self.reason.clone())
    }
}

use theme::backdrop::rgb;

/// The viewport's state between frames.
#[derive(Debug, Clone)]
pub struct Viewport {
    /// The camera.
    pub camera: Camera,
    /// The chosen tool.
    pub tool: Tool,
    /// Which corner controls are shown.
    pub overlays: Overlays,
    /// Whether the ≡ menu is open.
    menu_open: bool,
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
}

/// How the viewport is split into views. Saved per computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ViewLayout {
    /// One view.
    #[default]
    One,
    /// Two views side by side.
    SideBySide,
    /// Two views, one above the other.
    Stacked,
    /// Four views, two by two.
    Four,
}

impl ViewLayout {
    /// Every layout, in the menu's order.
    pub const ALL: [Self; 4] = [Self::One, Self::SideBySide, Self::Stacked, Self::Four];

    /// Its name in the menu.
    pub fn name(self) -> &'static str {
        match self {
            Self::One => "One view",
            Self::SideBySide => "Two side by side",
            Self::Stacked => "Two, one above the other",
            Self::Four => "Four views",
        }
    }

    /// Each view shown in `rect`, by number, with where it goes. A thin
    /// gap separates them.
    pub fn rects(self, rect: Rect) -> Vec<(usize, Rect)> {
        let gap = 2.0;
        let (w, h) = (rect.width(), rect.height());
        let cell = |x0: f32, y0: f32, x1: f32, y1: f32| {
            Rect::from_min_max(rect.min + vec2(x0, y0), rect.min + vec2(x1, y1))
        };
        let (mx, my) = ((w - gap) / 2.0, (h - gap) / 2.0);
        match self {
            Self::One => vec![(0, rect)],
            Self::SideBySide => vec![(0, cell(0.0, 0.0, mx, h)), (1, cell(mx + gap, 0.0, w, h))],
            Self::Stacked => vec![(0, cell(0.0, 0.0, w, my)), (1, cell(0.0, my + gap, w, h))],
            Self::Four => vec![
                (0, cell(0.0, 0.0, mx, my)),
                (1, cell(mx + gap, 0.0, w, my)),
                (2, cell(0.0, my + gap, mx, h)),
                (3, cell(mx + gap, my + gap, w, h)),
            ],
        }
    }
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
    menu_open: bool,
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
            menu_open: false,
        }
    }
}

/// One view's camera and look, as saved.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ViewSettings {
    /// Its camera (and whether it's a flat view).
    pub camera: Camera,
    /// How it shades models.
    pub look: Look,
}

/// Every view's settings and the layout, as saved per computer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Views {
    /// How the viewport is split.
    pub layout: ViewLayout,
    /// Each view's camera and look, first to last.
    pub views: Vec<ViewSettings>,
    /// The view shown alone, if one is.
    pub maximized: Option<usize>,
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
            menu_open: false,
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
    /// Swaps view `i`'s own state with the fields in use.
    fn exchange(&mut self, i: usize) {
        let pane = &mut self.panes[i];
        std::mem::swap(&mut self.camera, &mut pane.camera);
        std::mem::swap(&mut self.look, &mut pane.look);
        std::mem::swap(&mut self.tools, &mut pane.tools);
        std::mem::swap(&mut self.rect, &mut pane.rect);
        std::mem::swap(&mut self.covered, &mut pane.covered);
        std::mem::swap(&mut self.flying_from, &mut pane.flying_from);
        std::mem::swap(&mut self.wheel_carry, &mut pane.wheel_carry);
        std::mem::swap(&mut self.menu_open, &mut pane.menu_open);
    }

    /// Makes view `i` the one in use: its camera, look and tools move
    /// into the fields the editor reads.
    fn activate(&mut self, i: usize) {
        if i == self.active || i >= VIEWS {
            return;
        }
        self.exchange(self.active);
        self.exchange(i);
        self.active = i;
    }

    /// Which view is in use (the last one clicked).
    pub fn active(&self) -> usize {
        self.active
    }

    /// View `i`'s camera and look.
    pub fn view_settings(&self, i: usize) -> Option<ViewSettings> {
        if i == self.active {
            return Some(ViewSettings {
                camera: self.camera,
                look: self.look,
            });
        }
        self.panes.get(i).map(|p| ViewSettings {
            camera: p.camera,
            look: p.look,
        })
    }

    /// The layout and every view's settings, for saving.
    pub fn views(&self) -> Views {
        Views {
            layout: self.layout,
            views: (0..VIEWS).filter_map(|i| self.view_settings(i)).collect(),
            maximized: self.maximized,
        }
    }

    /// Uses `views` (as saved last time), with anything unusable put
    /// back. View 0 is in use afterwards.
    pub fn set_views(&mut self, views: &Views) {
        self.activate(0);
        self.layout = views.layout;
        self.maximized = views.maximized.filter(|&i| i < VIEWS);
        for (i, saved) in views.views.iter().take(VIEWS).enumerate() {
            let camera = saved.camera.sanitized();
            let look = saved.look.sanitized();
            if i == 0 {
                self.camera = camera;
                self.look = look;
            } else {
                self.panes[i].camera = camera;
                self.panes[i].look = look;
            }
        }
    }

    /// The views shown in `rect`, by number, with where each goes.
    pub fn shown(&self, rect: Rect) -> Vec<(usize, Rect)> {
        match self.maximized {
            Some(i) if self.layout != ViewLayout::One => vec![(i, rect)],
            _ => self.layout.rects(rect),
        }
    }

    /// Whether the ≡ menu is open.
    pub fn menu_open(&self) -> bool {
        self.menu_open
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
        self.is_flying()
            || self.tools.pick.is_some()
            || self.tools.pressing != viewport_tools::Pressing::Nothing
            || self
                .panes
                .iter()
                .any(|p| p.tools.pick.is_some() || p.flying_from.is_some())
    }

    /// The frame rate shown in the corner.
    pub fn frames_per_second(&self) -> f32 {
        1.0 / self.frame_time.max(1e-4)
    }

    /// The view for an image of `size_px` physical pixels.
    pub fn view(&self, size_px: [u32; 2], pixels_per_point: f32) -> ViewportView {
        let aspect = size_px[0].max(1) as f32 / size_px[1].max(1) as f32;
        ViewportView {
            size_px,
            view_projection: self.camera.view_projection(aspect),
            camera_position: self.camera.viewpoint(),
            focus: self.camera.focus(),
            sky_top: rgb(theme::backdrop::SKY_TOP),
            sky_horizon: rgb(theme::backdrop::HORIZON),
            ground: rgb(theme::backdrop::GROUND),
            models: self.area.models.clone(),
            look: self.look,
            outline_px: self.look.outline_px(pixels_per_point),
            sun: self.area.sun,
            lamps: self.area.lamps.clone(),
            environment: if self.camera.is_flat() {
                // Fog grows with distance, which a flat view doesn't show:
                // it would only hide the whole view.
                area::builtin::Environment {
                    fog: false,
                    ..self.area.environment
                }
            } else {
                self.area.environment
            },
            pick: None,
            image: VIEWPORT_IMAGE + self.active,
            editor_lines: true,
            kind: self.camera.kind,
        }
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
        let full = ui.max_rect();
        self.full = full;
        let shown = self.shown(full);
        // The view to use from now: the one being dragged or flown in, or
        // else one just pressed or scrolled in.
        let busy = self.is_flying() || self.tools.pressing != viewport_tools::Pressing::Nothing;
        let (pointer, poked) = ui.input(|i| {
            (
                i.pointer.latest_pos(),
                i.pointer.any_pressed()
                    || i.events
                        .iter()
                        .any(|e| matches!(e, egui::Event::MouseWheel { .. })),
            )
        });
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
            ui.painter().rect_filled(full, 0.0, theme::PANEL);
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

    /// Draws the view in use into `ui`, filling it. `in_use` shows its
    /// tool bar, snapping and frame rate; `several` adds a button to
    /// show it alone.
    fn show_view(
        &mut self,
        ui: &mut Ui,
        renderer: &mut dyn ViewportRenderer,
        accent: Accent,
        dt: f32,
        in_use: bool,
        several: bool,
    ) {
        let rect = ui.max_rect();
        self.rect = rect;
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        if response.clicked() {
            self.menu_open = false;
        }
        self.navigate(ui, &response, dt);
        let ppp = ui.ctx().pixels_per_point();
        let size_px = [
            (rect.width() * ppp).round().max(1.0) as u32,
            (rect.height() * ppp).round().max(1.0) as u32,
        ];
        let size = glam::Vec2::new(rect.width(), rect.height());
        self.tools
            .receive(renderer.picked(VIEWPORT_IMAGE + self.active));
        if !self.is_flying() {
            // egui gives the viewport the hover even under a corner button
            // (it's the only thing there that can be dragged), so a press
            // on the corner controls is left out by hand.
            let on_corner = ui
                .input(|i| i.pointer.latest_pos())
                .is_some_and(|p| self.covered.iter().any(|r| r.contains(p)));
            let input = ui.input(|i| ToolInput {
                mouse: i
                    .pointer
                    .latest_pos()
                    .map(|p| glam::Vec2::new(p.x - rect.min.x, p.y - rect.min.y)),
                pressed: i.pointer.primary_pressed(),
                down: i.pointer.primary_down(),
                released: i.pointer.primary_released(),
                right_pressed: i.pointer.secondary_pressed(),
                ctrl: i.modifiers.command,
                shift: i.modifiers.shift,
                on_viewport: response.hovered() && !on_corner,
            });
            self.tools
                .interact(input, &self.camera, &self.area, &self.snapping, size, ppp);
        }
        let mouse = ui
            .input(|i| i.pointer.latest_pos())
            .map(|p| glam::Vec2::new(p.x - rect.min.x, p.y - rect.min.y));
        if let Some(dropped) = response.dnd_release_payload::<crate::library::LibraryDrag>()
            && let (Some(asset), Some(at)) = (dropped.0.id, mouse)
            && dropped.0.kind.placeable()
        {
            self.tools
                .drop_asset(asset, at, &self.camera, &self.area, size, ppp);
        }
        let mut view = self.view(size_px, ppp);
        view.pick = self.tools.send_with(&self.area.pickable);
        let painter = ui.painter_at(rect);
        match renderer.render(&view) {
            Ok(texture) => {
                painter.image(
                    texture,
                    rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            Err(reason) => {
                let message = format!("No 3D view: {reason}");
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &message)
                });
                painter.rect_filled(rect, 0.0, theme::backdrop::HORIZON);
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    message,
                    egui::FontId::proportional(13.0),
                    theme::TEXT_WEAK,
                );
            }
        }
        viewport_tools::draw_cameras(ui, rect, &self.area.cameras, &self.camera);
        viewport_tools::draw(
            ui,
            rect,
            &self.tools,
            &self.camera,
            self.snapping.locked,
            accent,
        );
        if let Some(dragged) = response.dnd_hover_payload::<crate::library::LibraryDrag>()
            && dragged.0.kind.placeable()
            && let Some(at) = ui.input(|i| i.pointer.latest_pos())
        {
            painter.text(
                at + egui::vec2(18.0, 14.0),
                egui::Align2::LEFT_TOP,
                format!("Drop to place {}", dragged.0.name),
                egui::FontId::proportional(12.0),
                theme::TEXT_STRONG,
            );
        }
        if let viewport_tools::Pressing::Gizmo {
            last: Some(change), ..
        } = &self.tools.pressing
            && let Some(mouse) = ui.input(|i| i.pointer.latest_pos())
        {
            painter.text(
                mouse + egui::vec2(18.0, 14.0),
                egui::Align2::LEFT_TOP,
                viewport_tools::readout(*change),
                egui::FontId::monospace(12.0),
                theme::TEXT_STRONG,
            );
        }
        let inner = rect.shrink(8.0);
        let corner = |ui: &mut Ui, salt: &str, layout: Layout| {
            ui.new_child(
                UiBuilder::new()
                    .id_salt(("viewport corner", salt))
                    .max_rect(inner)
                    .layout(layout),
            )
        };
        // With one view, as in the mockup: frame rate top right, tools
        // bottom centre, snapping bottom right. With several, each view is
        // too narrow for a full bottom row, so the view in use has its
        // tools top left, snapping top right and frame rate bottom right.
        let mut covered = Vec::new();
        if self.overlays.stats && in_use {
            let layout = if several {
                Layout::bottom_up(Align::Max)
            } else {
                Layout::top_down(Align::Max)
            };
            let mut stats = corner(ui, "stats", layout);
            self.stats(&mut stats, several);
            covered.push(stats.min_rect());
        }
        let mut bottom_left = corner(ui, "view", Layout::bottom_up(Align::Min));
        self.view_controls(&mut bottom_left, several);
        covered.push(bottom_left.min_rect());
        if self.overlays.tools && in_use {
            // A centred layout would stretch the bar, so it gets a rect of
            // exactly its own width.
            let bar = if several {
                Rect::from_min_size(inner.min, TOOL_BAR_SIZE)
            } else {
                Rect::from_center_size(
                    egui::pos2(inner.center().x, inner.bottom() - TOOL_BAR_SIZE.y / 2.0),
                    TOOL_BAR_SIZE,
                )
            };
            let mut bar_ui = ui.new_child(
                UiBuilder::new()
                    .id_salt(("viewport corner", "tools"))
                    .max_rect(bar)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            self.tools(&mut bar_ui, accent);
            covered.push(bar_ui.min_rect());
        }
        if self.overlays.snapping && in_use {
            let layout = if several {
                Layout::top_down(Align::Max)
            } else {
                Layout::bottom_up(Align::Max)
            };
            let mut snapping = corner(ui, "snapping", layout);
            viewport_tools::snapping_corner(
                &mut snapping,
                &mut self.snapping,
                accent,
                self.camera.speed,
                !several,
            );
            covered.push(snapping.min_rect());
        }
        self.covered = covered;
    }

    /// Flies the camera from this frame's mouse and keys
    /// ([decision 0009](../../../docs/decisions/0009-fly-camera-controls.md)).
    fn navigate(&mut self, ui: &Ui, response: &egui::Response, dt: f32) {
        use egui::{Key, PointerButton, ViewportCommand};
        let ctx = ui.ctx();
        let (pressed_here, held, pointer) = ui.input(|i| {
            (
                i.pointer.button_pressed(PointerButton::Secondary),
                i.pointer.button_down(PointerButton::Secondary),
                i.pointer.interact_pos(),
            )
        });
        // Flat views don't fly: the right button slides them instead.
        let flat = self.camera.is_flat();
        let starting = !flat
            && self.flying_from.is_none()
            && !self.tools.dragging()
            && pressed_here
            && response.hovered()
            && response.is_pointer_button_down_on();
        if starting {
            self.flying_from = pointer.or(Some(response.rect.center()));
            self.menu_open = false;
            // Keys now steer the camera, not a text field.
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
            ctx.send_viewport_cmd(ViewportCommand::CursorVisible(false));
            ctx.send_viewport_cmd(ViewportCommand::CursorGrab(grab_mode()));
        }
        if let Some(from) = self.flying_from
            && !held
        {
            self.flying_from = None;
            ctx.send_viewport_cmd(ViewportCommand::CursorGrab(egui::CursorGrab::None));
            ctx.send_viewport_cmd(ViewportCommand::CursorVisible(true));
            ctx.send_viewport_cmd(ViewportCommand::CursorPosition(from));
        }

        let flying = self.flying_from.is_some();
        let notches = if flying || response.hovered() {
            let (whole, carry) = wheel_notches(ui, self.wheel_carry);
            self.wheel_carry = carry;
            whole
        } else {
            self.wheel_carry = 0.0;
            0.0
        };
        let mut input = FlyInput {
            dt,
            view_height: response.rect.height(),
            view_width: response.rect.width(),
            mouse: pointer
                .map(|p| glam::Vec2::new(p.x - response.rect.min.x, p.y - response.rect.min.y)),
            ..FlyInput::default()
        };
        if flying {
            ui.input(|i| {
                // Raw mouse movement keeps going when the hidden pointer
                // reaches the edge of the screen.
                let look = i.pointer.motion().unwrap_or(i.pointer.delta());
                input.look = glam::Vec2::new(look.x, look.y);
                let axis = |plus: Key, minus: Key| {
                    f32::from(u8::from(i.key_down(plus))) - f32::from(u8::from(i.key_down(minus)))
                };
                input.movement = glam::Vec3::new(
                    axis(Key::D, Key::A),
                    axis(Key::W, Key::S),
                    axis(Key::E, Key::Q),
                );
                input.boost = i.modifiers.shift;
            });
            input.speed_notches = notches as i32;
        } else {
            input.dolly_notches = notches;
            let slides = response.dragged_by(PointerButton::Middle)
                || (flat && response.dragged_by(PointerButton::Secondary));
            if slides {
                let delta = response.drag_delta();
                input.pan = glam::Vec2::new(delta.x, delta.y);
            }
        }
        self.camera.fly(&input);
    }

    /// The frame rate and counts. `upwards` for a corner that builds
    /// upwards (the lines go in last first, so they still read down).
    fn stats(&self, ui: &mut Ui, upwards: bool) {
        let s = if self.entities == 1 { "y" } else { "ies" };
        let mut lines = vec![
            if self.idle {
                // Nothing moves, so nothing is drawn until something does.
                "Idle".to_string()
            } else {
                format!(
                    "{:.0} fps · {:.1} ms",
                    self.frames_per_second(),
                    self.frame_time * 1000.0
                )
            },
            format!("{} entit{s}", crate::models::grouped(self.entities)),
        ];
        if self.area.triangles > 0 {
            lines.push(format!(
                "{} triangles",
                crate::models::grouped(self.area.triangles)
            ));
        }
        let layout = if upwards {
            lines.reverse();
            Layout::bottom_up(Align::Max)
        } else {
            Layout::top_down(Align::Max)
        };
        theme::overlay_frame()
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.with_layout(layout, |ui| {
                    for line in lines {
                        ui.label(RichText::new(line).monospace().size(11.0));
                    }
                });
            });
    }

    fn view_controls(&mut self, ui: &mut Ui, several: bool) {
        // Bottom-up: the row of buttons first, the ≡ menu above it.
        ui.horizontal(|ui| {
            let menu = ui
                .add(egui::Button::new("≡").selected(self.menu_open))
                .on_hover_text("What the viewport shows, and how many views");
            if menu.clicked() {
                self.menu_open = !self.menu_open;
            }
            let kind = self.camera.kind;
            let mut chosen = None;
            up_menu(ui, format!("{} ▴", kind.name()), |ui| {
                for each in ViewKind::ALL {
                    let hint = match each {
                        ViewKind::Perspective => "As the eye sees: fly with the right mouse button",
                        ViewKind::Top => {
                            "Flat, from above: right or middle drag slides, the wheel zooms"
                        }
                        ViewKind::Front => "Flat, from the front (looking north)",
                        ViewKind::Right => "Flat, from the right (looking west)",
                    };
                    if ui
                        .add(egui::Button::new(each.name()).selected(each == kind))
                        .on_hover_text(hint)
                        .clicked()
                    {
                        chosen = Some(each);
                        ui.close();
                    }
                }
            });
            if let Some(kind) = chosen {
                self.camera = self.camera.as_kind(kind);
            }
            let look = &mut self.look;
            up_menu(ui, format!("{} ▴", look.kind.name()), |ui| {
                look_menu(ui, look)
            });
            up_menu(ui, "Show ▴", |ui| {
                for thing in ["Grid", "Axes", "Lights", "Cameras", "Colliders"] {
                    ui.add_enabled(false, egui::Checkbox::new(&mut true, thing))
                        .on_disabled_hover_text(
                            "Choosing what to show arrives as there is more to show",
                        );
                }
            });
            let alone = self.maximized == Some(self.active);
            if several || alone {
                let (text, hint) = if alone {
                    ("All views", "Show every view again")
                } else {
                    ("Maximise", "Show this view alone, filling the viewport")
                };
                if ui.button(text).on_hover_text(hint).clicked() {
                    self.maximized = if alone { None } else { Some(self.active) };
                }
            }
        });
        if self.menu_open {
            theme::overlay_frame()
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.set_min_width(220.0);
                    self.menu(ui);
                });
        }
    }

    /// The ≡ menu's contents: the layout of views, and what to show.
    /// The ≡ menu's contents: the layout of views, and what to show.
    /// The corner builds upwards, so they're added last first.
    fn menu(&mut self, ui: &mut Ui) {
        ui.checkbox(&mut self.overlays.stats, "Frame rate and entity count");
        ui.checkbox(&mut self.overlays.snapping, "Grid, angle and camera speed");
        ui.checkbox(&mut self.overlays.tools, "Tools");
        ui.label(RichText::new("Show in the viewport").color(theme::TEXT_WEAK));
        ui.add_space(6.0);
        for layout in ViewLayout::ALL.into_iter().rev() {
            if ui.radio(self.layout == layout, layout.name()).clicked() {
                self.layout = layout;
                self.maximized = None;
            }
        }
        ui.label(RichText::new("Views").color(theme::TEXT_WEAK));
    }

    fn tools(&mut self, ui: &mut Ui, accent: Accent) {
        theme::overlay_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = TOOL_GAP;
                for tool in Tool::ALL {
                    if tool_button(ui, tool, self.tool == tool, accent).clicked() {
                        self.tool = tool;
                    }
                }
            });
        });
    }
}

/// A menu that opens upwards from its button (the ▴ says so), since the
/// buttons sit at the bottom of the viewport. It stays open while its
/// settings are changed, and closes on a click elsewhere.
fn up_menu(ui: &mut Ui, title: impl Into<egui::WidgetText>, content: impl FnOnce(&mut Ui)) {
    let response = ui.add(egui::Button::new(title));
    egui::Popup::menu(&response)
        .align(egui::RectAlign::TOP_START)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(content);
}

/// The look menu: the three looks, and the stylized look's settings.
fn look_menu(ui: &mut Ui, look: &mut Look) {
    for kind in LookKind::ALL {
        if ui
            .add(egui::Button::new(kind.name()).selected(look.kind == kind))
            .on_hover_text(kind.hint())
            .clicked()
        {
            look.kind = kind;
        }
    }
    ui.add_enabled(false, egui::Button::new("Wireframe"))
        .on_disabled_hover_text("Wireframe arrives with the editor's tools in Phase 3");
    ui.separator();
    let stylized = look.kind == LookKind::Stylized;
    ui.add_enabled_ui(stylized, |ui| {
        ui.label(RichText::new("Stylized look").color(theme::TEXT_WEAK));
        ui.horizontal(|ui| {
            ui.label("Bands of light");
            for bands in Look::BANDS {
                if ui
                    .add(egui::Button::new(bands.to_string()).selected(look.bands == bands))
                    .clicked()
                {
                    look.bands = bands;
                }
            }
        });
        ui.checkbox(&mut look.outlines, "Outlines");
    })
    .response
    .on_disabled_hover_text("Choose Stylized to change these");
}

/// How the hidden pointer is held while looking around: macOS can only
/// lock it in place, other systems can only keep it inside the window.
fn grab_mode() -> egui::CursorGrab {
    if cfg!(target_os = "macos") {
        egui::CursorGrab::Locked
    } else {
        egui::CursorGrab::Confined
    }
}

/// Points of scrolling that count as one wheel notch, for trackpads and
/// mice that report points.
const POINTS_PER_NOTCH: f32 = 50.0;

/// Whole wheel notches this frame (up is positive), and the part of a notch
/// left over, added to `carry` from earlier frames.
fn wheel_notches(ui: &Ui, carry: f32) -> (f32, f32) {
    let scrolled = ui.input(|i| {
        i.events
            .iter()
            .map(|event| match event {
                egui::Event::MouseWheel { unit, delta, .. } => match unit {
                    egui::MouseWheelUnit::Line => delta.y,
                    egui::MouseWheelUnit::Point => delta.y / POINTS_PER_NOTCH,
                    egui::MouseWheelUnit::Page => delta.y * 3.0,
                },
                _ => 0.0,
            })
            .sum::<f32>()
    });
    let total = carry + scrolled;
    if !total.is_finite() {
        return (0.0, 0.0);
    }
    let whole = total.trunc();
    (whole, total - whole)
}

/// One tool button's size.
const TOOL_BUTTON: Vec2 = vec2(30.0, 26.0);
/// Space between tool buttons.
const TOOL_GAP: f32 = 2.0;
/// The tool bar's frame: margin plus border, each side.
const TOOL_BAR_EDGE: f32 = 4.0;
/// The whole tool bar's size.
const TOOL_BAR_SIZE: Vec2 = vec2(
    Tool::ALL.len() as f32 * TOOL_BUTTON.x
        + (Tool::ALL.len() - 1) as f32 * TOOL_GAP
        + 2.0 * TOOL_BAR_EDGE,
    TOOL_BUTTON.y + 2.0 * TOOL_BAR_EDGE,
);

/// One tool's icon button, filled with the accent when chosen.
fn tool_button(ui: &mut Ui, tool: Tool, chosen: bool, accent: Accent) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(TOOL_BUTTON, Sense::click());
    let response = response.on_hover_text(tool.hint());
    let fill = if chosen {
        accent.colors().fill
    } else if response.hovered() {
        theme::HOVER
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 3.0, fill);
    let color = if chosen {
        Color32::WHITE
    } else {
        theme::TEXT_STRONG
    };
    icons::tool_icon(
        ui.painter(),
        Rect::from_center_size(rect.center(), vec2(16.0, 16.0)),
        tool,
        color,
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, chosen, tool.hint())
    });
    response
}

#[cfg(test)]
#[path = "views_tests.rs"]
mod views_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_has_its_own_key() {
        let mut keys: Vec<_> = Tool::ALL.iter().map(|t| t.key()).collect();
        keys.sort_by_key(|k| k.name());
        keys.dedup();
        assert_eq!(keys.len(), Tool::ALL.len());
        assert_eq!(Tool::Move.hint(), "Move (W)");
        assert_eq!(Tool::Transform.key(), egui::Key::T);
    }

    #[test]
    fn the_view_carries_the_size_and_backdrop() {
        let view = Viewport::default().view([800, 600], 1.0);
        assert_eq!(view.size_px, [800, 600]);
        assert_eq!(view.sky_top, rgb(theme::backdrop::SKY_TOP));
        assert!(view.view_projection.is_finite());
    }

    #[test]
    fn every_overlay_shows_by_default() {
        let o = Overlays::default();
        assert!(o.tools && o.snapping && o.stats);
    }

    #[test]
    fn old_saved_overlays_fill_in_new_switches() {
        let o: Overlays = ron::from_str("(stats: false)").unwrap();
        assert!(!o.stats);
        assert!(o.tools && o.snapping);
    }

    #[test]
    fn no_renderer_explains_itself() {
        let mut r = NoRenderer {
            reason: "no GPU".into(),
        };
        assert_eq!(
            r.render(&Viewport::default().view([1, 1], 1.0)),
            Err("no GPU".to_string())
        );
    }
}
