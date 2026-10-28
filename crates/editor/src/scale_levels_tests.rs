//! The scale levels (decision 0030), the CPU's share: whole editor
//! frames while the benchmark's ships play (its own Lust: each a model
//! flying a loop and firing lasers, ten in flight per ship), timed by the
//! Profiler. The GPU's
//! share is timed on a real graphics card, in the benchmark (Phase 5,
//! M7).
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
use reflect::EntityId;
use std::sync::Arc;

/// The benchmark's own game code (Help > Make the benchmark…): each
/// `Ship` flies a loop and fires a laser six times a second, keeping ten
/// in flight, so the levels' projectiles (decision 0030) come with their
/// units.
const BENCHMARK: &str = include_str!("../../../samples/benchmark/assets/main.lst");

/// Where the benchmark places ship number `n` (its Lust's `startOf`).
fn start_of(n: usize) -> Vec3 {
    let (team, k) = (n % 2, n / 2);
    let radius = 20.0 + (k % 40) as f32 * 3.0;
    let y = (k / 40 % 50) as f32 * 8.0 - 200.0;
    let z = 12.0 + (k % 3) as f32 * 4.0 + (k / 2000) as f32 * 14.0;
    let centre = if team == 0 { -120.0 } else { 120.0 };
    Vec3::new(centre + radius, y, z)
}

/// A workbench playing `units` of the benchmark's ships, each a model.
fn fleet(units: usize) -> Workbench {
    let mut files = script::describe::descriptions("Test Engine 9.9.9", "solis");
    files.push(("main.lst".into(), BENCHMARK.to_string()));
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
    let spawns = (0..units)
        .map(|i| {
            let mut data = EntityData::new(&format!("Ship {i}")).with(&Transform {
                position: start_of(i),
                ..Transform::default()
            });
            data.set(&area::builtin::Model {
                asset: Some(reflect::AssetRef { asset, part: None }),
            });
            data.components
                .insert("Ship".into(), reflect::ComponentData::default());
            area::Edit::Spawn {
                entities: vec![(EntityId::random(), data)],
            }
        })
        .collect();
    w.document.edit("Fleet", spawns, None, None).unwrap();
    w.play();
    w
}

/// The CPU's average frame with `units` playing, after enough to settle
/// (every unit's ten lasers in flight), and the stages' share of it.
fn cpu_frame(units: usize, frames: usize) -> (f64, [f64; 6]) {
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
    // A laser lives 100 ticks: by then each unit has its ten in flight.
    for _ in 0..100 {
        h.step();
    }
    for _ in 0..frames {
        h.step();
    }
    let running = h
        .state_mut()
        .0
        .playing
        .as_mut()
        .unwrap()
        .game
        .as_mut()
        .unwrap()
        .len();
    assert_eq!(running, units, "every ship runs");
    let flying = h
        .state()
        .0
        .playing
        .as_ref()
        .unwrap()
        .world
        .projectiles()
        .len();
    assert!(
        flying >= units * 9 && flying <= units * 10,
        "{flying} lasers for {units} ships"
    );
    let profiler = &h.state().0.profiler;
    let average = profiler.average(frames).expect("frames measured");
    (average.cpu_total(), average.cpu)
}

/// The stages, for the scale check's line.
fn split(cpu: [f64; 6]) -> String {
    Stage::ALL
        .iter()
        .zip(cpu)
        .map(|(stage, ms)| format!("{} {ms:.1}", stage.name().to_lowercase()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The fewest cores the stress level is judged on: game code runs on
/// every core (decision 0033), and the gate's PC has many; on fewer, the
/// level is reported, not judged. The target level is judged on any.
const STRESS_CORES: usize = 4;

fn level(name: &str, units: usize, budget: f64, judged: bool) {
    let (ms, cpu) = cpu_frame(units, 30);
    let cores = rayon::current_num_threads();
    let verdict = match (ms <= budget, judged) {
        (true, _) => "ok".to_string(),
        (false, true) => "SLOW".to_string(),
        (false, false) => format!("reported: judged on {STRESS_CORES}+ cores, this has {cores}"),
    };
    println!(
        "Scale check: the {name} level's CPU frame, {units} units and {} lasers playing \
         {ms:.2} ms (target {budget:.1} ms; {}) {verdict}",
        units * 10,
        split(cpu)
    );
    if judged {
        assert!(ms <= budget, "{name}: {ms:.2} ms");
    }
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_level_target() {
    level("target", 2_000, 8.0, true);
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_level_stress() {
    level(
        "stress",
        10_000,
        1000.0 / 60.0,
        rayon::current_num_threads() >= STRESS_CORES,
    );
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
        "Scale check: the ceiling: the CPU keeps 60 fps up to {shown} units (and ten times as many lasers) playing (reported)"
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
