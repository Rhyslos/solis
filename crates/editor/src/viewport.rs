//! The viewport panel: the 3D image, the toolbox down its left edge
//! (`toolbox.rs`) and the pills drawn over each view (`viewport_pills.rs`;
//! spec 177-182).
//!
//! The image itself comes from a [`ViewportRenderer`], so this crate never
//! touches the GPU and runs in tests without one.

use crate::area_view::{AreaView, ViewLight, ViewModel};
use crate::camera::{Camera, FlyInput, ViewKind};
use crate::icons;
use crate::look::Look;
use crate::theme::{self, Accent};
use crate::tokens::size;
use crate::transforming::Snapping;
use crate::viewport_tools::{self, PickAnswer, ToolInput, Tools};
use egui::{Align, Color32, Layout, Rect, Sense, Ui, UiBuilder, vec2};
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
    /// The hour it's drawn at (decision 0092), for the day's tint and the
    /// lamps that shine only at night.
    pub hour: f64,
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
    /// Projectiles to draw (the game's).
    pub bolts: std::sync::Arc<Vec<ViewBolt>>,
    /// The finest grid lines' spacing, in metres: 1 m, or the tile size in
    /// the 2D view.
    pub grid_step: f32,
    /// The sprites, a run per sheet (Phase 9, D2).
    pub sprites: std::sync::Arc<Vec<crate::sprite_sheets::SpriteBatch>>,
    /// Tile maps' layers as pictures (Phase 9, D4), drawn by the batches
    /// in `sprites` that name them: the renderer uploads one again when
    /// its key changes.
    pub tile_pictures: std::sync::Arc<Vec<std::sync::Arc<crate::tile_view::TilePicture>>>,
    /// For a 2D camera (Phase 9, D1): the picture is drawn this small,
    /// with `view_projection` for it, then scaled up to `size_px` by the
    /// pixel stage.
    pub pixel: Option<PixelView>,
    /// With `pixel`: parallax layers drawn as small pictures of their own,
    /// each slid by its own bit of a pixel and laid behind or over the
    /// world's (decision 0094), slowest first.
    pub layers: Vec<PixelLayer>,
    /// Game UI drawn over the finished picture (decision 0083), screen
    /// after screen: the renderer lays each out on the picture's real
    /// size.
    pub ui: Vec<std::sync::Arc<ui::Screen>>,
}

/// A parallax layer of a 2D game's picture drawn on its own (decision
/// 0094): its sprites seen by a camera of its own, whose bit of a pixel
/// slides it smoothly.
#[derive(Debug, Clone, PartialEq)]
pub struct PixelLayer {
    /// Its own camera's view and projection, for the small picture.
    pub view_projection: Mat4,
    /// Its own bit of a pixel, as [`PixelView::slide`].
    pub slide: [f32; 2],
    /// Drawn over the world's picture (in front) or behind it.
    pub in_front: bool,
    /// Its sprites, a run per sheet.
    pub sprites: std::sync::Arc<Vec<crate::sprite_sheets::SpriteBatch>>,
}

impl ViewportView {
    /// The lamps shining at this view's hour: those that shine only at
    /// night are left out by day (decision 0092).
    pub fn lamps_shining(&self) -> impl Iterator<Item = &ViewLight> {
        let night = self.environment.is_night(self.hour);
        self.lamps.iter().filter(move |l| night || !l.only_at_night)
    }
}

/// How a 2D game's picture is drawn small and scaled up: the renderer
/// draws the base resolution plus a pixel each side, then the pixel stage
/// scales it up ([`render`'s `pixel_layout`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelView {
    /// The base resolution, in pixels.
    pub base: [u32; 2],
    /// How it fills the screen.
    pub scaling: area::pixels::Scaling,
    /// The bit of a pixel the camera has moved beyond the whole pixel the
    /// picture is drawn at, in pixels (x right, y down the picture).
    pub slide: [f32; 2],
}

/// A projectile to draw: a glowing streak from its head back along its
/// trail. Laid out as the renderer's own (`render::Bolt`), so a frame's
/// bolts are handed over as they are, without a copy.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ViewBolt {
    /// Its head, in metres.
    pub head: Vec3,
    /// From its head back to its tail.
    pub trail: Vec3,
    /// Its colour, linear RGB (above 1 glows).
    pub colour: [f32; 3],
    /// How wide it is, in metres.
    pub width: f32,
}

/// How wide a projectile's streak is, in metres.
pub const BOLT_WIDTH: f32 = 0.12;

impl ViewBolt {
    /// The streak for `projectile`: as long as it flies in a thirtieth of
    /// a second (two frames), between 0.3 and 8 m.
    pub fn of(projectile: &world::Projectile) -> Self {
        Self::between(projectile, 1.0)
    }

    /// As [`Self::of`], drawn `t` of the way (0 to 1) from where it was
    /// when the last tick started to where it is now, like entities
    /// drawn between ticks (`world::World::placed_between`).
    pub fn between(projectile: &world::Projectile, t: f32) -> Self {
        let speed = projectile.velocity.length();
        let length = (speed / 30.0).clamp(0.3, 8.0);
        let back = (1.0 - t.clamp(0.0, 1.0)) * world::TICK as f32;
        Self {
            head: projectile.position - projectile.velocity * back,
            trail: -projectile.velocity.normalize_or_zero() * length,
            colour: projectile.colour,
            width: BOLT_WIDTH,
        }
    }
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

    /// What the frames drawn since the last call drew, all views
    /// together, with the GPU's newest times where it can tell them (for
    /// the Profiler). Never waits.
    fn take_report(&mut self) -> DrawReport {
        DrawReport::default()
    }
}

/// What a frame drew, for the Profiler.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DrawReport {
    /// Model copies drawn (those in view).
    pub instances: u64,
    /// Model copies drawn into the sun's shadow map (those whose shadow
    /// can fall in view).
    pub casting: u64,
    /// Bytes of model copies sent to the graphics card (only what
    /// changed).
    pub sent_bytes: u64,
    /// Model copies in view drawn at each level of detail: in full, then
    /// each simpler level.
    pub lods: [u64; 5],
    /// Projectiles drawn.
    pub bolts: u64,
    /// Triangles drawn.
    pub triangles: u64,
    /// Draw calls for models.
    pub batches: u64,
    /// The GPU's own times, if measured.
    pub gpu: Option<GpuSplit>,
    /// Whether the graphics card can time its passes (`None`: not known
    /// yet).
    pub gpu_timing: Option<bool>,
}

/// The GPU's time for a frame, in milliseconds, by pass.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GpuSplit {
    /// The sun's shadow map.
    pub shadows: f64,
    /// The sky, models and the rest.
    pub models: f64,
    /// Exposure and tone mapping.
    pub last_pass: f64,
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
    }

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

    /// The view for an image of `size_px` physical pixels.
    pub fn view(&self, size_px: [u32; 2], pixels_per_point: f32) -> ViewportView {
        let ahead = self.preview_seconds();
        let aspect = size_px[0].max(1) as f32 / size_px[1].max(1) as f32;
        let camera = self.seen(size_px);
        let view_projection = camera.view_projection(aspect);
        let (tile_pictures, tile_batches) = self.sheets.tile_pictures.draw(
            &self.area.view_tile_maps(&self.sheets),
            &self.sheets,
            self.pixels.tile_size(),
            &self.tile_overlay.hidden,
            &|z| crate::tile_view::ground_in_view(view_projection, z),
            self.tile_ms,
            self.tile_overlay
                .map
                .as_ref()
                .filter(|_| self.tile_overlay.paint.meaning)
                .map(|m| m.entity),
        );
        ViewportView {
            size_px,
            view_projection: camera.view_projection(aspect),
            camera_position: camera.viewpoint(),
            focus: camera.focus(),
            sky_top: rgb(theme::backdrop::sky_top()),
            sky_horizon: rgb(theme::backdrop::horizon()),
            ground: rgb(theme::backdrop::ground()),
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
            hour: self.preview_hour.unwrap_or(self.area.hour),
            pick: None,
            image: VIEWPORT_IMAGE + self.active,
            editor_lines: true,
            kind: self.camera.kind,
            bolts: std::sync::Arc::default(),
            grid_step: if self.camera.kind == ViewKind::TwoD {
                self.pixels.tile_size()[0]
            } else {
                1.0
            },
            // Sprites land on whole pixels in the 2D view, as in the game;
            // tile maps' layers under and over them.
            sprites: std::sync::Arc::new({
                let snap = (self.camera.kind == ViewKind::TwoD).then(|| self.pixels.pixel_size());
                // Parallax layers where they're placed: the editor's
                // camera isn't the game's (decision 0094).
                let (behind, front) = crate::sprite_parallax::parallax_layers(
                    &self.area.sprites,
                    &self.sheets,
                    self.pixels.pixel_size(),
                    snap,
                    ahead,
                    &crate::sprite_parallax::ParallaxView {
                        moved: None,
                        seen: &|z, _| crate::tile_view::ground_in_view(view_projection, z),
                    },
                );
                crate::sprite_parallax::in_order(
                    behind,
                    tile_batches,
                    crate::sprite_sheets::batches(
                        &self.area.sprites,
                        &self.sheets,
                        self.pixels.pixel_size(),
                        snap,
                        ahead,
                    ),
                    front,
                )
            }),
            tile_pictures: std::sync::Arc::new(tile_pictures),
            pixel: None,
            layers: Vec::new(),
            ui: Vec::new(),
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
        // The selected map's pointer and asks are found anew each frame,
        // by the view the pointer is in.
        let overlay = &mut self.tile_overlay;
        overlay.hovered = None;
        overlay.pointer = None;
        overlay.asked.clear();
        overlay.chunks_asked = false;
        overlay.walls_asked = false;
        overlay.turn_asked = None;
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
        self.tile_ms = (ui.input(|i| i.time).max(0.0) * 1000.0) as u64;
        if !self.area.tile_maps.is_empty() && self.sheets.tiles_animate() {
            // Animated tiles move on by themselves.
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(40));
        }
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        // While painting a tile map (D6), the left and right buttons
        // paint and empty: the middle button and the wheel still move.
        let painting = in_use && self.tile_overlay.paint.painting();
        self.navigate(ui, &response, dt, painting);
        let ppp = ui.ctx().pixels_per_point();
        let size_px = [
            (rect.width() * ppp).round().max(1.0) as u32,
            (rect.height() * ppp).round().max(1.0) as u32,
        ];
        let size = glam::Vec2::new(rect.width(), rect.height());
        // The 2D view shows as much as its zoom says at this size: kept, so
        // the next frame's drags and zooms start from it.
        let seen = self.seen(size_px);
        self.camera.flat_height = seen.flat_height;
        let two_d = self.camera.kind == ViewKind::TwoD;
        // The 2D view moves things across the ground only.
        let mut snapping = self.snapping;
        if two_d {
            snapping.locked[2] = true;
        }
        self.tools
            .receive(renderer.picked(VIEWPORT_IMAGE + self.active));
        if painting {
            let on_corner = ui.input(|i| i.pointer.latest_pos()).is_some_and(|p| {
                self.covered.iter().any(|r| r.contains(p))
                    || ui.ctx().layer_id_at(p).is_some_and(|l| l != ui.layer_id())
            });
            self.tile_overlay.input = ui.input(|i| {
                use crate::tile_paint::Button;
                crate::tile_paint::PaintInput {
                    on_view: response.hovered() && !on_corner,
                    left: Button {
                        pressed: i.pointer.primary_pressed(),
                        down: i.pointer.primary_down(),
                    },
                    right: Button {
                        pressed: i.pointer.secondary_pressed(),
                        down: i.pointer.secondary_down(),
                    },
                    ctrl: i.modifiers.command,
                    alt: i.modifiers.alt,
                }
            });
        } else if !self.is_flying() {
            // egui gives the viewport the hover even under a corner button
            // (it's the only thing there that can be dragged), so a press
            // on the corner controls is left out by hand.
            let on_corner = ui.input(|i| i.pointer.latest_pos()).is_some_and(|p| {
                self.covered.iter().any(|r| r.contains(p))
                    // A menu open over the view (its own layer) takes
                    // the press too.
                    || ui.ctx().layer_id_at(p).is_some_and(|l| l != ui.layer_id())
            });
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
                .interact(input, &seen, &self.area, &snapping, size, ppp);
        }
        let mouse = ui
            .input(|i| i.pointer.latest_pos())
            .map(|p| glam::Vec2::new(p.x - rect.min.x, p.y - rect.min.y));
        if let Some(dropped) = response.dnd_release_payload::<crate::library::LibraryDrag>()
            && let (Some(asset), Some(at)) = (dropped.0.id, mouse)
            && dropped.0.kind.placeable()
        {
            // Ctrl held while dropping places without snapping (answer 25).
            let free = ui.input(|i| i.modifiers.command);
            self.tools
                .drop_asset(asset, (at, free), &seen, &self.area, (size, ppp));
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
                painter.rect_filled(rect, 0.0, theme::backdrop::horizon());
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    message,
                    egui::FontId::proportional(13.0),
                    theme::text_weak(),
                );
            }
        }
        if two_d {
            draw_screens(ui, rect, &self.area.screens, &seen, self.pixels);
        }
        viewport_tools::draw_cameras(ui, rect, &self.area.cameras, &seen);
        let tile_bar = crate::tile_view::draw_overlay(
            ui,
            rect,
            &seen,
            &mut self.tile_overlay,
            self.pixels.tile_size(),
            mouse.filter(|_| response.hovered()),
            accent,
        );
        viewport_tools::draw(ui, rect, &self.tools, &seen, snapping.locked, accent);
        // In 2D, where the pointer is, in the game's pixels, and the map's
        // cell under it (decision 0085), beside the pointer.
        if two_d
            && response.hovered()
            && let Some(at) = mouse
            && let Some(ground) = seen
                .ray_through(at, glam::Vec2::new(rect.width(), rect.height()))
                .hit_ground(0.0)
            && let Some(pointer) = ui.input(|i| i.pointer.latest_pos())
        {
            let words = cursor_readout(ground, self.pixels, self.tile_overlay.hovered);
            let galley =
                painter.layout_no_wrap(words, egui::FontId::monospace(11.5), theme::text_strong());
            let corner = pointer + egui::vec2(16.0, 14.0);
            let back =
                egui::Rect::from_min_size(corner, galley.size()).expand2(egui::vec2(6.0, 3.0));
            painter.rect_filled(back, 4.0, theme::header().gamma_multiply(0.9));
            painter.galley(corner, galley, theme::text_strong());
        }
        if let Some(dragged) = response.dnd_hover_payload::<crate::library::LibraryDrag>()
            && dragged.0.kind.placeable()
            && let (Some(pointer), Some(at)) = (ui.input(|i| i.pointer.latest_pos()), mouse)
        {
            self.placing_overlay(ui, rect, (pointer, at), &seen, &dragged.0.name);
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
                theme::text_strong(),
            );
        }
        // The pills over the view (spec 182): the view's own at the top
        // left; the frame rate and the locks, space and camera speed on
        // the view in use only.
        let mut covered: Vec<Rect> = tile_bar.into_iter().collect();
        covered.push(self.top_left_pills(ui, rect, several));
        if self.overlays.stats && in_use {
            covered.push(self.stats_pill(ui, rect));
        }
        if in_use && self.tile_overlay.paint.allowed {
            // A selected map's painting options, until its own tab on the
            // ribbon takes them (milestone M10).
            let mut options = ui.new_child(
                UiBuilder::new()
                    .id_salt(("viewport corner", "paint options"))
                    .max_rect(rect.shrink(size::PILL_MARGIN))
                    .layout(Layout::left_to_right(Align::Max)),
            );
            theme::overlay_frame().show(&mut options, |ui| {
                ui.horizontal(|ui| {
                    crate::tile_paint::paint_options(ui, &mut self.tile_overlay.paint)
                });
            });
            covered.push(options.min_rect());
        }
        let mut bottom = rect;
        if self.overlays.snapping && in_use {
            let pills = if two_d {
                self.pixel_pills(ui, rect)
            } else {
                self.space_pills(ui, rect)
            };
            bottom.max.y = pills.top() - size::PILL_GAP + size::PILL_MARGIN;
            covered.push(pills);
        }
        if in_use
            && self.tile_overlay.paint.painting()
            && let Some(stamp) = &self.tile_overlay.paint.stamp
        {
            // The copy held: its buttons above the bottom-right pills.
            let mut stamp_ui = ui.new_child(
                UiBuilder::new()
                    .id_salt(("viewport corner", "stamp"))
                    .max_rect(bottom.shrink(size::PILL_MARGIN))
                    .layout(Layout::bottom_up(Align::Max)),
            );
            let asked = crate::tile_paint::stamp_bar(&mut stamp_ui, stamp);
            if asked.is_some() {
                self.tile_overlay.turn_asked = asked;
            }
            covered.push(stamp_ui.min_rect());
        }
        self.covered = covered;
    }

    /// Flies the camera from this frame's mouse and keys
    /// ([decision 0009](../../../docs/decisions/0009-fly-camera-controls.md)).
    fn navigate(&mut self, ui: &Ui, response: &egui::Response, dt: f32, painting: bool) {
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
        // While the left button is doing something for the tools (a click
        // on its way, a box, a gizmo drag), the right button is theirs: it
        // cancels a box or a drag, and never starts flying over one.
        let tools_idle = self.tools.pressing == viewport_tools::Pressing::Nothing;
        let starting = !flat
            && !painting
            && self.flying_from.is_none()
            && tools_idle
            && pressed_here
            && response.hovered()
            && response.is_pointer_button_down_on();
        if starting {
            self.flying_from = pointer.or(Some(response.rect.center()));
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
                || (flat && !painting && response.dragged_by(PointerButton::Secondary));
            if slides {
                let delta = response.drag_delta();
                input.pan = glam::Vec2::new(delta.x, delta.y);
            }
        }
        self.camera.fly(&input);
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
        match crate::toolbox::toolbox(ui, column, self.tool, paint) {
            Some(crate::toolbox::Picked::Tool(tool)) => {
                self.tool = tool;
                paint.tool = None;
            }
            Some(crate::toolbox::Picked::Paint(tool)) => paint.tool = Some(tool),
            None => {}
        }
    }
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
pub(crate) const POINTS_PER_NOTCH: f32 = 50.0;

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

#[path = "viewport_pills.rs"]
mod pills;

#[cfg(test)]
#[path = "views_tests.rs"]
mod views_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cursor_readout_is_in_the_game_s_pixels_y_counting_down() {
        let pixels = area::pixels::PixelSettings {
            pixels_per_unit: 16,
            ..Default::default()
        };
        assert_eq!(
            cursor_readout(glam::Vec3::new(12.0, -4.0, 0.0), pixels, Some([12, 4])),
            "x 192 · y 64 px · cell 12, 4"
        );
        assert_eq!(
            cursor_readout(glam::Vec3::new(0.5, 0.25, 0.0), pixels, None),
            "x 8 · y -4 px"
        );
        // A nonsense density counts as one pixel a metre.
        let none = area::pixels::PixelSettings {
            pixels_per_unit: 0,
            ..Default::default()
        };
        assert_eq!(
            cursor_readout(glam::Vec3::new(3.0, 0.0, 0.0), none, None),
            "x 3 · y 0 px"
        );
    }

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
        assert_eq!(view.sky_top, rgb(theme::backdrop::sky_top()));
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

/// The 2D view's frames: what each 2D camera shows of the world at the
/// project's base resolution, dashed, the game's own labelled "Game
/// screen".
fn draw_screens(
    ui: &Ui,
    rect: Rect,
    screens: &[(Vec3, bool)],
    camera: &Camera,
    pixels: area::pixels::PixelSettings,
) {
    let painter = ui.painter_at(rect);
    let size = glam::Vec2::new(rect.width(), rect.height());
    let half =
        glam::Vec2::new(pixels.base[0] as f32, pixels.base[1] as f32) * pixels.pixel_size() * 0.5;
    for &(at, game) in screens {
        let corner = |dx: f32, dy: f32| {
            camera
                .project(Vec3::new(at.x + dx, at.y + dy, 0.0), size)
                .map(|p| rect.min + vec2(p.x, p.y))
        };
        let (Some(top_left), Some(bottom_right)) =
            (corner(-half.x, half.y), corner(half.x, -half.y))
        else {
            continue;
        };
        let frame = Rect::from_two_pos(top_left, bottom_right);
        let colour = if game {
            theme::text_strong()
        } else {
            theme::text_weak()
        };
        let stroke = egui::Stroke::new(1.5, colour);
        for (a, b) in [
            (frame.left_top(), frame.right_top()),
            (frame.right_top(), frame.right_bottom()),
            (frame.right_bottom(), frame.left_bottom()),
            (frame.left_bottom(), frame.left_top()),
        ] {
            painter.add(egui::Shape::dashed_line(&[a, b], stroke, 8.0, 5.0));
        }
        let [w, h] = pixels.base;
        let label = if game {
            format!("Game screen, {w} × {h}")
        } else {
            format!("2D camera, {w} × {h}")
        };
        painter.text(
            frame.left_top() - vec2(0.0, 4.0),
            egui::Align2::LEFT_BOTTOM,
            label,
            egui::FontId::proportional(12.0),
            colour,
        );
    }
}

#[cfg(test)]
#[path = "two_d_tests.rs"]
mod two_d_tests;
