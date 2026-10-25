//! Whole-game invariant tests (decision 0042, the hardening milestone's
//! H2). Random games are played on a small level and checked against rules
//! that must always hold, whatever the game did:
//!
//! - **Loading is the same as never stopping:** a game loaded from a save
//!   and played on gives exactly the same checksums, tick by tick, as the
//!   game that was saved and played on.
//! - **Replays match** their recordings, saves included.
//! - **Stop puts everything back:** the area, the undo history and the
//!   selection are as they were before Play.
//! - **Nothing breaks:** no mistakes in the Console, every place finite.
//!
//! The level has a floor, a step, a wall, crates, a moving platform, a
//! character with the ready-made controller and third-person camera, a
//! hive that spawns drones (which wander with random numbers and despawn
//! themselves), and quick save and load keys. A game is a list of steps
//! (walk, jump, look, spawn, save, load, edit, pause and step, swap code);
//! when a rule breaks, proptest shrinks it to the shortest game that still
//! breaks it, and keeps that in `proptest-regressions/` to try first ever
//! after.

use crate::console::LogLevel;
use crate::properties::PropertyRequest;
use crate::test_support::*;
use crate::workbench::{Workbench, WorkbenchAction};
use area::EntityData;
use area::builtin::{BodyKind, Camera, CharacterBody, Collider, RigidBody, Transform};
use glam::Vec3;
use proptest::prelude::*;
use reflect::{ComponentData, EntityId, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

const MAIN: &str = "fetch \"solis/kit/character\":[CharacterController];\n\
    fetch \"solis/kit/cameras\":[ThirdPersonCamera];\n\
    get \"solis/world\":[spawn, despawn, saveGame, loadGame, randomRange, randomInt];\n\
    get \"solis/entity\":[positionOf, moveTo];\n\
    get \"solis/input\":[pressed];\n\
    get \"solis/physics\":[push];\n\
    \n\
    component Platform {\n\
        has {\n\
            float t = 0;\n\
        }\n\
        method update(float dt) {\n\
            t = t + dt;\n\
            is t > 4? then {\n\
                t = 0;\n\
            }\n\
            float way = 1;\n\
            is t > 2? then {\n\
                way = -1;\n\
            }\n\
            moveTo(me, positionOf(me) + [way * dt, 0, 0]);\n\
        }\n\
    }\n\
    \n\
    component Hive {\n\
        method update(float dt) {\n\
            is pressed(\"Interact\")? then {\n\
                entity made = spawn(\"drone\", [randomRange(-3, 3), randomRange(2, 6), 1]);\n\
            }\n\
        }\n\
    }\n\
    \n\
    component Drone {\n\
        has {\n\
            int age = 0;\n\
            int life = 0;\n\
        }\n\
        method update(float dt) {\n\
            is life == 0? then {\n\
                life = randomInt(20, 90);\n\
                push(me, [randomRange(-1, 1), randomRange(-1, 1), 2]);\n\
            }\n\
            age = age + 1;\n\
            is age >= life? then {\n\
                despawn(me);\n\
            }\n\
        }\n\
    }\n\
    \n\
    component Keys {\n\
        method update(float dt) {\n\
            is pressed(\"SaveA\")? then {\n\
                saveGame(\"a\");\n\
            }\n\
            is pressed(\"SaveB\")? then {\n\
                saveGame(\"b\");\n\
            }\n\
            is pressed(\"LoadA\")? then {\n\
                loadGame(\"a\");\n\
            }\n\
            is pressed(\"LoadB\")? then {\n\
                loadGame(\"b\");\n\
            }\n\
        }\n\
    }\n\
    \n\
    pass Platform, Hive, Drone, Keys;\n\
    end\n";

fn program() -> Arc<script::Compiled> {
    let mut files = script::describe::descriptions("Test Engine 9.9.9", "solis");
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

const PLAYER: u64 = 2;
const CAMERA: u64 = 3;
const PLATFORM: u64 = 6;
/// The crates are 20, 21, 22 …; edits add more from 40.
const CRATES: u64 = 20;
const CRATE_COUNT: u64 = 4;

fn block(at: Vec3, size: Vec3) -> EntityData {
    EntityData::new("Block")
        .with(&Transform {
            position: at,
            scale: size,
            ..Transform::default()
        })
        .with(&Collider::default())
}

fn a_crate(at: Vec3) -> EntityData {
    block(at, Vec3::splat(0.6)).with(&RigidBody::default())
}

/// The level, not yet playing, with the project's input actions and a
/// fixed seed.
fn level() -> Workbench {
    let mut w = Workbench::new(info(), None);
    w.project = Some(crate::library::ProjectInfo {
        name: "Invariants".into(),
        root: std::env::temp_dir(),
        assets: std::env::temp_dir(),
    });
    w.set_program(Some(program()));
    // The drone: a small dynamic body with its component.
    let mut drones = area::Area::new();
    let mut drone = a_crate(Vec3::ZERO);
    drone.name = "Drone".into();
    drone
        .components
        .insert("Drone".into(), ComponentData::default());
    drones.insert(id(1), drone);
    let asset = reflect::AssetId(reflect::Id::from_raw(901).unwrap());
    w.set_assembly(
        asset,
        Arc::new(area::text::AreaFile {
            kind: area::text::FileKind::Assembly { root: id(1) },
            area: drones,
            overrides: Default::default(),
        }),
    );
    w.set_library(vec![crate::library::LibraryItem {
        id: Some(asset),
        kind: crate::library::LibraryKind::Assembly,
        name: "drone".into(),
        path: "drones/drone.solasm".into(),
        thumbnail: None,
    }]);
    let mut bindings = input::Bindings::default();
    for (name, key) in [
        ("SaveA", "F6"),
        ("SaveB", "F8"),
        ("LoadA", "F7"),
        ("LoadB", "F9"),
    ] {
        bindings.actions.push(input::Action::button(name, &[key]));
    }
    w.set_bindings(bindings);
    let mut player = EntityData::new("Player")
        .with(&Transform::default())
        .with(&CharacterBody::default());
    let mut controller = ComponentData::default();
    controller.set("camera", Value::Entity(Some(id(CAMERA))));
    player
        .components
        .insert("CharacterController".into(), controller);
    let mut eye = EntityData::new("Camera")
        .with(&Transform {
            position: Vec3::new(0.0, -4.0, 2.0),
            ..Transform::default()
        })
        .with(&Camera::default());
    let mut follow = ComponentData::default();
    follow.set("target", Value::Entity(Some(id(PLAYER))));
    eye.components.insert("ThirdPersonCamera".into(), follow);
    let mut platform =
        block(Vec3::new(-6.0, 0.0, 0.1), Vec3::new(3.0, 3.0, 0.2)).with(&RigidBody {
            kind: BodyKind::Kinematic,
            ..RigidBody::default()
        });
    platform
        .components
        .insert("Platform".into(), ComponentData::default());
    let mut hive = EntityData::new("Hive");
    hive.components
        .insert("Hive".into(), ComponentData::default());
    let mut keys = EntityData::new("Keys");
    keys.components
        .insert("Keys".into(), ComponentData::default());
    let mut entities = vec![
        (
            id(1),
            block(Vec3::new(0.0, 0.0, -0.5), Vec3::new(60.0, 60.0, 1.0)),
        ),
        (id(PLAYER), player),
        (id(CAMERA), eye),
        (
            id(4),
            block(Vec3::new(0.0, 8.0, 2.0), Vec3::new(20.0, 0.5, 4.0)),
        ),
        (
            id(5),
            block(Vec3::new(5.0, 0.0, 0.125), Vec3::new(2.0, 6.0, 0.25)),
        ),
        (id(PLATFORM), platform),
        (id(7), hive),
        (id(8), keys),
    ];
    for n in 0..CRATE_COUNT {
        entities.push((
            id(CRATES + n),
            a_crate(Vec3::new(2.0 + n as f32 * 0.8, 2.0, 0.3 + n as f32 * 0.7)),
        ));
    }
    let edits = entities
        .into_iter()
        .map(|one| area::Edit::Spawn {
            entities: vec![one],
        })
        .collect();
    w.document.edit("Add", edits, None, None).unwrap();
    w.fixed_seed = Some(7);
    w
}

/// One step of a game.
#[derive(Debug, Clone)]
enum Step {
    /// Hold a movement key (0 to 3: W A S D), sprinting or not, for some
    /// ticks.
    Walk { key: u8, sprint: bool, ticks: u8 },
    /// Press Jump.
    Jump,
    /// Turn the camera.
    Look { x: i16, y: i16 },
    /// Nothing pressed, for some ticks.
    Wait { ticks: u8 },
    /// Press Interact: the hive spawns a drone.
    Spawn,
    /// Game code saves to slot a or b.
    Save { b: bool },
    /// Game code loads slot a or b (nothing happens if it was never
    /// saved).
    Load { b: bool },
    /// An edit while playing.
    Edit(Edit),
    /// Pause, step some ticks, and carry on.
    Paused { ticks: u8 },
    /// The same code arrives again (a save in the code editor).
    SwapCode,
}

/// An edit made in the editor while playing.
#[derive(Debug, Clone)]
enum Edit {
    /// Move crate `n` (of those there are) by some way.
    MoveCrate { n: u8, x: i8, y: i8, z: u8 },
    /// Delete crate `n`.
    DeleteCrate { n: u8 },
    /// Add a crate.
    AddCrate { x: i8, y: i8 },
    /// Change how heavy crate `n` is.
    CrateMass { n: u8, mass: u8 },
    /// Change the character's walking speed.
    WalkSpeed { speed: u8 },
}

fn edit() -> impl Strategy<Value = Edit> {
    prop_oneof![
        (any::<u8>(), -20i8..20, -20i8..20, 0u8..20).prop_map(|(n, x, y, z)| Edit::MoveCrate {
            n,
            x,
            y,
            z
        }),
        any::<u8>().prop_map(|n| Edit::DeleteCrate { n }),
        (-20i8..20, -20i8..20).prop_map(|(x, y)| Edit::AddCrate { x, y }),
        (any::<u8>(), 1u8..50).prop_map(|(n, mass)| Edit::CrateMass { n, mass }),
        (1u8..12).prop_map(|speed| Edit::WalkSpeed { speed }),
    ]
}

fn step(edits: bool) -> BoxedStrategy<Step> {
    let mut steps = vec![
        (
            4,
            (0u8..4, any::<bool>(), 1u8..40)
                .prop_map(|(key, sprint, ticks)| Step::Walk { key, sprint, ticks })
                .boxed(),
        ),
        (1, Just(Step::Jump).boxed()),
        (
            1,
            (-300i16..300, -100i16..100)
                .prop_map(|(x, y)| Step::Look { x, y })
                .boxed(),
        ),
        (2, (1u8..30).prop_map(|ticks| Step::Wait { ticks }).boxed()),
        (2, Just(Step::Spawn).boxed()),
        (2, any::<bool>().prop_map(|b| Step::Save { b }).boxed()),
        (1, (1u8..5).prop_map(|ticks| Step::Paused { ticks }).boxed()),
    ];
    if edits {
        steps.push((2, any::<bool>().prop_map(|b| Step::Load { b }).boxed()));
        steps.push((2, edit().prop_map(Step::Edit).boxed()));
        steps.push((1, Just(Step::SwapCode).boxed()));
    }
    proptest::strategy::Union::new_weighted(steps).boxed()
}

/// What a game played: each tick's checksums, the world's and game
/// code's, by tick number.
type Sums = BTreeMap<u64, (u64, u64)>;

/// Plays games on a workbench, doing what the app does with what game
/// code asks for (saves kept here, loads given back).
struct Player {
    w: Workbench,
    saves: BTreeMap<String, String>,
    sums: Sums,
}

impl Player {
    fn new(w: Workbench) -> Self {
        Self {
            w,
            saves: BTreeMap::new(),
            sums: Sums::new(),
        }
    }

    fn tick(&self) -> u64 {
        self.w.world().map_or(0, |world| world.clock().ticks)
    }

    /// Runs exactly one tick, then does what game code asked of the app,
    /// as the app does, and notes the tick's checksums.
    fn one_tick(&mut self) {
        let before = self.tick();
        // A load puts the clock back: one call is a tick, whichever way.
        for _ in 0..4 {
            self.w.play_for((world::TICK * 0.6) as f32);
            if self.tick() != before || self.w.playing.is_none() {
                break;
            }
            if self
                .w
                .outbox
                .iter()
                .any(|a| matches!(a, WorkbenchAction::LoadGame(_)))
            {
                break;
            }
        }
        self.serve();
        if let Some(playing) = &mut self.w.playing {
            let world = playing.world.checksum();
            let game = playing.game.as_mut().map_or(0, |g| g.checksum());
            let tick = playing.world.clock().ticks;
            self.sums.insert(tick, (world, game));
        }
    }

    /// What the app does with the outbox: saves are kept, loads given
    /// back to game code.
    fn serve(&mut self) {
        let asked = std::mem::take(&mut self.w.outbox);
        for action in asked {
            match action {
                WorkbenchAction::SaveGame { slot, text } => {
                    self.saves.insert(slot, text);
                }
                WorkbenchAction::LoadGame(slot) => {
                    if let Some(text) = self.saves.get(&slot).cloned() {
                        self.w.load_game_from_code(&text, &slot);
                    }
                }
                _ => {}
            }
        }
    }

    fn press(&mut self, key: &str) {
        self.w.devices.set(key, true);
        self.one_tick();
        self.w.devices.set(key, false);
        self.one_tick();
    }

    /// The crates there are now, in ID order.
    fn crates(&self) -> Vec<EntityId> {
        self.w
            .document
            .area()
            .iter()
            .filter(|(e, _)| {
                let raw = e.0.raw();
                (CRATES..CRATES + 100).contains(&raw)
            })
            .map(|(e, _)| e)
            .collect()
    }

    fn edit(&mut self, edit: &Edit, made: &mut u64) {
        let crates = self.crates();
        let pick = |n: u8| crates.get(n as usize % crates.len().max(1)).copied();
        let request = match *edit {
            Edit::MoveCrate { n, x, y, z } => pick(n).map(|e| PropertyRequest::SetField {
                entity: e,
                component: "Transform".into(),
                field: "position".into(),
                value: Value::floats(&[f32::from(x) * 0.5, f32::from(y) * 0.5, f32::from(z) * 0.3]),
            }),
            Edit::DeleteCrate { n } => pick(n).map(PropertyRequest::Delete),
            Edit::CrateMass { n, mass } => pick(n).map(|e| PropertyRequest::SetField {
                entity: e,
                component: "RigidBody".into(),
                field: "mass".into(),
                value: Value::Float(f64::from(mass)),
            }),
            Edit::WalkSpeed { speed } => Some(PropertyRequest::SetField {
                entity: id(PLAYER),
                component: "CharacterController".into(),
                field: "walkSpeed".into(),
                value: Value::Float(f64::from(speed)),
            }),
            Edit::AddCrate { x, y } => {
                *made += 1;
                let at = Vec3::new(f32::from(x) * 0.5, f32::from(y) * 0.5, 1.0);
                let _ = self.w.document.edit(
                    "Add crate",
                    vec![area::Edit::Spawn {
                        entities: vec![(id(CRATES + 40 + *made), a_crate(at))],
                    }],
                    None,
                    None,
                );
                None
            }
        };
        if let Some(request) = request {
            self.w.apply_property(request, 1000 + *made);
            *made += 1;
        }
    }

    /// Plays `steps`.
    fn play(&mut self, steps: &[Step]) {
        let mut made = 0;
        for step in steps {
            if self.w.playing.is_none() {
                return;
            }
            match step {
                Step::Walk { key, sprint, ticks } => {
                    let key = ["W", "A", "S", "D"][*key as usize % 4];
                    self.w.devices.set(key, true);
                    self.w.devices.set("Shift", *sprint);
                    for _ in 0..*ticks {
                        self.one_tick();
                    }
                    self.w.devices.set(key, false);
                    self.w.devices.set("Shift", false);
                }
                Step::Jump => self.press("Space"),
                Step::Look { x, y } => {
                    self.w
                        .devices
                        .mouse_moved(glam::Vec2::new(f32::from(*x), f32::from(*y)));
                    self.one_tick();
                }
                Step::Wait { ticks } => {
                    for _ in 0..*ticks {
                        self.one_tick();
                    }
                }
                Step::Spawn => self.press("F"),
                Step::Save { b } => self.press(if *b { "F8" } else { "F6" }),
                Step::Load { b } => self.press(if *b { "F9" } else { "F7" }),
                Step::Edit(edit) => {
                    self.edit(edit, &mut made);
                    self.one_tick();
                }
                Step::Paused { ticks } => {
                    self.w.pause();
                    for _ in 0..*ticks {
                        self.w.step();
                        self.one_tick();
                    }
                    self.w.pause();
                }
                Step::SwapCode => {
                    self.w.set_program(Some(program()));
                    self.one_tick();
                }
            }
        }
    }

    /// Starts playing, with the game holding the keys.
    fn start(&mut self, record: bool) {
        if record {
            self.w.play_and_record();
        } else {
            self.w.play();
        }
        self.w.give_game_input(&egui::Context::default());
    }

    /// Mistakes in the Console, and anything not finite in the world.
    fn broken(&self) -> Vec<String> {
        let mut found: Vec<String> = self
            .w
            .log_lines()
            .iter()
            .filter(|l| l.level == LogLevel::Error)
            .map(|l| l.text.clone())
            .collect();
        if let Some(world) = self.w.world() {
            for (e, _) in self.w.document.area().iter() {
                if let Some(m) = world.global(e)
                    && !m.is_finite()
                {
                    found.push(format!("{e} isn't anywhere: {m:?}"));
                }
            }
        }
        found
    }
}

fn config() -> ProptestConfig {
    ProptestConfig {
        // Kept to about a minute in CI; the weekly run sets more with
        // PROPTEST_CASES.
        cases: std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|n| n.parse().ok())
            .unwrap_or(12),
        max_shrink_iters: 300,
        ..ProptestConfig::default()
    }
}

proptest! {
    #![proptest_config(config())]

    /// A game saved at any moment and loaded again carries on exactly as
    /// the saved one did, tick by tick (the edits, loads and swaps after
    /// the save happen again in both).
    #[test]
    fn a_loaded_game_carries_on_exactly_as_the_saved_one_did(
        before in prop::collection::vec(step(true), 0..12),
        after in prop::collection::vec(step(true), 1..12),
    ) {
        let mut first = Player::new(level());
        first.start(false);
        first.play(&before);
        prop_assume!(first.w.playing.is_some());
        let saved = first.w.save_text().expect("a save");
        let slots = first.saves.clone();
        let cut = first.tick();
        first.sums.clear();
        first.play(&after);
        // The same from the save: loaded from the Play menu, the game
        // given the keys, and the slots as they were at the save.
        let mut second = Player::new(level());
        second.start(false);
        second.w.load_game(&saved, "saved.solsave");
        second.w.give_game_input(&egui::Context::default());
        second.saves = slots;
        prop_assert_eq!(
            second.tick(),
            cut,
            "{:?}",
            second.w.log_lines().iter().map(|l| &l.text).collect::<Vec<_>>()
        );
        second.play(&after);
        prop_assert_eq!(&first.sums, &second.sums);
        prop_assert_eq!(first.broken(), Vec::<String>::new());
        prop_assert_eq!(second.broken(), Vec::<String>::new());
    }

    /// A recording replays exactly, saves included.
    #[test]
    fn a_recording_replays_exactly(steps in prop::collection::vec(step(false), 1..16)) {
        let mut p = Player::new(level());
        p.start(true);
        p.play(&steps);
        p.w.stop();
        let recording = p.w.outbox.iter().find_map(|a| match a {
            WorkbenchAction::SaveRecording(text) => Some(text.clone()),
            _ => None,
        });
        // A load during the game ends the recording early (and saves it);
        // none is made from a game that recorded no ticks.
        let Some(recording) = recording else {
            return Ok(());
        };
        p.w.outbox.clear();
        p.w.replay(&recording, "game.solrec");
        for _ in 0..4000 {
            if p.w.play_state() == crate::play::PlayState::Paused {
                break;
            }
            p.w.play_for((world::TICK * 2.0) as f32);
            p.serve();
        }
        let said: Vec<&str> = p.w.log_lines().iter().map(|l| l.text.as_str()).collect();
        prop_assert!(
            said.iter().any(|l| l.contains("matched the recording")),
            "{:?}",
            said
        );
        prop_assert_eq!(p.broken(), Vec::<String>::new());
    }

    /// Stop puts the area, the undo history and the selection back as
    /// they were before Play, whatever happened while playing.
    #[test]
    fn stop_puts_everything_back(steps in prop::collection::vec(step(true), 1..16)) {
        let mut p = Player::new(level());
        p.w.select(id(CRATES));
        let registry = p.w.document.registry.clone();
        let area = area::text::write(p.w.document.area(), &registry, &|_| None);
        let undo = p.w.document.history().undo_len();
        let selected: Vec<EntityId> = p.w.document.selection.iter().collect();
        p.start(false);
        p.play(&steps);
        p.w.stop();
        prop_assert_eq!(
            area::text::write(p.w.document.area(), &registry, &|_| None),
            area
        );
        prop_assert_eq!(p.w.document.history().undo_len(), undo);
        prop_assert_eq!(p.w.document.selection.iter().collect::<Vec<_>>(), selected);
        prop_assert!(p.w.world().is_none());
    }
}
