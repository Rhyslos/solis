//! The zones' shapes in the view: footprints, 3D lines, labels, and a
//! handle's drag sizing a box (its far side kept) and a circle.

use super::sizing::{SizeDrag, handles};
use super::*;
use crate::screen::Ray;

fn zone(form: ZoneForm) -> TriggerZone {
    TriggerZone {
        form,
        size: Vec3::new(2.0, 1.0, 3.0),
        radius: 1.5,
        ..TriggerZone::default()
    }
}

#[test]
fn a_box_s_footprint_is_its_four_corners_and_a_circle_is_a_ring() {
    assert_eq!(
        footprint(&zone(ZoneForm::Box)),
        [
            Vec2::new(-1.0, -0.5),
            Vec2::new(1.0, -0.5),
            Vec2::new(1.0, 0.5),
            Vec2::new(-1.0, 0.5)
        ]
    );
    let ring = footprint(&zone(ZoneForm::Round));
    assert!(ring.len() > 16);
    assert!(ring.iter().all(|p| (p.length() - 1.5).abs() < 1e-5));
    // A freeform zone with no usable outline has none.
    assert!(footprint(&zone(ZoneForm::Freeform)).is_empty());
}

#[test]
fn a_box_has_twelve_edges_and_a_sphere_three_rings_in_3d() {
    assert_eq!(lines_3d(&zone(ZoneForm::Box)).len(), 12);
    assert_eq!(lines_3d(&zone(ZoneForm::Round)).len(), 48 * 3);
}

#[test]
fn the_label_says_the_name_and_size_as_the_mock_up_does() {
    assert_eq!(
        label("Door zone", &zone(ZoneForm::Box), true),
        "Door zone \u{b7} 2 \u{d7} 1 m"
    );
    assert_eq!(
        label("Door zone", &zone(ZoneForm::Box), false),
        "Door zone \u{b7} 2 \u{d7} 1 \u{d7} 3 m"
    );
    assert_eq!(
        label("Pit", &zone(ZoneForm::Round), true),
        "Pit \u{b7} circle 1.5 m"
    );
    assert_eq!(
        label("Pit", &zone(ZoneForm::Round), false),
        "Pit \u{b7} sphere 1.5 m"
    );
}

#[test]
fn handles_in_2d_and_3d() {
    assert_eq!(handles(&zone(ZoneForm::Box), true).len(), 8);
    assert_eq!(handles(&zone(ZoneForm::Box), false).len(), 6);
    assert_eq!(handles(&zone(ZoneForm::Round), true).len(), 4);
    assert_eq!(handles(&zone(ZoneForm::Round), false).len(), 6);
    assert!(handles(&zone(ZoneForm::Freeform), true).is_empty());
}

/// A ray straight down onto the ground at (x, y).
fn down_at(x: f32, y: f32) -> Ray {
    Ray {
        origin: Vec3::new(x, y, 10.0),
        direction: Vec3::NEG_Z,
    }
}

fn view_zone(form: ZoneForm, at: Vec3) -> ViewZone {
    ViewZone {
        entity: EntityId::random(),
        name: "Zone".into(),
        zone: zone(form),
        transform: Mat4::from_translation(at),
    }
}

#[test]
fn dragging_a_box_s_right_side_keeps_its_left_side_where_it_was() {
    let door = view_zone(ZoneForm::Box, Vec3::new(5.0, 0.0, 0.0));
    // Handle 5 is the right side (way +X), at x = 6.
    let drag = SizeDrag::new(&door, 5, true).unwrap();
    let (sized, middle) = drag.follow(&down_at(8.0, 0.3)).unwrap();
    assert_eq!(sized.size, Vec3::new(4.0, 1.0, 3.0), "4 m wide now");
    assert_eq!(middle, Vec3::new(6.0, 0.0, 0.0), "its left side stays at 4");
    // Dragged past the far side, it stays the least size, never turned
    // inside out.
    let (tiny, _) = drag.follow(&down_at(0.0, 0.0)).unwrap();
    assert_eq!(tiny.size.x, TriggerZone::MIN_SIZE_M);
}

#[test]
fn dragging_a_corner_sizes_both_ways_and_a_circle_s_handle_its_radius() {
    let door = view_zone(ZoneForm::Box, Vec3::ZERO);
    // Handle 2 is the top-right corner.
    let drag = SizeDrag::new(&door, 2, true).unwrap();
    let (sized, middle) = drag.follow(&down_at(3.0, 1.5)).unwrap();
    assert_eq!(sized.size.truncate(), Vec2::new(4.0, 2.0));
    assert_eq!(middle, Vec3::new(1.0, 0.5, 0.0));
    let pit = view_zone(ZoneForm::Round, Vec3::ZERO);
    let drag = SizeDrag::new(&pit, 0, true).unwrap();
    let (round, middle) = drag.follow(&down_at(0.0, 2.5)).unwrap();
    assert_eq!(round.radius, 2.5);
    assert_eq!(middle, Vec3::ZERO, "a circle sizes about its middle");
}

#[test]
fn a_3d_handle_follows_its_own_line() {
    let room = view_zone(ZoneForm::Box, Vec3::ZERO);
    // Handle 4 is the top face (way +Z), at z = 1.5: a ray from the side
    // at height 2.5 pulls it up to there.
    let drag = SizeDrag::new(&room, 4, false).unwrap();
    let side = Ray {
        origin: Vec3::new(10.0, 0.0, 2.5),
        direction: Vec3::NEG_X,
    };
    let (sized, middle) = drag.follow(&side).unwrap();
    assert!((sized.size.z - 4.0).abs() < 1e-5, "{}", sized.size);
    assert!((middle.z - 0.5).abs() < 1e-5, "{middle}");
}
