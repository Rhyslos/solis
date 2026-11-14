//! Tests for deleting parts and closing the gap: touching faces merge
//! into one, an edge's two faces merge, a point's faces merge; straight
//! leftover points go; a lone face, a ring and the whole brush are
//! refused, changing nothing.

use super::*;
use crate::split::{split_edges, split_faces_across};
use crate::{Broken, Shape, ShapeSettings, check_whole};
use glam::Vec3;

/// A box 4 m along X, 2 m along Y and 2 m high.
fn long_box() -> Mesh {
    Shape::Box.build(&ShapeSettings {
        size: Vec3::new(4.0, 2.0, 2.0),
        ..Shape::Box.standard()
    })
}

fn face_facing(mesh: &Mesh, normal: Vec3) -> FaceId {
    mesh.faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).dot(normal) > 0.99)
        .unwrap()
}

fn check(mesh: &Mesh) -> Result<(), Broken> {
    let all: Vec<VertexId> = mesh.vertices().map(|(v, _)| v).collect();
    check_whole(mesh, &all)
}

#[test]
fn two_halves_of_a_split_face_merge_back_into_the_box() {
    let box_ = long_box();
    let mut mesh = box_.clone();
    let top = face_facing(&mesh, Vec3::Z);
    let made = split_faces_across(&mut mesh, &[top]).unwrap();
    assert_eq!(mesh.face_count(), 7);
    let merged = merge_faces(&mut mesh, &[top, made[0]]).unwrap();
    assert_eq!(merged, [top], "the lowest id is kept");
    assert_eq!(mesh.face_count(), 6);
    // The cut's ends were left in the middle of straight sides: gone.
    assert_eq!(mesh.vertex_count(), 8);
    assert!(mesh.faces().all(|(_, c)| c.len() == 4));
    assert_eq!(check(&mesh), Ok(()));
}

#[test]
fn deleting_an_edge_merges_the_faces_on_either_side() {
    let mut mesh = long_box();
    let top = face_facing(&mesh, Vec3::Z);
    let made = split_faces_across(&mut mesh, &[top]).unwrap();
    let cut = mesh
        .topology()
        .face_edges(top)
        .iter()
        .copied()
        .find(|e| mesh.topology().edge_faces(*e).any(|f| f == made[0]))
        .unwrap();
    let merged = dissolve_edges(&mut mesh, &[cut]).unwrap();
    assert_eq!(merged, [top]);
    assert_eq!((mesh.face_count(), mesh.vertex_count()), (6, 8));
    assert_eq!(check(&mesh), Ok(()));
    // A box's own edge: the top and a side merge into one bent face.
    let mut box_ = long_box();
    let edge = box_.edges()[0];
    dissolve_edges(&mut box_, &[edge]).unwrap();
    assert_eq!(box_.face_count(), 5);
    assert_eq!(check(&box_), Ok(()), "a bent face is fine");
}

#[test]
fn deleting_a_point_merges_the_faces_around_it() {
    let mut mesh = long_box();
    let corner = mesh.vertices().next().unwrap().0;
    let merged = dissolve_points(&mut mesh, &[corner]).unwrap();
    assert_eq!(merged.len(), 1);
    assert!(mesh.position(corner).is_none(), "the point is gone");
    assert_eq!(mesh.face_count(), 4);
    assert_eq!(mesh.face(merged[0]).unwrap().len(), 6);
    // A point put in an edge's middle goes again cleanly.
    let mut split = long_box();
    let edge = split.edges()[0];
    let middle = split_edges(&mut split, &[edge]);
    dissolve_points(&mut split, &middle).unwrap();
    assert_eq!(split.face_count(), 5, "its two faces merged");
    assert_eq!(check(&split), Ok(()));
}

#[test]
fn a_lone_face_a_ring_and_everything_are_refused_changing_nothing() {
    let mut mesh = long_box();
    let before = mesh.clone();
    let top = face_facing(&mesh, Vec3::Z);
    assert_eq!(merge_faces(&mut mesh, &[top]), Err(LONE_FACE));
    let sides: Vec<FaceId> = [Vec3::X, Vec3::Y, -Vec3::X, -Vec3::Y]
        .into_iter()
        .map(|n| face_facing(&mesh, n))
        .collect();
    assert_eq!(merge_faces(&mut mesh, &sides), Err(NOT_ONE_LOOP));
    let every: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
    assert_eq!(merge_faces(&mut mesh, &every), Err(EVERYTHING));
    assert_eq!(mesh, before);
}

#[test]
fn merges_that_leave_too_few_faces_are_caught_by_the_whole_check() {
    // Two neighbouring sides and the top of a box merge into one face:
    // four faces left, still closed. Three at a corner merge too.
    let mut mesh = long_box();
    let faces = [Vec3::Z, Vec3::X, Vec3::Y].map(|n| face_facing(&mesh, n));
    merge_faces(&mut mesh, &faces).unwrap();
    assert_eq!(mesh.face_count(), 4);
    assert_eq!(check(&mesh), Ok(()));
    let corner = mesh.vertices().next().unwrap().0;
    let mut fewer = mesh.clone();
    if dissolve_points(&mut fewer, &[corner]).is_ok() {
        assert!(check(&fewer).is_err(), "fewer than four faces, or flat");
    }
}

#[test]
fn many_faces_merge_into_one_quickly() {
    // A 10 × 10 patch of one side of a heavy box, merged.
    let mesh = crate::Primitive::Cube(crate::CubeParams {
        size: Vec3::splat(20.0),
        segments: [29, 29, 29],
    })
    .mesh();
    let top = mesh.bounds().unwrap().max.z;
    let patch: Vec<FaceId> = mesh
        .faces()
        .filter(|(f, c)| {
            mesh.face_normal(*f).z > 0.99
                && c.iter().all(|v| {
                    let p = mesh.position(*v).unwrap();
                    p.z > top - 1e-3
                        && p.x < -10.0 + 10.0 * 20.0 / 29.0 + 1e-3
                        && p.y < -10.0 + 10.0 * 20.0 / 29.0 + 1e-3
                })
        })
        .map(|(f, _)| f)
        .collect();
    assert_eq!(patch.len(), 100);
    let mut merged = mesh.clone();
    let start = std::time::Instant::now();
    merge_faces(&mut merged, &patch).unwrap();
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(merged.face_count(), mesh.face_count() - 99);
    assert_eq!(check(&merged), Ok(()));
    assert!(ms < 500.0, "{ms:.1} ms");
}
