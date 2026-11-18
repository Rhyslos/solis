//! The game's state part by part: every part is named after its entity
//! and component, a change shows in exactly the part it changed, and
//! numbers are written exactly.

use super::*;
use glam::Vec3;
use level::builtin::Transform;
use level::{EntityData, Level};

fn crate_and_turret() -> (World, EntityId, EntityId) {
    let mut level = Level::new();
    let boxed = level.spawn(EntityData::new("Crate").with(&Transform::default()));
    let mut turret = EntityData::new("Turret").with(&Transform::default());
    let mut aim = ComponentData::default();
    aim.set("speed", Value::Float(2.5));
    aim.set("label", Value::Text("left\nright".into()));
    turret.components.insert("Aim".into(), aim);
    let turret = level.spawn(turret);
    (World::from_level(&level), boxed, turret)
}

#[test]
fn each_part_is_named_after_its_entity_and_component() {
    let (world, boxed, turret) = crate_and_turret();
    let parts = world.state_parts();
    let place = format!("{boxed} \"Crate\" Transform (in the world)");
    assert!(parts.contains_key(&place), "{parts:#?}");
    let aim = &parts[&format!("{turret} \"Turret\" Aim (as placed)")];
    assert_eq!(aim, "label = \"left\\nright\", speed = 2.5");
    assert_eq!(parts["game seed"], world.seed().to_string());
    // One line each: nothing breaks a line.
    assert!(
        parts
            .iter()
            .all(|(k, v)| !k.contains('\n') && !v.contains('\n'))
    );
}

#[test]
fn a_change_shows_in_the_part_it_changed_and_nowhere_else() {
    let (mut world, boxed, _) = crate_and_turret();
    let before = world.state_parts();
    let nudged = Transform {
        position: Vec3::new(0.0, 0.0, 0.001),
        ..Transform::default()
    };
    world.set_transform(boxed, nudged);
    world.step(crate::TICK);
    let after = world.state_parts();
    let changed: Vec<&String> = after
        .iter()
        .filter(|(k, v)| before.get(*k) != Some(*v))
        .map(|(k, _)| k)
        .collect();
    assert_eq!(
        changed,
        vec![&format!("{boxed} \"Crate\" Transform (in the world)")]
    );
}

#[test]
fn numbers_are_written_exactly() {
    let third = 1.0_f64 / 3.0;
    let text = value_text(&Value::Float(third));
    assert_eq!(text.parse::<f64>().unwrap().to_bits(), third.to_bits());
    let list = Value::List(vec![Value::Int(-2), Value::Bool(true), Value::Entity(None)]);
    assert_eq!(value_text(&list), "[-2, true, none]");
}
