//! Finding a brush's parts: the face, corner or side under the pointer,
//! from the side facing the camera; what a box holds, through the brush;
//! nothing when the pointer misses; a brush placed and stretched by its
//! entity.

use super::*;
use crate::camera::Camera;
use mesh::Shape;

const SIZE: Vec2 = Vec2::new(800.0, 600.0);

/// A 2 m box standing on the ground, and a camera looking down at it from
/// the front, a little to the right.
fn scene() -> (PartCache, Camera) {
    let mesh = Arc::new(Shape::Box.build(&Shape::Box.standard()));
    let mut camera = Camera {
        position: Vec3::new(1.5, -6.0, 5.0),
        ..Camera::default()
    };
    camera.look_at(Vec3::new(0.0, 0.0, 1.0));
    (PartCache::of(mesh, 7), camera)
}

fn under(cache: &PartCache, camera: &Camera, mode: Mode, p: Vec3, world: Mat4) -> Option<Part> {
    let at = camera.project(world.transform_point3(p), SIZE).unwrap();
    let project = camera.projector(SIZE);
    cache.under(mode, (world, camera.ray_through(at, SIZE)), &project, at)
}

/// The face of the box whose normal is `normal`.
fn face_facing(cache: &PartCache, normal: Vec3) -> FaceId {
    let mesh = cache.mesh();
    mesh.faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).dot(normal) > 0.99)
        .unwrap()
}

#[test]
fn the_face_under_the_pointer_is_the_one_facing_the_camera() {
    let (cache, camera) = scene();
    // The middle of the front face (y = -1), seen through the box's top
    // edge: the front, never the back.
    let front = face_facing(&cache, -Vec3::Y);
    let hit = under(
        &cache,
        &camera,
        Mode::Faces,
        Vec3::new(0.0, -1.0, 1.0),
        Mat4::IDENTITY,
    );
    assert_eq!(hit, Some(Part::Face(front)));
    let top = face_facing(&cache, Vec3::Z);
    let hit = under(
        &cache,
        &camera,
        Mode::Faces,
        Vec3::new(0.3, 0.2, 2.0),
        Mat4::IDENTITY,
    );
    assert_eq!(hit, Some(Part::Face(top)));
}

#[test]
fn a_point_or_an_edge_is_the_nearest_corner_or_side_of_the_face_hit() {
    let (cache, camera) = scene();
    let mesh = cache.mesh();
    let corner = mesh
        .vertices()
        .find(|(_, p)| p.distance(Vec3::new(1.0, -1.0, 2.0)) < 1e-4)
        .map(|(v, _)| v)
        .unwrap();
    // Just inside the front face, near its top right corner.
    let near_corner = Vec3::new(0.9, -1.0, 1.9);
    assert_eq!(
        under(&cache, &camera, Mode::Points, near_corner, Mat4::IDENTITY),
        Some(Part::Point(corner))
    );
    // Near the front face's top side: the edge between its top corners.
    let near_top = Vec3::new(0.0, -1.0, 1.95);
    let Some(Part::Edge(edge)) = under(&cache, &camera, Mode::Edges, near_top, Mat4::IDENTITY)
    else {
        panic!("an edge");
    };
    let (a, b) = edge.ends();
    for v in [a, b] {
        let p = mesh.position(v).unwrap();
        assert!((p.z - 2.0).abs() < 1e-4 && (p.y + 1.0).abs() < 1e-4, "{p}");
    }
}

#[test]
fn nothing_is_under_the_pointer_off_the_brush() {
    let (cache, camera) = scene();
    let project = camera.projector(SIZE);
    let corner = Vec2::new(5.0, 5.0);
    let ray = camera.ray_through(corner, SIZE);
    assert_eq!(
        cache.under(Mode::Faces, (Mat4::IDENTITY, ray), &project, corner),
        None
    );
    // A brush squashed flat by its entity can't be found.
    let flat = Mat4::from_scale(Vec3::new(1.0, 0.0, 1.0));
    assert_eq!(
        cache.under(Mode::Faces, (flat, ray), &project, corner),
        None
    );
}

#[test]
fn a_placed_and_stretched_brush_is_found_where_it_is_drawn() {
    let (cache, camera) = scene();
    let world = Mat4::from_scale_rotation_translation(
        Vec3::new(2.0, 1.0, 0.5),
        glam::Quat::from_rotation_z(0.3),
        Vec3::new(0.5, 0.0, 0.0),
    );
    let top = face_facing(&cache, Vec3::Z);
    let hit = under(
        &cache,
        &camera,
        Mode::Faces,
        Vec3::new(0.2, 0.0, 2.0),
        world,
    );
    assert_eq!(hit, Some(Part::Face(top)));
}

#[test]
fn a_box_holds_every_part_whose_place_is_inside_it_far_side_too() {
    let (cache, camera) = scene();
    let project = camera.projector(SIZE);
    let all = (Vec2::ZERO, SIZE);
    assert_eq!(
        cache
            .in_box(Mode::Points, Mat4::IDENTITY, &project, all)
            .len(),
        8
    );
    assert_eq!(
        cache
            .in_box(Mode::Edges, Mat4::IDENTITY, &project, all)
            .len(),
        12
    );
    assert_eq!(
        cache
            .in_box(Mode::Faces, Mat4::IDENTITY, &project, all)
            .len(),
        6
    );
    // A box round the top face's middle alone.
    let middle = project(Vec3::new(0.0, 0.0, 2.0)).unwrap();
    let small = (middle - Vec2::splat(4.0), middle + Vec2::splat(4.0));
    let top = face_facing(&cache, Vec3::Z);
    assert_eq!(
        cache.in_box(Mode::Faces, Mat4::IDENTITY, &project, small),
        vec![Part::Face(top)]
    );
    assert!(
        cache
            .in_box(Mode::Points, Mat4::IDENTITY, &project, small)
            .is_empty()
    );
}

#[test]
fn a_distance_to_a_segment_is_to_its_nearest_point() {
    let (a, b) = (Vec2::ZERO, Vec2::new(10.0, 0.0));
    assert_eq!(to_segment(Vec2::new(5.0, 3.0), a, b), 3.0);
    assert_eq!(to_segment(Vec2::new(-4.0, 3.0), a, b), 5.0);
    assert_eq!(to_segment(Vec2::new(1.0, 1.0), a, a), 2f32.sqrt());
}

#[test]
fn a_5000_face_brush_is_tried_quickly() {
    let cube = mesh::Primitive::Cube(mesh::CubeParams {
        size: Vec3::splat(10.0),
        segments: [29, 29, 29],
    })
    .mesh();
    let cache = PartCache::of(Arc::new(cube), 1);
    let mut camera = Camera {
        position: Vec3::new(4.0, -30.0, 20.0),
        ..Camera::default()
    };
    camera.look_at(Vec3::ZERO);
    let project = camera.projector(SIZE);
    let start = std::time::Instant::now();
    let mut found = 0;
    for i in 0..20 {
        let at = Vec2::new(300.0 + i as f32 * 10.0, 300.0);
        let ray = camera.ray_through(at, SIZE);
        found += usize::from(
            cache
                .under(Mode::Faces, (Mat4::IDENTITY, ray), &project, at)
                .is_some(),
        );
    }
    assert!(found > 0);
    // Generous for a debug build; the scale check measures it in release.
    assert!(start.elapsed().as_secs_f64() < 2.0);
}
