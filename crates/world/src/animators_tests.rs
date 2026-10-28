//! Animators in the running game: transitions on the right tick, the
//! sprite's clip and speed set, facing kept in step, game code's settings
//! and jumps, the checksum, saves' values, and runs kept over edits.

use super::*;
use crate::TICK;
use animation::{
    AnimatorFile, Clip, ClipFrame, ClipSet, Condition, Direction, Facing, Parameter, ParameterKind,
    Plays, State, Transition,
};
use area::builtin::Transform;
use area::{Area, EntityData};
use reflect::Component;
use reflect::{AssetRef, Id as RawId};

fn id(n: u64) -> RawId {
    RawId::from_raw(n).unwrap()
}

/// Clips idle_down, idle_up, walk_down, walk_up (4 frames of 100 ms, over
/// and over) and hoe_down (2 frames, once).
fn clip_sets(sheet: AssetId) -> Arc<crate::ClipSets> {
    let frames = |n: u32| -> Vec<ClipFrame> {
        (0..n)
            .map(|i| ClipFrame {
                frame: i,
                ms: 100,
                events: Vec::new(),
            })
            .collect()
    };
    let clip =
        |name: &str, n, repeats| Clip::new(name, frames(n), Direction::Forward, repeats, 1.0, true);
    let set = ClipSet {
        clips: vec![
            clip("idle_down", 4, 0),
            clip("idle_up", 4, 0),
            clip("walk_down", 4, 0),
            clip("walk_up", 4, 0),
            clip("hoe_down", 2, 1),
        ],
        first: Some(0),
        notes: Vec::new(),
    };
    let mut sets = crate::ClipSets::new();
    sets.insert(sheet, Arc::new(set));
    Arc::new(sets)
}

/// Idle and Walk by facing (up and down), Use tool; moving, speed,
/// facing, swing.
fn farmer() -> AnimatorFile {
    let by = |stem: &str| Plays::ByDirection {
        by: "facing".into(),
        clips: vec![
            (Facing::Down, format!("{stem}_down")),
            (Facing::Up, format!("{stem}_up")),
        ],
    };
    let state = |n, name: &str, plays| State {
        id: id(n),
        name: name.into(),
        place: [0.0, 0.0],
        plays,
        speed: 1.0,
        speed_by: String::new(),
    };
    let mut walk = state(2, "Walk", by("walk"));
    walk.speed_by = "speed".into();
    let param = |n, name: &str, kind| Parameter {
        id: id(n),
        name: name.into(),
        kind,
    };
    let way = |n, from: Option<u64>, to, when| Transition {
        id: id(n),
        from: from.map(id),
        to: id(to),
        when,
    };
    AnimatorFile {
        sheet: None,
        entry: Some(id(1)),
        any_place: [120.0, 300.0],
        parameters: vec![
            param(10, "facing", ParameterKind::Direction(Facing::Down)),
            param(11, "moving", ParameterKind::Switch(false)),
            param(12, "speed", ParameterKind::Number(1.0)),
            param(13, "swing", ParameterKind::Trigger),
        ],
        states: vec![
            state(1, "Idle", by("idle")),
            walk,
            state(3, "Use tool", Plays::Clip("hoe_down".into())),
        ],
        transitions: vec![
            way(20, Some(1), 2, vec![Condition::Is("moving".into())]),
            way(21, Some(2), 1, vec![Condition::Not("moving".into())]),
            way(22, None, 3, vec![Condition::Is("swing".into())]),
            way(23, Some(3), 1, vec![Condition::Finished]),
        ],
    }
}

struct Farm {
    world: World,
    area: Area,
    ids: Vec<EntityId>,
    sheet: AssetId,
    file: AssetId,
}

fn farm(count: usize) -> Farm {
    let (sheet, file) = (AssetId::random(), AssetId::random());
    let mut area = Area::new();
    let ids = (0..count)
        .map(|i| {
            let n = i as u64 + 1;
            let entity = EntityId(RawId::from_raw(n).unwrap());
            area.insert(
                entity,
                EntityData::new(&format!("Farmer {n}"))
                    .with(&Transform::default())
                    .with(&Sprite {
                        sheet: Some(AssetRef {
                            asset: sheet,
                            part: None,
                        }),
                        ..Sprite::default()
                    })
                    .with(&Animator {
                        animator: Some(AssetRef {
                            asset: file,
                            part: None,
                        }),
                        ..Animator::default()
                    }),
            );
            entity
        })
        .collect();
    let mut world = World::from_area(&area);
    world.set_clip_sets(clip_sets(sheet));
    let mut machines = Animators::new();
    machines.insert(file, Arc::new(Machine::new(&farmer())));
    world.set_animators(Arc::new(machines));
    Farm {
        world,
        area,
        ids,
        sheet,
        file,
    }
}

/// One tick as the game runs it: animators, then sprites.
fn tick(world: &mut World) {
    world.step_animators();
    world.step_sprites(TICK);
}

fn sprite(world: &World, id: EntityId) -> Sprite {
    world.get::<Sprite>(id).unwrap()
}

#[test]
fn it_starts_in_its_entry_and_plays_its_clip_by_facing() {
    let mut f = farm(1);
    let me = f.ids[0];
    tick(&mut f.world);
    assert_eq!(sprite(&f.world, me).clip, "idle_down");
    assert_eq!(f.world.get::<Animator>(me).unwrap().state, "Idle");
    f.world
        .set_animator_value(me, "moving", ParamValue::Switch(true))
        .unwrap();
    f.world
        .set_animator_value(me, "speed", ParamValue::Number(2.0))
        .unwrap();
    tick(&mut f.world);
    let s = sprite(&f.world, me);
    assert_eq!((s.clip.as_str(), s.speed), ("walk_down", 2.0));
    assert!(
        (s.time - 2.0 * TICK as f32).abs() < 1e-6,
        "started this tick"
    );
    // Turning mid-step keeps the step.
    for _ in 0..5 {
        tick(&mut f.world);
    }
    let walked = sprite(&f.world, me).time;
    f.world
        .set_animator_value(me, "facing", ParamValue::Direction([0.2, 1.0]))
        .unwrap();
    f.world.step_animators();
    let s = sprite(&f.world, me);
    assert_eq!((s.clip.as_str(), s.time), ("walk_up", walked));
}

#[test]
fn a_trigger_plays_a_clip_once_and_finished_goes_back() {
    let mut f = farm(1);
    let me = f.ids[0];
    tick(&mut f.world);
    f.world
        .set_animator_value(me, "swing", ParamValue::Trigger(true))
        .unwrap();
    tick(&mut f.world);
    assert_eq!(sprite(&f.world, me).clip, "hoe_down");
    // 0.2 s of hoe, then back to Idle the tick after it finished.
    let mut ticks = 1;
    while f.world.get::<Animator>(me).unwrap().state == "Use tool" {
        tick(&mut f.world);
        ticks += 1;
        assert!(ticks < 40);
    }
    assert_eq!(ticks, 13, "12 ticks of 1/60 s is 0.2 s, then one more");
    assert_eq!(sprite(&f.world, me).clip, "idle_down");
    assert!(sprite(&f.world, me).playing);
}

#[test]
fn game_code_s_jumps_and_mistakes() {
    let mut f = farm(1);
    let me = f.ids[0];
    assert!(f.world.jump_to_animator_state(me, "Walk"));
    assert_eq!(sprite(&f.world, me).clip, "walk_down");
    assert!(!f.world.jump_to_animator_state(me, "Dance"));
    assert_eq!(
        f.world
            .set_animator_value(me, "moving", ParamValue::Number(1.0)),
        Err(SetError::WrongKind("switch"))
    );
    let nobody = EntityId::random();
    assert!(
        f.world
            .set_animator_value(nobody, "moving", ParamValue::Switch(true))
            .is_err()
    );
    assert!(!f.world.jump_to_animator_state(nobody, "Walk"));
    let (state, values) = f.world.animator_of(me).unwrap();
    assert_eq!(state, "Walk");
    assert_eq!(values[1], ("moving".to_owned(), ParamValue::Switch(false)));
}

#[test]
fn the_checksum_follows_states_and_values() {
    let run = |set_moving: bool| {
        let mut f = farm(3);
        tick(&mut f.world);
        if set_moving {
            f.world
                .set_animator_value(f.ids[1], "moving", ParamValue::Switch(true))
                .unwrap();
        }
        f.world.checksum()
    };
    assert_eq!(run(false), run(false));
    assert_ne!(run(false), run(true));
}

#[test]
fn a_save_s_values_are_taken_and_sum_the_same_before_the_first_tick() {
    let mut f = farm(1);
    let me = f.ids[0];
    tick(&mut f.world);
    f.world
        .set_animator_value(me, "moving", ParamValue::Switch(true))
        .unwrap();
    f.world
        .set_animator_value(me, "facing", ParamValue::Direction([0.0, 1.0]))
        .unwrap();
    f.world
        .set_animator_value(me, "swing", ParamValue::Trigger(true))
        .unwrap();
    let values = f.world.animator_values();
    assert_eq!(values.len(), 1);
    // Saved as words and read back exactly.
    for (_, v) in &values[0].1 {
        assert_eq!(
            crate::animators::value_of_words(&crate::animators::value_words(*v)),
            Some(*v)
        );
    }
    // The "save": the area with the animator's state, and the values.
    let mut area = f.area.clone();
    let data = area.get_mut(me).unwrap();
    data.set(&f.world.get::<Animator>(me).unwrap());
    data.set(&sprite(&f.world, me));
    let mut loaded = World::from_area(&area);
    loaded.set_clip_sets(clip_sets(f.sheet));
    loaded.set_animators(Arc::clone(f.world.animators()));
    loaded.restore_animator_values(values);
    assert_eq!(loaded.checksum(), f.world.checksum(), "before a tick");
    for _ in 0..30 {
        tick(&mut f.world);
        tick(&mut loaded);
        assert_eq!(loaded.checksum(), f.world.checksum());
    }
    assert_eq!(
        sprite(&loaded, me).clip,
        "walk_up",
        "swung, then walking on, facing up"
    );
}

#[test]
fn bad_words_aren_t_values() {
    for w in [
        "",
        "n",
        "nzz",
        "s2",
        "t",
        "d1",
        "d0,zz",
        "x1",
        "n7fc00000",
        "é1",
    ] {
        assert_eq!(crate::animators::value_of_words(w), None, "{w}");
    }
}

#[test]
fn edits_while_playing_keep_each_run_and_a_changed_file_carries_it_over() {
    let mut f = farm(2);
    let me = f.ids[0];
    tick(&mut f.world);
    f.world
        .set_animator_value(me, "moving", ParamValue::Switch(true))
        .unwrap();
    tick(&mut f.world);
    // Tinting it in the editor keeps it walking.
    let mut tinted = f.area.get(me).unwrap().get::<Sprite>().unwrap();
    tinted.tint = [1.0, 0.5, 0.5];
    f.area.get_mut(me).unwrap().set(&tinted);
    f.world.update(&f.area, [(me, false)]);
    assert_eq!(f.world.animator_of(me).unwrap().0, "Walk");
    tick(&mut f.world);
    assert_eq!(sprite(&f.world, me).clip, "walk_down");
    // A rebuild too.
    f.world.rebuild(&f.area);
    assert_eq!(f.world.animator_of(me).unwrap().0, "Walk");
    // The file changes (Walk faster): the run carries over.
    let mut file = farmer();
    file.states[1].speed = 3.0;
    let mut machines = Animators::new();
    machines.insert(f.file, Arc::new(Machine::new(&file)));
    f.world.set_animators(Arc::new(machines));
    tick(&mut f.world);
    assert_eq!(f.world.animator_of(me).unwrap().0, "Walk");
    assert_eq!(sprite(&f.world, me).speed, 3.0);
    // Its Animator taken away: the run goes with it.
    let mut data = f.area.get(me).unwrap().clone();
    data.components.remove(Animator::NAME);
    *f.area.get_mut(me).unwrap() = data;
    f.world.update(&f.area, [(me, false)]);
    assert!(f.world.animator_of(me).is_none());
    tick(&mut f.world);
}

#[test]
fn without_its_file_or_a_sprite_it_does_no_harm() {
    let mut area = Area::new();
    let lone = area.spawn(EntityData::new("Lone").with(&Animator::default()));
    let missing = area.spawn(EntityData::new("Missing").with(&Animator {
        animator: Some(AssetRef {
            asset: AssetId::random(),
            part: None,
        }),
        ..Animator::default()
    }));
    let mut world = World::from_area(&area);
    tick(&mut world);
    assert!(world.animator_of(lone).is_none());
    assert!(world.animator_of(missing).is_none());
    // An animator with no sprite still changes state.
    let file = AssetId::random();
    let mut area = Area::new();
    let ghost = area.spawn(EntityData::new("Ghost").with(&Animator {
        animator: Some(AssetRef {
            asset: file,
            part: None,
        }),
        ..Animator::default()
    }));
    let mut world = World::from_area(&area);
    let mut machines = Animators::new();
    machines.insert(file, Arc::new(Machine::new(&farmer())));
    world.set_animators(Arc::new(machines));
    world
        .set_animator_value(ghost, "moving", ParamValue::Switch(true))
        .unwrap();
    tick(&mut world);
    assert_eq!(world.get::<Animator>(ghost).unwrap().state, "Walk");
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_10000_animators() {
    // A farm's worth of animated sprites, each animator stepped every
    // tick and some changing state.
    const TARGET_MS: f64 = 1.5;
    let mut f = farm(10_000);
    let ticks: u32 = 120;
    let start = std::time::Instant::now();
    for t in 0..ticks as usize {
        for (i, &id) in f.ids.iter().enumerate().step_by(97) {
            let _ = f
                .world
                .set_animator_value(id, "moving", ParamValue::Switch((t + i) % 2 == 0));
        }
        f.world.step_animators();
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(ticks);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!("Scale check: 10,000 animators {ms:.3} ms a tick (target {TARGET_MS} ms) {verdict}");
    assert!(ms <= TARGET_MS, "animators take {ms:.3} ms a tick");
}
