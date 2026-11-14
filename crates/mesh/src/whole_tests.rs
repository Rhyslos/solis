//! Tests for the whole check after faces are added, removed or rewired:
//! extrudes, insets, splits and merges that keep a box whole pass; a
//! hole, an inside-out face, a flattened or tangled brush, too few faces
//! and passing through are refused.

use super::*;
use crate::edit::{Grouping, extrude_polygons, inset_polygons};
use crate::{Broken, Shape};

/// A 2 m box standing on the ground: eight points, six faces.
fn cube() -> Mesh {
    Shape::Box.build(&Shape::Box.standard())
}

fn face_facing(mesh: &Mesh, normal: Vec3) -> FaceId {
    mesh.faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).dot(normal) > 0.99)
        .unwrap()
}

fn every_point(mesh: &Mesh) -> Vec<VertexId> {
    mesh.vertices().map(|(v, _)| v).collect()
}

#[test]
fn a_box_and_its_extrudes_and_insets_are_whole() {
    let box_ = cube();
    assert_eq!(check_whole(&box_, &every_point(&box_)), Ok(()));
    assert!(room_inside(&box_) > 7.9 && room_inside(&box_) < 8.1);
    let top = face_facing(&box_, Vec3::Z);
    for distance in [1.0, -0.5] {
        let mut out = box_.clone();
        extrude_polygons(&mut out, &[top], distance, Grouping::Together);
        assert_eq!(check_whole(&out, &every_point(&out)), Ok(()), "{distance}");
    }
    let mut inset = box_.clone();
    inset_polygons(&mut inset, &[top], 0.5, Grouping::Together);
    assert_eq!(check_whole(&inset, &every_point(&inset)), Ok(()));
}

#[test]
fn an_extrude_of_nothing_flattens_its_walls() {
    let box_ = cube();
    let top = face_facing(&box_, Vec3::Z);
    let mut out = box_.clone();
    extrude_polygons(&mut out, &[top], 0.0, Grouping::Together);
    assert!(matches!(
        check_whole(&out, &every_point(&out)),
        Err(Broken::Flattened(_))
    ));
}

#[test]
fn pushing_the_top_through_the_bottom_passes_through() {
    let box_ = cube();
    let top = face_facing(&box_, Vec3::Z);
    let mut out = box_.clone();
    extrude_polygons(&mut out, &[top], -2.5, Grouping::Together);
    assert!(check_whole(&out, &every_point(&out)).is_err());
}

#[test]
fn a_face_taken_away_leaves_a_hole() {
    let mut open = cube();
    let top = face_facing(&open, Vec3::Z);
    let corners = open.face(top).unwrap().to_vec();
    open.delete_faces(&[top]);
    assert_eq!(check_whole(&open, &corners), Err(Broken::Open));
}

#[test]
fn a_face_turned_round_is_inside_out_and_so_is_the_whole_box() {
    let mut turned = cube();
    let top = face_facing(&turned, Vec3::Z);
    let mut corners = turned.face(top).unwrap().to_vec();
    corners.reverse();
    turned.replace_corners(top, &corners).unwrap();
    assert_eq!(check_whole(&turned, &corners), Err(Broken::InsideOut));
    let mut inside_out = cube();
    let faces: Vec<FaceId> = inside_out.faces().map(|(f, _)| f).collect();
    for face in faces {
        let mut corners = inside_out.face(face).unwrap().to_vec();
        corners.reverse();
        inside_out.replace_corners(face, &corners).unwrap();
    }
    let all = every_point(&inside_out);
    assert_eq!(check_whole(&inside_out, &all), Err(Broken::InsideOut));
}

#[test]
fn too_few_faces_tangled_edges_and_a_flat_brush_are_refused() {
    let mut three = cube();
    let faces: Vec<FaceId> = three.faces().map(|(f, _)| f).take(3).collect();
    three.delete_faces(&faces);
    assert_eq!(check_whole(&three, &[]), Err(Broken::TooFewFaces));
    // A seventh face across the box's top edge: three faces at it.
    let mut tangled = cube();
    let top = face_facing(&tangled, Vec3::Z);
    let ring = tangled.face(top).unwrap().to_vec();
    let below = tangled.add_vertex(Vec3::new(0.0, 0.0, -5.0)).unwrap();
    tangled.add_face(&[ring[1], ring[0], below]).unwrap();
    assert_eq!(check_whole(&tangled, &ring), Err(Broken::Tangled));
    // Every point on the ground: no room inside.
    let mut flat = cube();
    for v in every_point(&flat) {
        let p = flat.position(v).unwrap();
        flat.set_position(v, Vec3::new(p.x, p.y, 0.0));
    }
    assert!(check_whole(&flat, &every_point(&flat)).is_err());
}
