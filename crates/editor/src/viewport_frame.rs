//! What the viewport hands its renderer each frame (`Viewport::view`),
//! and the renderer's side: the [`ViewportRenderer`] trait, what it
//! reports back, and the images it draws into.

use super::Viewport;
use crate::area_view::{ViewLight, ViewModel};
use crate::camera::ViewKind;
use crate::look::Look;
use crate::theme;
use crate::viewport_tools::PickAnswer;
use glam::{Mat4, Vec3};

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
    /// The brushes to draw, with their pick numbers (Tools 1a, B1).
    pub brushes: Vec<crate::brushes::ViewBrush>,
    /// How models are shaded.
    pub look: Look,
    /// The stylized look's outline width in pixels; 0 for none.
    pub outline_px: f32,
    /// The pick number of the one model or brush drawn as it is while
    /// every other is dimmed: the brush whose insides are being edited
    /// (Tools 1a, B3).
    pub dim_all_but: Option<u32>,
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

impl Viewport {
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
            crate::tile_view::Viewer::View(self.active),
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
            brushes: self.area.brushes.clone(),
            look: self.look,
            outline_px: self.look.outline_px(pixels_per_point),
            dim_all_but: self.brushing.edit.as_ref().and_then(|edit| {
                self.area
                    .brushes
                    .iter()
                    .find(|b| b.entity == edit.entity && b.pick > 0)
                    .map(|b| b.pick)
            }),
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
}
