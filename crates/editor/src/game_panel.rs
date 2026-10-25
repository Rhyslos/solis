//! The Game panel: the running game, through the first camera in the
//! area (or the editor's view, with a note, when there's none).

use crate::area_view::AreaView;
use crate::camera::Camera;
use crate::look::Look;
use crate::models::ModelInfo;
use crate::play::PlayState;
use crate::theme;
use crate::viewport::{GAME_IMAGE, ViewportRenderer, ViewportView};
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
    /// What the running `world` shows.
    pub(crate) fn of(
        world: &world::World,
        models: &BTreeMap<AssetId, ModelInfo>,
        state: PlayState,
    ) -> Self {
        Self {
            view: AreaView::of_world(world, models),
            camera: world.camera().map(|(_, camera, global)| (camera, global)),
            state,
            seconds: world.clock().time,
            bolts: std::sync::Arc::new({
                use rayon::prelude::*;
                world
                    .projectiles()
                    .flying()
                    .par_iter()
                    .map(crate::viewport::ViewBolt::of)
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

/// Draws the Game panel: the game's picture with its state in a corner,
/// or, when nothing runs, how to start it.
pub(crate) fn game_panel(
    ui: &mut Ui,
    scene: Option<&GameScene>,
    playing: bool,
    editor_camera: &Camera,
    renderer: &mut dyn ViewportRenderer,
) {
    let Some(scene) = scene else {
        if playing {
            // The first frame after Play, before the panel was in front.
            crate::panels::placeholder(ui, "Starting…");
            ui.ctx().request_repaint();
            return;
        }
        crate::panels::placeholder(
            ui,
            "Press Play (at the top) to run the area here. The game looks through the first \
             camera in the area: Entity > Add camera adds one where you're looking from.",
        );
        return;
    };
    let rect = ui.max_rect();
    ui.allocate_rect(rect, egui::Sense::hover());
    let ppp = ui.ctx().pixels_per_point();
    let size_px = [
        (rect.width() * ppp).round().max(1.0) as u32,
        (rect.height() * ppp).round().max(1.0) as u32,
    ];
    let aspect = size_px[0] as f32 / size_px[1] as f32;
    let (view_projection, position) = match scene.camera {
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
}
