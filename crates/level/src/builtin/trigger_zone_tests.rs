//! Tests for the TriggerZone component: its values, its text round trip
//! in a level file, the example level, and damaged or hostile values read
//! as something usable, never a crash.

use super::*;
use crate::document::{EntityData, Level};
use crate::text;

fn registry() -> reflect::Registry {
    crate::builtin::registry().unwrap()
}

fn an_ell() -> Vec<Vec2> {
    [
        (0.0, 0.0),
        (4.0, 0.0),
        (4.0, 2.0),
        (2.0, 2.0),
        (2.0, 4.0),
        (0.0, 4.0),
    ]
    .into_iter()
    .map(|(x, y)| Vec2::new(x, y))
    .collect()
}

#[test]
fn a_new_zone_is_a_two_metre_box_that_fires_every_time() {
    let zone = TriggerZone::default();
    assert_eq!(zone.form, ZoneForm::Box);
    assert_eq!(zone.size, Vec3::splat(2.0));
    assert_eq!(zone.fires, Fires::EveryTime);
    assert_eq!(TriggerZone::from_data(&ComponentData::default()), zone);
}

#[test]
fn every_shape_round_trips_through_its_values() {
    for zone in [
        TriggerZone::default(),
        TriggerZone {
            form: ZoneForm::Round,
            radius: 3.5,
            fires: Fires::Once,
            ..TriggerZone::default()
        },
        TriggerZone {
            form: ZoneForm::Freeform,
            points: an_ell(),
            ..TriggerZone::default()
        },
    ] {
        assert_eq!(TriggerZone::from_data(&zone.to_data()), zone);
    }
}

#[test]
fn round_is_a_circle_in_2d_and_a_sphere_in_3d() {
    assert_eq!(ZoneForm::Round.shown(true), "Circle");
    assert_eq!(ZoneForm::Round.shown(false), "Sphere");
    assert_eq!(ZoneForm::Box.shown(true), "Box");
    assert_eq!(ZoneForm::named("Freeform"), ZoneForm::Freeform);
    assert_eq!(ZoneForm::named("Hexagon"), ZoneForm::Box);
}

#[test]
fn odd_sizes_are_put_right() {
    let odd = TriggerZone {
        size: Vec3::new(f32::NAN, -3.0, 1e12),
        radius: f32::INFINITY,
        ..TriggerZone::default()
    }
    .sanitized();
    assert_eq!(
        odd.size,
        Vec3::new(2.0, TriggerZone::MIN_SIZE_M, TriggerZone::MAX_SIZE_M)
    );
    assert_eq!(odd.radius, 1.0);
}

#[test]
fn hostile_point_lists_read_as_none() {
    let with_points = |points: Value| {
        let mut data = TriggerZone::default().to_data();
        data.set("points", points);
        TriggerZone::from_data(&data).points
    };
    let odd = Value::List(vec![Value::Float(1.0); 3]);
    assert!(with_points(odd).is_empty());
    let too_many = Value::List(vec![Value::Float(1.0); (MAX_OUTLINE_POINTS + 1) * 2]);
    assert!(with_points(too_many).is_empty());
    let words = Value::List(vec![Value::Text("x".into()), Value::Float(1.0)]);
    assert!(with_points(words).is_empty());
    let nan = Value::List(vec![Value::Float(f64::NAN), Value::Float(1.0)]);
    assert!(with_points(nan).is_empty());
    assert!(with_points(Value::Int(4)).is_empty());
    let ints = Value::List(vec![Value::Int(1), Value::Int(2)]);
    assert_eq!(with_points(ints), [Vec2::new(1.0, 2.0)]);
}

#[test]
fn an_outline_that_crosses_itself_is_kept_but_not_usable() {
    let bow_tie = TriggerZone {
        form: ZoneForm::Freeform,
        points: vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(0.0, 2.0),
        ],
        ..TriggerZone::default()
    };
    let read = TriggerZone::from_data(&bow_tie.to_data());
    assert_eq!(read.points.len(), 4, "nothing is lost");
    assert!(read.outline().is_err());
    let ell = TriggerZone {
        points: an_ell(),
        ..bow_tie
    };
    assert_eq!(ell.outline().map(<[Vec2]>::len), Ok(6));
}

#[test]
fn zones_round_trip_through_a_level_file() {
    let mut level = Level::new();
    level.spawn(EntityData::new("Door zone").with(&TriggerZone::default()));
    level.spawn(EntityData::new("Pit").with(&TriggerZone {
        form: ZoneForm::Round,
        radius: 1.5,
        fires: Fires::Once,
        ..TriggerZone::default()
    }));
    level.spawn(EntityData::new("Yard").with(&TriggerZone {
        form: ZoneForm::Freeform,
        points: an_ell(),
        ..TriggerZone::default()
    }));
    let written = text::write(&level, &registry(), &|_| None);
    let read = text::read(&written, &registry()).unwrap();
    assert_eq!(read, level);
    assert_eq!(text::write(&read, &registry(), &|_| None), written);
}

#[test]
fn the_zones_example_is_what_this_build_writes() {
    // tests/fixtures/area/v6/zones.sollevel: a box, a circle that fires
    // once, and a freeform L.
    let zones = include_str!("../../../../tests/fixtures/area/v6/zones.sollevel");
    assert_eq!((text::FORMAT.version_of)(zones), Some(6));
    let level = text::read(zones, &registry()).unwrap();
    let forms: Vec<ZoneForm> = level.with::<TriggerZone>().map(|(_, z)| z.form).collect();
    assert_eq!(forms, [ZoneForm::Box, ZoneForm::Round, ZoneForm::Freeform]);
    for (_, zone) in level.with::<TriggerZone>() {
        if zone.form == ZoneForm::Freeform {
            assert!(zone.outline().is_ok());
        }
    }
    assert_eq!(text::write(&level, &registry(), &|_| None), zones);
}

#[test]
fn randomly_damaged_zone_files_never_crash() {
    let zones = include_str!("../../../../tests/fixtures/area/v6/zones.sollevel");
    let bytes = zones.as_bytes();
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    for _ in 0..3_000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut damaged = bytes.to_vec();
        for k in 0..1 + (seed % 4) as usize {
            let at = ((seed >> (8 * k)) as usize) % damaged.len();
            damaged[at] = (seed >> (3 + k)) as u8;
        }
        let text = String::from_utf8_lossy(&damaged);
        if let Ok(level) = text::read(&text, &registry()) {
            for (_, zone) in level.with::<TriggerZone>() {
                assert!(zone.points.len() <= MAX_OUTLINE_POINTS);
                let _ = zone.outline();
            }
        }
    }
}
