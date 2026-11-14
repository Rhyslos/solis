//! Tests for mending a brush read from a file: a whole box needs
//! nothing; a face turned round, a box inside out and a hole with one
//! flat border are mended; a hole that isn't flat and three faces at an
//! edge are left, with why.

use super::*;
use crate::{Shape, check_whole};

fn cube() -> Mesh {
    Shape::Box.build(&Shape::Box.standard())
}

fn face_facing(mesh: &Mesh, normal: Vec3) -> FaceId {
    mesh.faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).dot(normal) > 0.99)
        .unwrap()
}

fn whole(mesh: &Mesh) -> bool {
    let all: Vec<VertexId> = mesh.vertices().map(|(v, _)| v).collect();
    check_whole(mesh, &all).is_ok()
}

#[test]
fn a_whole_box_and_an_empty_mesh_need_nothing() {
    assert_eq!(repair(&cube()), Ok(None));
    assert_eq!(why_not_whole(&cube()), None);
    assert_eq!(repair(&Mesh::new()), Ok(None));
    for shape in Shape::ALL {
        assert_eq!(
            repair(&shape.build(&shape.standard())),
            Ok(None),
            "{shape:?}"
        );
    }
}

#[test]
fn a_face_turned_round_is_turned_back() {
    let mut mesh = cube();
    let top = face_facing(&mesh, Vec3::Z);
    turn_round(&mut mesh, &[top]);
    assert_eq!(why_not_whole(&mesh), Some("it is inside out"));
    let (fixed, done) = repair(&mesh).unwrap().unwrap();
    assert_eq!(
        done,
        Mended {
            turned: 1,
            filled: 0
        }
    );
    assert_eq!(fixed, cube());
}

#[test]
fn a_box_inside_out_is_turned_the_right_way() {
    let mut mesh = cube();
    let every: Vec<FaceId> = mesh.faces().map(|(f, _)| f).collect();
    turn_round(&mut mesh, &every);
    let (fixed, done) = repair(&mesh).unwrap().unwrap();
    assert_eq!(done.turned, 6);
    assert_eq!(fixed, cube());
}

#[test]
fn a_hole_with_one_flat_border_is_filled() {
    let mut mesh = cube();
    let top = face_facing(&mesh, Vec3::Z);
    mesh.delete_faces(&[top]);
    assert_eq!(why_not_whole(&mesh), Some("it has a hole"));
    let (fixed, done) = repair(&mesh).unwrap().unwrap();
    assert_eq!(
        done,
        Mended {
            turned: 0,
            filled: 1
        }
    );
    assert_eq!(fixed.face_count(), 6);
    assert!(whole(&fixed));
    let filled = fixed.faces().last().unwrap().0;
    assert!(fixed.face_normal(filled).z > 0.99, "facing out");
    // Open and inside out at once: both mended.
    let mut both = mesh.clone();
    let every: Vec<FaceId> = both.faces().map(|(f, _)| f).collect();
    turn_round(&mut both, &every);
    let (fixed, done) = repair(&both).unwrap().unwrap();
    assert_eq!(
        done,
        Mended {
            turned: 5,
            filled: 1
        }
    );
    assert!(whole(&fixed));
}

#[test]
fn a_hole_that_isnt_flat_or_three_faces_at_an_edge_are_left_with_why() {
    // Two neighbouring faces gone: the hole's border bends round a corner.
    let mut mesh = cube();
    let gone = [face_facing(&mesh, Vec3::Z), face_facing(&mesh, Vec3::X)];
    mesh.delete_faces(&gone);
    assert_eq!(repair(&mesh), Err(UNEVEN_HOLE));
    assert_eq!(why_not_whole(&mesh), Some("it has a hole"));
    let mut tangled = cube();
    let top = face_facing(&tangled, Vec3::Z);
    let ring = tangled.face(top).unwrap().to_vec();
    let below = tangled.add_vertex(Vec3::new(0.0, 0.0, -5.0)).unwrap();
    tangled.add_face(&[ring[1], ring[0], below]).unwrap();
    assert_eq!(repair(&tangled), Err(TANGLED));
    assert_eq!(why_not_whole(&tangled), Some(TANGLED));
}
