//! The 2D view and the project's 2D settings (Phase 9, D1): whole-step
//! zoom, the world's pixels on whole screen pixels, the corner's grid,
//! snap and zoom, snapping moves to pixels or tiles, 2D cameras and their
//! frames, and Settings > 2D.

use super::*;
use crate::camera::{FIRST_ZOOM, ZOOM_STEPS};
use crate::test_support::*;
use crate::transforming::{PixelSnap, Snapping, Targets, snapped_in_2d};
use area::pixels::{PixelSettings, Scaling};
use egui_kittest::kittest::Queryable;

fn two_d(position: Vec3, zoom: f32) -> Camera {
    Camera {
        position,
        zoom,
        ..Camera::default().as_kind(ViewKind::TwoD)
    }
}

/// Where world point `p` lands in a view of `size` pixels (one pixel a
/// point).
fn on_screen(camera: &Camera, p: Vec3, size: [u32; 2]) -> glam::Vec2 {
    camera
        .project(p, glam::Vec2::new(size[0] as f32, size[1] as f32))
        .unwrap()
}

fn whole(v: glam::Vec2) -> bool {
    (v.x - v.x.round()).abs() < 2e-3 && (v.y - v.y.round()).abs() < 2e-3
}

#[test]
fn the_2d_view_puts_the_world_s_pixels_on_whole_screen_pixels() {
    for size in [[800, 600], [801, 601], [1107, 611]] {
        for zoom in [1.0, 2.0, 3.0, 5.0] {
            for at in [
                Vec3::ZERO,
                Vec3::new(3.017, -2.33, 40.0),
                Vec3::new(-100.5, 7.031_25, 1.0),
            ] {
                let seen = two_d(at, zoom).seen(size, 16);
                // As tall as the zoom says.
                let height = size[1] as f32 / (16.0 * zoom);
                assert!((seen.flat_height - height).abs() < 1e-4);
                // The world's pixel corners near the middle land on screen
                // pixel corners.
                for (dx, dy) in [(0.0, 0.0), (5.0, -3.0), (-17.0, 11.0)] {
                    let p = Vec3::new(
                        (at.x * 16.0).round() / 16.0 + dx / 16.0,
                        (at.y * 16.0).round() / 16.0 + dy / 16.0,
                        0.0,
                    );
                    let s = on_screen(&seen, p, size);
                    assert!(whole(s), "{size:?} ×{zoom} {at}: {s}");
                }
                // Within a screen pixel of where it was asked to be.
                assert!((seen.position - at).truncate().length() <= 1.0 / (16.0 * zoom));
            }
        }
    }
    // Any other kind of view is left as it is.
    let top = Camera::default().as_kind(ViewKind::Top);
    assert_eq!(top.seen([800, 600], 16), top);
}

#[test]
fn zoom_goes_in_whole_steps_and_a_damaged_one_is_put_right() {
    assert_eq!(Camera::zoom_step(2.0, 1), 3.0);
    assert_eq!(Camera::zoom_step(2.0, -2), 0.5);
    assert_eq!(Camera::zoom_step(24.0, 3), 24.0);
    assert_eq!(Camera::zoom_step(0.25, -9), 0.25);
    assert_eq!(
        Camera::zoom_step(7.0, 0),
        8.0,
        "an odd one lands on the next"
    );
    assert!(ZOOM_STEPS.contains(&FIRST_ZOOM));
    let damaged = Camera {
        zoom: 7.5,
        ..Camera::default()
    };
    assert_eq!(damaged.sanitized().zoom, FIRST_ZOOM);
}

#[test]
fn the_wheel_zooms_the_2d_view_a_step_a_notch_towards_the_mouse() {
    let (mut w, _) = with_a_crate();
    w.viewport.camera = two_d(Vec3::new(1.0, 2.0, 0.0), 2.0);
    let mut h = harness(w);
    h.run();
    let rect = h.state().0.viewport.rect;
    let at = rect.center() + egui::vec2(60.0, -30.0);
    h.hover_at(at);
    h.run();
    let size = [rect.width().round() as u32, rect.height().round() as u32];
    let before = h.state().0.camera();
    let under = before.ray_through(
        glam::Vec2::new(at.x - rect.min.x, at.y - rect.min.y),
        glam::Vec2::new(rect.width(), rect.height()),
    );
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, 1.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
    h.run();
    h.run();
    let after = h.state().0.camera();
    assert_eq!(after.zoom, 3.0);
    assert!((after.flat_height - size[1] as f32 / 48.0).abs() < 0.05);
    // The point under the mouse stays (nearly) under it.
    let still = after.ray_through(
        glam::Vec2::new(at.x - rect.min.x, at.y - rect.min.y),
        glam::Vec2::new(rect.width(), rect.height()),
    );
    let drift = (still.origin - under.origin).truncate().length();
    assert!(drift < 0.1, "{drift}");
}

#[test]
fn choosing_2d_shows_the_tile_grid_and_the_2d_corner() {
    let (w, _) = with_a_crate();
    let mut h = harness(w);
    h.run();
    h.query_all_by_label("Perspective ▴")
        .next()
        .unwrap()
        .click();
    h.run();
    h.get_by_label("2D  flat, in pixels").click();
    h.run();
    h.run();
    assert_eq!(h.state().0.camera().kind, ViewKind::TwoD);
    let view = h.state().1.views.last().unwrap().clone();
    assert_eq!(view.kind, ViewKind::TwoD);
    assert_eq!(view.grid_step, 1.0, "16-pixel tiles at 16 pixels a metre");
    for label in [
        "Grid 16 px",
        "Snap: whole pixels ▴",
        "Zoom 200% ▴",
        "Lock X",
        "Lock Y",
    ] {
        assert!(h.query_by_label(label).is_some(), "{label}");
    }
    // Angle and camera speed belong to the other views.
    assert!(h.query_by_label("Lock Z").is_none());
    assert!(h.query_by_label("Angle 15°").is_none());
    // Other tile sizes show as they are.
    h.state_mut().0.set_pixels(PixelSettings {
        tile: [32, 16],
        ..PixelSettings::default()
    });
    h.run();
    assert!(h.query_by_label("Grid 32 × 16 px").is_some());
    let view = h.state().1.views.last().unwrap().clone();
    assert_eq!(view.grid_step, 2.0);
    // The zoom menu sets the zoom.
    h.get_by_label("Zoom 200% ▴").click();
    h.run();
    h.get_by_label("400%").click();
    h.run();
    assert_eq!(h.state().0.camera().zoom, 4.0);
    // And the snap menu what moves land on.
    h.get_by_label("Snap: whole pixels ▴").click();
    h.run();
    h.get_by_label("Tiles").click();
    h.run();
    assert_eq!(h.state().0.viewport.snapping.pixel_snap, PixelSnap::Tiles);
}

/// One entity at `at`, to move.
fn targets(at: Vec3, axes: [Vec3; 3]) -> Targets {
    Targets {
        entities: Vec::new(),
        pivot: at,
        axes,
    }
}

const WORLD: [Vec3; 3] = [Vec3::X, Vec3::Y, Vec3::Z];

#[test]
fn moves_in_the_2d_view_land_on_pixels_or_tiles() {
    use crate::gizmo::Change;
    let s = PixelSettings {
        tile: [32, 16],
        ..PixelSettings::default()
    };
    let t = targets(Vec3::new(0.01, 0.02, 3.0), WORLD);
    let moved = |snap: PixelSnap, flip: bool, v: Vec3| {
        let snapping = Snapping {
            pixel_snap: snap,
            ..Snapping::default()
        };
        match snapped_in_2d(Change::Translate(v), &t, &snapping, s, flip) {
            Change::Translate(d) => t.pivot + d,
            other => panic!("{other:?}"),
        }
    };
    let v = Vec3::new(1.03, 0.51, 0.0);
    // Whole pixels: sixteenths of a metre.
    let p = moved(PixelSnap::Pixels, false, v);
    assert!((p.x * 16.0 - (p.x * 16.0).round()).abs() < 1e-4);
    assert!((p.y * 16.0 - (p.y * 16.0).round()).abs() < 1e-4);
    assert_eq!(p.z, 3.0, "height is left alone");
    // Tiles: two metres across, one up.
    let p = moved(PixelSnap::Tiles, false, v);
    assert!((p.x - 2.0).abs() < 1e-5 && (p.y - 1.0).abs() < 1e-5, "{p}");
    // Off: where it was dragged; Ctrl flips it either way.
    assert_eq!(moved(PixelSnap::Off, false, v), t.pivot + v);
    assert_eq!(moved(PixelSnap::Tiles, true, v), t.pivot + v);
    let p = moved(PixelSnap::Off, true, v);
    assert!((p.x * 16.0 - (p.x * 16.0).round()).abs() < 1e-4);
    // An axis that didn't move stays where it was.
    let p = moved(PixelSnap::Tiles, false, Vec3::new(1.03, 0.0, 0.0));
    assert_eq!(p.y, t.pivot.y);
    // The entity's own axes: steps along them.
    let turned = [Vec3::new(0.6, 0.8, 0.0), Vec3::new(-0.8, 0.6, 0.0), Vec3::Z];
    let t = targets(Vec3::ZERO, turned);
    let snapping = Snapping::default();
    let Change::Translate(d) =
        snapped_in_2d(Change::Translate(turned[0] * 0.53), &t, &snapping, s, false)
    else {
        panic!()
    };
    assert!((d.dot(turned[0]) * 16.0 - (d.dot(turned[0]) * 16.0).round()).abs() < 1e-4);
    // Turning and scaling snap as they always do.
    let turn = Change::Rotate {
        axis: Vec3::Z,
        angle: 0.3,
    };
    assert_eq!(
        snapped_in_2d(turn, &t, &snapping, s, false),
        crate::transforming::snapped(turn, &t, &snapping, false)
    );
}

#[test]
fn a_camera_added_in_the_2d_view_is_a_2d_camera_on_a_whole_pixel_and_is_framed() {
    let mut w = crate::workbench::Workbench::new(info(), None);
    w.viewport.camera = two_d(Vec3::new(3.01, -1.47, 0.0), 2.0);
    let id = w.add_camera();
    let entity = w.document.area().get(id).unwrap();
    let camera = entity.get::<area::builtin::Camera>().unwrap();
    assert_eq!(camera.view, area::builtin::CameraView::TwoD);
    let at = entity.get::<area::builtin::Transform>().unwrap().position;
    assert_eq!((at.x * 16.0, at.y * 16.0), (48.0, -24.0));
    assert!(at.z > 10.0, "above what it sees");
    // The 2D view frames what it shows; it's the game's own.
    let view = crate::area_view::AreaView::of(w.document.area(), &w.models);
    assert_eq!(view.screens, vec![(at, true)]);
    // A second 2D camera is framed too, but isn't the game's.
    let second = w.add_camera();
    let view = crate::area_view::AreaView::of(w.document.area(), &w.models);
    assert_eq!(view.screens.len(), 2);
    assert_eq!(view.screens.iter().filter(|(_, game)| *game).count(), 1);
    let _ = second;
    // From a 3D view, a camera is as it was.
    w.viewport.camera = Camera::default();
    let third = w.add_camera();
    let camera = w
        .document
        .area()
        .get(third)
        .unwrap()
        .get::<area::builtin::Camera>()
        .unwrap();
    assert_eq!(camera.view, area::builtin::CameraView::Perspective);
}

/// Opens Edit > Settings… on the 2D page, with a project open or not.
fn settings_2d(project: bool) -> egui_kittest::Harness<'static, State> {
    let mut w = crate::workbench::Workbench::new(info(), None);
    if project {
        w.set_project(Some(crate::library::ProjectInfo {
            name: "Farm".into(),
            root: std::env::temp_dir().join("solis-two-d-settings"),
            assets: std::env::temp_dir().join("solis-two-d-settings/assets"),
        }));
    }
    w.open_settings();
    let mut h = harness(w);
    h.run();
    h.get_by_label("2D").click();
    h.run();
    h
}

fn saved(h: &egui_kittest::Harness<'_, State>) -> Vec<PixelSettings> {
    h.state()
        .1
        .actions
        .iter()
        .filter_map(|a| match a {
            crate::workbench::WorkbenchAction::SavePixels(s) => Some(*s),
            _ => None,
        })
        .collect()
}

#[test]
fn the_2d_settings_page_changes_and_saves_the_project_s_settings() {
    let mut h = settings_2d(true);
    for label in [
        "Base resolution",
        "Pixels per unit",
        "Tile size",
        "Scaling to the screen",
        "This project",
    ] {
        assert!(h.query_by_label(label).is_some(), "{label}");
    }
    // A common size from the menu.
    let combos = || egui::accesskit::Role::ComboBox;
    h.get_all_by_role(combos()).next().unwrap().click();
    h.run();
    h.get_by_label("640 × 360").click();
    h.run();
    assert_eq!(h.state().0.pixels().base, [640, 360]);
    // Scaling.
    h.get_all_by_role(combos()).nth(1).unwrap().click();
    h.run();
    h.get_by_label("Fill the screen").click();
    h.run();
    assert_eq!(h.state().0.pixels().scaling, Scaling::Fill);
    let saves = saved(&h);
    assert_eq!(saves.last().unwrap().base, [640, 360]);
    assert_eq!(saves.last().unwrap().scaling, Scaling::Fill);
    // The viewport's 2D view follows at once.
    assert_eq!(h.state().0.viewport.pixels.base, [640, 360]);
    // Reset this page: the standard ones, saved.
    h.get_by_label("Reset this page").click();
    h.run();
    assert_eq!(h.state().0.pixels(), PixelSettings::default());
    assert_eq!(*saved(&h).last().unwrap(), PixelSettings::default());
}

#[test]
fn without_a_project_the_2d_settings_can_t_be_changed() {
    let mut h = settings_2d(false);
    assert!(
        h.query_by_label("These belong to a project: open or make one to change them.")
            .is_some()
    );
    h.get_by_label("Reset this page").click();
    h.run();
    assert!(saved(&h).is_empty());
}

#[test]
fn the_2d_page_says_how_the_base_resolution_fits_common_screens() {
    let fits = crate::settings_window::fits(PixelSettings::default());
    assert_eq!(
        fits,
        "480 × 270 on 1080p (×4), 1440p (×5 with bars) and 4K (×8)."
    );
    let fill = crate::settings_window::fits(PixelSettings {
        scaling: Scaling::Fill,
        ..PixelSettings::default()
    });
    assert!(fill.contains("1440p (×5.33, smoothed)"), "{fill}");
    // Search finds the 2D settings by name and page.
    let mut w = crate::workbench::Workbench::new(info(), None);
    w.open_settings();
    w.settings_window.as_mut().unwrap().search = "tile".into();
    let mut h = harness(w);
    h.run();
    assert!(h.query_by_label("Tile size").is_some());
    assert!(h.query_by_label("Accent colour").is_none());
}
