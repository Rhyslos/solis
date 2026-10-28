//! Game code's sounds on their way out (Phase 7, S3): requests land in
//! order after their tick, with the tick and seed that pick their take;
//! the listener is the game's camera; and sound never changes the game.

use crate::properties::PropertyRequest;
use crate::test_support::*;
use crate::workbench::Workbench;
use crate::{GameSound, SoundMode};
use area::EntityData;
use area::builtin::{Camera, Transform};
use glam::Vec3;
use reflect::EntityId;
use std::sync::Arc;

const SPEAKER: &str = "get \"solis/sound\":[playSound, playSoundAt, playSoundOn];\n\
    get \"solis/entity\":[positionOf, moveTo];\n\
    component Speaker {\n\
        has {\n\
            int ticks = 0;\n\
        }\n\
        method update(float dt) {\n\
            ticks = ticks + 1;\n\
            playSoundAt(\"sfx/step\", positionOf(me));\n\
            playSoundOn(\"engine\", me);\n\
            moveTo(me, positionOf(me) + [1, 0, 0]);\n\
        }\n\
    }\n\
    pass Speaker;\nend\n";

fn program(main: &str) -> Arc<script::Compiled> {
    let mut files = script::describe::descriptions("Test Engine 9.9.9", "solis");
    files.push(("main.lst".into(), main.to_string()));
    let code = script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    match script::compile(&code) {
        Ok(Some(c)) => Arc::new(c),
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    }
}

/// A workbench with a speaker and a camera 10 m behind it.
fn with_a_speaker() -> (Workbench, EntityId) {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(SPEAKER)));
    // Fixed IDs: a checksum includes them.
    let (speaker, camera) = (
        EntityId(reflect::Id::from_raw(11).unwrap()),
        EntityId(reflect::Id::from_raw(12).unwrap()),
    );
    let camera_data = EntityData::new("Camera")
        .with(&Transform {
            position: Vec3::new(0.0, -10.0, 0.0),
            ..Transform::default()
        })
        .with(&Camera::default());
    for (id, data) in [
        (
            speaker,
            EntityData::new("Speaker").with(&Transform::default()),
        ),
        (camera, camera_data),
    ] {
        w.document
            .edit(
                "Add",
                vec![area::Edit::Spawn {
                    entities: vec![(id, data)],
                }],
                None,
                None,
            )
            .unwrap();
    }
    w.apply_property(
        PropertyRequest::AddComponent {
            entity: speaker,
            component: "Speaker".into(),
        },
        1,
    );
    (w, speaker)
}

fn names(sounds: &[GameSound]) -> Vec<(u64, u32, String)> {
    sounds
        .iter()
        .map(|s| match &s.request {
            crate::game_sound::Request::Play(name, _, _) => (s.tick, s.order, name.clone()),
            crate::game_sound::Request::StopOn(_) => (s.tick, s.order, "stop".into()),
            crate::game_sound::Request::BusVolume(bus, _) => {
                (s.tick, s.order, format!("bus {bus}"))
            }
        })
        .collect()
}

#[test]
fn a_game_s_sounds_land_after_each_tick_in_order() {
    let (mut w, speaker) = with_a_speaker();
    assert_eq!(w.game_sound_mode(), SoundMode::Off);
    assert!(w.listener().is_none());
    w.play();
    assert_eq!(w.game_sound_mode(), SoundMode::Playing);
    w.run_game(3.0 / 60.0);
    let sounds = w.take_game_sounds();
    let listed = names(&sounds);
    let ticks: Vec<&(u64, u32, String)> = listed.iter().collect();
    assert_eq!(ticks.len(), 6, "two a tick for three ticks: {listed:?}");
    assert_eq!((ticks[0].1, ticks[0].2.as_str()), (0, "sfx/step"));
    assert_eq!((ticks[1].1, ticks[1].2.as_str()), (1, "engine"));
    assert!(ticks[2].0 > ticks[0].0, "a later tick");
    match sounds[0].request {
        crate::game_sound::Request::Play(_, script::SoundAt::Place(p), _) => {
            assert_eq!(p, Vec3::ZERO, "where it was as the tick started");
        }
        ref other => panic!("{other:?}"),
    }
    assert!(w.take_game_sounds().is_empty(), "taken once");
    // The listener is the camera; the speaker is where it's drawn.
    let (place, _) = w.listener().expect("a camera");
    assert!((place - Vec3::new(0.0, -10.0, 0.0)).length() < 1e-4);
    let at = w.place_between(speaker).expect("in the game");
    assert!(at.x > 0.0, "it moved: {at}");
    w.pause();
    assert_eq!(w.game_sound_mode(), SoundMode::Paused);
    w.stop();
    assert_eq!(w.game_sound_mode(), SoundMode::Off);
    assert!(w.place_between(speaker).is_none());
}

#[test]
fn sound_never_changes_the_game() {
    // The same game played twice in one area, with its sounds taken every
    // frame and never: the same world, tick by tick.
    let (mut w, _) = with_a_speaker();
    w.fixed_seed = Some(7);
    let mut run = |take: bool| {
        w.play();
        let mut sums = Vec::new();
        for _ in 0..20 {
            w.run_game(1.0 / 60.0);
            if take {
                let _ = w.take_game_sounds();
            }
            sums.push(w.world().unwrap().checksum());
        }
        w.stop();
        let _ = w.take_game_sounds();
        sums
    };
    let quiet = run(false);
    assert_eq!(run(false), quiet, "the baseline repeats");
    assert_eq!(run(true), quiet);
}

#[test]
fn the_listener_turns_with_the_camera() {
    let (mut w, _) = with_a_speaker();
    w.play();
    w.run_game(1.0 / 60.0);
    // A camera looks along its -Z with +Y up, as the sound's listener
    // does, so its turn goes over as it is: its right is the listener's.
    let (_, turn) = w.listener().unwrap();
    let camera = w.world().unwrap().camera().unwrap().2;
    let right = camera.x_axis.truncate().normalize();
    assert!((turn * Vec3::X - right).length() < 1e-4);
}

#[test]
fn the_first_listener_hearing_now_hears_or_else_the_camera() {
    use area::builtin::{Camera, Listener};
    let mut w = Workbench::new(info(), None);
    let camera = w.add_entity();
    let off = w.add_entity();
    let on = w.add_entity();
    let child = w.add_entity();
    let add = |w: &mut Workbench, id, edits| {
        w.document.edit("Add", edits, None, None).unwrap();
        let _ = id;
    };
    add(
        &mut w,
        camera,
        crate::commands::component_edits(camera, None, &Camera::default()),
    );
    let quiet = Listener {
        hears_now: false,
        groups: "Team Blue".into(),
    };
    add(
        &mut w,
        off,
        crate::commands::component_edits(off, None, &quiet),
    );
    w.play();
    // No listener hearing now: the camera, in no group.
    let h = w.hearer().unwrap();
    assert_eq!((h.entity, h.groups.len()), (camera, 0));
    w.stop();
    let red = Listener {
        hears_now: true,
        groups: "Team Red, Radio".into(),
    };
    add(&mut w, on, crate::commands::component_edits(on, None, &red));
    w.apply_area_request(crate::area_panel::AreaRequest::Reparent {
        entity: child,
        onto: Some(on),
    });
    w.play();
    let h = w.hearer().unwrap();
    assert_eq!(h.entity, on);
    assert_eq!(h.groups, ["Team Red", "Radio"]);
    assert!(w.listener().is_some());
    // Family: itself, what's inside it, and what it's inside.
    assert!(w.same_family(on, on));
    assert!(w.same_family(child, on) && w.same_family(on, child));
    assert!(!w.same_family(camera, on));
}

#[test]
fn a_new_play_starts_its_sounds_afresh() {
    // Regression (Phase 7 review): sounds asked for in one play never
    // wait to be heard in the next, and each play has its own number, so
    // the app starts the game's sound again even with no frame between.
    let (mut w, _) = with_a_speaker();
    let first = w.play_number();
    w.play();
    w.run_game(2.0 / 60.0);
    assert!(!w.take_game_sounds().is_empty());
    w.run_game(1.0 / 60.0);
    w.stop();
    assert!(w.take_game_sounds().is_empty(), "Stop leaves none waiting");
    w.play();
    assert!(w.play_number() > first + 1, "a number each play");
    w.pause();
    let paused = w.play_number();
    w.play();
    assert_eq!(w.play_number(), paused, "carrying on isn't a new play");
}
