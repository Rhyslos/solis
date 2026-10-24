use crate::properties::PropertyRequest;
use crate::test_support::*;
use crate::workbench::Workbench;
use area::EntityData;
use area::builtin::Transform;
use glam::Vec3;
use reflect::{EntityId, Value};
use std::sync::Arc;

/// The program for a `main.lst` holding `main`, with the engine's
/// description files.
fn program(main: &str) -> Arc<script::Compiled> {
    match compiled(main) {
        Ok(chunk) => Arc::new(chunk),
        Err(problems) => panic!("{}", problems[0]),
    }
}

/// [`program`], or its mistakes.
fn compiled(main: &str) -> Result<script::Compiled, Vec<script::CodeProblem>> {
    let mut files = script::describe::descriptions("Test Engine 9.9.9", "solis");
    files.push((
        "main.lst".into(),
        format!(
            "get \"solis/world\":[say, time, find];\n\
             get \"solis/entity\":[positionOf, moveTo, rotationOf, turnTo, nameOf];\n{main}"
        ),
    ));
    let code = script::Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    script::compile(&code).map(|c| c.expect("there's code"))
}

const MOVER: &str = "component Mover {\n\
        has {\n\
            float speed = 6;\n\
            int steps = 0;\n\
            string label = \"mover\";\n\
        }\n\
        method update(float dt) {\n\
            vec3 ahead = [1, 0, 0];\n\
            moveTo(me, positionOf(me) + ahead * speed * dt);\n\
            steps = steps + 1;\n\
            is steps == 1? then {\n\
                say(nameOf(me) + \" sets off\");\n\
            }\n\
        }\n\
    }\n\
    pass Mover;\nend\n";

/// A workbench with the Mover program and an entity carrying it.
fn with_a_mover() -> (Workbench, EntityId) {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(MOVER)));
    let id = EntityId::random();
    w.document
        .edit(
            "Add",
            vec![area::Edit::Spawn {
                entities: vec![(id, EntityData::new("Cart").with(&Transform::default()))],
            }],
            None,
            None,
        )
        .unwrap();
    w.apply_property(
        PropertyRequest::AddComponent {
            entity: id,
            component: "Mover".into(),
        },
        1,
    );
    (w, id)
}

fn x_in_game(w: &Workbench, id: EntityId) -> f32 {
    w.world().unwrap().global(id).unwrap().w_axis.x
}

#[test]
fn the_program_s_components_join_the_registry_and_replace_the_last_ones() {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(MOVER)));
    let schema = w.document.registry.get("Mover").expect("registered");
    assert_eq!(schema.field("speed").unwrap().default, Value::Float(6.0));
    assert_eq!(w.game_components().collect::<Vec<_>>(), ["Mover"]);
    // New code: Mover is gone, Spinner arrives.
    w.set_program(Some(program(
        "component Spinner {\n    has {\n        float turn = 1;\n    }\n}\npass Spinner;\nend\n",
    )));
    assert!(w.document.registry.get("Mover").is_none());
    assert!(w.document.registry.get("Spinner").is_some());
    // Another project: none of this one's code is left.
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Other".into(),
        root: "/other".into(),
        assets: "/other/assets".into(),
    }));
    assert!(w.document.registry.get("Spinner").is_none());
    w.set_program(Some(program(MOVER)));
    // No code: none left; the built-in ones stay.
    w.set_program(None);
    assert!(w.document.registry.get("Spinner").is_none());
    assert!(w.document.registry.get("Light").is_some());
}

#[test]
fn a_component_named_like_a_built_in_one_or_a_failing_start_is_a_problem() {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(
        "component Light {\n    has {\n        int on = 1;\n    }\n}\npass Light;\nend\n",
    )));
    assert_eq!(w.code_problems().len(), 1);
    assert!(w.code_problems()[0].message.contains("Light"));
    assert!(
        w.document
            .registry
            .get("Light")
            .unwrap()
            .field("on")
            .is_none()
    );
    // A start that fails is found while compiling, on the app's worker,
    // so the editor never runs it on the frame.
    let problems = compiled("int ways = 0;\nint each = 12 / ways;\nend\n").unwrap_err();
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].line, 4);
    assert!(problems[0].message.contains("stopped while starting"));
}

#[test]
fn play_runs_the_game_s_components_and_stop_puts_everything_back() {
    let (mut w, id) = with_a_mover();
    w.play();
    // Ten ticks of 1/60 s at 6 m/s: 1 m.
    for _ in 0..10 {
        w.run_game(1.0 / 60.0);
    }
    assert!(
        (x_in_game(&w, id) - 1.0).abs() < 1e-3,
        "{}",
        x_in_game(&w, id)
    );
    assert!(
        w.log_lines().iter().any(|l| l.text == "Cart sets off"),
        "said once, in the Console"
    );
    // The area itself didn't move: only the game's world.
    assert_eq!(
        w.document().area().world_matrix(id).w_axis.x,
        0.0,
        "the editor's area is untouched"
    );
    w.stop();
    assert!(w.world().is_none());
}

#[test]
fn a_change_in_properties_while_playing_reaches_the_running_component() {
    let (mut w, id) = with_a_mover();
    w.play();
    w.run_game(1.0 / 60.0);
    w.apply_property(
        PropertyRequest::SetField {
            entity: id,
            component: "Mover".into(),
            field: "speed".into(),
            value: Value::Float(0.0),
        },
        2,
    );
    let before = x_in_game(&w, id);
    for _ in 0..5 {
        w.run_game(1.0 / 60.0);
    }
    assert!((x_in_game(&w, id) - before).abs() < 1e-5, "stopped moving");
    let running = w.playing.as_mut().unwrap().game.as_mut().unwrap();
    assert_eq!(
        running.values(id, "Mover").unwrap().get("steps"),
        Some(&Value::Int(6)),
        "the steps counted so far are kept"
    );
    // Taking the component off stops it.
    w.apply_property(
        PropertyRequest::RemoveComponent {
            entity: id,
            component: "Mover".into(),
        },
        3,
    );
    w.run_game(1.0 / 60.0);
    let running = w.playing.as_mut().unwrap().game.as_mut().unwrap();
    assert!(running.is_empty());
}

#[test]
fn a_failing_component_is_reported_once_naming_its_entity() {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(
        "component Divider {\n    has {\n        int ways = 0;\n    }\n    method update(float dt) {\n        int each = 12 / ways;\n    }\n}\npass Divider;\nend\n",
    )));
    let id = EntityId::random();
    let mut data = EntityData::new("Splitter");
    data.set(&Transform {
        position: Vec3::ZERO,
        ..Transform::default()
    });
    data.components
        .insert("Divider".into(), reflect::ComponentData::default());
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
    w.play();
    for _ in 0..5 {
        w.run_game(1.0 / 60.0);
    }
    let reports: Vec<_> = w
        .log_lines()
        .iter()
        .filter(|l| {
            l.text
                .starts_with("Divider on Splitter stopped: main.lst, line 8")
        })
        .collect();
    assert_eq!(reports.len(), 1, "{:?}", w.log_lines().last());
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_playing_with_2000_lust_components() {
    // Each of 2,000 entities carries a Lust component that moves it every
    // tick: a frame of play mode runs one tick of all of them.
    const TARGET_MS: f64 = 16.0;
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(MOVER)));
    let spawns: Vec<_> = (0..2_000)
        .map(|i| {
            let mut data = EntityData::new(&format!("Cart {i}")).with(&Transform {
                position: Vec3::new(0.0, i as f32, 0.0),
                ..Transform::default()
            });
            data.components
                .insert("Mover".into(), reflect::ComponentData::default());
            area::Edit::Spawn {
                entities: vec![(EntityId::random(), data)],
            }
        })
        .collect();
    w.document.edit("Fill", spawns, None, None).unwrap();
    w.play();
    w.run_game(1.0 / 60.0);
    let frames = 60;
    let start = std::time::Instant::now();
    for _ in 0..frames {
        w.run_game(1.0 / 60.0);
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: a play-mode tick of 2,000 Lust components that move their entities {ms:.2} ms (target {TARGET_MS} ms, one frame) {verdict}"
    );
    assert!(ms <= TARGET_MS, "a tick takes {ms:.2} ms");
}

/// A Rust component standing in for a game's: it answers each Ping with a
/// Rang carrying how many it has heard.
struct Bell {
    rung: i64,
}

impl script::RustComponent for Bell {
    fn handle(&mut self, cx: &mut script::Context<'_>, event: &script::Event) -> bool {
        if event.name != "Ping" {
            return false;
        }
        self.rung += 1;
        let me = cx.me();
        cx.send(Some(me), "Rang", vec![Value::Int(self.rung)]);
        true
    }
    fn values(&self) -> reflect::ComponentData {
        let mut data = reflect::ComponentData::default();
        data.set("rung", Value::Int(self.rung));
        data
    }
    fn set(&mut self, data: &reflect::ComponentData) {
        if let Some(Value::Int(n)) = data.get("rung") {
            self.rung = *n;
        }
    }
}

#[test]
fn a_lust_and_a_rust_component_on_one_entity_talk_through_events() {
    let mut w = Workbench::new(info(), None);
    w.add_rust_component(script::RustKind {
        schema: reflect::ComponentSchema {
            name: "Bell".into(),
            help: "Rings.".into(),
            fields: vec![reflect::FieldInfo::new(
                "rung",
                "Rung",
                reflect::FieldKind::Int,
                Value::Int(0),
            )],
        },
        make: |_| Box::new(Bell { rung: 0 }),
    })
    .unwrap();
    assert!(w.document.registry.get("Bell").is_some(), "in Properties");
    w.set_program(Some(program(
        "event Ping();\n\
         event Rang(int times);\n\
         component Ringer {\n\
             has {\n\
                 int asked = 0;\n\
             }\n\
             method update(float dt) {\n\
                 is asked < 2? then {\n\
                     asked = asked + 1;\n\
                     send Ping() to me;\n\
                 }\n\
             }\n\
             on Rang(int times) {\n\
                 say(\"the bell rang \" + times.text());\n\
             }\n\
         }\n\
         pass Ringer;\nend\n",
    )));
    let id = EntityId::random();
    let mut data = EntityData::new("Tower");
    data.components
        .insert("Ringer".into(), reflect::ComponentData::default());
    data.components
        .insert("Bell".into(), reflect::ComponentData::default());
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
    w.play();
    for _ in 0..3 {
        w.run_game(1.0 / 60.0);
    }
    let said: Vec<&str> = w
        .log_lines()
        .iter()
        .map(|l| l.text.as_str())
        .filter(|t| t.starts_with("the bell"))
        .collect();
    assert_eq!(said, ["the bell rang 1", "the bell rang 2"]);
}

#[test]
fn new_code_while_playing_is_swapped_in_and_keeps_where_things_are() {
    let (mut w, id) = with_a_mover();
    w.play();
    for _ in 0..10 {
        w.run_game(1.0 / 60.0);
    }
    let before = x_in_game(&w, id);
    // The Mover now goes the other way, and has a new field.
    w.set_program(Some(program(
        "component Mover {\n\
             has {\n\
                 float speed = 6;\n\
                 int steps = 0;\n\
                 string label = \"mover\";\n\
                 bool loud = false;\n\
             }\n\
             method update(float dt) {\n\
                 vec3 back = [-1, 0, 0];\n\
                 moveTo(me, positionOf(me) + back * speed * dt);\n\
                 steps = steps + 1;\n\
             }\n\
         }\n\
         pass Mover;\nend\n",
    )));
    assert!(
        w.log_lines()
            .iter()
            .any(|l| l.text.contains("took the new code (Mover changed)")),
        "{:?}",
        w.log_lines().last()
    );
    let running = w.playing.as_mut().unwrap().game.as_mut().unwrap();
    assert_eq!(
        running.values(id, "Mover").unwrap().get("steps"),
        Some(&Value::Int(10)),
        "the steps so far are kept"
    );
    assert!(
        w.document
            .registry
            .get("Mover")
            .unwrap()
            .field("loud")
            .is_some()
    );
    w.run_game(1.0 / 60.0);
    assert!(
        x_in_game(&w, id) < before,
        "it goes back now, from where it was"
    );
    assert!(x_in_game(&w, id) > before - 0.2);
}

#[test]
fn a_lua_component_joins_properties_and_play_mode() {
    let mut w = Workbench::new(info(), None);
    let code = script::Code {
        files: Vec::new(),
        lua: vec![(
            "spinner.lua".into(),
            "local S = component(\"Spinner\", { { \"turns\", \"int\", 0 } })\n\
             function S:update(dt) self.turns = self.turns + 1 end\n"
                .into(),
        )],
        notes: Vec::new(),
    };
    let compiled = script::compile(&code).unwrap().unwrap();
    w.set_program(Some(Arc::new(compiled)));
    assert!(w.document.registry.get("Spinner").is_some());
    let id = EntityId::random();
    let mut data = EntityData::new("Top");
    data.components
        .insert("Spinner".into(), reflect::ComponentData::default());
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
    w.play();
    for _ in 0..3 {
        w.run_game(1.0 / 60.0);
    }
    let running = w.playing.as_mut().unwrap().game.as_mut().unwrap();
    assert_eq!(
        running.values(id, "Spinner").unwrap().get("turns"),
        Some(&Value::Int(3))
    );
}

#[test]
fn a_tick_that_takes_too_long_pauses_the_game_and_says_why() {
    let mut w = Workbench::new(info(), None);
    let code = script::Code {
        files: Vec::new(),
        lua: vec![(
            "slow.lua".into(),
            // Four slow components on one entity: one core runs them all
            // (the tick limit is per core).
            (0..4)
                .map(|n| {
                    format!(
                        "local S{n} = component(\"Slow{n}\", {{}})\n\
                         function S{n}:update(dt)\n\
                           local start = os.clock()\n\
                           while os.clock() - start < 0.09 do end\n\
                         end\n"
                    )
                })
                .collect(),
        )],
        notes: Vec::new(),
    };
    w.set_program(Some(Arc::new(script::compile(&code).unwrap().unwrap())));
    let mut data = EntityData::new("Slow");
    for n in 0..4 {
        data.components
            .insert(format!("Slow{n}"), reflect::ComponentData::default());
    }
    {
        w.document
            .edit(
                "Add",
                vec![area::Edit::Spawn {
                    entities: vec![(EntityId::random(), data)],
                }],
                None,
                None,
            )
            .unwrap();
    }
    w.play();
    // A long frame would catch up with several ticks; the first is too
    // slow, so the rest don't run.
    let start = std::time::Instant::now();
    w.run_game(0.1);
    assert!(start.elapsed() < std::time::Duration::from_millis(600));
    assert!(w.playing.as_ref().unwrap().paused);
    assert!(
        w.log_lines()
            .last()
            .unwrap()
            .text
            .contains("so the game paused")
    );
}

/// Spinner's Lust, in place of Mover.
const SPINNER: &str =
    "component Spinner {\n    has {\n        float turn = 1;\n    }\n}\npass Spinner;\nend\n";

#[test]
fn code_changed_while_playing_keeps_its_components_after_stop() {
    let (mut w, _) = with_a_mover();
    w.play();
    w.set_program(Some(program(SPINNER)));
    w.stop();
    assert!(w.document.registry.get("Spinner").is_some());
    assert!(w.document.registry.get("Mover").is_none());
    assert_eq!(w.game_components().collect::<Vec<_>>(), ["Spinner"]);
}

#[test]
fn a_component_named_like_a_built_in_one_doesn_t_run() {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(
        "component Light {\n    has {\n        int on = 1;\n    }\n\
         \x20   method update(float dt) {\n        say(\"lust light ran\");\n    }\n}\n\
         pass Light;\nend\n",
    )));
    assert_eq!(w.code_problems()[0].line, 3, "where it's written");
    w.play();
    w.run_game(1.0 / 60.0);
    assert!(
        !w.log_lines()
            .iter()
            .any(|l| l.text.contains("lust light ran")),
        "the built-in Light is the sun's; the code's doesn't run"
    );
}

#[test]
fn adding_an_entity_while_playing_keeps_where_the_game_moved_things() {
    let (mut w, id) = with_a_mover();
    w.play();
    for _ in 0..30 {
        w.run_game(1.0 / 60.0);
    }
    let x = x_in_game(&w, id);
    assert!(x > 2.0, "{x}");
    w.document
        .edit(
            "Add",
            vec![area::Edit::Spawn {
                entities: vec![(EntityId::random(), EntityData::new("New"))],
            }],
            None,
            None,
        )
        .unwrap();
    w.run_game(1.0 / 60.0);
    assert!(x_in_game(&w, id) > x, "carried on from where it was");
}

/// Lust whose start fails only when the game's world has a Cart.
const NEEDS_NO_CART: &str = "entity cart = find(\"Cart\");\n\
    is cart != nothing? then {\n    int zero = 0;\n    int bad = 1 / zero;\n}\n";

#[test]
fn new_code_the_running_game_can_t_take_leaves_the_last_program_and_registry() {
    let (mut w, id) = with_a_mover();
    w.play();
    w.run_game(1.0 / 60.0);
    w.set_program(Some(program(&format!("{NEEDS_NO_CART}{SPINNER}"))));
    assert!(w.document.registry.get("Mover").is_some(), "still known");
    assert!(w.document.registry.get("Spinner").is_none());
    assert_eq!(w.code_problems().len(), 1, "{:?}", w.code_problems());
    let x = x_in_game(&w, id);
    w.run_game(1.0 / 60.0);
    assert!(x_in_game(&w, id) > x, "still running the last code");
}

/// A Rust component that counts its ticks.
struct Ticker(i64);

impl script::RustComponent for Ticker {
    fn update(&mut self, _cx: &mut script::Context<'_>, _dt: f64) {
        self.0 += 1;
    }
    fn values(&self) -> reflect::ComponentData {
        let mut data = reflect::ComponentData::default();
        data.set("ticks", Value::Int(self.0));
        data
    }
    fn set(&mut self, _data: &reflect::ComponentData) {}
}

fn ticker() -> script::RustKind {
    script::RustKind {
        schema: reflect::ComponentSchema {
            name: "Ticker".into(),
            help: String::new(),
            fields: vec![reflect::FieldInfo::new(
                "ticks",
                "Ticks",
                reflect::FieldKind::Int,
                Value::Int(0),
            )],
        },
        make: |_| Box::new(Ticker(0)),
    }
}

#[test]
fn rust_components_run_even_when_the_code_s_start_fails_at_play() {
    let (mut w, id) = with_a_mover();
    w.set_rust_components(vec![ticker()]);
    // Accepted while editing (no world), but its start fails at Play.
    w.set_program(Some(program(&format!("{NEEDS_NO_CART}{MOVER}"))));
    w.apply_property(
        PropertyRequest::AddComponent {
            entity: id,
            component: "Ticker".into(),
        },
        2,
    );
    w.play();
    assert!(
        w.log_lines()
            .iter()
            .any(|l| l.text.contains("stopped while starting")),
        "said"
    );
    w.run_game(1.0 / 60.0);
    let game = w.playing.as_mut().unwrap().game.as_mut().unwrap();
    assert_eq!(
        game.values(id, "Ticker").unwrap().get("ticks"),
        Some(&Value::Int(1))
    );
}

#[test]
fn a_lust_component_named_like_a_rust_one_says_so_and_the_rust_one_runs() {
    let mut w = Workbench::new(info(), None);
    w.set_rust_components(vec![ticker()]);
    w.set_program(Some(program(
        "component Ticker {\n    has {\n        int ticks = 0;\n    }\n}\npass Ticker;\nend\n",
    )));
    assert_eq!(w.code_problems().len(), 1);
    assert!(
        w.code_problems()[0]
            .message
            .contains("a component written in Rust"),
        "{}",
        w.code_problems()[0]
    );
}

#[test]
fn a_failure_that_doesn_t_stop_its_component_isn_t_said_to() {
    let area = area::builtin::starter_area();
    let (id, sun) = area.iter().next().unwrap();
    let failure = script::Failure {
        entity: Some(id),
        component: "Health".into(),
        at: script::CodeProblem {
            file: String::new(),
            line: 0,
            column: 0,
            message: "couldn't take the event Hit".into(),
        },
        stopped: false,
    };
    let text = crate::game_code::failure_text(&area, &failure);
    assert_eq!(
        text,
        format!("Health on {}: couldn't take the event Hit", sun.name)
    );
}

/// A workbench with `program`'s Lust and `count` entities ("Cart 0" …)
/// each carrying its `component`, one metre apart.
fn crowd_with(main: &str, component: &str, count: usize) -> (Workbench, Vec<EntityId>) {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(main)));
    let mut ids = Vec::with_capacity(count);
    let spawns: Vec<_> = (0..count)
        .map(|i| {
            let mut data = EntityData::new(&format!("Cart {i}")).with(&Transform {
                position: Vec3::new(0.0, i as f32, 0.0),
                ..Transform::default()
            });
            data.components
                .insert(component.into(), reflect::ComponentData::default());
            let id = EntityId::random();
            ids.push(id);
            area::Edit::Spawn {
                entities: vec![(id, data)],
            }
        })
        .collect();
    w.document.edit("Fill", spawns, None, None).unwrap();
    (w, ids)
}

/// Prints a scale check's line and checks its target.
fn verdict(what: &str, ms: f64, target: f64) {
    let verdict = if ms <= target { "ok" } else { "SLOW" };
    println!("Scale check: {what} {ms:.2} ms (target {target} ms) {verdict}");
    assert!(ms <= target, "{what}: {ms:.2} ms");
}

/// A component that finds its target by name every tick (the turret and
/// AI way).
const SEEKER: &str = "component Seeker {\n\
        has {\n\
            int seen = 0;\n\
        }\n\
        method update(float dt) {\n\
            entity target = find(\"Cart 1999\");\n\
            is target != nothing? then {\n\
                seen = seen + 1;\n\
            }\n\
        }\n\
    }\n\
    pass Seeker;\nend\n";

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_2000_lust_components_finding_by_name() {
    // The efficiency review of Phase 4: find was a scan of every entity,
    // so 2,000 finds a tick took 300 ms.
    let (mut w, _) = crowd_with(SEEKER, "Seeker", 2_000);
    w.play();
    w.run_game(1.0 / 60.0);
    let frames = 60;
    let start = std::time::Instant::now();
    for _ in 0..frames {
        w.run_game(1.0 / 60.0);
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    verdict(
        "a play-mode tick of 2,000 Lust components each finding an entity by name",
        ms,
        16.0,
    );
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_pressing_play_with_2000_lust_components() {
    // Play starts every component: the code's start, then each one made
    // from the area.
    let (mut w, _) = crowd_with(MOVER, "Mover", 2_000);
    let start = std::time::Instant::now();
    w.play();
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(
        w.playing.as_mut().unwrap().game.as_mut().unwrap().len(),
        2_000
    );
    verdict("pressing Play with 2,000 Lust components", ms, 100.0);
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_new_code_into_2000_running_lust_components() {
    // Saving Lust while playing: the frame that takes the new code in
    // swaps it into every running component.
    let (mut w, _) = crowd_with(MOVER, "Mover", 2_000);
    w.play();
    w.run_game(1.0 / 60.0);
    let faster = program(&MOVER.replace("speed * dt", "speed * dt * 2"));
    let start = std::time::Instant::now();
    w.set_program(Some(faster));
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    assert!(
        w.log_lines()
            .iter()
            .any(|l| l.text.contains("took the new code"))
    );
    verdict("new Lust swapped into 2,000 running components", ms, 100.0);
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_dragging_512_entities_with_running_components() {
    // A drag of a big selection while playing: every frame, each dragged
    // entity's components hear of it. Its components have no update, so
    // this times the editor's side alone.
    const TAG: &str = "component Tag {\n    has {\n        int n = 0;\n    }\n}\npass Tag;\nend\n";
    let (mut w, ids) = crowd_with(TAG, "Tag", 2_000);
    w.play();
    w.run_game(1.0 / 60.0);
    let frames = 60;
    let start = std::time::Instant::now();
    for frame in 0..frames {
        let edits = ids[..512]
            .iter()
            .map(|&entity| area::Edit::SetField {
                entity,
                component: "Transform".into(),
                field: "position".into(),
                value: Some(Value::vec3(Vec3::new(frame as f32, 0.0, 0.0))),
            })
            .collect();
        w.document.edit("Drag", edits, None, None).unwrap();
        w.run_game(1.0 / 60.0);
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    verdict(
        "a frame dragging 512 of 2,000 entities whose components play",
        ms,
        8.0,
    );
}
