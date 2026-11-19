//! States' small parts: Lust's rule for what a state ends, Lust's runtime
//! told about states, and which state each component has of its own.

use super::*;
use crate::code::Code;

#[test]
fn options_inside_options_count_as_lust_says() {
    // The same option, or one inside the other, either way round.
    assert!(still_in("Combat", "Combat"));
    assert!(still_in("Combat.Melee", "Combat"));
    assert!(still_in("Combat", "Combat.Melee"));
    // Another option, a sibling, or one that only starts the same.
    assert!(!still_in("Calm", "Combat"));
    assert!(!still_in("Combat.Ranged", "Combat.Melee"));
    assert!(!still_in("CombatOver", "Combat"));
    assert!(!still_in("Combat", "CombatOver"));
    assert!(!still_in("", "Combat"));
}

/// Lust's chunk for `main`, as Lust itself makes it.
fn chunk(main: &str) -> lust::LuaChunk {
    let (mut files, _) = crate::describe::descriptions_for(
        "Test 1.0",
        "solis",
        &crate::project_names::Names::standard(),
    );
    files.push(("main.lst".into(), main.to_string()));
    let engine = lust::Engine {
        folders: crate::describe::engine_folders(),
    };
    lust::to_lua_with(&engine, "main.lst", &files).unwrap()
}

const HERO: &str = "state Mood = [Calm, Stunned];\n\
    component Hero {\n\
        has {\n\
            state = Mood.Calm;\n\
        }\n\
    }\n\
    pass Hero;\n\
    end\n";

#[test]
fn the_runtime_is_told_about_states_without_moving_a_line() {
    let before = chunk(HERO);
    let after = own_states(before.clone()).unwrap();
    assert_eq!(before.code.lines().count(), after.code.lines().count());
    for (_, to) in PATCHES {
        assert!(after.code.contains(to.trim_end()), "{to}");
    }
    // The game's own code, after the runtime, is never changed.
    let game = |c: &lust::LuaChunk| c.code[c.code.find(RUNTIME_END).unwrap()..].to_string();
    assert_eq!(game(&before), game(&after));
}

#[test]
fn a_runtime_that_reads_otherwise_is_a_plain_mistake_not_a_half_change() {
    let mut odd = chunk(HERO);
    odd.code = odd
        .code
        .replacen("  o.state = new\n", "  o.state = new -- changed\n", 1);
    let why = own_states(odd).unwrap_err();
    assert!(why.contains("bug in Solis"), "{why}");
}

/// The state each component of `files` has of its own.
fn types(files: &[(&str, &str)]) -> Vec<(String, String)> {
    let code = Code {
        files: files
            .iter()
            .map(|(p, t)| ((*p).to_string(), (*t).to_string()))
            .collect(),
        lua: Vec::new(),
        notes: Vec::new(),
    };
    crate::state_names::state_types(&code)
        .into_iter()
        .map(|(component, own)| (component, own.state))
        .collect()
}

#[test]
fn a_components_own_state_is_read_from_where_it_starts() {
    let found = types(&[(
        "main.lst",
        "state Mood = [Calm, Stunned];\n\
         -- component Fake { has { state = Wrong.Calm; } }\n\
         component Hero {\n\
             has {\n\
                 int lives = 3;\n\
                 state = Mood.Calm;\n\
             }\n\
             method update(float dt) {\n\
                 state = Other.Thing;\n\
             }\n\
         }\n\
         component Plain {\n\
             has { Mood mood = Mood.Calm; }\n\
         }\n\
         class Bulb {\n\
             has { state = Light.Off; }\n\
         }\n\
         component Door { has { state = Door_State.Shut; } method update(float dt) { } }\n",
    )]);
    assert_eq!(
        found,
        [
            ("Door".to_string(), "Door_State".to_string()),
            ("Hero".to_string(), "Mood".to_string()),
        ]
    );
}

#[test]
fn damaged_code_finds_nothing_and_never_stops() {
    for text in [
        "component",
        "component Hero {",
        "component Hero { has { state = ",
        "component Hero { has { state = Mood",
        "}}}} component { has { state = Mood.Calm; } }",
        "component Hero { has { { state = Mood.Calm; } } }",
        "\"component Hero { has { state = Mood.Calm; } }\"",
    ] {
        assert!(types(&[("main.lst", text)]).is_empty(), "{text}");
    }
}

#[test]
fn only_a_component_that_handles_its_change_is_followed() {
    let code = Code {
        files: vec![(
            "main.lst".to_string(),
            "component Hero {\n\
                 has { state = Mood.Calm; }\n\
                 method update(float dt) { -- on MoodChanged( in a comment\n\
                 }\n\
             }\n\
             component Guard {\n\
                 has { state = Mood.Calm; }\n\
                 on MoodChanged(Mood from, Mood to) { }\n\
             }\n\
             component Other {\n\
                 has { state = Gate.Shut; }\n\
                 on MoodChanged(Mood from, Mood to) { }\n\
             }\n"
            .to_string(),
        )],
        lua: Vec::new(),
        notes: Vec::new(),
    };
    let found = crate::state_names::state_types(&code);
    let handled = |name: &str| found[name].handled;
    assert!(!handled("Hero"));
    assert!(handled("Guard"));
    // Another state's event isn't its own.
    assert!(!handled("Other"));
}
