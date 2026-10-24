//! The scale levels (decision 0030), the CPU's share: whole editor
//! frames while units play, each a model moved by a Lust component, timed
//! by the Profiler. The GPU's share is timed on a real graphics card, in
//! the benchmark (Phase 5, M6). Projectiles join in M5.
//!
//! The CPU may use half of a frame at each level, leaving the other half
//! for the GPU: 8 ms at the target (60 fps), 16.7 ms at stress (30 fps).
//! The ceiling is where the CPU alone can't keep 60 fps any more.

use crate::profiler::Stage;
use crate::test_support::*;
use crate::workbench::Workbench;
use area::EntityData;
use area::builtin::Transform;
use glam::Vec3;
use reflect::{EntityId, Value};
use std::sync::Arc;

/// A unit: flies along its heading, turning back at the edge of the
/// field.
const SHIP: &str = "get \"solis/entity\":[positionOf, moveTo];\n\
    component Ship {\n\
        has {\n\
            vec3 heading = [1, 0, 0];\n\
            float speed = 8;\n\
        }\n\
        method update(float dt) {\n\
            vec3 at = positionOf(me) + heading * speed * dt;\n\
            is at.x > 200 || at.x < -200? then {\n\
                heading = heading * -1;\n\
            }\n\
            moveTo(me, at);\n\
        }\n\
    }\n\
    pass Ship;\nend\n";

/// A workbench playing `units` ships, each a model, spread over a field.
fn fleet(units: usize) -> Workbench {
    let mut files = script::describe::descriptions("Test Engine 9.9.9", "solis");
    files.push(("main.lst".into(), SHIP.to_string()));
    let code = script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    let program = match script::compile(&code) {
        Ok(Some(program)) => program,
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    };
    let mut w = Workbench::new(info(), None);
    let model = crate_model();
    let asset = model.asset;
    w.models.insert(asset, model);
    w.set_program(Some(Arc::new(program)));
    let side = (units as f32).sqrt().ceil() as usize;
    let spawns = (0..units)
        .map(|i| {
            let mut data = EntityData::new(&format!("Ship {i}")).with(&Transform {
                position: Vec3::new(
                    (i % side) as f32 * 4.0 - 200.0,
                    (i / side) as f32 * 4.0 - 200.0,
                    10.0,
                ),
                ..Transform::default()
            });
            data.set(&area::builtin::Model {
                asset: Some(reflect::AssetRef { asset, part: None }),
            });
            let mut ship = reflect::ComponentData::default();
            let turn = (i as f32 * 0.37).sin();
            ship.set("heading", Value::floats(&[turn, 1.0 - turn.abs(), 0.0]));
            data.components.insert("Ship".into(), ship);
            area::Edit::Spawn {
                entities: vec![(EntityId::random(), data)],
            }
        })
        .collect();
    w.document.edit("Fleet", spawns, None, None).unwrap();
    w.play();
    w
}

/// The CPU's average frame with `units` playing, after a few to settle,
/// and the stages' share of it.
fn cpu_frame(units: usize, frames: usize) -> (f64, [f64; 5]) {
    // Frames of 1/60 s, as at 60 fps: one tick each.
    let mut h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            |ui, state: &mut State| {
                let (workbench, picker) = state;
                let actions = workbench.show(ui, picker);
                picker.actions.extend(actions);
            },
            (fleet(units), Picker::default()),
        );
    for _ in 0..3 {
        h.step();
    }
    for _ in 0..frames {
        h.step();
    }
    let profiler = &h.state().0.profiler;
    assert_eq!(
        h.state()
            .0
            .playing
            .as_ref()
            .unwrap()
            .game
            .as_ref()
            .unwrap()
            .len(),
        units,
        "every ship runs"
    );
    let average = profiler.average(frames).expect("frames measured");
    (average.cpu_total(), average.cpu)
}

/// The stages, for the scale check's line.
fn split(cpu: [f64; 5]) -> String {
    Stage::ALL
        .iter()
        .zip(cpu)
        .map(|(stage, ms)| format!("{} {ms:.1}", stage.name().to_lowercase()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Whether the levels must already hold. Phase 5's milestones build what
/// they need; until its end (M6) the scale check reports them only.
const PHASE_5_DONE: bool = false;

fn level(name: &str, units: usize, budget: f64) {
    let (ms, cpu) = cpu_frame(units, 30);
    let verdict = match (ms <= budget, PHASE_5_DONE) {
        (true, _) => "ok",
        (false, true) => "SLOW",
        (false, false) => "not yet (Phase 5 builds it)",
    };
    println!(
        "Scale check: the {name} level's CPU frame, {units} units playing {ms:.2} ms \
         (target {budget:.1} ms; {}) {verdict}",
        split(cpu)
    );
    if PHASE_5_DONE {
        assert!(ms <= budget, "{name}: {ms:.2} ms");
    }
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_level_target() {
    level("target", 2_000, 8.0);
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_level_stress() {
    level("stress", 10_000, 1000.0 / 60.0);
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_level_ceiling() {
    // Doubles until the CPU alone takes more than a 60 fps frame, then
    // narrows down to within a tenth.
    const FRAME: f64 = 1000.0 / 60.0;
    const MOST: usize = 256_000;
    let fits = |units: usize| cpu_frame(units, 10).0 <= FRAME;
    let mut low = 0;
    let mut high = 1_000;
    while high <= MOST && fits(high) {
        low = high;
        high *= 2;
    }
    while high - low > (low / 10).max(250) {
        let middle = (low + high) / 2;
        if fits(middle) {
            low = middle;
        } else {
            high = middle;
        }
    }
    let shown = if high > MOST {
        format!("over {MOST}")
    } else {
        format!("about {low}")
    };
    println!(
        "Scale check: the ceiling: the CPU keeps 60 fps up to {shown} units playing (reported)"
    );
}

#[test]
fn a_small_fleet_plays_and_the_profiler_sees_its_game_code() {
    let (ms, cpu) = cpu_frame(50, 5);
    assert!(ms > 0.0);
    assert!(cpu[Stage::GameCode as usize] > 0.0, "{cpu:?}");
    assert!(cpu[Stage::World as usize] > 0.0, "{cpu:?}");
}

#[test]
#[ignore = "for profiling by hand"]
fn profile_game_code_by_hand() {
    // Few enough that a tick stays inside the tick limit under a
    // profiler's slowness.
    let mut w = fleet(200);
    for _ in 0..600 {
        w.play_for(1.0 / 60.0);
    }
    assert!(!w.playing.as_ref().unwrap().paused);
}

#[test]
#[ignore = "for profiling by hand"]
fn profile_editor_frames_by_hand() {
    let (ms, cpu) = cpu_frame(1_000, 40);
    println!("{ms:.2} ms: {}", split(cpu));
}
