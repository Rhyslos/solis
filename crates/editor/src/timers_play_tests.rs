//! Timers while playing (stage 1, timers): a door's Lust starts a timer
//! and hears `TimerFinished` on the exact tick at the game's speed; a
//! save made while it waits loads and finishes on the same tick; an
//! entity despawned before its timer finishes takes it with it; and a
//! clock set past a timer's hour finishes it once.

use crate::test_support::*;
use crate::workbench::Workbench;
use level::EntityData;
use reflect::{ComponentData, EntityId, Value};
use std::sync::Arc;

const MAIN: &str = "fetch \"solis/world\":World;\n\
    fetch \"solis/project\":Timer;\n\
    fetch \"solis/timers\":TimerFinished;\n\n\
    component Door {\n\
        has {\n\
            int ticks = 0;\n\
            float speed = 1;\n\
            bool vanish = false;\n\
        }\n\
        method update(float dt) {\n\
            ticks = ticks + 1;\n\
            is ticks == 1? then {\n\
                World.gameSpeed = speed;\n\
                startTimer(Timer.Opens, 2.0);\n\
                startTimerAtHour(Timer.Dawn, 6.0);\n\
                World.timeOfDay = 5.0;\n\
            }\n\
            is ticks == 3? then {\n\
                World.timeOfDay = 7.0;\n\
            }\n\
            is ticks == 2 && vanish? then {\n\
                despawn();\n\
            }\n\
        }\n\
        on TimerFinished(Timer which) {\n\
            is which == Timer.Opens? then {\n\
                World:say(\"opened on \" + ticks.text());\n\
            }\n\
            is which == Timer.Dawn? then {\n\
                World:say(\"dawn on \" + ticks.text());\n\
            }\n\
        }\n\
    }\n\
    pass Door;\nend\n";

fn program() -> Arc<script::Compiled> {
    let (mut files, _) = script::describe::descriptions_for(
        "Test Engine 9.9.9",
        "solis",
        &script::project_names::Names::standard().and_timers(&["Dawn", "Opens"]),
    );
    files.push(("main.lst".into(), MAIN.to_string()));
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

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

/// A door whose code runs the game at `speed`, and despawns itself on its
/// second tick if `vanish`.
fn door(speed: f64, vanish: bool) -> Workbench {
    let mut w = Workbench::new(info());
    unhurried(&mut w);
    w.open.project = Some(crate::library::ProjectInfo {
        name: "Doors".into(),
        root: std::env::temp_dir(),
        assets: std::env::temp_dir(),
    });
    w.set_program(Some(program()));
    let mut door = EntityData::new("Door");
    let mut values = ComponentData::default();
    values.set("speed", Value::Float(speed));
    values.set("vanish", Value::Bool(vanish));
    door.components.insert("Door".into(), values);
    w.open
        .document
        .edit(
            "Add",
            vec![level::Edit::Spawn {
                entities: vec![(id(1), door)],
            }],
            None,
            None,
        )
        .unwrap();
    w.open.fixed_seed = Some(3);
    w
}

fn ticks(w: &mut Workbench, n: u64) {
    let until = w.world().unwrap().clock().ticks + n;
    while w.world().unwrap().clock().ticks < until {
        w.run_game((world::TICK * 0.6) as f32);
    }
}

/// What the door said, in order.
fn heard(w: &Workbench) -> Vec<String> {
    w.log_lines()
        .iter()
        .map(|l| l.text.clone())
        .filter(|t| t.starts_with("opened") || t.starts_with("dawn"))
        .collect()
}

fn waiting(w: &mut Workbench) -> usize {
    let playing = w.open.playing.as_mut().unwrap();
    playing.game.as_ref().map_or(0, |g| g.timers().len())
}

#[test]
fn a_timer_finishes_on_the_exact_tick_at_the_game_s_speed() {
    // Started on tick 1, two seconds of game time on: 120 ticks at speed
    // 1, twice as many at half speed, half as many at double.
    for (speed, tick) in [(1.0, 121), (0.5, 241), (2.0, 61)] {
        let mut w = door(speed, false);
        w.play();
        ticks(&mut w, 300);
        assert_eq!(
            heard(&w),
            ["dawn on 4".to_string(), format!("opened on {tick}")],
            "speed {speed}"
        );
    }
}

#[test]
fn nothing_counts_while_the_game_s_speed_is_0() {
    let mut w = door(0.0, false);
    w.play();
    ticks(&mut w, 400);
    assert!(
        !heard(&w).iter().any(|h| h.starts_with("opened")),
        "{:?}",
        heard(&w)
    );
    assert_eq!(waiting(&mut w), 2, "both still wait");
}

#[test]
fn a_save_made_while_a_timer_waits_finishes_on_the_same_tick() {
    let mut w = door(1.0, false);
    w.play();
    ticks(&mut w, 50);
    let text = w.save_of_game().unwrap().write().unwrap();
    assert!(text.contains("\ntimers "), "the save holds the timers");
    ticks(&mut w, 100);
    let carried = heard(&w);
    assert_eq!(carried, ["dawn on 4", "opened on 121"]);
    w.stop();
    w.play();
    w.load_game(&text, "quick");
    ticks(&mut w, 100);
    let mut heard_now = heard(&w);
    let after = heard_now.split_off(carried.len());
    assert_eq!(after, ["opened on 121"]);
    assert_eq!(waiting(&mut w), 1, "the dawn waits for tomorrow");
}

#[test]
fn a_despawned_entity_s_timers_end_with_it() {
    let mut w = door(1.0, true);
    w.play();
    ticks(&mut w, 200);
    assert!(heard(&w).is_empty(), "{:?}", heard(&w));
    assert_eq!(waiting(&mut w), 0);
}
