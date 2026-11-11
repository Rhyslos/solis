//! What game code set on an entity outlives an edit while playing, of
//! one entity (`update`) and of everything (`rebuild`); an edit that
//! changes what the area gives, or sets it by hand, wins, part by part.

use super::*;
use area::builtin::{Light, Transform};
use area::{Area, EntityData};

/// A sign with a Sprite, a sound that plays, an Animator, a Light (the
/// component the edits change) and no Screen.
fn sign() -> (World, Area, EntityId) {
    let mut area = Area::new();
    let id = area.spawn(
        EntityData::new("Sign")
            .with(&Transform::default())
            .with(&Light::default())
            .with(&Sprite::default())
            .with(&AudioSource::default())
            .with(&Animator::default()),
    );
    (World::from_area(&area), area, id)
}

/// Game code mirrors the sign, plays a clip on it, stops its sound,
/// plays its animator at double speed and shows a screen for it.
fn game_sets(world: &mut World, id: EntityId) {
    world.set_flip(id, true, false);
    world.play_clip(id, "wave", false);
    world.set_sound_playing(id, false);
    world.set_animator_speed(id, 2.0);
    world.set_screen(id, "Hud", true);
}

/// What the game made of the sign: flipped across, the clip, whether it
/// sounds, its animator's speed and the screen it shows.
type Made = (bool, String, bool, f32, Option<String>);

fn made(world: &World, id: EntityId) -> Made {
    let sprite = world.get::<Sprite>(id).unwrap();
    (
        sprite.flip_x,
        sprite.clip,
        world.get::<AudioSource>(id).unwrap().playing,
        world.get::<Animator>(id).unwrap().speed,
        world.screen_shown(id),
    )
}

fn game_made() -> Made {
    (true, "wave".into(), false, 2.0, Some("Hud".into()))
}

/// The area with the sign's `T` changed by `change`.
fn edit<T: Component>(area: &mut Area, id: EntityId, change: impl FnOnce(&mut T)) {
    let mut data = area.get(id).unwrap().clone();
    let mut component = data.get::<T>().unwrap();
    change(&mut component);
    data.set(&component);
    area.insert(id, data);
}

/// Editing any component of an entity while playing made it again from
/// the area and dropped what game code had set on it: a screen it showed
/// was hidden, a sound it stopped started again, its flip and its
/// animator's speed went back (the full review). A rebuild kept the
/// screen and dropped the rest.
#[test]
fn an_edit_of_another_component_keeps_everything_game_code_set() {
    let (mut world, mut area, id) = sign();
    assert_eq!(made(&world, id), (false, String::new(), true, 1.0, None));
    game_sets(&mut world, id);
    assert_eq!(made(&world, id), game_made());
    let sum = world.checksum();
    edit::<Light>(&mut area, id, |light| light.strength += 1.0);
    world.update(&area, [(id, false)]);
    assert_eq!(made(&world, id), game_made(), "after an edit");
    assert_eq!(world.checksum(), sum, "the game is where it was");
    edit::<Light>(&mut area, id, |light| light.strength += 1.0);
    world.rebuild(&area);
    assert_eq!(made(&world, id), game_made(), "after a rebuild");
    assert_eq!(world.checksum(), sum);
    // And again: what's kept once is kept every time.
    world.update(&area, [(id, false)]);
    world.rebuild(&area);
    assert_eq!(made(&world, id), game_made());
    assert_eq!(world.checksum(), sum);
}

#[test]
fn an_edit_that_changes_what_the_area_gives_wins_for_that_part_alone() {
    for rebuild in [false, true] {
        let again = |world: &mut World, area: &Area, id| {
            if rebuild {
                world.rebuild(area);
            } else {
                world.update(area, [(id, false)]);
            }
        };
        let (mut world, mut area, id) = sign();
        game_sets(&mut world, id);
        // The sound's volume changed: its being on or off is still the
        // game's.
        edit::<AudioSource>(&mut area, id, |source| source.volume_db = -6.0);
        again(&mut world, &area, id);
        assert_eq!(made(&world, id), game_made(), "{rebuild}");
        assert_eq!(world.get::<AudioSource>(id).unwrap().volume_db, -6.0);
        // The area's flip changed: the area's now, the rest the game's.
        edit::<Sprite>(&mut area, id, |sprite| sprite.flip_y = true);
        again(&mut world, &area, id);
        let sprite = world.get::<Sprite>(id).unwrap();
        assert_eq!((sprite.flip_x, sprite.flip_y), (false, true), "{rebuild}");
        assert_eq!(sprite.clip, "wave", "its clip is another part");
        // Each of the others in turn.
        edit::<Sprite>(&mut area, id, |sprite| sprite.clip = "bow".into());
        edit::<AudioSource>(&mut area, id, |source| source.playing = false);
        edit::<AudioSource>(&mut area, id, |source| source.playing = true);
        again(&mut world, &area, id);
        assert_eq!(world.get::<Sprite>(id).unwrap().clip, "bow", "{rebuild}");
        // Off and on again leaves the area as it was: the game's stays.
        assert!(!world.get::<AudioSource>(id).unwrap().playing, "{rebuild}");
        edit::<Animator>(&mut area, id, |animator| animator.speed = 0.5);
        again(&mut world, &area, id);
        assert_eq!(world.get::<Animator>(id).unwrap().speed, 0.5, "{rebuild}");
        assert_eq!(world.screen_shown(id).as_deref(), Some("Hud"));
        // A component taken away takes the game's part of it along.
        let mut data = area.get(id).unwrap().clone();
        data.components.remove(AudioSource::NAME);
        area.insert(id, data);
        again(&mut world, &area, id);
        assert!(world.get::<AudioSource>(id).is_none(), "{rebuild}");
        assert_eq!(world.screen_shown(id).as_deref(), Some("Hud"));
    }
}

/// A field set by hand in the editor takes the area's value even when
/// that's the value the area held all along (the review's BUG-54, for
/// Solis's own components): the part it belongs to goes back to the
/// area's, and everything else game code set stays.
#[test]
fn a_field_set_by_hand_takes_the_area_s_value_and_the_rest_stays_the_game_s() {
    for rebuild in [false, true] {
        let (mut world, area, id) = sign();
        game_sets(&mut world, id);
        if rebuild {
            world.rebuild(&area);
        } else {
            world.update(&area, [(id, false)]);
        }
        world.set_by_hand(id, AudioSource::NAME, "playing");
        let (flip, clip, playing, speed, screen) = made(&world, id);
        assert!(playing, "the area's, set by hand ({rebuild})");
        assert_eq!(
            (flip, clip, speed, screen),
            (true, "wave".into(), 2.0, Some("Hud".into()))
        );
        // A field game code doesn't set is the area's already, and an
        // entity or a field that isn't there is no trouble.
        let before = made(&world, id);
        world.set_by_hand(id, AudioSource::NAME, "volume_db");
        world.set_by_hand(id, Light::NAME, "strength");
        world.set_by_hand(id, "Nothing", "playing");
        world.set_by_hand(EntityId::random(), AudioSource::NAME, "playing");
        assert_eq!(made(&world, id), before);
        // Each other part by one of its fields.
        world.set_by_hand(id, Sprite::NAME, "flip_y");
        world.set_by_hand(id, Sprite::NAME, "time");
        world.set_by_hand(id, Animator::NAME, "speed");
        assert_eq!(
            made(&world, id),
            (false, String::new(), true, 1.0, Some("Hud".into()))
        );
    }
}

#[test]
fn a_screen_game_code_showed_or_hid_is_kept_part_by_part_too() {
    use area::builtin::Screen;
    let mut area = Area::new();
    let menu = area.spawn(
        EntityData::new("Menu")
            .with(&Light::default())
            .with(&Screen::showing("Menu")),
    );
    let mut world = World::from_area(&area);
    world.set_screen(menu, "Menu", false);
    assert_eq!(world.screen_shown(menu), None, "game code hid it");
    let order = |world: &World| {
        Screen::from_data(&world.values_named_for(menu, Screen::NAME).unwrap()).order
    };
    // Another field of the Screen itself edited: the new order is taken,
    // and it's still hidden.
    edit::<Screen>(&mut area, menu, |screen| screen.order = 5);
    world.update(&area, [(menu, false)]);
    assert_eq!((world.screen_shown(menu), order(&world)), (None, 5));
    world.rebuild(&area);
    assert_eq!((world.screen_shown(menu), order(&world)), (None, 5));
    // Set by hand to shown, as the area has it all along: shown.
    world.set_by_hand(menu, Screen::NAME, "shown");
    assert_eq!(world.screen_shown(menu).as_deref(), Some("Menu"));
    // Game code shows another; the area then names a third: the area's.
    world.set_screen(menu, "Pause", true);
    edit::<Screen>(&mut area, menu, |screen| screen.screen = "Shop".into());
    world.update(&area, [(menu, false)]);
    assert_eq!(world.screen_shown(menu).as_deref(), Some("Shop"));
    // A Screen game code gave an entity that had none goes when the
    // entity does, and never lands on another.
    let other = area.spawn(EntityData::new("Other").with(&Light::default()));
    world.rebuild(&area);
    world.set_screen(other, "Hud", true);
    area.remove(other);
    world.rebuild(&area);
    assert_eq!(world.screens_shown().len(), 1);
}

/// The fields each part names are fields its component has: a field
/// renamed in the area's components can't quietly stop being kept.
#[test]
fn every_part_names_fields_its_component_has() {
    use area::builtin::Screen;
    for part in Part::ALL {
        let (component, fields) = part.fields();
        let data = match part {
            Part::Playback | Part::SpriteSpeed | Part::Flip => Sprite::default().to_data(),
            Part::Sounding => AudioSource::default().to_data(),
            Part::AnimatorState | Part::AnimatorSpeed => Animator::default().to_data(),
            Part::Screen => Screen::default().to_data(),
        };
        assert!(!fields.is_empty(), "{part:?}");
        for field in fields {
            assert!(data.get(field).is_some(), "{component} has no {field}");
            assert_eq!(Part::of(component, field), Some(part));
        }
    }
    assert_eq!(Part::of(Sprite::NAME, "tint"), None);
}
