//! An extrude or inset following the mouse, frame by frame: pulling out
//! along the faces' way, snapped to the grid or free with Ctrl, pushing
//! in, a click keeping and a right-click giving up, the camera looking
//! along the way, an inset's width, and only the moving points changing.

use super::*;
use crate::camera::ViewKind;
use mesh::Shape;

const SIZE: Vec2 = Vec2::new(800.0, 600.0);
const GRID: f32 = 0.5;

fn cube() -> Mesh {
    Shape::Box.build(&Shape::Box.standard())
}

fn top(mesh: &Mesh) -> FaceId {
    mesh.faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).z > 0.99)
        .unwrap()
}

fn at(camera: &Camera, p: Vec3) -> Vec2 {
    camera.project(p, SIZE).expect("in front of the camera")
}

fn moved_to(mouse: Vec2, ctrl: bool) -> PullInput {
    PullInput {
        mouse: Some(mouse),
        ctrl,
        on_view: true,
        ..PullInput::default()
    }
}

/// The highest point of the pull's brush now.
fn height(pull: &Pull) -> f32 {
    pull.now().bounds().unwrap().max.z
}

#[test]
fn the_mouse_pulls_the_top_out_snapped_to_the_grid_or_free_with_ctrl() {
    let mesh = cube();
    let camera = Camera::default();
    let mut pull = Pull::extrude(&mesh, &[top(&mesh)], Mat4::IDENTITY, "1 face".into()).unwrap();
    assert_eq!(pull.moving_points().len(), 4);
    assert_eq!(pull.now().face_count(), 10, "the walls made at once");
    let from = at(&camera, Vec3::new(0.0, 0.0, 2.0));
    let view = (&camera, SIZE);
    assert_eq!(
        pull.update(moved_to(from, false), view, GRID),
        PullOutcome::Going(false)
    );
    let up = at(&camera, Vec3::new(0.0, 0.0, 3.13));
    assert_eq!(
        pull.update(moved_to(up, false), view, GRID),
        PullOutcome::Going(true)
    );
    assert_eq!(pull.metres, 1.0, "snapped to the grid");
    assert!((height(&pull) - 3.0).abs() < 1e-4);
    pull.update(moved_to(up, true), view, GRID);
    assert!((pull.metres - 1.13).abs() < 0.02, "{}", pull.metres);
    assert!(pull.free);
    assert_eq!(
        pull.top_words(),
        format!("Extruding 1 face \u{b7} {:.2} m", pull.metres)
    );
    // Pushed in.
    let down = at(&camera, Vec3::new(0.0, 0.0, 1.0));
    pull.update(moved_to(down, false), view, GRID);
    assert_eq!(pull.metres, -1.0);
    assert!((height(&pull) - 2.0).abs() < 1e-4, "the walls go down");
    assert_eq!(pull.label(), "Extrude 1 face");
}

#[test]
fn a_click_keeps_a_right_click_gives_up_and_a_click_off_the_view_doesnt_keep() {
    let mesh = cube();
    let camera = Camera::default();
    let view = (&camera, SIZE);
    let mut pull = Pull::extrude(&mesh, &[top(&mesh)], Mat4::IDENTITY, "1 face".into()).unwrap();
    let mouse = at(&camera, Vec3::new(0.0, 0.0, 2.0));
    let click = PullInput {
        pressed: true,
        ..moved_to(mouse, false)
    };
    assert_eq!(pull.update(click, view, GRID), PullOutcome::Keep);
    let off = PullInput {
        on_view: false,
        ..click
    };
    assert_eq!(pull.update(off, view, GRID), PullOutcome::Going(false));
    let right = PullInput {
        right_pressed: true,
        ..moved_to(mouse, false)
    };
    assert_eq!(pull.update(right, view, GRID), PullOutcome::GiveUp);
    let none = PullInput::default();
    assert_eq!(pull.update(none, view, GRID), PullOutcome::Going(false));
}

#[test]
fn looking_straight_down_the_way_the_mouse_goes_up_the_screen() {
    let mesh = cube();
    let camera = Camera::default().as_kind(ViewKind::Top);
    let view = (&camera, SIZE);
    let mut pull = Pull::extrude(&mesh, &[top(&mesh)], Mat4::IDENTITY, "1 face".into()).unwrap();
    let middle = SIZE / 2.0;
    pull.update(moved_to(middle, true), view, GRID);
    pull.update(moved_to(middle - Vec2::Y * 40.0, true), view, GRID);
    assert!(
        pull.metres > 0.0,
        "up the screen pulls out: {}",
        pull.metres
    );
}

#[test]
fn an_inset_s_width_grows_as_the_pointer_moves_away_never_below_nothing() {
    let mesh = cube();
    let camera = Camera::default();
    let view = (&camera, SIZE);
    let face = top(&mesh);
    let mut pull = Pull::inset(&mesh, &[face], Mat4::IDENTITY, "1 face".into()).unwrap();
    assert_eq!(pull.kind, PullKind::Inset);
    assert_eq!(pull.now().face_count(), 10, "the border's four faces");
    let from = at(&camera, Vec3::new(0.0, 0.0, 2.0));
    pull.update(moved_to(from, false), view, GRID);
    let away = at(&camera, Vec3::new(0.0, 0.0, 2.0) + camera.right() * 0.6);
    pull.update(moved_to(away, false), view, GRID);
    assert_eq!(pull.metres, 0.5);
    let corners = pull.now().face(face).unwrap().to_vec();
    let size = mesh::Bounds::around(corners.iter().map(|v| pull.now().position(*v).unwrap()))
        .unwrap()
        .size();
    assert!(
        (size.x - 1.0).abs() < 1e-4 && (size.y - 1.0).abs() < 1e-4,
        "{size}"
    );
    assert!(pull.status_words().starts_with("Insetting 1 face: 0.50 m"));
}

#[test]
fn a_brush_scaled_in_the_world_pulls_in_world_metres() {
    let mesh = cube();
    let camera = Camera::default();
    let view = (&camera, SIZE);
    let world = Mat4::from_scale(Vec3::splat(2.0));
    let mut pull = Pull::extrude(&mesh, &[top(&mesh)], world, "1 face".into()).unwrap();
    pull.set(2.0);
    assert!(
        (height(&pull) - 3.0).abs() < 1e-4,
        "2 m is 1 unit of the brush"
    );
    let _ = view;
}

#[test]
fn nothing_to_pull_makes_no_pull_and_only_the_moving_points_change() {
    let mesh = cube();
    assert!(
        Pull::extrude(
            &mesh,
            &[FaceId::from_raw(99)],
            Mat4::IDENTITY,
            String::new()
        )
        .is_none()
    );
    let mut pull = Pull::extrude(&mesh, &[top(&mesh)], Mat4::IDENTITY, String::new()).unwrap();
    pull.set(1.5);
    let moving: Vec<VertexId> = pull.moving_points();
    for (v, p) in mesh.vertices() {
        if !moving.contains(&v) {
            assert_eq!(pull.now().position(v), Some(p));
        }
    }
    pull.set(f32::NAN);
    assert_eq!(pull.metres, 1.5, "a broken distance is ignored");
}
