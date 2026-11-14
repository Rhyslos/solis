//! Tests for the brushes' shapes: each closed, facing out, filling its
//! box, and within limits whatever it is asked for.

use super::*;
use crate::MeshReport;
use crate::flat::solid_triangles;

/// The space inside a closed mesh: positive when its faces face out.
fn volume(mesh: &Mesh) -> f32 {
    let (points, triangles) = solid_triangles(mesh);
    triangles
        .iter()
        .map(|[a, b, c]| {
            let [a, b, c] = [a, b, c].map(|i| points[*i as usize]);
            a.dot(b.cross(c)) / 6.0
        })
        .sum()
}

fn check_solid(shape: Shape, settings: &ShapeSettings) {
    let mesh = shape.build(settings);
    let report = MeshReport::new(&mesh);
    let name = shape.name();
    assert!(report.is_closed(), "{name}: {report:?}");
    assert!(!report.has_problems(), "{name}: {report:?}");
    assert_eq!(report.pieces, 1, "{name}");
    assert!(volume(&mesh) > 0.0, "{name} faces out");
    let size = settings.clamped().size;
    let bounds = mesh.bounds().unwrap();
    let slack = size.max_element() * 1e-4;
    let (low, high) = (
        Vec3::new(-size.x / 2.0, -size.y / 2.0, 0.0),
        Vec3::new(size.x / 2.0, size.y / 2.0, size.z),
    );
    assert!(
        bounds.min.cmpge(low - slack).all() && bounds.max.cmple(high + slack).all(),
        "{name} stays in its box: {bounds:?}"
    );
    // A round shape with a few sides can't touch every side of its box;
    // the others fill it.
    let round = matches!(shape, Shape::Cylinder | Shape::Sphere);
    if !round || settings.clamped().sides.is_multiple_of(4) {
        let close = |a: Vec3, b: Vec3| (a - b).abs().max_element() < slack;
        assert!(
            close(bounds.min, low) && close(bounds.max, high),
            "{name}: {bounds:?}"
        );
    }
}

#[test]
fn every_shape_is_closed_faces_out_and_fills_its_box() {
    for shape in Shape::ALL {
        check_solid(shape, &shape.standard());
        let odd = ShapeSettings {
            size: Vec3::new(0.5, 7.0, 3.0),
            sides: 5,
            steps: 3,
            roundness: 0.4,
        };
        check_solid(shape, &odd);
    }
}

#[test]
fn every_setting_s_extremes_still_make_a_solid() {
    for shape in Shape::ALL {
        for sides in [0, MIN_SIDES, MAX_SIDES, u32::MAX] {
            for steps in [0, 1, MAX_STEPS, u32::MAX] {
                for roundness in [0.0, 1.0, -5.0, f32::NAN] {
                    let settings = ShapeSettings {
                        size: Vec3::new(1.0, 2.0, 3.0),
                        sides,
                        steps,
                        roundness,
                    };
                    check_solid(shape, &settings);
                }
            }
        }
    }
}

#[test]
fn sizes_out_of_range_or_not_numbers_are_brought_in() {
    let wild = ShapeSettings {
        size: Vec3::new(0.0, f32::INFINITY, -3.0),
        ..ShapeSettings::default()
    };
    let clamped = wild.clamped();
    assert_eq!(clamped.size, Vec3::new(MIN_SIZE, 2.0, MIN_SIZE));
    let huge = ShapeSettings {
        size: Vec3::splat(1e30),
        sides: 1_000_000,
        steps: 1_000_000,
        roundness: 9.0,
    }
    .clamped();
    assert_eq!(huge.size, Vec3::splat(MAX_SIZE));
    assert_eq!(
        (huge.sides, huge.steps, huge.roundness),
        (MAX_SIDES, MAX_STEPS, 1.0)
    );
    for shape in Shape::ALL {
        check_solid(shape, &wild);
        check_solid(shape, &huge);
    }
}

#[test]
fn no_setting_makes_a_huge_mesh() {
    let most = ShapeSettings {
        sides: u32::MAX,
        steps: u32::MAX,
        ..ShapeSettings::default()
    };
    for shape in Shape::ALL {
        let faces = shape.build(&most).face_count();
        assert!(faces <= 2_048, "{}: {faces} faces", shape.name());
    }
}

#[test]
fn the_box_is_six_faces_and_the_stairs_have_their_steps() {
    let cube = Shape::Box.build(&Shape::Box.standard());
    assert_eq!((cube.vertex_count(), cube.face_count()), (8, 6));
    let stairs = ShapeSettings {
        steps: 4,
        ..Shape::Stairs.standard()
    };
    // Two sides, the bottom, the back, and a tread and a riser per step.
    assert_eq!(Shape::Stairs.build(&stairs).face_count(), 4 + 2 * 4);
}

#[test]
fn ramps_and_stairs_climb_away_from_the_front() {
    for shape in [Shape::Ramp, Shape::Stairs] {
        let mesh = shape.build(&shape.standard());
        let height_at = |y: f32| {
            mesh.vertices()
                .filter(|(_, p)| (p.y - y).abs() < 1e-4)
                .map(|(_, p)| p.z)
                .fold(0.0_f32, f32::max)
        };
        let size = shape.standard().size;
        assert!(height_at(size.y / 2.0) > height_at(-size.y / 2.0) + 0.1);
    }
}

#[test]
fn an_arch_has_a_way_through() {
    let mesh = Shape::Arch.build(&Shape::Arch.standard());
    // Nothing of the arch is in the middle of its doorway.
    let (points, triangles) = solid_triangles(&mesh);
    let crossed = triangles.iter().any(|[a, b, c]| {
        let [a, b, c] = [a, b, c].map(|i| points[*i as usize]);
        // A ray along Y through the doorway's middle, low down.
        let (o, dir) = (Vec3::new(0.0, -5.0, 0.5), Vec3::Y);
        let (e1, e2) = (b - a, c - a);
        let p = dir.cross(e2);
        let det = e1.dot(p);
        if det.abs() < 1e-9 {
            return false;
        }
        let t = o - a;
        let u = t.dot(p) / det;
        let q = t.cross(e1);
        let v = dir.dot(q) / det;
        u >= 0.0 && v >= 0.0 && u + v <= 1.0
    });
    assert!(!crossed);
}

#[test]
fn the_same_settings_give_the_same_mesh() {
    for shape in Shape::ALL {
        assert_eq!(
            shape.build(&shape.standard()),
            shape.build(&shape.standard())
        );
    }
}

#[test]
fn names_read_back() {
    for shape in Shape::ALL {
        assert_eq!(Shape::from_name(shape.name()), Some(shape));
    }
    assert_eq!(Shape::from_name("Teapot"), None);
}
