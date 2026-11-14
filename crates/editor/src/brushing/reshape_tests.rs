//! A drag of a brush's parts and a size typed: each gizmo on points,
//! edges and faces, a placed brush's own axes, nothing moved, refused
//! changes, and stretching about the middle.

use super::*;
use crate::brushing::edit::{Part, points_of};
use crate::transforming::WORLD_AXES;
use mesh::Shape;
use std::collections::BTreeSet;

fn cube() -> Arc<Mesh> {
    Arc::new(Shape::Box.build(&Shape::Box.standard()))
}

/// The box's top face, as a pick.
fn top(mesh: &Mesh) -> BTreeSet<Part> {
    let face = mesh
        .faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).z > 0.99)
        .unwrap();
    BTreeSet::from([Part::Face(face)])
}

fn drag_of(mesh: &Arc<Mesh>, picked: &BTreeSet<Part>, world: Mat4) -> Drag {
    let points = points_of(mesh, picked);
    let pivot = pivot_of(mesh, &points, world).unwrap();
    Drag::begin(Arc::clone(mesh), points, (pivot, WORLD_AXES), world)
}

#[test]
fn the_pivot_is_the_middle_of_what_is_picked() {
    let mesh = cube();
    let points = points_of(&mesh, &top(&mesh));
    assert_eq!(points.len(), 4);
    assert_eq!(
        pivot_of(&mesh, &points, Mat4::IDENTITY),
        Some(Vec3::new(0.0, 0.0, 2.0))
    );
    let placed = Mat4::from_translation(Vec3::new(5.0, 0.0, 1.0));
    assert_eq!(
        pivot_of(&mesh, &points, placed),
        Some(Vec3::new(5.0, 0.0, 3.0))
    );
    assert_eq!(pivot_of(&mesh, &[], Mat4::IDENTITY), None);
}

#[test]
fn moving_the_top_face_moves_its_points_and_only_them() {
    let mesh = cube();
    let mut drag = drag_of(&mesh, &top(&mesh), Mat4::IDENTITY);
    drag.set(Change::Translate(Vec3::Z));
    let now = drag.finish().unwrap().unwrap();
    let size = now.bounds().unwrap().size();
    assert_eq!(size, Vec3::new(2.0, 2.0, 3.0));
    assert_eq!(drag.label("1 face"), "Move 1 face");
    // Each frame starts from where the drag began: a second set replaces
    // the first.
    drag.set(Change::Translate(Vec3::Z * 0.5));
    assert_eq!(drag.now().bounds().unwrap().size().z, 2.5);
}

#[test]
fn turning_and_stretching_work_on_points_edges_and_faces() {
    let mesh = cube();
    // An edge of the top turned a little about its middle.
    let edge = mesh
        .edges()
        .iter()
        .copied()
        .find(|e| {
            let (a, b) = e.ends();
            mesh.position(a).unwrap().z > 1.0 && mesh.position(b).unwrap().z > 1.0
        })
        .unwrap();
    let mut drag = drag_of(&mesh, &BTreeSet::from([Part::Edge(edge)]), Mat4::IDENTITY);
    drag.set(Change::Rotate {
        axis: Vec3::Z,
        angle: 0.2,
    });
    assert!(drag.finish().unwrap().is_some(), "a small turn is fine");
    assert_eq!(drag.label("1 edge"), "Rotate 1 edge");
    // The top stretched to twice its width.
    let mut drag = drag_of(&mesh, &top(&mesh), Mat4::IDENTITY);
    drag.set(Change::Scale(Vec3::new(2.0, 1.0, 1.0)));
    let now = drag.finish().unwrap().unwrap();
    assert_eq!(now.bounds().unwrap().size().x, 4.0);
    // One point moved up.
    let point = mesh.vertices().next().unwrap().0;
    let mut drag = drag_of(&mesh, &BTreeSet::from([Part::Point(point)]), Mat4::IDENTITY);
    drag.set(Change::Translate(Vec3::new(0.0, 0.0, -0.3)));
    let now = drag.finish().unwrap().unwrap();
    assert_eq!(
        now.position(point).unwrap(),
        mesh.position(point).unwrap() - Vec3::Z * 0.3
    );
}

#[test]
fn a_placed_brush_moves_by_the_world_s_change_in_its_own_space() {
    // The entity stretched twice as tall: a metre up in the world is half
    // a metre in the brush's own space.
    let mesh = cube();
    let world = Mat4::from_scale(Vec3::new(1.0, 1.0, 2.0));
    let mut drag = drag_of(&mesh, &top(&mesh), world);
    drag.set(Change::Translate(Vec3::Z));
    let now = drag.finish().unwrap().unwrap();
    assert_eq!(now.bounds().unwrap().size().z, 2.5);
}

#[test]
fn nothing_moved_is_no_change_and_a_fold_is_refused() {
    let mesh = cube();
    let mut drag = drag_of(&mesh, &top(&mesh), Mat4::IDENTITY);
    assert_eq!(drag.finish(), Ok(None));
    drag.set(Change::Translate(Vec3::ZERO));
    assert_eq!(drag.finish(), Ok(None));
    drag.set(Change::Translate(-Vec3::Z * 3.0));
    let refused = drag.finish().unwrap_err();
    assert_eq!(
        refused_words(refused),
        "Put back: it would fold a face over itself"
    );
    drag.set(Change::Translate(-Vec3::Z * 2.0));
    assert!(matches!(drag.finish(), Err(mesh::Broken::Flattened(_))));
}

#[test]
fn a_broken_change_moves_nothing() {
    let mesh = cube();
    let mut drag = drag_of(&mesh, &top(&mesh), Mat4::IDENTITY);
    drag.set(Change::Translate(Vec3::NAN));
    assert_eq!(drag.finish(), Ok(None));
    // A world squashed flat has no inverse: the brush stays.
    let flat = Mat4::from_scale(Vec3::new(1.0, 0.0, 1.0));
    let mut drag = drag_of(&mesh, &top(&mesh), flat);
    drag.set(Change::Translate(Vec3::Z));
    assert_eq!(drag.finish(), Ok(None));
}

#[test]
fn a_size_typed_stretches_the_brush_about_its_middle() {
    let mesh = cube();
    let (bigger, _) = stretched(&mesh, Vec3::new(4.0, 2.5, 3.0)).unwrap().unwrap();
    let bounds = bigger.bounds().unwrap();
    assert_eq!(bounds.size(), Vec3::new(4.0, 2.5, 3.0));
    assert_eq!(
        bounds.centre(),
        Vec3::new(0.0, 0.0, 1.0),
        "about its middle"
    );
    assert_eq!(size_of(&bigger), Vec3::new(4.0, 2.5, 3.0));
    // The same size, or one that isn't a number: no change.
    assert_eq!(stretched(&mesh, Vec3::splat(2.0)), Ok(None));
    assert_eq!(stretched(&mesh, Vec3::NAN), Ok(None));
    assert_eq!(stretched(&Mesh::new(), Vec3::ONE), Ok(None));
    // Too small or too big is kept in range.
    let (tiny, _) = stretched(&mesh, Vec3::new(0.0, -5.0, 1.0))
        .unwrap()
        .unwrap();
    let size = tiny.bounds().unwrap().size();
    let wanted = Vec3::new(SMALLEST, SMALLEST, 1.0);
    assert!((size - wanted).abs().max_element() < 1e-5, "{size}");
    let (huge, _) = stretched(&mesh, Vec3::new(2.0, 2.0, 1e9)).unwrap().unwrap();
    assert_eq!(huge.bounds().unwrap().size().z, LARGEST);
}
