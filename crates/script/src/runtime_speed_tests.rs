//! The speed fixes of the Phase 4 review keep behaving the same: syncing
//! in batches, nothing done for nothing changed, and code compiled on the
//! worker still saying where it failed.

use super::*;
use crate::code::Code;
use crate::host::NoHost;
use reflect::Value;
use std::cell::Cell;

fn id(n: u64) -> EntityId {
    EntityId(reflect::Id::from_raw(n).unwrap())
}

thread_local! {
    static SETS: Cell<u32> = const { Cell::new(0) };
}

/// Counts the changes it's given.
struct Counted;

impl crate::RustComponent for Counted {
    fn values(&self) -> ComponentData {
        ComponentData::default()
    }
    fn set(&mut self, _data: &ComponentData) {
        SETS.with(|s| s.set(s.get() + 1));
    }
}

#[test]
fn syncing_the_same_values_again_does_nothing_and_batches_stop_what_s_gone() {
    let mut runtime = Runtime::new().unwrap();
    runtime.add_rust(RustKind {
        schema: reflect::ComponentSchema {
            name: "Counted".into(),
            help: String::new(),
            fields: vec![reflect::FieldInfo::new(
                "n",
                "N",
                reflect::FieldKind::Int,
                Value::Int(0),
            )],
        },
        make: |_| Box::new(Counted),
    });
    let mut data = ComponentData::default();
    data.set("n", Value::Int(1));
    let all: Vec<_> = (1..=3).map(|n| (id(n), "Counted", &data)).collect();
    assert!(runtime.sync_many(all.clone(), &mut NoHost).is_empty());
    assert_eq!(runtime.len(), 3);
    assert!(runtime.sync_many(all, &mut NoHost).is_empty());
    assert_eq!(SETS.with(Cell::get), 0, "the same values: no call");
    let mut more = ComponentData::default();
    more.set("n", Value::Int(2));
    runtime.sync(id(2), "Counted", &more, &mut NoHost).unwrap();
    assert_eq!(SETS.with(Cell::get), 1);
    runtime.retain_on(id(2), |_| false);
    assert_eq!(runtime.len(), 2);
    assert!(runtime.values(id(2), "Counted").is_none());
    assert!(runtime.values(id(3), "Counted").is_some());
}

#[test]
fn code_compiled_on_the_worker_still_says_its_file_and_line() {
    let code = Code {
        files: Vec::new(),
        lua: vec![(
            "parts/broken.lua".into(),
            "local B = component('Broken', {})\nfunction B:update(dt)\n  local x = nil + 1\nend\n"
                .into(),
        )],
        notes: Vec::new(),
    };
    let compiled = crate::compile(&code).unwrap().unwrap();
    assert_eq!(compiled.bytecode.lua.len(), 1, "compiled on the worker");
    let mut runtime = Runtime::new().unwrap();
    runtime.load(&compiled, &mut NoHost).unwrap();
    runtime
        .sync(id(4), "Broken", &ComponentData::default(), &mut NoHost)
        .unwrap();
    let failures = runtime.tick(0.1, &mut NoHost).failures;
    assert_eq!(failures.len(), 1);
    assert_eq!(
        (failures[0].at.file.as_str(), failures[0].at.line),
        ("parts/broken.lua", 3)
    );
}
