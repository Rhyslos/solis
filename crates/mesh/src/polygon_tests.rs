//! Tests for the polygon mesh.

use super::points::CHUNK;
use super::*;

fn square() -> (Mesh, [VertexId; 4]) {
    let mut mesh = Mesh::new();
    let v = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ]
    .map(|p| mesh.add_vertex(p).unwrap());
    mesh.add_face(&v).unwrap();
    (mesh, v)
}

#[test]
fn unused_points_go_in_one_pass_and_used_ones_stay() {
    let (mut mesh, v) = square();
    let loose = mesh.add_vertex(Vec3::new(5.0, 0.0, 0.0)).unwrap();
    let other = mesh.add_vertex(Vec3::new(6.0, 0.0, 0.0)).unwrap();
    mesh.set_hard(loose, other, true);
    mesh.set_hard(v[0], v[1], true);
    let ghost = VertexId(99);
    // Used, unused, repeated and unknown points together.
    let gone = mesh.remove_unused_vertices(&[v[0], loose, loose, ghost, other]);
    assert_eq!(gone, 2);
    assert_eq!(mesh.vertex_count(), 4);
    assert!(mesh.position(loose).is_none() && mesh.position(other).is_none());
    assert!(!mesh.is_hard(loose, other), "its hard edge went with it");
    assert!(mesh.is_hard(v[0], v[1]), "other hard edges stay");
    assert_eq!(mesh.remove_unused_vertices(&[]), 0);
    assert_eq!(mesh.remove_unused_vertices(&[loose]), 0, "already gone");
    assert_eq!(
        mesh.add_vertex(Vec3::ZERO).unwrap(),
        VertexId(6),
        "ids are never used again"
    );
}

#[test]
fn memory_size_counts_the_topology_before_and_after_it_is_worked_out() {
    let sphere = crate::PrimitiveKind::Sphere.standard().mesh();
    let estimated = sphere.memory_size();
    let topology = sphere.topology().memory_size();
    let counted = sphere.memory_size();
    assert!(counted > topology, "the topology is part of it");
    // The estimate is within a third of the real thing.
    let (a, b) = (estimated as f64, counted as f64);
    assert!((a - b).abs() / b < 0.33, "estimated {a}, counted {b}");
}

#[test]
fn faces_need_three_distinct_known_corners() {
    let (mut mesh, v) = square();
    assert_eq!(mesh.add_face(&v[..2]), Err(MeshError::TooFewCorners));
    assert_eq!(
        mesh.add_face(&[v[0], v[1], v[0]]),
        Err(MeshError::RepeatedVertex(v[0]))
    );
    let ghost = VertexId(99);
    assert_eq!(
        mesh.add_face(&[v[0], v[1], ghost]),
        Err(MeshError::UnknownVertex(ghost))
    );
    assert_eq!(mesh.face_count(), 1, "nothing added");
    assert_eq!(
        MeshError::TooFewCorners.to_string(),
        "a face needs at least three corners"
    );
}

#[test]
fn ids_count_up_in_order() {
    let (mesh, v) = square();
    assert_eq!(v.map(VertexId::get), [0, 1, 2, 3]);
    assert_eq!(mesh.faces().next().unwrap().0.get(), 0);
    assert_eq!(mesh.vertex_count(), 4);
}

#[test]
fn edges_are_listed_once_and_marked_hard_either_way_round() {
    let (mut mesh, v) = square();
    assert_eq!(mesh.edges().len(), 4);
    mesh.set_hard(v[1], v[0], true);
    assert!(mesh.is_hard(v[0], v[1]));
    assert_eq!(mesh.hard_edge_count(), 1);
    mesh.set_hard(v[0], v[1], false);
    assert!(!mesh.is_hard(v[1], v[0]));
}

#[test]
fn a_square_facing_up_has_an_upward_normal_and_flat_bounds() {
    let (mesh, _) = square();
    let face = mesh.faces().next().unwrap().0;
    assert!((mesh.face_normal(face) - Vec3::Z).length() < 1e-6);
    let b = mesh.bounds().unwrap();
    assert_eq!(b.size(), Vec3::new(1.0, 1.0, 0.0));
}

#[test]
fn a_face_with_no_area_still_gets_a_usable_normal() {
    let mut mesh = Mesh::new();
    let v = [Vec3::ZERO, Vec3::X, Vec3::X * 2.0].map(|p| mesh.add_vertex(p).unwrap());
    let face = mesh.add_face(&v).unwrap();
    assert_eq!(mesh.face_normal(face), Vec3::Z);
    assert_eq!(mesh.face_normal(FaceId(42)), Vec3::Z, "unknown face");
}

#[test]
fn points_move_only_if_they_exist() {
    let (mut mesh, v) = square();
    assert!(mesh.set_position(v[0], Vec3::splat(5.0)));
    assert_eq!(mesh.position(v[0]), Some(Vec3::splat(5.0)));
    assert!(!mesh.set_position(VertexId::from_raw(40), Vec3::ONE));
    assert_eq!(mesh.vertex_count(), 4);
    assert_eq!(VertexId::from_raw(3), v[3]);
    assert_eq!(FaceId::from_raw(0).get(), 0);
}

#[test]
fn memory_size_grows_with_the_mesh() {
    let empty = Mesh::new().memory_size();
    assert!(empty >= std::mem::size_of::<Mesh>());
    let (square, _) = square();
    let small = square.memory_size();
    assert!(small > empty);
    let sphere = crate::PrimitiveKind::Sphere.standard().mesh();
    assert!(
        sphere.memory_size() > small * 20,
        "many more points and faces"
    );
    // A sphere's points and faces take at least their raw size.
    let raw = sphere.vertex_count() * 12 + sphere.face_count() * 3 * 4;
    assert!(sphere.memory_size() >= raw);
}

#[test]
fn a_mesh_rebuilt_from_its_slots_is_the_same_ids_and_all() {
    let mut mesh = crate::PrimitiveKind::Cylinder.standard().mesh();
    // Only the shape: layers aren't part of the slots.
    // A deleted point slot (no face uses the last point of a fresh
    // vertex, so add one and leave it out of any face).
    let extra = mesh.add_vertex(Vec3::splat(9.0)).unwrap();
    mesh.positions.set(extra.0 as usize, None);
    mesh.live_points -= 1;
    let rebuilt = Mesh::from_slots(
        mesh.point_slots().collect(),
        mesh.face_slots().map(|f| f.map(<[_]>::to_vec)).collect(),
        mesh.hard_edges(),
    )
    .unwrap();
    assert_eq!(rebuilt, mesh);
    assert_eq!(rebuilt.point_slot_count(), mesh.point_slot_count());
}

#[test]
fn damaged_slots_are_refused_with_a_reason() {
    let p = |x: f32| Some(Vec3::new(x, 0.0, 0.0));
    let v = VertexId;
    let points = vec![p(0.0), p(1.0), Some(Vec3::Y), None];
    let ok = |faces: Vec<Option<Vec<VertexId>>>| {
        Mesh::from_slots(points.clone(), faces, std::iter::empty())
    };
    assert!(ok(vec![Some(vec![v(0), v(1), v(2)]), None]).is_ok());
    assert_eq!(
        ok(vec![Some(vec![v(0), v(1)])]),
        Err(MeshError::TooFewCorners)
    );
    assert_eq!(
        ok(vec![Some(vec![v(0), v(1), v(3)])]),
        Err(MeshError::UnknownVertex(v(3))),
        "a deleted point"
    );
    assert_eq!(
        ok(vec![Some(vec![v(0), v(1), v(0)])]),
        Err(MeshError::RepeatedVertex(v(0)))
    );
    assert_eq!(
        Mesh::from_slots(vec![p(0.0), Some(Vec3::NAN)], vec![], std::iter::empty()),
        Err(MeshError::NotFinite(v(1)))
    );
    // Bad hard edges are left out (they only change shading), so a
    // project with one still opens.
    let bad_edge = EdgeKey::new(v(0), v(7));
    let loop_edge = EdgeKey::new(v(1), v(1));
    let good_edge = EdgeKey::new(v(0), v(1));
    let mesh = Mesh::from_slots(points, vec![], [bad_edge, loop_edge, good_edge]).unwrap();
    assert_eq!(mesh.hard_edges().collect::<Vec<_>>(), vec![good_edge]);
    // And none can be made in the first place.
    let mut m = mesh.clone();
    m.set_hard(v(1), v(1), true);
    m.set_hard(v(0), v(99), true);
    assert_eq!(m.hard_edge_count(), 1);
    // A long polygon with a repeat is still caught.
    let many: Vec<Option<Vec3>> = (0..100).map(|i| p(i as f32)).collect();
    let mut corners: Vec<VertexId> = (0..100).map(v).collect();
    corners[60] = v(10);
    assert_eq!(
        Mesh::from_slots(many, vec![Some(corners)], std::iter::empty()),
        Err(MeshError::RepeatedVertex(v(10)))
    );
    assert!(MeshError::NotFinite(v(1)).to_string().contains("point 1"));
}

/// A strip of quads long enough to span several chunks of points.
fn long_strip(quads: usize) -> Mesh {
    let mut mesh = Mesh::new();
    let mut last = None;
    for i in 0..=quads {
        let a = mesh.add_vertex(Vec3::new(i as f32, 0.0, 0.0)).unwrap();
        let b = mesh.add_vertex(Vec3::new(i as f32, 1.0, 0.0)).unwrap();
        if let Some((p, q)) = last {
            mesh.add_face(&[p, a, b, q]).unwrap();
        }
        last = Some((a, b));
    }
    mesh
}

#[test]
fn copies_share_their_parts_until_one_changes() {
    let before = long_strip(5_000);
    assert!(before.vertex_count() > 2 * CHUNK, "several chunks");
    let mut after = before.clone();
    assert!(after.same_structure(&before));
    assert_eq!(after.moved_since(&before), Some(Vec::new()));
    // Moving points in two chunks: only those are found, and the
    // faces are still shared.
    let (a, b) = (VertexId(3), VertexId(CHUNK as u32 + 7));
    after.set_position(a, Vec3::new(0.0, 0.0, 5.0));
    after.set_position(b, Vec3::new(0.0, 0.0, -2.0));
    assert_eq!(after.moved_since(&before), Some(vec![a, b]));
    assert!(after.same_structure(&before));
    assert_ne!(after, before);
    // The box follows the moves, and the original is untouched.
    let bounds = after.bounds().unwrap();
    assert_eq!((bounds.min.z, bounds.max.z), (-2.0, 5.0));
    assert_eq!(before.bounds().unwrap().max.z, 0.0);
    // A new face makes a different structure.
    let mut grown = before.clone();
    grown
        .add_face(&[VertexId(0), VertexId(1), VertexId(3)])
        .unwrap();
    assert!(!grown.same_structure(&before));
    assert_eq!(grown.moved_since(&before), None);
    assert!(grown.topology().edge_count() > before.topology().edge_count());
}

#[test]
fn slots_read_back_across_chunks_and_faces_lay_out_flat() {
    let strip = long_strip(3_000);
    let slots: Vec<Option<Vec3>> = strip.point_slots().collect();
    assert_eq!(slots.len(), strip.point_slot_count());
    assert_eq!(slots[CHUNK + 1], strip.position(VertexId(CHUNK as u32 + 1)));
    let (starts, corners) = strip.face_layout();
    assert_eq!(starts.len(), strip.face_count() + 1);
    assert_eq!(corners.len(), strip.face_count() * 4);
    assert_eq!(strip.face_slots().len(), strip.face_count());
    // A deleted face's slot reads as none, and keeps its id.
    let rebuilt = Mesh::from_slots(
        slots,
        vec![None, Some(strip.face(FaceId(1)).unwrap().to_vec())],
        std::iter::empty(),
    )
    .unwrap();
    assert_eq!(rebuilt.face(FaceId(0)), None);
    assert_eq!(rebuilt.face(FaceId(1)), strip.face(FaceId(1)));
    assert_eq!(rebuilt.face_count(), 1);
    assert_eq!(rebuilt.faces().next().unwrap().0, FaceId(1));
    assert!(format!("{rebuilt:?}").contains("faces: 1"));
}

#[test]
fn an_empty_mesh_has_no_bounds() {
    assert_eq!(Mesh::new().bounds(), None);
    assert!(Mesh::new().edges().is_empty());
}
