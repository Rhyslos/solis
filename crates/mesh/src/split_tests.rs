//! Tests for splitting: a face cut between two of its points, a point
//! put in the middle of edges, faces cut across their middle along the
//! longer side; refused cuts change nothing; the brush stays whole.

use super::*;
use crate::{Shape, ShapeSettings, check_whole};

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

fn whole(mesh: &Mesh) {
    let all: Vec<VertexId> = mesh.vertices().map(|(v, _)| v).collect();
    assert_eq!(check_whole(mesh, &all), Ok(()));
}

#[test]
fn two_points_across_a_face_cut_it_in_two() {
    let mut mesh = long_box();
    let top = face_facing(&mesh, Vec3::Z);
    let corners = mesh.face(top).unwrap().to_vec();
    let (keep, other) = split_face_between(&mut mesh, corners[0], corners[2]).unwrap();
    assert_eq!(keep, top, "the face keeps its id for one half");
    assert_eq!(mesh.face_count(), 7);
    assert_eq!(mesh.face(keep).unwrap().len(), 3);
    assert_eq!(mesh.face(other).unwrap().len(), 3);
    assert_eq!(mesh.vertex_count(), 8, "no new points");
    whole(&mesh);
}

#[test]
fn points_on_no_one_face_or_already_joined_are_refused() {
    let mut mesh = long_box();
    let before = mesh.clone();
    let top = face_facing(&mesh, Vec3::Z);
    let bottom = face_facing(&mesh, -Vec3::Z);
    let t = mesh.face(top).unwrap().to_vec();
    let b = mesh.face(bottom).unwrap().to_vec();
    // A top corner and the bottom corner furthest from it share no face.
    let far = *b
        .iter()
        .max_by(|x, y| {
            let d = |v: &VertexId| {
                mesh.position(*v)
                    .unwrap()
                    .distance(mesh.position(t[0]).unwrap())
            };
            d(x).total_cmp(&d(y))
        })
        .unwrap();
    assert_eq!(
        split_face_between(&mut mesh, t[0], far),
        Err(NOT_ON_ONE_FACE)
    );
    assert_eq!(
        split_face_between(&mut mesh, t[0], t[1]),
        Err(ALREADY_JOINED)
    );
    assert_eq!(
        split_face_between(&mut mesh, t[0], t[0]),
        Err(NOT_ON_ONE_FACE)
    );
    assert_eq!(mesh, before, "nothing changed");
}

#[test]
fn edges_get_a_point_in_their_middle_in_every_face_along_them() {
    let mut mesh = long_box();
    let top = face_facing(&mesh, Vec3::Z);
    let corners = mesh.face(top).unwrap().to_vec();
    let edge = EdgeKey::new(corners[0], corners[1]);
    let (a, b) = edge.ends();
    let middle_at = (mesh.position(a).unwrap() + mesh.position(b).unwrap()) / 2.0;
    let made = split_edges(&mut mesh, &[edge, edge]);
    assert_eq!(made.len(), 1, "each edge once");
    assert_eq!(mesh.position(made[0]), Some(middle_at));
    let users: Vec<FaceId> = mesh.topology().vertex_faces(made[0]).to_vec();
    assert_eq!(users.len(), 2, "both faces along the edge have it");
    assert_eq!(mesh.face(top).unwrap().len(), 5);
    assert_eq!(mesh.face_count(), 6);
    whole(&mesh);
}

#[test]
fn a_long_face_is_cut_across_its_middle_into_two_squares() {
    let mut mesh = long_box();
    let top = face_facing(&mesh, Vec3::Z);
    let made = split_faces_across(&mut mesh, &[top]).unwrap();
    assert_eq!(made.len(), 1);
    for half in [top, made[0]] {
        let corners = mesh.face(half).unwrap();
        assert_eq!(corners.len(), 4);
        let points: Vec<Vec3> = corners.iter().map(|v| mesh.position(*v).unwrap()).collect();
        let size = crate::Bounds::around(points.iter().copied())
            .unwrap()
            .size();
        assert!(
            (size.x - 2.0).abs() < 1e-4 && (size.y - 2.0).abs() < 1e-4,
            "{size}"
        );
    }
    // The cut's ends are on the long sides, in the faces beside them too.
    assert_eq!(mesh.vertex_count(), 10);
    assert_eq!(mesh.face_count(), 7);
    whole(&mesh);
}

#[test]
fn a_triangle_is_cut_through_its_longest_side() {
    let mut mesh = long_box();
    let top = face_facing(&mesh, Vec3::Z);
    let corners = mesh.face(top).unwrap().to_vec();
    let (keep, other) = split_face_between(&mut mesh, corners[0], corners[2]).unwrap();
    let made = split_faces_across(&mut mesh, &[keep, other]).unwrap();
    assert_eq!(made.len(), 2);
    whole(&mesh);
}

#[test]
fn a_cut_that_fails_changes_nothing() {
    // A face whose corners are all on one spot can't be cut.
    let mut mesh = Mesh::new();
    let a = mesh.add_vertex(Vec3::ZERO).unwrap();
    let b = mesh.add_vertex(Vec3::ZERO).unwrap();
    let c = mesh.add_vertex(Vec3::ZERO).unwrap();
    let face = mesh.add_face(&[a, b, c]).unwrap();
    let before = mesh.clone();
    assert_eq!(split_faces_across(&mut mesh, &[face]), Err(NOT_ACROSS));
    assert_eq!(mesh, before);
    // Faces not in the mesh are passed over.
    let mut box_ = long_box();
    assert_eq!(
        split_faces_across(&mut box_, &[FaceId::from_raw(99)]),
        Ok(Vec::new())
    );
}
