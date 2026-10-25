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

#[test]
fn guns_in_lust_and_lua_fire_projectiles_that_fly_and_end() {
    // A Lust gun and a Lua gun, each firing one projectile a tick along
    // +Y at 30 m/s, living a quarter of a second.
    let mut files = script::describe::descriptions("Test Engine 9.9.9", "solis");
    files.push((
        "main.lst".into(),
        "get \"solis/world\":[fire];\n\
         get \"solis/entity\":[positionOf];\n\
         component LustGun {\n\
             has {\n\
                 int shots = 0;\n\
             }\n\
             method update(float dt) {\n\
                 fire(positionOf(me), [0, 1, 0], 30, 0.25, [4, 0.2, 0.1]);\n\
             }\n\
         }\n\
         pass LustGun;\nend\n"
            .into(),
    ));
    let code = script::Code {
        files,
        lua: vec![(
            "gun.lua".into(),
            "local G = component(\"LuaGun\", {})\n\
             function G:update(dt)\n\
               solis.fire(solis.positionOf(self.me), vec3(0, 1, 0), 30, 0.25, vec3(0.1, 4, 0.2))\n\
             end\n"
                .into(),
        )],
        notes: Vec::new(),
    };
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(Arc::new(script::compile(&code).unwrap().unwrap())));
    let mut data = EntityData::new("Guns");
    for gun in ["LustGun", "LuaGun"] {
        data.components
            .insert(gun.into(), reflect::ComponentData::default());
    }
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
    w.play();
    for _ in 0..6 {
        w.run_game(1.0 / 60.0);
    }
    let playing = w.playing.as_ref().unwrap();
    let flying = playing.world.projectiles().flying();
    assert_eq!(flying.len(), 12, "two a tick for six ticks");
    // The first pair has flown five ticks at 30 m/s; both colours fly.
    assert!((flying[0].position.y - 2.5).abs() < 1e-3, "{:?}", flying[0]);
    assert!(flying.iter().any(|p| p.colour[1] > 1.0));
    assert!(flying.iter().any(|p| p.colour[0] > 1.0));
    // The Game panel draws them.
    let scene = crate::game_panel::GameScene::of(
        &playing.world,
        &w.models,
        crate::play::PlayState::Playing,
    );
    assert_eq!(scene.bolts.len(), 12);
    // A quarter of a second on, the first have gone: 15 ticks' worth fly.
    for _ in 0..30 {
        w.run_game(1.0 / 60.0);
    }
    assert_eq!(w.playing.as_ref().unwrap().world.projectiles().len(), 30);
    // Stop lets them all go with the world.
    w.stop();
    assert!(w.playing.is_none());
}

#[test]
fn past_the_projectile_limit_the_console_says_so_once() {
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(
        "get \"solis/world\":[fire];\n\
         component Gun {\n\
             has {\n\
                 int shots = 0;\n\
             }\n\
             method update(float dt) {\n\
                 fire([0, 0, 0], [0, 1, 0], 30, 50, [4, 0.2, 0.1]);\n\
             }\n\
         }\n\
         pass Gun;\nend\n",
    )));
    let mut data = EntityData::new("Gun");
    data.components
        .insert("Gun".into(), reflect::ComponentData::default());
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
    w.play();
    // Nearly full already.
    let projectiles = w.playing.as_mut().unwrap().world.projectiles_mut();
    for _ in 0..world::MAX_PROJECTILES - 1 {
        projectiles.fire(world::Shot {
            from: glam::Vec3::ZERO,
            direction: glam::Vec3::Y,
            speed: 1.0,
            life: 50.0,
            colour: [1.0; 3],
        });
    }
    for _ in 0..3 {
        w.run_game(1.0 / 60.0);
    }
    let told: Vec<_> = w
        .log_lines()
        .iter()
        .filter(|l| l.text.contains("projectiles were flying at once"))
        .collect();
    assert_eq!(told.len(), 1, "said once");
    assert!(told[0].text.contains("1,000,000"));
    assert_eq!(
        w.playing.as_ref().unwrap().world.projectiles().len(),
        world::MAX_PROJECTILES
    );
}

/// A workbench knowing a "Drone" assembly (`drones/drone.solasm` in the
/// Library) whose root carries a Lust `Drone` component, playing `main`.
fn with_drones(main: &str) -> Workbench {
    let id = |n| EntityId(reflect::Id::from_raw(n).unwrap());
    let mut area = area::Area::new();
    let mut drone = EntityData::new("Drone").with(&Transform::default());
    drone
        .components
        .insert("Drone".into(), reflect::ComponentData::default());
    area.insert(id(1), drone);
    let file = area::text::AreaFile {
        kind: area::text::FileKind::Assembly { root: id(1) },
        area,
        overrides: Default::default(),
    };
    let asset = reflect::AssetId(reflect::Id::from_raw(901).unwrap());
    let mut w = Workbench::new(info(), None);
    w.set_program(Some(program(main)));
    w.set_assembly(asset, Arc::new(file));
    w.set_library(vec![crate::library::LibraryItem {
        id: Some(asset),
        kind: crate::library::LibraryKind::Assembly,
        name: "drone".into(),
        path: "drones/drone.solasm".into(),
        thumbnail: None,
    }]);
    w
}

/// A hive that spawns three drones on its first tick; each drone climbs
/// and takes itself out after ten ticks.
const HIVE: &str = "get \"solis/world\":[spawn, despawn];\n\
    component Hive {\n\
        has {\n\
            int made = 0;\n\
        }\n\
        method update(float dt) {\n\
            is made == 0? then {\n\
                entity a = spawn(\"drones/drone.solasm\", [0, 0, 1]);\n\
                entity b = spawn(\"drones/drone\", [2, 0, 1]);\n\
                entity c = spawn(\"drone\", [4, 0, 1]);\n\
                entity d = spawn(\"nowhere\", [6, 0, 1]);\n\
                made = 3;\n\
            }\n\
        }\n\
    }\n\
    component Drone {\n\
        has {\n\
            int age = 0;\n\
        }\n\
        method update(float dt) {\n\
            age = age + 1;\n\
            moveTo(me, positionOf(me) + [0, 0, 1]);\n\
            is age >= 10? then {\n\
                despawn(me);\n\
            }\n\
        }\n\
    }\n\
    pass Hive, Drone;\nend\n";

#[test]
fn game_code_spawns_copies_of_assemblies_and_takes_them_out() {
    let mut w = with_drones(HIVE);
    let mut hive = EntityData::new("Hive");
    hive.components
        .insert("Hive".into(), reflect::ComponentData::default());
    w.document
        .edit(
            "Add",
            vec![area::Edit::Spawn {
                entities: vec![(EntityId::random(), hive)],
            }],
            None,
            None,
        )
        .unwrap();
    let steps = w.document.history().undo_len();
    let before = w.document.area().len();
    w.play();
    w.run_game(1.0 / 60.0);
    // Three made (by path, path without extension, and name); the fourth
    // name isn't an assembly, and the Console says so.
    let drones: Vec<EntityId> = w
        .document
        .area()
        .iter()
        .filter(|(_, e)| e.name == "drone")
        .map(|(id, _)| id)
        .collect();
    assert_eq!(drones.len(), 3);
    assert_eq!(w.document.history().undo_len(), steps, "not undo steps");
    assert!(
        w.log_lines()
            .iter()
            .any(|l| l.text.contains("spawned \"nowhere\""))
    );
    // Next frames: they're in the game, and their own code runs.
    for _ in 0..3 {
        w.run_game(1.0 / 60.0);
    }
    let playing = w.playing.as_ref().unwrap();
    for &drone in &drones {
        let z = playing.world.global(drone).unwrap().w_axis.z;
        assert!(z > 2.5, "climbing: {z}");
    }
    // Ten ticks on, each took itself out.
    for _ in 0..10 {
        w.run_game(1.0 / 60.0);
    }
    assert!(drones.iter().all(|&d| w.document.area().get(d).is_none()));
    assert!(
        drones
            .iter()
            .all(|&d| w.playing.as_ref().unwrap().world.global(d).is_none())
    );
    // Stop puts the area back as it was.
    w.stop();
    assert_eq!(w.document.area().len(), before);
}

#[test]
fn spawned_entities_get_the_same_ids_every_time() {
    // The same game twice: the drones get the same IDs (made from where
    // they were spawned in the tick's order).
    let ids = || {
        let mut w = with_drones(HIVE);
        let mut hive = EntityData::new("Hive");
        hive.components
            .insert("Hive".into(), reflect::ComponentData::default());
        let hive_id = EntityId(reflect::Id::from_raw(77).unwrap());
        w.document
            .edit(
                "Add",
                vec![area::Edit::Spawn {
                    entities: vec![(hive_id, hive)],
                }],
                None,
                None,
            )
            .unwrap();
        w.play();
        w.run_game(1.0 / 60.0);
        let mut ids: Vec<EntityId> = w
            .document
            .area()
            .iter()
            .filter(|(_, e)| e.name == "drone")
            .map(|(id, _)| id)
            .collect();
        ids.sort();
        ids
    };
    assert_eq!(ids(), ids());
}

#[test]
fn spawning_too_much_in_one_frame_is_refused_and_said_once() {
    let mut w = with_drones(
        "get \"solis/world\":[spawn];\n\
         component Hive {\n\
             has {\n\
                 int made = 0;\n\
             }\n\
             method update(float dt) {\n\
                 is made == 0? then {\n\
                     loop 10010 {\n\
                         entity e = spawn(\"drone\", [0, 0, 1]);\n\
                     }\n\
                     made = 1;\n\
                 }\n\
             }\n\
         }\n\
         component Drone {\n\
             has {\n\
                 int age = 0;\n\
             }\n\
         }\n\
         pass Hive, Drone;\nend\n",
    );
    let mut hive = EntityData::new("Hive");
    hive.components
        .insert("Hive".into(), reflect::ComponentData::default());
    w.document
        .edit(
            "Add",
            vec![area::Edit::Spawn {
                entities: vec![(EntityId::random(), hive)],
            }],
            None,
            None,
        )
        .unwrap();
    let before = w.document.area().len();
    w.play();
    w.run_game(1.0 / 60.0);
    w.run_game(1.0 / 60.0);
    assert_eq!(
        w.document.area().len(),
        before + crate::play::MAX_SPAWNS_PER_TICK
    );
    let told = w
        .log_lines()
        .iter()
        .filter(|l| l.text.contains("weren't made"))
        .count();
    assert_eq!(told, 1);
}

#[test]
fn spawned_entities_join_on_the_next_tick_even_within_one_frame() {
    // A slow frame runs several ticks; drones spawned on the first run
    // their own code on every tick after it, without waiting for the
    // next frame (C3 of the Phase 5 review).
    let mut w = with_drones(HIVE);
    let mut hive = EntityData::new("Hive");
    hive.components
        .insert("Hive".into(), reflect::ComponentData::default());
    w.document
        .edit(
            "Add",
            vec![area::Edit::Spawn {
                entities: vec![(EntityId::random(), hive)],
            }],
            None,
            None,
        )
        .unwrap();
    let last = w.document.area().children(None).last().copied();
    w.play();
    w.run_game((5.5 * world::TICK) as f32);
    let playing = w.playing.as_ref().unwrap();
    let drones: Vec<EntityId> = w
        .document
        .area()
        .children(None)
        .into_iter()
        .filter(|&id| w.document.area().get(id).is_some_and(|e| e.name == "drone"))
        .collect();
    assert_eq!(drones.len(), 3);
    for &drone in &drones {
        let z = playing.world.global(drone).unwrap().w_axis.z;
        assert!((z - 5.0).abs() < 1e-4, "four ticks of climbing: {z}");
    }
    // They join at the end of the area's tree, in the order spawned.
    let roots = w.document.area().children(None);
    let at = roots.iter().position(|&r| Some(r) == last).unwrap();
    assert_eq!(&roots[at + 1..], &drones[..]);
    let xs: Vec<f32> = drones
        .iter()
        .map(|&d| playing.world.global(d).unwrap().w_axis.x)
        .collect();
    assert_eq!(xs, vec![0.0, 2.0, 4.0]);
}

#[test]
fn step_runs_one_tick_while_paused_and_only_then() {
    let (mut w, id) = with_a_mover();
    w.play();
    w.run_game(0.1);
    w.step();
    assert!(
        w.log_lines().last().unwrap().text.contains("Pause first"),
        "only while paused"
    );
    w.pause();
    let ticks = w.world().unwrap().clock().ticks;
    let x = x_in_game(&w, id);
    w.step();
    w.run_game(0.5);
    assert_eq!(w.world().unwrap().clock().ticks, ticks + 1, "exactly one");
    assert!(x_in_game(&w, id) > x, "game code ran in it");
    w.run_game(0.5);
    assert_eq!(w.world().unwrap().clock().ticks, ticks + 1, "still paused");
    assert_eq!(w.play_state(), crate::play::PlayState::Paused);
}

#[test]
fn any_frame_rate_gives_the_same_world_at_the_same_tick() {
    // 30, 60 and 144 frames a second, run to the same tick: the same
    // place, to the bit (the simulation's step doesn't follow the frames).
    let at_tick = |fps: f64| {
        let (mut w, id) = with_a_mover();
        w.play();
        while w.world().unwrap().clock().ticks < 120 {
            let left = 120 - w.world().unwrap().clock().ticks;
            // The last frame lands on tick 120 exactly.
            let seconds = (1.0 / fps).min(left as f64 * world::TICK + 1e-9);
            w.run_game(seconds as f32);
        }
        (
            w.world().unwrap().clock().ticks,
            x_in_game(&w, id).to_bits(),
        )
    };
    let sixty = at_tick(60.0);
    assert_eq!(sixty.0, 120);
    assert_eq!(at_tick(30.0), sixty);
    assert_eq!(at_tick(144.0), sixty);
}

#[test]
fn the_game_drawn_between_ticks_lags_where_it_is_by_part_of_a_tick() {
    let (mut w, id) = with_a_mover();
    w.play();
    // Half a tick past a tick: drawn about half way from the last tick.
    w.run_game((world::TICK * 3.5) as f32);
    let world = w.world().unwrap();
    let t = world.progress();
    assert!((t - 0.5).abs() < 0.05, "{t}");
    let now = x_in_game(&w, id);
    let drawn_at = |t: f32| {
        world
            .placed_between::<Transform>(t)
            .into_iter()
            .find(|(e, _, _)| *e == id)
            .unwrap()
            .2
            .w_axis
            .x
    };
    let between = drawn_at(t);
    assert!(between < now, "behind where it is now: {between} {now}");
    assert!(between > drawn_at(0.0), "ahead of the last tick");
    assert_eq!(drawn_at(1.0), now);
}
