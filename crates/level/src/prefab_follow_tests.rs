//! Copies following a prefab's new values give exactly what rebuilding
//! them the whole way gives (collapsing each to its own changes and
//! expanding it from the new prefab), and anything but values is left to
//! that whole way.

use super::*;
use crate::Origin;
use crate::builtin::{Light, Transform};
use crate::prefab::Expander;
use crate::text::{FileKind, LevelFile};
use std::sync::Arc;

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

const POST: u64 = 900;

fn asset(n: u64) -> AssetId {
    AssetId(reflect::Id::from_raw(n).unwrap())
}

/// A lamp post: a post with a lamp under it, the lamp as `lamp` has it.
fn lamp_post(lamp: Light) -> LevelFile {
    let mut level = Level::new();
    level.insert(
        id(1),
        EntityData::new("Lamp post").with(&Transform::default()),
    );
    let mut part = EntityData::new("Lamp").with(&lamp);
    part.parent = Some(id(1));
    level.insert(id(2), part);
    LevelFile {
        kind: FileKind::Prefab { root: id(1) },
        level,
        overrides: Default::default(),
    }
}

/// `copies` copies of `file` placed in a level beside a plain entity.
fn placed(file: &LevelFile, copies: u64) -> Level {
    let file = Arc::new(file.clone());
    let lookup = move |_| Some(file.clone());
    let mut expander = Expander::new(&lookup);
    let mut level = Level::new();
    level.insert(
        id(5000),
        EntityData::new("Bench").with(&Transform::default()),
    );
    for n in 0..copies {
        let root = id(1000 + n);
        let mut at = EntityData::new(&format!("Post {n}"));
        at.order = format!("a{n}");
        for (part, data) in expander.place(asset(POST), root, &at).unwrap() {
            level.insert(part, data);
        }
    }
    level
}

/// The copies rebuilt the whole way: collapsed against `old`, expanded
/// from `new`.
fn rebuilt(level: &Level, old: &LevelFile, new: &LevelFile) -> Level {
    let (old, new) = (Arc::new(old.clone()), Arc::new(new.clone()));
    let old_lookup = move |_| Some(old.clone());
    let new_lookup = move |_| Some(new.clone());
    let file = Expander::new(&old_lookup)
        .collapse(level, FileKind::Level)
        .unwrap();
    Expander::new(&new_lookup).expand(&file).unwrap()
}

/// The copies following by [`ValueChanges::follow`], if it applies.
fn followed(level: &Level, old: &LevelFile, new: &LevelFile) -> Option<Level> {
    let (old, new) = (Arc::new(old.clone()), Arc::new(new.clone()));
    let old_lookup = move |_| Some(old.clone());
    let new_lookup = move |_| Some(new.clone());
    let was = Expander::new(&old_lookup).prefab(asset(POST)).unwrap();
    let now = Expander::new(&new_lookup).prefab(asset(POST)).unwrap();
    let changes = ValueChanges::between((&was.0, was.1), (&now.0, now.1))?;
    let mut level = level.clone();
    for edit in changes.follow(&level, asset(POST)) {
        level.apply(edit).unwrap();
    }
    Some(level)
}

/// The lamp of copy `n`.
fn lamp_of(n: u64) -> EntityId {
    crate::prefab::derive(id(1000 + n), &[id(2)])
}

#[test]
fn copies_follow_new_values_and_keep_their_own() {
    let old = lamp_post(Light::point());
    let new = lamp_post(Light {
        strength: 7.0,
        color: [1.0, 0.5, 0.25],
        ..Light::point()
    });
    let mut level = placed(&old, 4);
    // Copy 1 has its own strength, copy 2 its own colour, copy 3 took
    // its Light off.
    level.get_mut(lamp_of(1)).unwrap().set(&Light {
        strength: 42.0,
        ..Light::point()
    });
    let data = level.get_mut(lamp_of(2)).unwrap();
    data.components
        .get_mut("Light")
        .unwrap()
        .fields
        .insert("color".into(), Value::floats(&[0.0, 1.0, 0.0]));
    level
        .get_mut(lamp_of(3))
        .unwrap()
        .components
        .remove("Light");
    let fast = followed(&level, &old, &new).expect("only values changed");
    assert_eq!(fast, rebuilt(&level, &old, &new));
    let strength = |l: &Level, n| {
        l.get(lamp_of(n))
            .unwrap()
            .get::<Light>()
            .map(|l| l.strength)
    };
    assert_eq!(strength(&fast, 0), Some(7.0));
    assert_eq!(strength(&fast, 1), Some(42.0), "its own change wins");
    assert_eq!(strength(&fast, 2), Some(7.0));
    assert_eq!(strength(&fast, 3), None, "taken off stays off");
}

#[test]
fn the_same_prefab_changes_nothing() {
    let old = lamp_post(Light::point());
    let level = placed(&old, 3);
    assert_eq!(followed(&level, &old, &old), Some(level));
}

#[test]
fn anything_but_values_is_left_to_the_whole_way() {
    let old = lamp_post(Light::point());
    let level = placed(&old, 2);
    // A part renamed, a part added, a component added, a part moved.
    let mut renamed = old.clone();
    renamed.level.get_mut(id(2)).unwrap().name = "Bulb".into();
    let mut added = old.clone();
    let mut extra = EntityData::new("Sign");
    extra.parent = Some(id(1));
    added.level.insert(id(3), extra);
    let mut component = old.clone();
    component
        .level
        .get_mut(id(2))
        .unwrap()
        .set(&Transform::default());
    let mut moved = old.clone();
    moved.level.get_mut(id(2)).unwrap().order = "z".into();
    for new in [renamed, added, component, moved] {
        assert_eq!(followed(&level, &old, &new), None);
    }
    // A value naming an entity is each copy's own part.
    let mut aimed = old.clone();
    aimed
        .level
        .get_mut(id(2))
        .unwrap()
        .components
        .get_mut("Light")
        .unwrap()
        .fields
        .insert("target".into(), Value::Entity(Some(id(1))));
    assert_eq!(followed(&level, &old, &aimed), None);
    // The root's name is each copy's own, so it isn't a change.
    let mut root_named = old.clone();
    root_named.level.get_mut(id(1)).unwrap().name = "Street lamp".into();
    let fast = followed(&level, &old, &root_named).expect("the root's name is the copy's");
    assert_eq!(fast, rebuilt(&level, &old, &root_named));
}

#[test]
fn many_random_changes_follow_as_the_whole_way_does() {
    // A small fixed-seed generator: the same cases every run.
    let mut seed = 0x5EED_u64;
    let mut next = move |n: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % n
    };
    let strengths = [1.0, 7.0, 42.0, 1000.0];
    for case in 0..200 {
        let pick = |n: u64| Light {
            strength: strengths[n as usize % 4],
            shadows: n.is_multiple_of(2),
            ..Light::point()
        };
        let (was, now) = (pick(next(8)), pick(next(8)));
        let shadows_changed = was.shadows != now.shadows;
        let (old, new) = (lamp_post(was), lamp_post(now));
        let mut level = placed(&old, 6);
        for n in 0..6 {
            match next(4) {
                0 => {}
                1 => level.get_mut(lamp_of(n)).unwrap().set(&pick(next(8))),
                2 => drop(
                    level
                        .get_mut(lamp_of(n))
                        .unwrap()
                        .components
                        .remove("Light"),
                ),
                // A copy without the field takes the prefab's new value.
                // (One the prefab didn't change stays as it is: the whole
                // way would fill it in too, but that isn't this save's.)
                _ if shadows_changed => drop(
                    level
                        .get_mut(lamp_of(n))
                        .unwrap()
                        .components
                        .get_mut("Light")
                        .unwrap()
                        .fields
                        .remove("shadows"),
                ),
                _ => {}
            }
        }
        let fast = followed(&level, &old, &new).expect("only values changed");
        assert_eq!(fast, rebuilt(&level, &old, &new), "case {case}");
    }
}

#[test]
fn copies_of_another_prefab_are_left_alone() {
    let old = lamp_post(Light::point());
    let new = lamp_post(Light {
        strength: 7.0,
        ..Light::point()
    });
    let mut level = placed(&old, 1);
    let lamp = lamp_of(0);
    level.get_mut(lamp).unwrap().origin = Some(Origin {
        prefab: asset(1),
        instance: id(1000),
        path: vec![id(2)],
    });
    let (was, now) = (old.level.clone(), new.level.clone());
    let changes = ValueChanges::between((&was, id(1)), (&now, id(1))).unwrap();
    assert!(!changes.is_empty());
    let edits = changes.follow(&level, asset(POST));
    assert!(
        !edits
            .iter()
            .any(|e| matches!(e, Edit::SetField { entity, .. } if *entity == lamp)),
        "{edits:?}"
    );
}
