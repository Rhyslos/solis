//! The Game panel: the running game, through the first camera in the
//! area (or the editor's view, with a note, when there's none).

use crate::area_view::AreaView;
use crate::camera::Camera;
use crate::look::Look;
use crate::models::ModelInfo;
use crate::play::PlayState;
use crate::theme;
use crate::viewport::{GAME_IMAGE, PixelView, ViewportRenderer, ViewportView};
use egui::{Color32, Rect, RichText, Ui};
use glam::{Mat4, Vec3};
use reflect::AssetId;
use std::collections::BTreeMap;

/// What the Game panel shows this frame.
#[derive(Debug, Clone)]
pub(crate) struct GameScene {
    /// What to draw.
    pub(crate) view: AreaView,
    /// The camera looked through and where it is, if the area has one.
    pub(crate) camera: Option<(area::builtin::Camera, Mat4)>,
    /// Running or paused.
    pub(crate) state: PlayState,
    /// The game's time, in seconds.
    pub(crate) seconds: f64,
    /// The projectiles in flight.
    pub(crate) bolts: std::sync::Arc<Vec<crate::viewport::ViewBolt>>,
}

impl GameScene {
    /// What the running `world` shows, `state` being whether it runs.
    pub(crate) fn of(
        world: &world::World,
        models: &BTreeMap<AssetId, ModelInfo>,
        state: PlayState,
    ) -> Self {
        // Drawn part way between the last tick and the next, so a screen
        // faster than the ticks shows smooth movement; paused, as it is.
        let t = match state {
            PlayState::Playing => world.progress(),
            _ => 1.0,
        };
        Self {
            view: AreaView::of_world(world, models, t),
            camera: world
                .camera_between(t)
                .map(|(_, camera, global)| (camera, global)),
            state,
            seconds: world.clock().time,
            bolts: std::sync::Arc::new({
                use rayon::prelude::*;
                world
                    .projectiles()
                    .flying()
                    .par_iter()
                    .map(|p| crate::viewport::ViewBolt::between(p, t))
                    .collect()
            }),
        }
    }
}

/// The view through a camera entity at `global`: its -Z ahead, +Y up.
/// Scale is left out, so a stretched entity doesn't stretch the picture.
pub(crate) fn camera_view(global: Mat4) -> (Mat4, Vec3) {
    let (_, rotation, position) = global.to_scale_rotation_translation();
    let rotation = if rotation.is_finite() {
        rotation.normalize()
    } else {
        glam::Quat::IDENTITY
    };
    let position = if position.is_finite() {
        position
    } else {
        Vec3::ZERO
    };
    let view = Mat4::from_rotation_translation(rotation, position).inverse();
    (view, position)
}

/// What a 2D camera at `position` (in metres) sees with the project's 2D
/// `settings`: the view and projection for the small picture (the base
/// resolution plus a pixel each side, on whole pixels of the world), and
/// the pixel stage's slide for the bit of a pixel the camera is past
/// that. `near` and `far` are how far below the camera it sees.
pub(crate) fn pixel_camera(
    position: Vec3,
    settings: area::pixels::PixelSettings,
    near: f32,
    far: f32,
) -> (Mat4, PixelView) {
    let s = settings.sanitized();
    let ppu = s.pixels_per_unit as f32;
    let position = if position.is_finite() {
        position
    } else {
        Vec3::ZERO
    };
    // Where the camera is, in whole pixels and the bit beyond.
    let at = [position.x * ppu, position.y * ppu];
    let whole = at.map(|v| v.round());
    let past = [at[0] - whole[0], at[1] - whole[1]];
    // The picture's pixels line up with the world's: its left and bottom
    // edges sit on whole pixels, the middle on the camera's whole pixel.
    let size = [s.base[0] + 2, s.base[1] + 2];
    let left = whole[0] - (size[0] / 2) as f32;
    let bottom = whole[1] - (size[1] / 2) as f32;
    let centre = Vec3::new(
        (left + size[0] as f32 / 2.0) / ppu,
        (bottom + size[1] as f32 / 2.0) / ppu,
        position.z,
    );
    let (near, far) = if near.is_finite() && far.is_finite() && far > near {
        (near, far)
    } else {
        (0.1, 2000.0)
    };
    let range = far - near;
    let half = [size[0] as f32 / ppu / 2.0, size[1] as f32 / ppu / 2.0];
    // Flat, depth reversed (1 at near, 0 at far), as the 3D camera has it.
    let projection = Mat4::from_cols(
        glam::Vec4::new(1.0 / half[0], 0.0, 0.0, 0.0),
        glam::Vec4::new(0.0, 1.0 / half[1], 0.0, 0.0),
        glam::Vec4::new(0.0, 0.0, 1.0 / range, 0.0),
        glam::Vec4::new(0.0, 0.0, far / range, 1.0),
    );
    // Looking straight down, X right and Y up the picture: the camera's
    // own axes are the world's, so its view only moves the world.
    let view = Mat4::from_translation(-centre);
    (
        projection * view,
        PixelView {
            base: s.base,
            scaling: s.scaling,
            // Picture x runs with the world's; picture y runs down, the
            // world's up.
            slide: [past[0], -past[1]],
        },
    )
}

/// Draws the Game panel: the game's picture with its state in a corner,
/// or, when nothing runs, how to start it. Returns whether the picture
/// was clicked (the game then takes the mouse and keys); `has_input` is
/// whether it has them.
#[allow(clippy::too_many_arguments)]
pub(crate) fn game_panel(
    ui: &mut Ui,
    scene: Option<&GameScene>,
    playing: bool,
    has_input: bool,
    editor_camera: &Camera,
    pixels: area::pixels::PixelSettings,
    sheets: &crate::sprite_sheets::SpriteSheets,
    renderer: &mut dyn ViewportRenderer,
) -> bool {
    let Some(scene) = scene else {
        if playing {
            // The first frame after Play, before the panel was in front.
            crate::panels::placeholder(ui, "Starting…");
            ui.ctx().request_repaint();
            return false;
        }
        crate::panels::placeholder(
            ui,
            "Press Play (at the top) to run the area here. The game looks through the first \
             camera in the area: Entity > Add camera adds one where you're looking from.",
        );
        return false;
    };
    let rect = ui.max_rect();
    let clicked = ui.allocate_rect(rect, egui::Sense::click()).clicked();
    let ppp = ui.ctx().pixels_per_point();
    let size_px = [
        (rect.width() * ppp).round().max(1.0) as u32,
        (rect.height() * ppp).round().max(1.0) as u32,
    ];
    let aspect = size_px[0] as f32 / size_px[1] as f32;
    let mut pixel = None;
    let (view_projection, position) = match scene.camera {
        Some((camera, global)) if camera.view == area::builtin::CameraView::TwoD => {
            let position = global.w_axis.truncate();
            let c = camera.sanitized();
            let (view_projection, view) = pixel_camera(position, pixels, c.near, c.far);
            pixel = Some(view);
            (view_projection, position)
        }
        Some((camera, global)) => {
            let (view, position) = camera_view(global);
            (camera.projection(aspect) * view, position)
        }
        // The editor's view, as seen in perspective even from a flat view
        // (the game has no flat views).
        None => {
            let seen = editor_camera.as_kind(crate::camera::ViewKind::Perspective);
            (seen.view_projection(aspect), seen.position)
        }
    };
    let view = ViewportView {
        size_px,
        view_projection,
        camera_position: position,
        focus: position,
        sky_top: theme::backdrop::rgb(theme::backdrop::SKY_TOP),
        sky_horizon: theme::backdrop::rgb(theme::backdrop::HORIZON),
        ground: theme::backdrop::rgb(theme::backdrop::GROUND),
        models: scene.view.models.clone(),
        // The game's own look, whatever the viewport uses.
        look: Look::default(),
        outline_px: 0.0,
        sun: scene.view.sun,
        lamps: scene.view.lamps.clone(),
        environment: scene.view.environment,
        pick: None,
        image: GAME_IMAGE,
        editor_lines: false,
        kind: crate::camera::ViewKind::Perspective,
        bolts: std::sync::Arc::clone(&scene.bolts),
        grid_step: 1.0,
        // Through a 2D camera, sprites land on whole pixels.
        sprites: std::sync::Arc::new(crate::sprite_sheets::batches(
            &scene.view.sprites,
            sheets,
            pixels.pixel_size(),
            pixel.map(|_| pixels.pixel_size()),
            0.0,
        )),
        pixel,
    };
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
            painter.rect_filled(rect, 0.0, theme::backdrop::HORIZON);
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("No 3D view: {reason}"),
                egui::FontId::proportional(13.0),
                theme::TEXT_WEAK,
            );
        }
    }
    let state = match scene.state {
        PlayState::Paused => "Paused",
        _ => "Playing",
    };
    let mut notes = vec![format!("{state} · {:.1} s", scene.seconds)];
    if scene.camera.is_none() {
        notes.push("No camera in the area: showing the editor's view".into());
    }
    notes.push(if has_input {
        "The game has the mouse and keys · Esc gives them back".into()
    } else {
        "Click to give the game the mouse and keys".into()
    });
    let mut overlay = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink(8.0))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    overlay.spacing_mut().item_spacing.y = 4.0;
    for note in notes {
        egui::Frame::new()
            .fill(Color32::from_black_alpha(150))
            .corner_radius(4)
            .inner_margin(egui::Margin::symmetric(6, 3))
            .show(&mut overlay, |ui| {
                ui.label(RichText::new(note).size(12.0).color(theme::TEXT_STRONG));
            });
    }
    clicked
}

#[cfg(test)]
mod pixel_tests {
    use super::*;
    use area::pixels::PixelSettings;

    /// Where world point `p` lands in the small picture, in its pixels
    /// from the top-left (the picture is the base plus a pixel each side).
    fn in_picture(view_projection: Mat4, settings: PixelSettings, p: Vec3) -> [f32; 2] {
        let size = [settings.base[0] + 2, settings.base[1] + 2];
        let clip = view_projection.project_point3(p);
        [
            (clip.x + 1.0) / 2.0 * size[0] as f32,
            (1.0 - clip.y) / 2.0 * size[1] as f32,
        ]
    }

    fn whole(v: [f32; 2]) -> bool {
        v.iter().all(|c| (c - c.round()).abs() < 1e-3)
    }

    #[test]
    fn the_world_s_pixels_land_on_the_picture_s_pixels() {
        for base in [[480, 270], [481, 271], [320, 180]] {
            let s = PixelSettings {
                base,
                ..PixelSettings::default()
            };
            for camera in [
                Vec3::ZERO,
                Vec3::new(10.0, -3.0, 20.0),
                Vec3::new(10.03, 7.77, 5.0),
            ] {
                let (vp, _) = pixel_camera(camera, s, 0.1, 100.0);
                // Corners of the world's pixels near the camera are corners
                // of the picture's.
                for (dx, dy) in [(0, 0), (3, -2), (-40, 17)] {
                    let p = Vec3::new(
                        ((camera.x * 16.0).round() + dx as f32) / 16.0,
                        ((camera.y * 16.0).round() + dy as f32) / 16.0,
                        0.0,
                    );
                    assert!(whole(in_picture(vp, s, p)), "{base:?} {camera} {dx},{dy}");
                }
            }
        }
    }

    #[test]
    fn a_camera_between_pixels_draws_the_same_picture_and_slides_it() {
        let s = PixelSettings::default();
        let (still, view) = pixel_camera(Vec3::new(5.0, 5.0, 10.0), s, 0.1, 100.0);
        assert_eq!(view.slide, [0.0, 0.0]);
        assert_eq!(view.base, [480, 270]);
        // A third of a pixel right and a quarter up: the same small picture,
        // slid right and up.
        let moved = Vec3::new(5.0 + 0.33 / 16.0, 5.0 + 0.25 / 16.0, 10.0);
        let (vp, view) = pixel_camera(moved, s, 0.1, 100.0);
        assert!((vp - still).abs_diff_eq(Mat4::ZERO, 1e-5));
        assert!((view.slide[0] - 0.33).abs() < 1e-3);
        assert!((view.slide[1] + 0.25).abs() < 1e-3, "picture y runs down");
        // The camera's whole pixel is in the middle of the picture.
        let middle = in_picture(still, s, Vec3::new(5.0, 5.0, 0.0));
        assert_eq!(middle.map(f32::round), [241.0, 136.0]);
    }

    #[test]
    fn it_sees_from_near_to_far_below_the_camera_and_never_breaks() {
        let s = PixelSettings::default();
        let (vp, _) = pixel_camera(Vec3::new(0.0, 0.0, 10.0), s, 1.0, 101.0);
        let depth = |z: f32| vp.project_point3(Vec3::new(0.0, 0.0, z)).z;
        assert!((depth(9.0) - 1.0).abs() < 1e-5, "near: 1");
        assert!(depth(-91.0).abs() < 1e-5, "far: 0");
        let (vp, view) = pixel_camera(Vec3::splat(f32::NAN), s, f32::NAN, -1.0);
        assert!(vp.is_finite() && view.slide == [0.0, 0.0]);
    }
}
