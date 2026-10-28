//! Sprites playing clips in the running game: moved on tick by tick, the
//! same however the ticks fall, their events and endings handed back,
//! game code's plays and stops, and the game's playback kept over edits.

use super::*;
use crate::TICK;
use animation::{Clip, ClipFrame, Direction};
use area::Area;
use area::EntityData;
use area::builtin::Transform;
use reflect::AssetRef;

/// A sheet with two clips: walk (4 frames of 100 ms, over and over, a
/// footstep on the third) and swing (2 frames of 100 ms, once).
fn sets() -> (AssetId, Arc<ClipSets>) {
    let sheet = AssetId::random();
    let frames = |n: u32, event_at: Option<u32>| -> Vec<ClipFrame> {
        (0..n)
            .map(|i| ClipFrame {
                frame: i,
                ms: 100,
                events: if Some(i) == event_at {
                    vec!["footstep".into()]
                } else {
                    Vec::new()
                },
            })
            .collect()
    };
    let set = ClipSet {
        clips: vec![
            Clip::new("walk", frames(4, Some(2)), Direction::Forward, 0, 1.0, true),
            Clip::new("swing", frames(2, None), Direction::Forward, 1, 1.0, true),
        ],
        first: Some(0),
        notes: Vec::new(),
    };
    let mut map = ClipSets::new();
    map.insert(sheet, Arc::new(set));
    (sheet, Arc::new(map))
}

fn walker(sheet: AssetId) -> Sprite {
    Sprite {
        sheet: Some(AssetRef {
            asset: sheet,
            part: None,
        }),
        ..Sprite::default()
    }
}

fn world_with(sprites: &[Sprite]) -> (World, Vec<EntityId>) {
    let mut area = Area::new();
    let ids = sprites
        .iter()
        .enumerate()
        .map(|(i, s)| {
            area.spawn(
                EntityData::new(&format!("S{i}"))
                    .with(&Transform::default())
                    .with(s),
            )
        })
        .collect();
    (World::from_area(&area), ids)
}

fn time_of(world: &World, id: EntityId) -> f32 {
    world.get::<Sprite>(id).unwrap().time
}

#[test]
fn a_playing_clip_moves_on_each_tick_and_its_events_come_back_once() {
    let (sheet, sets) = sets();
    let (mut world, ids) = world_with(&[walker(sheet)]);
    world.set_clip_sets(sets);
    let mut cues = Vec::new();
    // Two passes of walk (0.8 s) at 60 ticks a second.
    for _ in 0..48 {
        cues.extend(world.step_sprites(TICK));
    }
    let steps = cues
        .iter()
        .filter(|c| matches!(c, SpriteCue::Event(id, n) if *id == ids[0] && n == "footstep"))
        .count();
    assert_eq!(steps, 2);
    // Over and over: its time stays within one pass.
    assert!(time_of(&world, ids[0]) < 0.4);
    assert!(world.get::<Sprite>(ids[0]).unwrap().playing);
}

#[test]
fn speed_a_still_sprite_and_one_without_clips_are_respected() {
    let (sheet, sets) = sets();
    let fast = Sprite {
        speed: 2.0,
        ..walker(sheet)
    };
    let stopped = Sprite {
        playing: false,
        ..walker(sheet)
    };
    let no_sheet = Sprite::default();
    let unknown = Sprite {
        clip: "dance".into(),
        ..walker(sheet)
    };
    let (mut world, ids) = world_with(&[fast, stopped, no_sheet, unknown]);
    world.set_clip_sets(sets);
    for _ in 0..6 {
        world.step_sprites(TICK);
    }
    assert!(
        (time_of(&world, ids[0]) - 0.2).abs() < 1e-4,
        "twice as fast"
    );
    assert_eq!(time_of(&world, ids[1]), 0.0);
    assert_eq!(time_of(&world, ids[2]), 0.0);
    assert_eq!(time_of(&world, ids[3]), 0.0, "a clip it hasn't got");
}

#[test]
fn a_clip_played_once_finishes_says_so_and_holds_its_last_frame() {
    let (sheet, sets) = sets();
    let (mut world, ids) = world_with(&[Sprite {
        clip: "swing".into(),
        ..walker(sheet)
    }]);
    world.set_clip_sets(sets);
    let mut cues = Vec::new();
    for _ in 0..30 {
        cues.extend(world.step_sprites(TICK));
    }
    assert_eq!(
        cues,
        [SpriteCue::Finished(ids[0], "swing".into())],
        "once, as it ends"
    );
    let s = world.get::<Sprite>(ids[0]).unwrap();
    assert!(!s.playing);
    assert!((s.time - 0.2).abs() < 1e-6);
    // playClip on a finished clip leaves it finished (safe every tick);
    // replayClip starts it again.
    world.play_clip(ids[0], "swing", false);
    assert!(!world.get::<Sprite>(ids[0]).unwrap().playing);
    world.play_clip(ids[0], "swing", true);
    assert_eq!(time_of(&world, ids[0]), 0.0);
    assert!(world.get::<Sprite>(ids[0]).unwrap().playing);
}

#[test]
fn play_clip_carries_on_the_same_clip_and_starts_another() {
    let (sheet, sets) = sets();
    let (mut world, ids) = world_with(&[walker(sheet)]);
    world.set_clip_sets(sets);
    for _ in 0..9 {
        world.step_sprites(TICK);
    }
    let walked = time_of(&world, ids[0]);
    assert!(walked > 0.1);
    world.play_clip(ids[0], "", false);
    assert_eq!(time_of(&world, ids[0]), walked, "already playing it");
    world.stop_clip(ids[0]);
    world.step_sprites(TICK);
    assert_eq!(time_of(&world, ids[0]), walked, "stopped");
    world.play_clip(ids[0], "", false);
    assert_eq!(
        time_of(&world, ids[0]),
        walked,
        "carries on from where it stopped"
    );
    world.play_clip(ids[0], "swing", false);
    let s = world.get::<Sprite>(ids[0]).unwrap();
    assert_eq!((s.clip.as_str(), s.time), ("swing", 0.0));
    // Entities without a Sprite, or unknown: nothing.
    world.play_clip(EntityId::random(), "walk", false);
    world.stop_clip(EntityId::random());
}

#[test]
fn the_same_ticks_give_the_same_checksum_and_playback_is_in_it() {
    let (sheet, sets) = sets();
    let run = |ticks: usize| {
        let mut area = Area::new();
        for n in 1..=2 {
            let id = EntityId(reflect::Id::from_raw(n).unwrap());
            area.insert(id, EntityData::new("W").with(&walker(sheet)));
        }
        let mut world = World::from_area(&area);
        world.set_clip_sets(Arc::clone(&sets));
        for _ in 0..ticks {
            world.step_sprites(TICK);
        }
        world.checksum()
    };
    assert_eq!(run(10), run(10));
    assert_ne!(run(10), run(11));
}

#[test]
fn an_edit_while_playing_keeps_the_game_s_playback_unless_it_changed_it() {
    let (sheet, sets) = sets();
    let mut area = Area::new();
    let id = area.spawn(EntityData::new("W").with(&walker(sheet)));
    let mut world = World::from_area(&area);
    world.set_clip_sets(sets);
    for _ in 0..9 {
        world.step_sprites(TICK);
    }
    let walked = time_of(&world, id);
    // Tinting it in the editor: still where it was in its clip.
    let tinted = Sprite {
        tint: [1.0, 0.5, 0.5],
        ..walker(sheet)
    };
    area.get_mut(id).unwrap().set(&tinted);
    world.update(&area, [(id, false)]);
    assert_eq!(time_of(&world, id), walked);
    assert_eq!(world.get::<Sprite>(id).unwrap().tint, [1.0, 0.5, 0.5]);
    // A rebuild keeps it too, and the clip sets.
    world.rebuild(&area);
    assert_eq!(time_of(&world, id), walked);
    assert!(!world.clip_sets().is_empty());
    // Choosing another clip in the editor: the editor's choice.
    let swinging = Sprite {
        clip: "swing".into(),
        ..tinted
    };
    area.get_mut(id).unwrap().set(&swinging);
    world.update(&area, [(id, false)]);
    let s = world.get::<Sprite>(id).unwrap();
    assert_eq!((s.clip.as_str(), s.time), ("swing", 0.0));
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_10000_sprites_playing_clips() {
    // A farm's worth of animated sprites, each moved on every tick.
    const TARGET_MS: f64 = 1.0;
    let (sheet, sets) = sets();
    let sprites: Vec<Sprite> = (0..10_000)
        .map(|i| Sprite {
            time: (i % 7) as f32 * 0.05,
            ..walker(sheet)
        })
        .collect();
    let (mut world, _) = world_with(&sprites);
    world.set_clip_sets(sets);
    let ticks = 120;
    let start = std::time::Instant::now();
    let mut cues = 0;
    for _ in 0..ticks {
        cues += world.step_sprites(TICK).len();
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(ticks);
    assert!(cues > 0);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: 10,000 sprites playing clips {ms:.3} ms a tick (target {TARGET_MS} ms) {verdict}"
    );
    assert!(ms <= TARGET_MS, "sprites take {ms:.3} ms a tick");
}
