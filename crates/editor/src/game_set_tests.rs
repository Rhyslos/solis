//! What game code sets on an entity outlives edits made while playing
//! (the full review, BUG-16): every one of game code's calls is listed
//! here as setting something on an entity or not, and each that does is
//! played, then an edit of another component of that entity (which makes
//! the one entity again from the area) and an edit that builds the whole
//! world again leave the game exactly where the same game without the
//! edits is.
//!
//! A call added to game code fails `every_call_is_listed` until it's put
//! in one of the two lists; in the first, it's played like the rest, and
//! fails until the world keeps what it sets (`world::game_set`).

use crate::game_code::PlayWorld;
use crate::properties::PropertyRequest;
use crate::test_support::*;
use crate::workbench::Workbench;
use animation::{AnimatorFile, Parameter, ParameterKind, Plays, State};
use area::EntityData;
use area::builtin::{
    Animator, AudioSource, CharacterBody, Collider, Light, RigidBody, Screen, Sprite, TileMap,
    Transform,
};
use glam::{Quat, Vec3};
use reflect::{AssetId, AssetRef, Component, EntityId, Value};
use script::{AnimatorValue, Change, GameWorld};
use std::sync::Arc;
use tiles::{CellValue, ValueKind};

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

/// The sign: a Sprite playing "wave", a sound that plays, an Animator, a
/// tile map, a body, a Screen showing the Hud, and a Light for the edits
/// to change.
const SIGN: u64 = 1;
/// The walker: a character, with a Light too.
const WALKER: u64 = 2;
/// The tile map's file, as game code names it.
const MAP: &str = "maps/farm";

/// What one of game code's calls asks of the world.
type Asks = fn() -> Change;

/// Every call of game code's that sets something on an entity, by its
/// key ([`script::describe::Native::key`]), with what it asks of the
/// world here.
const ON_AN_ENTITY: &[(&str, Asks)] = &[
    ("entity_moveTo", || {
        Change::Move(id(SIGN), Vec3::new(3.0, 4.0, 5.0))
    }),
    ("entity_turnTo", || {
        Change::Turn(id(SIGN), Quat::from_rotation_z(1.0))
    }),
    ("entity_face", || {
        Change::Turn(id(SIGN), Quat::from_rotation_z(-2.0))
    }),
    ("entity_push", || Change::Push(id(SIGN), Vec3::X * 4.0)),
    ("entity_setVelocity", || {
        Change::Velocity(id(SIGN), Vec3::Y * 2.0)
    }),
    ("entity_walk", || {
        Change::Character(id(WALKER), Vec3::X * 0.1)
    }),
    ("entity_setSoundPlaying", || {
        Change::SoundPlaying(id(SIGN), false)
    }),
    ("entity_animate", || {
        Change::PlayClip(id(SIGN), "bow".into(), false)
    }),
    ("entity_animateFromStart", || {
        Change::PlayClip(id(SIGN), "wave".into(), true)
    }),
    ("entity_stopAnimation", || Change::StopClip(id(SIGN))),
    ("entity_setFlip", || Change::Flip(id(SIGN), true, false)),
    ("Animators_setNumber", || {
        Change::AnimatorSet(id(SIGN), "pace".into(), AnimatorValue::Number(3.0))
    }),
    ("Animators_setSwitch", || {
        Change::AnimatorSet(id(SIGN), "moving".into(), AnimatorValue::Switch(true))
    }),
    ("Animators_fire", || {
        Change::AnimatorSet(id(SIGN), "swing".into(), AnimatorValue::Fire)
    }),
    ("Animators_setDirection", || {
        Change::AnimatorSet(id(SIGN), "facing".into(), AnimatorValue::Direction(Vec3::Y))
    }),
    ("Animators_jumpTo", || {
        Change::AnimatorJump(id(SIGN), "Bow".into())
    }),
    ("Animators_setPlaybackSpeed", || {
        Change::AnimatorSpeed(id(SIGN), 2.0)
    }),
    ("Maps_setTerrain", || {
        Change::MapTile(MAP.into(), 0, [1, 2], b't')
    }),
    ("Maps_setNumber", || {
        Change::MapValue(MAP.into(), 0, [1, 2], CellValue::Number(4))
    }),
    ("Maps_setSwitch", || {
        Change::MapValue(MAP.into(), 1, [1, 2], CellValue::Switch(true))
    }),
    ("Maps_setChoice", || {
        Change::MapValue(MAP.into(), 2, [1, 2], CellValue::Choice(Some(1)))
    }),
    ("entity_showScreen", || {
        Change::ShowScreen(id(SIGN), "Menu".into(), true)
    }),
    ("entity_hideScreen", || {
        Change::ShowScreen(id(SIGN), "Hud".into(), false)
    }),
];

/// Every other call: it asks something, or what it sets isn't on an
/// entity (the whole game's, the sound mixer's, the player's settings,
/// input) or is the area's own (an entity made or taken out).
const NOT_ON_AN_ENTITY: &[&str] = &[
    "World_say",
    "World_time",
    "World_find",
    "World_fire",
    "World_spawn",
    "entity_despawn",
    "World_frameTime",
    "World_viewSize",
    "World_random",
    "World_randomRange",
    "World_randomInt",
    "World_setGameSpeed",
    "World_gameSpeed",
    "World_timeOfDay",
    "World_setTimeOfDay",
    "World_setSeed",
    "Saves_saveTo",
    "Saves_loadFrom",
    "Audio_play",
    "Audio_playAt",
    "entity_playSound",
    "entity_stopSounds",
    "Audio_setVolume",
    "Animators_stateOf",
    "Maps_width",
    "Maps_height",
    "Maps_terrain",
    "Maps_number",
    "Maps_isOn",
    "Maps_choice",
    "Maps_cellAt",
    "Maps_placeOf",
    "entity_mapBounds",
    "Input_held",
    "Input_pressed",
    "Input_released",
    "Input_axis",
    "Input_binding",
    "Input_rebind",
    "entity_velocity",
    "Physics_castRay",
    "entity_grounded",
    "entity_position",
    "entity_rotation",
    "facingAlong",
    "entity_name",
    "entity_screenShown",
    "Screens_textSize",
    "Screens_setTextSize",
    "Screens_uiSize",
    "Screens_setUiSize",
    "Window_fullscreen",
    "Window_setFullscreen",
    "Window_resolution",
    "Window_setResolution",
    "Window_useNextResolution",
    "Window_quit",
];

#[test]
fn every_call_is_listed_as_setting_something_on_an_entity_or_not() {
    let listed: Vec<&str> = ON_AN_ENTITY
        .iter()
        .map(|(key, _)| *key)
        .chain(NOT_ON_AN_ENTITY.iter().copied())
        .collect();
    for native in script::describe::NATIVES {
        let key = native.key();
        let times = listed.iter().filter(|listed| **listed == key).count();
        assert_eq!(
            times,
            1,
            "{} is listed {times} times: put a new call in ON_AN_ENTITY if it sets something on \
             an entity (and keep what it sets over edits, world::game_set), in NOT_ON_AN_ENTITY \
             if not",
            native.shown()
        );
    }
    assert_eq!(
        listed.len(),
        script::describe::NATIVES.len(),
        "a call listed here that game code no longer has"
    );
}

/// An animator with two states and a parameter of each kind, and nothing
/// that takes it from one state to the other by itself.
fn animator() -> AnimatorFile {
    let raw = |n| reflect::Id::from_raw(n).unwrap();
    let state = |n, name: &str, clip: &str| State {
        id: raw(n),
        name: name.into(),
        place: [0.0, 0.0],
        plays: Plays::Clip(clip.into()),
        speed: 1.0,
        speed_by: String::new(),
    };
    let parameter = |n, name: &str, kind| Parameter {
        id: raw(n),
        name: name.into(),
        kind,
    };
    AnimatorFile {
        entry: Some(raw(1)),
        parameters: vec![
            parameter(10, "pace", ParameterKind::Number(1.0)),
            parameter(11, "moving", ParameterKind::Switch(false)),
            parameter(12, "swing", ParameterKind::Trigger),
            parameter(
                13,
                "facing",
                ParameterKind::Direction(animation::Facing::Down),
            ),
        ],
        states: vec![state(1, "Idle", ""), state(2, "Bow", "bow")],
        ..AnimatorFile::default()
    }
}

/// A playing game with the sign and the walker, the same every time:
/// fixed IDs, and `files` for its animator and its tile map.
fn stage(files: (AssetId, AssetId)) -> Workbench {
    let (animator_file, map_file) = files;
    let asset = |asset| Some(AssetRef { asset, part: None });
    let mut w = Workbench::new(info());
    // Its games are compared with each other.
    unhurried(&mut w);
    // No starter area: its entities' IDs are new every time.
    w.open.document = crate::document::Document::new(area::Area::new());
    let sign = EntityData::new("Sign")
        .with(&Transform::default())
        .with(&Light::default())
        .with(&Sprite {
            clip: "wave".into(),
            time: 0.25,
            ..Sprite::default()
        })
        .with(&AudioSource::default())
        .with(&Animator {
            animator: asset(animator_file),
            ..Animator::default()
        })
        .with(&TileMap {
            map: asset(map_file),
        })
        .with(&RigidBody::default())
        .with(&Collider::default())
        .with(&Screen::showing("Hud"));
    let walker = EntityData::new("Walker")
        .with(&Transform {
            position: Vec3::new(20.0, 0.0, 0.0),
            ..Transform::default()
        })
        .with(&Light::default())
        .with(&CharacterBody::default());
    let spawn = |n: u64, data: EntityData| area::Edit::Spawn {
        entities: vec![(id(n), data)],
    };
    w.open
        .document
        .edit(
            "Add",
            vec![spawn(SIGN, sign), spawn(WALKER, walker)],
            None,
            None,
        )
        .unwrap();
    // The files the running game reads, as the project's would be.
    let mut map = tiles::TileMap::new([4, 4], None, Some(b'.'));
    map.add_value("growth", ValueKind::Number).unwrap();
    map.add_value("watered", ValueKind::Switch).unwrap();
    map.add_value(
        "crop",
        ValueKind::Choice(vec!["wheat".into(), "corn".into()]),
    )
    .unwrap();
    let sheets = Arc::make_mut(&mut w.viewport.sheets);
    sheets.animator_set = Arc::new(world::Animators::from([(
        animator_file,
        Arc::new(animation::Animator::new(&animator())),
    )]));
    sheets.map_set = Arc::new(world::MapFiles::from([(map_file, Arc::new(map))]));
    sheets
        .paths
        .insert(map_file, reflect::data_files::file_name(MAP, "tilemap"));
    w.open.fixed_seed = Some(3);
    w.play();
    ticks(&mut w, 2);
    w
}

fn ticks(w: &mut Workbench, n: u64) {
    let until = w.world().unwrap().clock().ticks + n;
    while w.world().unwrap().clock().ticks < until {
        w.run_game((world::TICK * 0.6) as f32);
    }
}

/// Game code's call lands, as a tick's changes do.
fn ask(w: &mut Workbench, asks: Asks) {
    let playing = w.open.playing.as_mut().unwrap();
    let time = playing.world.clock().time;
    let mut host = PlayWorld::new(&mut playing.world, &mut playing.view, time);
    // What game code would read first (which map is which entity's).
    let _ = host.view();
    host.apply(&[asks()]);
}

/// Another component of `entity` edited by hand: with nothing else
/// changed, the game makes that one entity again from the area.
fn edit_another_component(w: &mut Workbench, entity: u64) {
    w.apply_property(
        PropertyRequest::SetField {
            entity: id(entity),
            component: Light::NAME.into(),
            field: "strength".into(),
            value: Value::Float(7.5),
        },
        entity,
    );
}

/// An entity added and taken out again: the area is as it was, and the
/// game builds its whole world again from it.
fn build_everything_again(w: &mut Workbench) {
    let passer_by = id(99);
    w.open
        .document
        .edit(
            "Pass by",
            vec![
                area::Edit::Spawn {
                    entities: vec![(passer_by, EntityData::new("Passer-by"))],
                },
                area::Edit::Despawn { root: passer_by },
            ],
            None,
            None,
        )
        .unwrap();
}

/// Everything the game has of the sign and the walker, but the Light the
/// edits change: every component as the game has it, where each is and
/// how it moves, the sign's animator, map and screen.
fn game_has(w: &mut Workbench) -> String {
    let mut all = String::new();
    for entity in [id(SIGN), id(WALKER)] {
        let names: Vec<String> = w
            .open
            .document
            .area()
            .get(entity)
            .unwrap()
            .components
            .keys()
            .filter(|name| *name != Light::NAME)
            .cloned()
            .collect();
        for name in names {
            let values = w.values_now(entity, &name);
            all += &format!("{name}: {values:?}\n");
        }
        let world = w.world().unwrap();
        all += &format!(
            "at {:?}\nmoving {:?}\nanimator {:?}\nscreen {:?}\nmap {:?}\n",
            world.global(entity),
            world.physics().state().velocity(entity),
            world.animator_of(entity),
            world.screen_shown(entity),
            world.tile_map(entity).map(|map| map.checksum()),
        );
    }
    all
}

#[test]
fn what_each_call_sets_on_an_entity_outlives_edits_while_playing() {
    let files = (AssetId::random(), AssetId::random());
    // The game left alone, to see that each call does change something.
    let mut untouched = stage(files);
    let nothing_asked = game_has(&mut untouched);
    ticks(&mut untouched, 1);
    let nothing_asked = (nothing_asked, game_has(&mut untouched));
    for (key, asks) in ON_AN_ENTITY {
        let entity = if *key == "entity_walk" { WALKER } else { SIGN };
        let mut played = stage(files);
        ask(&mut played, *asks);
        let asked = game_has(&mut played);
        // One entity made again from the area: nothing of the game moves.
        let mut edited = stage(files);
        ask(&mut edited, *asks);
        edit_another_component(&mut edited, entity);
        edited.run_game(0.0);
        assert_eq!(game_has(&mut edited), asked, "{key}: after an edit");
        assert_eq!(
            edited.world().unwrap().checksum(),
            played.world().unwrap().checksum(),
            "{key}: after an edit"
        );
        // The whole world built again (physics too, so only what's
        // listed is compared: a body built again isn't the same body).
        let mut rebuilt = stage(files);
        ask(&mut rebuilt, *asks);
        edit_another_component(&mut rebuilt, entity);
        build_everything_again(&mut rebuilt);
        rebuilt.run_game(0.0);
        assert_eq!(game_has(&mut rebuilt), asked, "{key}: built again");
        // Each goes on from there as the game never edited does.
        for w in [&mut played, &mut edited, &mut rebuilt] {
            ticks(w, 1);
        }
        let later = game_has(&mut played);
        assert_eq!(game_has(&mut edited), later, "{key}: a tick later");
        assert_eq!(
            edited.world().unwrap().checksum(),
            played.world().unwrap().checksum(),
            "{key}: a tick later"
        );
        assert_eq!(game_has(&mut rebuilt), later, "{key}: built again, later");
        assert_ne!((asked, later), nothing_asked, "{key} changed nothing");
    }
}

/// The other side (the review's BUG-54, kept working): what's set by
/// hand in the editor wins for the field set, even set to the value the
/// area held all along, and everything else game code set stays.
#[test]
fn a_field_set_by_hand_wins_over_what_game_code_set_and_the_rest_stays() {
    for rebuild in [false, true] {
        let mut w = stage((AssetId::random(), AssetId::random()));
        ask(&mut w, || Change::SoundPlaying(id(SIGN), false));
        ask(&mut w, || Change::Flip(id(SIGN), true, false));
        let playing = |w: &mut Workbench| {
            let source = w.values_now(id(SIGN), AudioSource::NAME).unwrap();
            (
                AudioSource::from_data(&source).playing,
                Sprite::from_data(&w.values_now(id(SIGN), Sprite::NAME).unwrap()).flip_x,
            )
        };
        assert_eq!(playing(&mut w), (false, true), "as game code set them");
        // Playing, as the area holds it already.
        w.apply_property(
            PropertyRequest::SetField {
                entity: id(SIGN),
                component: AudioSource::NAME.into(),
                field: "playing".into(),
                value: Value::Bool(true),
            },
            1,
        );
        if rebuild {
            build_everything_again(&mut w);
        }
        w.run_game(0.0);
        assert_eq!(playing(&mut w), (true, true), "built again: {rebuild}");
    }
}
