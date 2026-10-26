//! Events between Lust and Rust components.

use super::*;
use crate::events::{Event, MAX_DELIVERIES};
use crate::host::NoHost;
use crate::rust_component::{Context, RustComponent};
use reflect::Value;
use reflect::{FieldInfo, FieldKind};

/// Says go into a list, so tests can see the order things happened in.
#[derive(Default)]
struct Log(Vec<String>);

impl Host for Log {
    fn say(&mut self, words: &str) {
        self.0.push(words.to_string());
    }
    fn time(&self) -> f64 {
        0.0
    }
    fn find(&self, _name: &str) -> Option<EntityId> {
        None
    }
    fn position(&self, _which: EntityId) -> Option<glam::Vec3> {
        None
    }
    fn move_to(&mut self, _which: EntityId, _place: glam::Vec3) {}
    fn rotation(&self, _which: EntityId) -> Option<glam::Quat> {
        None
    }
    fn turn_to(&mut self, _which: EntityId, _facing: glam::Quat) {}
    fn name(&self, _which: EntityId) -> Option<String> {
        None
    }
}

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

fn loaded(main: &str) -> Runtime {
    let mut files = crate::describe::descriptions("Test 1.0", "solis");
    files.push((
        "main.lst".into(),
        format!("get \"solis/world\":[say];\n{main}"),
    ));
    let chunk = match lust::to_lua("main.lst", &files) {
        Ok(chunk) => crate::code::Compiled {
            lust: Some(chunk),
            lua: Vec::new(),
            schemas: Vec::new(),
            places: Default::default(),
            bytecode: Default::default(),
        },
        Err(problems) => panic!("{}", problems[0]),
    };
    let mut runtime = Runtime::new().unwrap();
    runtime.add_rust(armour());
    runtime.load(&chunk, &mut NoHost).unwrap();
    runtime
}

fn run(runtime: &mut Runtime, entity: EntityId, name: &str) {
    runtime
        .sync(entity, name, &ComponentData::default(), &mut NoHost)
        .unwrap();
}

/// A Rust component standing in for a game's: it soaks up hits, says so,
/// and tells the entity when it breaks.
struct Armour {
    plates: i64,
}

impl RustComponent for Armour {
    fn handle(&mut self, cx: &mut Context<'_>, event: &Event) -> bool {
        if event.name != "Hit" {
            return false;
        }
        if self.plates == 99 {
            panic!("the armour fell apart");
        }
        self.plates -= 1;
        let me = cx.me();
        cx.host().say(&format!("armour has {} plates", self.plates));
        if self.plates == 0 {
            cx.send(Some(me), "Broken", vec![Value::Int(3)]);
        }
        true
    }
    fn values(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("plates", Value::Int(self.plates));
        data
    }
    fn set(&mut self, data: &ComponentData) {
        if let Some(Value::Int(n)) = data.get("plates") {
            self.plates = *n;
        }
    }
}

fn armour() -> RustKind {
    RustKind {
        schema: ComponentSchema {
            name: "Armour".into(),
            help: "Soaks up hits.".into(),
            fields: vec![FieldInfo::new(
                "plates",
                "Plates",
                FieldKind::Int,
                Value::Int(2),
            )],
        },
        make: |data| {
            let mut armour = Armour { plates: 2 };
            armour.set(data);
            Box::new(armour)
        },
    }
}

const GAME: &str = "event Hit(entity by, float damage);\n\
    event Broken(int pieces);\n\
    component Gun {\n\
        has {\n\
            entity target = nothing;\n\
            int shots = 0;\n\
        }\n\
        method update(float dt) {\n\
            is shots < 2? then {\n\
                shots = shots + 1;\n\
                say(\"bang\");\n\
                send Hit(me, 2.5) to target;\n\
            }\n\
        }\n\
    }\n\
    component Health {\n\
        has {\n\
            float hp = 10;\n\
        }\n\
        on Hit(entity by, float damage) {\n\
            hp = hp - damage;\n\
            say(\"hp \" + hp.text());\n\
        }\n\
        on Broken(int pieces) {\n\
            say(\"broken into \" + pieces.text());\n\
        }\n\
    }\n\
    pass Gun, Health;\nend\n";

#[test]
fn events_reach_every_component_on_the_entity_in_name_order_lust_and_rust_alike() {
    let mut runtime = loaded(GAME);
    let (gun, target) = (id(1), id(2));
    let mut aimed = ComponentData::default();
    aimed.set("target", Value::Entity(Some(target)));
    runtime.sync(gun, "Gun", &aimed, &mut NoHost).unwrap();
    run(&mut runtime, target, "Health");
    run(&mut runtime, target, "Armour");
    let mut log = Log::default();
    assert!(runtime.tick(0.1, &mut log).failures.is_empty());
    // After the updates; Armour before Health (by name).
    assert_eq!(log.0, ["bang", "armour has 1 plates", "hp 7.5"]);
    log.0.clear();
    assert!(runtime.tick(0.1, &mut log).failures.is_empty());
    // Armour breaks and sends Broken, which joins the end of the same
    // tick's queue: after Health has taken the Hit.
    assert_eq!(
        log.0,
        ["bang", "armour has 0 plates", "hp 5.0", "broken into 3"]
    );
    assert_eq!(
        runtime.values(target, "Armour").unwrap().get("plates"),
        Some(&Value::Int(0))
    );
    assert!(runtime.has_component("Armour"));
}

#[test]
fn a_failing_rust_component_stops_alone_and_wrong_values_leave_the_handler_running() {
    let mut runtime = loaded(GAME);
    let (gun, target) = (id(1), id(2));
    let mut aimed = ComponentData::default();
    aimed.set("target", Value::Entity(Some(target)));
    runtime.sync(gun, "Gun", &aimed, &mut NoHost).unwrap();
    let mut crumbling = ComponentData::default();
    crumbling.set("plates", Value::Int(99));
    runtime
        .sync(target, "Armour", &crumbling, &mut NoHost)
        .unwrap();
    run(&mut runtime, target, "Health");
    let mut log = Log::default();
    let failures = runtime.tick(0.1, &mut log).failures;
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert_eq!(failures[0].component, "Armour");
    assert!(failures[0].at.message.contains("fell apart"));
    assert!(
        log.0.contains(&"hp 7.5".to_string()),
        "Health still took it"
    );
    // A Rust component sending an event with the wrong values: the Lust
    // handler reports it and keeps running.
    runtime
        .queue
        .borrow_mut()
        .push(Event {
            to: target,
            name: "Hit".into(),
            values: vec![Value::Text("oops".into())],
        })
        .unwrap();
    let failures = runtime.tick(0.1, &mut log).failures;
    let wrong: Vec<_> = failures
        .iter()
        .filter(|f| f.component == "Health")
        .collect();
    assert_eq!(wrong.len(), 1, "{failures:?}");
    assert!(!wrong[0].stopped);
    assert!(wrong[0].to_string().contains("couldn't take the event Hit"));
}

#[test]
fn components_that_keep_sending_to_each_other_are_cut_off() {
    let mut runtime = loaded(
        "event Ping();\n\
         component Echo {\n\
             has {\n\
                 int heard = 0;\n\
                 bool started = false;\n\
             }\n\
             method update(float dt) {\n\
                 is not started? then {\n\
                     started = true;\n\
                     send Ping() to me;\n\
                 }\n\
             }\n\
             on Ping() {\n\
                 heard = heard + 1;\n\
                 send Ping() to me;\n\
             }\n\
         }\n\
         pass Echo;\nend\n",
    );
    run(&mut runtime, id(3), "Echo");
    let ticked = runtime.tick(0.1, &mut NoHost);
    // Cut off either way: by the count of events, or (on a busy computer,
    // where 10,000 deliveries take longer than a tick may) by the tick's
    // time limit, which pauses the game and says why. The hardening
    // milestone's mutation runs found this test failing under load when
    // it accepted only the first.
    if let Some(why) = &ticked.pause {
        assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
        assert!(!why.is_empty());
        let heard = runtime.values(id(3), "Echo").unwrap().get("heard").cloned();
        assert!(
            matches!(heard, Some(Value::Int(n)) if n > 0 && n <= MAX_DELIVERIES as i64),
            "{heard:?}"
        );
        return;
    }
    let failures = ticked.failures;
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(failures[0].at.message.contains("more than 10000 events"));
    assert_eq!(
        runtime.values(id(3), "Echo").unwrap().get("heard"),
        Some(&Value::Int(MAX_DELIVERIES as i64))
    );
    // The next tick starts with an empty queue.
    assert!(runtime.tick(0.1, &mut NoHost).failures.is_empty());
}
