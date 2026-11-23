//! Field hints from a whole game's code (decision 0127): Lust's comments,
//! Lua's hints tables and the project's names as kinds, through
//! [`crate::compile`], as the editor gets them.

use crate::project_names::Names;
use reflect::{FieldKind, NameKind, Offer, Value};

/// The game with these Lust (`main.lst`) and Lua files, and the engine's
/// files for a project with a few sounds and modules.
fn compiled(main: &str, lua: &[(&str, &str)]) -> crate::Compiled {
    let names = Names::with_modules(&["coin", "ships/fighter"])
        .and_sounds(&["sfx/alert", "sfx/step"])
        .and_levels(&["town"]);
    let (mut files, _) = crate::describe::descriptions_for("Test 1.0", "solis", &names);
    if !main.is_empty() {
        files.push(("main.lst".into(), main.into()));
    }
    let code = crate::Code {
        files,
        lua: lua
            .iter()
            .map(|(p, t)| ((*p).to_string(), (*t).to_string()))
            .collect(),
        notes: Vec::new(),
    };
    match crate::compile(&code) {
        Ok(Some(compiled)) => compiled,
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{problems:?}"),
    }
}

const GUARD: &str = "fetch \"solis/project\":[Sound, Module, Level];\n\
\n\
component Guard {\n\
    has {\n\
        float speed = 4.0; -- [range 0 10, step 0.5, unit \"m/s\", tip \"Walking speed\"]\n\
        float sightRange = 8.0; -- [unit \"m\", group \"Seeing\"]\n\
        Sound alertSound = Sound.Sfx.Alert;\n\
        Module drops = Module.Coin; -- [tip \"Left behind\"]\n\
        Level home = Level.Town; -- [range 0 1]\n\
        string notes = \"\"; -- [multiline, hidden]\n\
    }\n\
}\n\
pass Guard;\n\
end\n";

#[test]
fn lust_hints_and_pickers_reach_the_schema() {
    let compiled = compiled(GUARD, &[]);
    let guard = compiled.schemas.iter().find(|s| s.name == "Guard").unwrap();
    let speed = &guard.field("speed").unwrap().hint;
    assert_eq!(speed.range, Some((0.0, 10.0)));
    assert!(speed.slider);
    assert_eq!(speed.step, Some(0.5));
    assert_eq!(speed.unit, " m/s");
    assert_eq!(speed.tip, "Walking speed");
    assert_eq!(guard.field("sightRange").unwrap().hint.group, "Seeing");
    let alert = guard.field("alertSound").unwrap();
    assert_eq!(alert.hint.offers, Offer::Names(NameKind::Sound));
    assert!(matches!(&alert.kind, FieldKind::Choice(o) if o.contains(&"Sfx.Alert".to_string())));
    assert_eq!(
        guard.field("drops").unwrap().hint.offers,
        Offer::Names(NameKind::Module)
    );
    assert_eq!(
        guard.field("home").unwrap().hint.offers,
        Offer::Names(NameKind::Level)
    );
    let notes = &guard.field("notes").unwrap().hint;
    assert!(notes.multiline && notes.hidden);
    // A range on a level is a mistake with its line; the rest runs.
    assert_eq!(
        compiled.hint_problems.len(),
        1,
        "{:?}",
        compiled.hint_problems
    );
    let problem = &compiled.hint_problems[0];
    assert_eq!((problem.file.as_str(), problem.line), ("main.lst", 9));
    assert!(
        problem.message.contains("range is for numbers"),
        "{problem}"
    );
}

#[test]
fn hints_change_with_new_code() {
    let before = compiled(GUARD, &[]);
    let after = compiled(&GUARD.replace("range 0 10, step 0.5", "range 5 9"), &[]);
    let speed = |c: &crate::Compiled| {
        c.schemas
            .iter()
            .find(|s| s.name == "Guard")
            .and_then(|s| s.field("speed"))
            .map(|f| (f.hint.range, f.hint.step))
    };
    assert_eq!(speed(&before), Some((Some((0.0, 10.0)), Some(0.5))));
    assert_eq!(speed(&after), Some((Some((5.0, 9.0)), None)));
    assert!(
        after.hint_problems.iter().any(|p| p
            .message
            .contains("speed starts outside its range (5 to 9)")),
        "{:?}",
        after.hint_problems
    );
}

#[test]
fn lua_hints_are_a_fifth_entry_and_the_project_s_names_are_kinds() {
    let lua = "local Guard = component(\"Guard\", {\n\
        { \"speed\", \"float\", 4.0, nil, { range = { 0, 10 }, step = 0.5, unit = \"m/s\", tip = \"Walking speed\" } },\n\
        { \"alertSound\", \"Sound\", \"Sfx.Alert\" },\n\
        { \"says\", \"string\", \"\", nil, { multiline = true, colour = 3 } },\n\
    })\n\
    function Guard:update(dt) end\n";
    let compiled = compiled("", &[("guard.lua", lua)]);
    let guard = compiled.schemas.iter().find(|s| s.name == "Guard").unwrap();
    let speed = &guard.field("speed").unwrap().hint;
    assert_eq!((speed.range, speed.step), (Some((0.0, 10.0)), Some(0.5)));
    assert_eq!((&*speed.unit, &*speed.tip), (" m/s", "Walking speed"));
    let alert = guard.field("alertSound").unwrap();
    assert_eq!(alert.hint.offers, Offer::Names(NameKind::Sound));
    assert_eq!(alert.default, Value::Text("Sfx.Alert".into()));
    assert!(matches!(&alert.kind, FieldKind::Choice(o) if o.contains(&"Sfx.Step".to_string())));
    assert!(guard.field("says").unwrap().hint.multiline);
    assert_eq!(
        compiled.hint_problems.len(),
        1,
        "{:?}",
        compiled.hint_problems
    );
    let problem = &compiled.hint_problems[0];
    assert_eq!((problem.file.as_str(), problem.line), ("guard.lua", 4));
    assert!(
        problem.message.contains("colour isn't a hint Solis knows"),
        "{problem}"
    );
}

#[test]
fn rust_hints_are_the_schema_s_own() {
    // A Rust component writes its hints into its schema: the same fields
    // the others' are read into.
    let field = reflect::FieldInfo::new("speed", "Speed", FieldKind::Float, Value::Float(4.0))
        .hint(reflect::Hint {
            range: Some((0.0, 10.0)),
            slider: true,
            step: Some(0.5),
            unit: " m/s".into(),
            tip: "Walking speed".into(),
            ..reflect::Hint::default()
        });
    assert_eq!(crate::field_hints::outside_range(&field), None);
    assert!(field.hint.slider);
}
