//! Tests for the check that a mesh stays whole after points move: moves,
//! turns and stretches that keep a brush whole pass; folding a face,
//! flattening one or pushing the mesh through itself is refused; bent
//! faces are fine.

use super::*;
use crate::Shape;
use glam::{Mat3, Mat4};

/// A 2 m box standing on the ground: eight points, six faces.
fn cube() -> Mesh {
    Shape::Box.build(&Shape::Box.standard())
}

/// The points with `z` above `height`: the top of the box.
fn above(mesh: &Mesh, height: f32) -> Vec<VertexId> {
    mesh.vertices()
        .filter(|(_, p)| p.z > height)
        .map(|(v, _)| v)
        .collect()
}

fn moved(mesh: &Mesh, points: &[VertexId], matrix: Mat4) -> Mesh {
    let mut after = mesh.clone();
    crate::edit::transform_points(&mut after, points, matrix);
    after
}

fn check(before: &Mesh, points: &[VertexId], matrix: Mat4) -> Result<(), Broken> {
    let after = moved(before, points, matrix);
    check_moved(before, &after, points, Mat3::from_mat4(matrix))
}

#[test]
fn moving_the_top_up_or_turning_it_a_little_keeps_the_box_whole() {
    let box_ = cube();
    let top = above(&box_, 1.0);
    assert_eq!(top.len(), 4);
    assert_eq!(check(&box_, &top, Mat4::from_translation(Vec3::Z)), Ok(()));
    assert_eq!(
        check(&box_, &top, Mat4::from_translation(-Vec3::Z * 1.5)),
        Ok(())
    );
    let turn = Mat4::from_translation(Vec3::Z * 2.0)
        * Mat4::from_rotation_z(0.3)
        * Mat4::from_translation(-Vec3::Z * 2.0);
    assert_eq!(check(&box_, &top, turn), Ok(()));
    let wider = Mat4::from_scale(Vec3::new(2.0, 1.5, 1.0));
    assert_eq!(check(&box_, &top, wider), Ok(()));
}

#[test]
fn a_bent_face_is_allowed() {
    // One corner of the top raised: the top and two sides are no longer
    // flat, which is fine.
    let box_ = cube();
    let corner = above(&box_, 1.0)[0];
    assert_eq!(
        check(&box_, &[corner], Mat4::from_translation(Vec3::Z * 0.7)),
        Ok(())
    );
}

#[test]
fn the_whole_brush_turned_over_is_fine_but_mirrored_is_refused() {
    let box_ = cube();
    let all: Vec<VertexId> = box_.vertices().map(|(v, _)| v).collect();
    let over = Mat4::from_rotation_x(std::f32::consts::PI);
    assert_eq!(check(&box_, &all, over), Ok(()));
    let mirror = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
    assert!(matches!(check(&box_, &all, mirror), Err(Broken::Folded(_))));
}

#[test]
fn pushing_the_top_through_the_bottom_folds_the_sides() {
    let box_ = cube();
    let top = above(&box_, 1.0);
    let through = Mat4::from_translation(-Vec3::Z * 3.0);
    assert!(matches!(
        check(&box_, &top, through),
        Err(Broken::Folded(_))
    ));
}

#[test]
fn squashing_the_top_onto_the_bottom_flattens_the_sides() {
    let box_ = cube();
    let top = above(&box_, 1.0);
    let down = Mat4::from_translation(-Vec3::Z * 2.0);
    assert!(matches!(
        check(&box_, &top, down),
        Err(Broken::Flattened(_))
    ));
}

#[test]
fn one_corner_onto_its_neighbour_flattens_a_side() {
    let box_ = cube();
    let top = above(&box_, 1.0);
    let (a, b) = (top[0], top[1]);
    let to = box_.position(b).unwrap() - box_.position(a).unwrap();
    assert!(matches!(
        check(&box_, &[a], Mat4::from_translation(to)),
        Err(Broken::Flattened(_))
    ));
}

#[test]
fn a_face_twisted_across_itself_is_folded() {
    // Two neighbouring corners of the top swapped over: the top crosses
    // itself like a bow tie.
    let box_ = cube();
    let top = above(&box_, 1.0);
    let ring: Vec<VertexId> = box_
        .faces()
        .map(|(_, c)| c)
        .find(|c| c.iter().all(|v| top.contains(v)))
        .unwrap()
        .to_vec();
    let (a, b) = (ring[0], ring[1]);
    let (pa, pb) = (box_.position(a).unwrap(), box_.position(b).unwrap());
    let mut after = box_.clone();
    after.set_position(a, pb + Vec3::new(0.0, 0.01, 0.0));
    after.set_position(b, pa - Vec3::new(0.0, 0.01, 0.0));
    assert!(matches!(
        check_moved(&box_, &after, &[a, b], Mat3::IDENTITY),
        Err(Broken::Folded(_))
    ));
}

#[test]
fn a_face_pushed_into_another_part_passes_through() {
    // Two boxes in one mesh, side by side: the first's top pushed sideways
    // and down into the second.
    let mut two = cube();
    let other = cube();
    let offset = Vec3::new(3.0, 0.3, 0.5);
    let first: Vec<VertexId> = (0..other.point_slot_count() as u32)
        .map(|i| {
            two.add_vertex(other.position(VertexId::from_raw(i)).unwrap() + offset)
                .unwrap()
        })
        .collect();
    for (_, corners) in other.faces() {
        let mapped: Vec<VertexId> = corners.iter().map(|v| first[v.get() as usize]).collect();
        two.add_face(&mapped).unwrap();
    }
    let before = two.clone();
    // The first box's +X side, moved 1.5 m into the second box (which
    // stands a little higher and to one side, so no faces lie flat
    // against each other).
    let side: Vec<VertexId> = before
        .vertices()
        .filter(|(_, p)| (p.x - 1.0).abs() < 1e-4)
        .map(|(v, _)| v)
        .collect();
    assert_eq!(side.len(), 4);
    let result = check(&before, &side, Mat4::from_translation(Vec3::X * 2.5));
    assert!(
        matches!(result, Err(Broken::PassesThrough(..))),
        "{result:?}"
    );
    // Moved up to touch it, not into it: fine.
    assert_eq!(
        check(&before, &side, Mat4::from_translation(Vec3::X * 1.0)),
        Ok(())
    );
}

#[test]
fn nothing_moved_or_points_not_used_is_fine() {
    let box_ = cube();
    assert_eq!(check_moved(&box_, &box_, &[], Mat3::IDENTITY), Ok(()));
    let mut lonely = box_.clone();
    let loose = lonely.add_vertex(Vec3::splat(9.0)).unwrap();
    let after = moved(&lonely, &[loose], Mat4::from_translation(Vec3::X));
    assert_eq!(
        check_moved(&lonely, &after, &[loose], Mat3::IDENTITY),
        Ok(())
    );
}

#[test]
fn triangles_that_only_touch_do_not_overlap() {
    let a = [Vec3::ZERO, Vec3::X, Vec3::Y];
    let touching = [Vec3::X, Vec3::new(2.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0)];
    assert!(!triangles_overlap(&a, &touching, 1e-5));
    let crossing = [
        Vec3::new(0.2, 0.2, -1.0),
        Vec3::new(0.2, 0.2, 1.0),
        Vec3::new(0.3, -1.0, 0.0),
    ];
    assert!(triangles_overlap(&a, &crossing, 1e-5));
    let apart = crossing.map(|p| p + Vec3::Z * 5.0);
    assert!(!triangles_overlap(&a, &apart, 1e-5));
}

#[test]
fn a_5000_face_brush_with_1000_faces_moved_is_checked_quickly() {
    let big = crate::Primitive::Cube(crate::CubeParams {
        size: Vec3::splat(100.0),
        segments: [29, 29, 29],
    })
    .mesh();
    assert!(big.face_count() >= 5_000);
    // The top's faces and a ring round them: about a thousand.
    let top = above(&big, 30.0);
    let after = moved(&big, &top, Mat4::from_translation(Vec3::Z * 0.5));
    let start = std::time::Instant::now();
    assert_eq!(check_moved(&big, &after, &top, Mat3::IDENTITY), Ok(()));
    // Generous for a debug build on a slow machine; the scale check
    // measures it in release.
    assert!(start.elapsed().as_secs_f64() < 2.0);
}
