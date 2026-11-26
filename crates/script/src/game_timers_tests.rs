//! A game's timers (stage 1, timers): `TimerFinished` reaches only the
//! component that started the timer, in the tick's first round, in the
//! same order on one core or four; in Lust, Lua and Rust; ended with
//! their component; kept or dropped by new code; saved and loaded.

use super::tests::{TestWorld, id, unhurried};
use super::*;
use crate::code::Code;
use crate::rust_component::{Context, RustComponent, RustKind};
use crate::timers::Timing;
use reflect::{FieldInfo, FieldKind, Value};

/// The timers the code below starts.
const NAMES: &[&str] = &["Opens", "Beat", "Never", "Dawn"];

/// A door that opens in three ticks, beats every two and starts a timer
/// it stops; a gate on the same entity with a timer of the same name; and
/// a rooster crowing at six.
const DOORS: &str = "fetch \"solis/world\":World;\n\
    fetch \"solis/project\":Timer;\n\
    fetch \"solis/made\":[Opens, Beat, Never];\n\
    fetch \"solis/made\":Dawn;\n\
    fetch \"solis/timers\":TimerFinished;\n\
    \n\
    component Door {\n\
    has {\n\
    int ticks = 0;\n\
    }\n\
    method update(float dt) {\n\
    ticks = ticks + 1;\n\
    is ticks == 1? then {\n\
    new Opens { after = 0.05 };\n\
    new Beat { every = 2.0 / 60.0 };\n\
    new Never { after = 1.0 };\n\
    }\n\
    is ticks == 2? then {\n\
    stopTimer(Timer.Never);\n\
    int left = (timeLeft(Timer.Opens) * 60 + 0.5).int();\n\
    World:say(name() + \" Opens has \" + left.text() + \" ticks left\");\n\
    }\n\
    is ticks == 3 && not timerRunning(Timer.Never)? then {\n\
    World:say(name() + \" stopped Never\");\n\
    }\n\
    }\n\
    on {\n\
    \x20   TimerFinished(Timer which) {\n\
    \x20   is which == Timer.Opens? then {\n\
    \x20   World:say(name() + \" Door opens on \" + ticks.text());\n\
    \x20   }\n\
    \x20   is which == Timer.Beat? then {\n\
    \x20   World:say(name() + \" Door beats on \" + ticks.text());\n\
    \x20   }\n\
    \x20   is which == Timer.Never? then {\n\
    \x20   World:say(\"never\");\n\
    \x20   }\n\
    \x20   }\n\
    }\n\
    }\n\
    component Gate {\n\
    has {\n\
    bool started = false;\n\
    }\n\
    method update(float dt) {\n\
    is not started? then {\n\
    started = true;\n\
    new Opens { after = 0.05 };\n\
    }\n\
    }\n\
    on {\n\
    \x20   TimerFinished(Timer which) {\n\
    \x20   World:say(name() + \" Gate hears its own timer\");\n\
    \x20   }\n\
    }\n\
    }\n\
    component Rooster {\n\
    has {\n\
    bool started = false;\n\
    }\n\
    method update(float dt) {\n\
    is not started? then {\n\
    started = true;\n\
    new Dawn { atHour = 6.0 };\n\
    }\n\
    }\n\
    on {\n\
    \x20   TimerFinished(Timer which) {\n\
    \x20   World:say(name() + \" crows at dawn\");\n\
    \x20   }\n\
    }\n\
    }\n\
    pass Door, Gate, Rooster;\n\
    end\n\
    ";

/// A bell in Lua, ringing a tick after it starts.
const BELL: &str = "local Bell = component(\"Bell\", { { \"started\", \"bool\", false } })\n\
    function Bell:update(dt)\n\
        if not self.started then\n\
            self.started = true\n\
            solis.entity.startTimer(self.me, \"Ring\", 0)\n\
        end\n\
    end\n\
    function Bell:on_TimerFinished(which)\n\
        solis.World:say(solis.entity.name(self.me) .. \" bell \" .. which)\n\
    end\n";

/// A clock in Rust, ticking every three ticks.
struct Clock {
    started: bool,
}

impl RustComponent for Clock {
    fn update(&mut self, cx: &mut Context<'_>, _dt: f64) {
        if !std::mem::replace(&mut self.started, true) {
            let me = cx.me();
            let started = cx.host().start_timer(me, "Tock", Timing::Every(0.05));
            assert_eq!(started, Ok(()));
        }
    }
    fn handle(&mut self, cx: &mut Context<'_>, event: &crate::Event) -> bool {
        if event.name != "TimerFinished" {
            return false;
        }
        let me = cx.me();
        let name = cx.host().name(me).unwrap_or_default();
        cx.host().say(&format!("{name} clock {:?}", event.values));
        true
    }
    fn values(&self) -> ComponentData {
        ComponentData::default()
    }
    fn set(&mut self, _data: &ComponentData) {}
}

fn clock() -> RustKind {
    RustKind {
        schema: ComponentSchema {
            name: "Clock".into(),
            help: "Ticks.".into(),
            fields: vec![FieldInfo::new(
                "on",
                "On",
                FieldKind::Bool,
                Value::Bool(true),
            )],
        },
        make: |_| Box::new(Clock { started: false }),
    }
}

fn compile_with(main: &str, names: &[&str]) -> Result<Arc<Compiled>, Vec<crate::CodeProblem>> {
    let (mut files, _) = crate::describe::descriptions_for(
        "Test 1.0",
        "solis",
        &crate::project_names::Names::with_modules(&[]).and_timers(names),
    );
    files.push(("main.lst".into(), main.to_string()));
    let code = Code {
        files,
        lua: vec![("bell.lua".into(), BELL.to_string())],
        notes: Vec::new(),
    };
    crate::code::compile_within(&code, crate::luau::Limits::unhurried())
        .map(|c| Arc::new(c.expect("there's code")))
}

/// A world of four entities, A to D: A has a door and a gate, B a door,
/// C a rooster, a bell and a clock, D a door.
fn started(shards: usize) -> (Game, TestWorld) {
    let mut world = TestWorld::with(&["A", "B", "C", "D"]);
    let mut game = unhurried(shards);
    game.add_rust(clock());
    let code = compile_with(DOORS, NAMES).unwrap();
    // The names the code starts, gathered from it, sorted.
    assert_eq!(code.timers, ["Beat", "Dawn", "Never", "Opens"]);
    game.load(&code, &mut world).unwrap();
    let empty = ComponentData::default();
    let components = vec![
        (id(1), "Door", &empty),
        (id(1), "Gate", &empty),
        (id(2), "Door", &empty),
        (id(3), "Rooster", &empty),
        (id(3), "Bell", &empty),
        (id(3), "Clock", &empty),
        (id(4), "Door", &empty),
    ];
    assert!(game.sync_many(components, &mut world).is_empty());
    (game, world)
}

/// Runs `ticks` ticks with the day at `hours` (one a tick, the last held).
fn run(game: &mut Game, world: &mut TestWorld, ticks: usize, hours: &[f64]) {
    for tick in 0..ticks {
        world.hour = hours.get(tick).or(hours.last()).copied();
        let ticked = game.tick(1.0 / 60.0, world);
        assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
    }
}

#[test]
fn timers_finish_in_order_for_the_component_that_started_them() {
    let (mut game, mut world) = started(1);
    run(&mut game, &mut world, 7, &[5.0, 5.5, 6.5]);
    let said = world.said.clone();
    assert_eq!(
        said,
        [
            // Tick 2: the updates first, asking as the tick started; then
            // the bell's 0-second timer, a tick on.
            "A Opens has 2 ticks left",
            "B Opens has 2 ticks left",
            "D Opens has 2 ticks left",
            "C bell Ring",
            // Tick 3: the beats every two ticks, and the dawn the clock
            // passed, in entity order.
            "A stopped Never",
            "B stopped Never",
            "D stopped Never",
            "A Door beats on 3",
            "B Door beats on 3",
            "C crows at dawn",
            "D Door beats on 3",
            // Tick 4: the doors open and the gate hears its own, in
            // entity then component order, with the clock's first tock.
            "A Door opens on 4",
            "A Gate hears its own timer",
            "B Door opens on 4",
            "C clock [Text(\"Tock\")]",
            "D Door opens on 4",
            "A Door beats on 5",
            "B Door beats on 5",
            "D Door beats on 5",
            "A Door beats on 7",
            "B Door beats on 7",
            "C clock [Text(\"Tock\")]",
            "D Door beats on 7",
        ]
    );
    // The same on four cores, checksum and all.
    let (mut four, mut world_four) = started(4);
    run(&mut four, &mut world_four, 7, &[5.0, 5.5, 6.5]);
    assert_eq!(world_four.said, said);
    assert_eq!(four.checksum(), game.checksum());
    assert_eq!(four.timers(), game.timers());
}

#[test]
fn a_timer_made_where_no_entity_s_code_runs_is_a_mistake() {
    // A timer is its entity's: made in a game-wide object's update, it
    // has none, and says so on its line.
    let clock = DOORS.replace(
        "component Rooster {",
        "object Clock {\n    method update(float dt) {\n        new Never { after = 1.0 };\n    }\n}\npass Clock;\ncomponent Rooster {",
    );
    let mut world = TestWorld::with(&["A", "B"]);
    let mut game = unhurried(1);
    game.load(&compile_with(&clock, NAMES).unwrap(), &mut world)
        .unwrap();
    let ticked = game.tick(1.0 / 60.0, &mut world);
    let message = format!("{:?}", ticked.failures);
    assert!(
        message.contains("new Never: a timer belongs to an entity"),
        "{message}"
    );
}

#[test]
fn a_misspelt_timer_fails_the_check_before_anything_runs() {
    let misspelt = DOORS.replace("stopTimer(Timer.Never)", "stopTimer(Timer.Nevr)");
    let Err(problems) = compile_with(&misspelt, NAMES) else {
        panic!("a misspelt timer is a mistake");
    };
    assert!(problems[0].to_string().contains("Nevr"), "{}", problems[0]);
}

#[test]
fn a_despawned_entity_or_a_component_taken_off_ends_its_timers() {
    let (mut game, mut world) = started(2);
    run(&mut game, &mut world, 1, &[5.0]);
    // B despawned; A's gate taken off.
    game.retain_on(vec![(id(2), Vec::new()), (id(1), vec!["Door".to_string()])]);
    run(&mut game, &mut world, 4, &[5.0]);
    let said = world.said.join("\n");
    assert!(!said.contains("B Door"), "{said}");
    assert!(!said.contains("Gate"), "{said}");
    assert!(said.contains("A Door opens on 4"), "{said}");
    // Every component gone: none left waiting.
    game.retain_only(std::collections::BTreeSet::new());
    assert!(game.timers().is_empty());
}

#[test]
fn new_code_keeps_a_timer_while_its_component_and_name_exist() {
    let (mut game, mut world) = started(1);
    run(&mut game, &mut world, 1, &[5.0]);
    // The rooster is gone from the code, and Never is called Later now;
    // the door no longer starts Opens, but the gate still does.
    let new = DOORS
        .replace("Never", "Later")
        .replace("new Opens { after = 0.05 };\nnew Beat", "new Beat")
        .replace("new Dawn { atHour = 6.0 };", "")
        .replace("fetch \"solis/made\":Dawn;\n", "")
        .replace("component Rooster", "component Hen")
        .replace("pass Door, Gate, Rooster;", "pass Door, Gate, Hen;");
    let code = compile_with(&new, &["Beat", "Later", "Opens"]).unwrap();
    let swapped = game.swap(&code, &mut world).unwrap();
    let notes: Vec<&String> = swapped
        .notes
        .iter()
        .filter(|n| n.contains("timer"))
        .collect();
    assert_eq!(notes.len(), 1, "one note: {notes:?}");
    assert_eq!(
        notes[0],
        "The new code no longer has 4 timers, so they stopped: Door's Never (3), Rooster's Dawn."
    );
    assert!(game.timers().running(id(1), "Door", "Opens"));
    assert!(game.timers().running(id(1), "Door", "Beat"));
    assert!(!game.timers().running(id(1), "Door", "Never"));
    assert!(!game.timers().running(id(3), "Rooster", "Dawn"));
    // Lua's and Rust's are kept while their component is.
    assert!(game.timers().running(id(3), "Clock", "Tock"));
}

#[test]
fn a_saved_game_s_timers_fire_on_the_tick_they_would_have() {
    let (mut game, mut world) = started(1);
    run(&mut game, &mut world, 2, &[5.0]);
    let text = game.timers().write();
    let checksum = game.checksum();
    // Carried on…
    world.said.clear();
    run(&mut game, &mut world, 4, &[5.0, 6.5]);
    let carried = world.said.clone();
    assert!(carried.len() > 5, "{carried:?}");
    // …and loaded from the save, with the components' values.
    let (mut loaded, mut loaded_world) = started(1);
    run(&mut loaded, &mut loaded_world, 2, &[5.0]);
    loaded.set_timers(crate::Timers::read(&text).unwrap());
    assert_eq!(loaded.checksum(), checksum);
    loaded_world.said.clear();
    run(&mut loaded, &mut loaded_world, 4, &[5.0, 6.5]);
    assert_eq!(loaded_world.said, carried);
}

#[test]
fn a_game_without_timers_sums_as_before() {
    let mut world = TestWorld::with(&["A"]);
    let mut game = unhurried(1);
    let code = compile_with(
        "component Quiet {\n    method update(float dt) {\n    }\n}\npass Quiet;\nend\n",
        &[],
    )
    .unwrap();
    game.load(&code, &mut world).unwrap();
    let before = game.checksum();
    run(&mut game, &mut world, 3, &[8.0]);
    assert_eq!(game.checksum(), before);
    assert!(game.timers().is_empty());
}
