//! Every engine object within reach of all three languages (the review,
//! BUG-46): Lua's `solis` once offered six of them by a list written by
//! hand, and `solis.Window`, `solis.Screens`, `solis.Maps` and
//! `solis.Animators` stopped the component that asked.

use super::*;
use crate::code::Code;
use crate::describe::NATIVES;
use crate::rust_component::{Context, RustComponent};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

/// One call of each owner in [`NATIVES`] (an engine object, `entity`, or
/// `""` for a method that's nobody's): the owner, the call in Lua, and
/// the call in Lust. Each reaches the [`Asked`] host, which notes the
/// owner (all but nobody's, which only works something out).
const CALLS: &[(&str, &str, &str)] = &[
    (
        "",
        "solis.facingAlong(vec3(1, 0, 0))",
        "rotation facing = facingAlong([1, 0, 0]);",
    ),
    (
        "Animators",
        "solis.Animators:fire(self.me, 'Jump')",
        "Animators:fire(this, \"Jump\");",
    ),
    (
        "Audio",
        "solis.Audio:setVolume('Music', 0.5)",
        "Audio:setVolume(Bus.Music, 0.5);",
    ),
    (
        "Input",
        "solis.Input:held('Jump')",
        "bool held = Input:held(Action.Jump);",
    ),
    (
        "Maps",
        "solis.Maps:width('maps/farm')",
        "int width = Maps:width(\"maps/farm\");",
    ),
    (
        "Physics",
        "solis.Physics:castRay(vec3(0, 0, 0), vec3(1, 0, 0), 5)",
        "bool found = Physics:castRay([0, 0, 0], [1, 0, 0], 5).found;",
    ),
    (
        "Saves",
        "solis.Saves:saveTo('quick')",
        "Saves:saveTo(\"quick\");",
    ),
    (
        "Screens",
        "solis.Screens:setTextSize(1.5)",
        "Screens:setTextSize(1.5);",
    ),
    ("Window", "solis.Window:quit()", "Window:quit();"),
    ("World", "solis.World:say('hello')", "World:say(\"hello\");"),
    (
        "entity",
        "solis.entity.moveTo(self.me, vec3(1, 2, 3))",
        "moveTo([1, 2, 3]);",
    ),
];

/// Every owner a native has, as [`NATIVES`] lists them.
fn owners() -> BTreeSet<&'static str> {
    NATIVES.iter().map(|native| native.owner).collect()
}

/// The owners [`Asked`] should have noted once every call in [`CALLS`]
/// ran: all of them but nobody's, which are the groups Lua's `solis` has.
fn owners_asked() -> BTreeSet<&'static str> {
    crate::describe::owners().into_iter().collect()
}

/// A host that notes whose native reached it.
#[derive(Default)]
struct Asked(RefCell<BTreeSet<&'static str>>);

impl Asked {
    fn note(&self, owner: &'static str) {
        self.0.borrow_mut().insert(owner);
    }
}

impl Host for Asked {
    fn say(&mut self, _words: &str) {
        self.note("World");
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
    fn move_to(&mut self, _which: EntityId, _place: glam::Vec3) {
        self.note("entity");
    }
    fn rotation(&self, _which: EntityId) -> Option<glam::Quat> {
        None
    }
    fn turn_to(&mut self, _which: EntityId, _facing: glam::Quat) {}
    fn name(&self, _which: EntityId) -> Option<String> {
        None
    }
    fn save_game(&mut self, _slot: &str, _load: bool) {
        self.note("Saves");
    }
    fn action(&self, _name: &str) -> input::ActionState {
        self.note("Input");
        input::ActionState::default()
    }
    fn cast_ray(&self, _from: glam::Vec3, _way: glam::Vec3, _reach: f32) -> Option<physics::Hit> {
        self.note("Physics");
        None
    }
    fn set_bus_volume(&mut self, _bus: &str, _share: f32) {
        self.note("Audio");
    }
    fn animator_set(&mut self, _which: EntityId, _name: &str, _value: crate::AnimatorValue) {
        self.note("Animators");
    }
    fn map(&self, _file: &str) -> Option<std::sync::Arc<tiles::TileMap>> {
        self.note("Maps");
        None
    }
    fn set_setting(&mut self, _change: crate::settings::Setting) {
        self.note("Screens");
    }
    fn quit(&mut self) {
        self.note("Window");
    }
}

/// Loads `code`, runs its component `name` for one tick on [`Asked`], and
/// gives back whose natives reached the host.
fn asked_by(code: &Code, name: &str) -> BTreeSet<&'static str> {
    let compiled = match crate::compile(code) {
        Ok(compiled) => compiled.expect("there's code"),
        Err(problems) => panic!("{}", problems[0]),
    };
    let mut runtime = Runtime::new().unwrap();
    let mut host = Asked::default();
    runtime.load(&compiled, &mut host).unwrap();
    runtime
        .sync(id(1), name, &ComponentData::default(), &mut host)
        .unwrap();
    // Only what the component's update asks counts.
    host.0.borrow_mut().clear();
    let ticked = runtime.tick(1.0 / 60.0, &mut host);
    assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
    host.0.into_inner()
}

#[test]
fn there_s_a_call_to_try_for_every_owner_a_native_has() {
    // A new engine object fails here until it has a call in the list, so
    // the tests below can never quietly leave one out.
    let tried: BTreeSet<&str> = CALLS.iter().map(|(owner, ..)| *owner).collect();
    assert_eq!(tried, owners());
    assert_eq!(tried.len(), CALLS.len(), "each owner once");
}

#[test]
fn lua_reaches_every_engine_object() {
    let mut lua =
        String::from("local Caller = component('Caller', {})\nfunction Caller:update(dt)\n");
    for (_, call, _) in CALLS {
        lua.push_str(&format!("  local _ = {call}\n"));
    }
    lua.push_str("end\n");
    let code = Code {
        files: Vec::new(),
        lua: vec![("caller.lua".into(), lua)],
        notes: Vec::new(),
    };
    assert_eq!(asked_by(&code, "Caller"), owners_asked());
}

#[test]
fn lust_reaches_every_engine_object() {
    // Each engine object is fetched from the file that describes it.
    let mut fetched: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for native in NATIVES {
        if !matches!(native.owner, "" | "entity") {
            fetched.entry(native.file).or_default().insert(native.owner);
        }
    }
    let mut main =
        String::from("get \"solis/entity\":facingAlong;\nfetch \"solis/project\":[Action, Bus];\n");
    for (file, objects) in fetched {
        let objects: Vec<&str> = objects.into_iter().collect();
        main.push_str(&format!(
            "fetch \"solis/{file}\":[{}];\n",
            objects.join(", ")
        ));
    }
    main.push_str("component Caller {\n    method update(float dt) {\n");
    for (_, _, call) in CALLS {
        main.push_str(&format!("        {call}\n"));
    }
    main.push_str("    }\n}\npass Caller;\nend\n");
    let mut files = crate::describe::descriptions("Test 1.0", "solis");
    files.push(("main.lst".into(), main));
    let code = Code {
        files,
        lua: Vec::new(),
        notes: Vec::new(),
    };
    assert_eq!(asked_by(&code, "Caller"), owners_asked());
}

#[test]
fn rust_reaches_every_engine_object() {
    // A Rust component has the whole host (`Context::host`): one call for
    // each engine object, as the natives make them.
    struct Caller;
    impl RustComponent for Caller {
        fn update(&mut self, cx: &mut Context<'_>, _dt: f64) {
            let me = cx.me();
            let host = cx.host();
            host.animator_set(me, "Jump", crate::AnimatorValue::Fire);
            host.set_bus_volume("Music", 0.5);
            let _ = host.action("Jump");
            let _ = host.map("maps/farm");
            let _ = host.cast_ray(glam::Vec3::ZERO, glam::Vec3::X, 5.0);
            host.save_game("quick", false);
            host.set_text_size(1.5);
            host.quit();
            host.say("hello");
            host.move_to(me, glam::Vec3::ONE);
            // Nobody's: worked out without the host.
            let _ = crate::host::facing_along(glam::Vec3::X);
        }
        fn values(&self) -> ComponentData {
            ComponentData::default()
        }
        fn set(&mut self, _data: &ComponentData) {}
    }
    let mut runtime = Runtime::new().unwrap();
    runtime.add_rust(RustKind {
        schema: ComponentSchema {
            name: "Caller".into(),
            help: "Calls one native of each engine object.".into(),
            fields: Vec::new(),
        },
        make: |_| Box::new(Caller),
    });
    let mut host = Asked::default();
    runtime
        .sync(id(1), "Caller", &ComponentData::default(), &mut host)
        .unwrap();
    let ticked = runtime.tick(1.0 / 60.0, &mut host);
    assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
    assert_eq!(host.0.into_inner(), owners_asked());
}
