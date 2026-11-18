//! Tags in the running game: as the level gives them, added and taken
//! away, listed with places in ID order, in the checksum and the state
//! parts, kept through an edit while playing, and zones limited to tags.

use super::*;
use level::builtin::{Collider2D, Transform, TriggerZone};
use level::{EntityData, Level};

fn at(x: f32, y: f32) -> Transform {
    Transform {
        position: Vec3::new(x, y, 0.0),
        ..Transform::default()
    }
}

#[test]
fn tags_come_from_the_level_and_change_while_playing() {
    let (enemy, flying) = (TagId::random(), TagId::random());
    let mut level = Level::new();
    let bat = level.spawn(EntityData::new("Bat").with(&at(1.0, 2.0)).with(&Tags::of([enemy])));
    let rock = level.spawn(EntityData::new("Rock").with(&at(3.0, 0.0)));
    let mut world = World::from_level(&level);
    assert_eq!(world.tags_of(bat).ids, [enemy]);
    assert!(world.tags_of(rock).ids.is_empty());
    assert!(world.set_tag(bat, flying, true));
    assert!(!world.set_tag(bat, flying, true), "never twice");
    assert!(world.set_tag(rock, enemy, true), "given a Tags");
    assert!(!world.set_tag(rock, flying, false), "it hasn't that one");
    assert!(!world.set_tag(EntityId::random(), enemy, true));
    assert_eq!(world.tags_of(bat).ids, [enemy, flying]);
    let tagged = world.tagged();
    let mut ids = vec![bat, rock];
    ids.sort_unstable();
    assert_eq!(tagged.iter().map(|t| t.0).collect::<Vec<_>>(), ids);
    let bat_place = tagged.iter().find(|t| t.0 == bat).unwrap().1;
    assert_eq!(bat_place, Vec3::new(1.0, 2.0, 0.0));
    assert!(world.set_tag(bat, enemy, false));
    assert_eq!(world.tags_of(bat).ids, [flying]);
}

#[test]
fn tags_are_in_the_checksum_and_the_state_parts_only_once_carried() {
    let mut level = Level::new();
    let bat = level.spawn(EntityData::new("Bat").with(&at(1.0, 2.0)));
    let mut world = World::from_level(&level);
    let before = world.checksum();
    assert!(
        !world.state_parts().keys().any(|k| k.ends_with(" Tags")),
        "nothing while nothing is tagged"
    );
    let enemy = TagId::random();
    world.set_tag(bat, enemy, true);
    assert_ne!(world.checksum(), before);
    let parts = world.state_parts();
    let (_, text) = parts.iter().find(|(k, _)| k.ends_with(" Tags")).unwrap();
    assert_eq!(text, &enemy.to_string());
    world.set_tag(bat, enemy, false);
    assert_eq!(world.checksum(), before, "an entity carrying none adds nothing");
}

#[test]
fn tags_game_code_set_outlive_an_edit_while_playing() {
    let enemy = TagId::random();
    let mut level = Level::new();
    let bat = level.spawn(EntityData::new("Bat").with(&at(1.0, 2.0)));
    let tagged = level.spawn(
        EntityData::new("Crow")
            .with(&at(0.0, 0.0))
            .with(&Tags::of([enemy])),
    );
    let mut world = World::from_level(&level);
    world.set_tag(bat, enemy, true);
    world.set_tag(tagged, enemy, false);
    // The level edited elsewhere: the world is made again from it.
    level.get_mut(bat).unwrap().name = "Big bat".into();
    world.rebuild(&level);
    assert_eq!(world.tags_of(bat).ids, [enemy], "the game's tag is kept");
    assert!(world.tags_of(tagged).ids.is_empty(), "and the one it took away");
    // An edit of the level's own tags wins.
    level
        .get_mut(tagged)
        .unwrap()
        .set(&Tags::of([enemy, TagId::random()]));
    world.update(&level, [(tagged, false)]);
    assert_eq!(world.tags_of(tagged).ids.len(), 2);
}

#[test]
fn a_zone_limited_to_tags_notices_only_those_carrying_one() {
    let (player, ghost) = (TagId::random(), TagId::random());
    let mut level = Level::new();
    let walker = |name: &str, y: f32| {
        EntityData::new(name).with(&at(5.0, y)).with(&Collider2D {
            radius: 4.0,
            character: true,
            ..Collider2D::default()
        })
    };
    let door = level.spawn(EntityData::new("Door").with(&at(5.0, 0.0)).with(&TriggerZone {
        only: vec![player, ghost],
        ..TriggerZone::default()
    }));
    let hero = level.spawn(walker("Hero", 0.2).with(&Tags::of([player])));
    let cat = level.spawn(walker("Cat", -0.2));
    let mut world = World::from_level(&level);
    world.step_physics();
    let crossings = world.overlap_zones();
    assert_eq!(world.inside(door), [hero], "{crossings:?}");
    // The cat is given the tag: from the next pass it counts.
    world.set_tag(cat, ghost, true);
    world.step_physics();
    world.overlap_zones();
    let mut both = vec![hero, cat];
    both.sort_unstable();
    assert_eq!(world.inside(door), both.as_slice());
    // And the hero losing it walks out, as far as the zone is concerned.
    world.set_tag(hero, player, false);
    world.step_physics();
    let crossings = world.overlap_zones();
    assert_eq!(world.inside(door), [cat]);
    assert!(crossings.iter().any(|c| c.who == hero && !c.entered));
}
