//! What Lust 2.0.0-alpha.74 changed (decision 0123): a state is set with
//! `=`, like any value, and `is` only asks questions. Code from before
//! that sets one with `is` is refused with Lust's own mistake, which
//! shows the `=` way; the same code with `=` runs, the same on any number
//! of shards.

use super::tests::{TestWorld, compiled, id, unhurried};
use super::*;
use crate::code::{CodeProblem, check, gather};
use reflect::Value;

/// Lamps that go from calm to busy on their first update and say so; a
/// bulb of their own (a class with a state of its own) is rushed and
/// asked. `set` is what sets a state: `is` before alpha 74, `=` now.
fn lamps(set: &str) -> String {
    format!(
        "fetch \"solis/world\":World;\n\
         \n\
         state Mood = [Calm, Busy];\n\
         \n\
         class Bulb {{\n\
             has {{\n\
                 state {set} Mood.Calm;\n\
             }}\n\
             method rush() {{\n\
                 state {set} Mood.Busy;\n\
             }}\n\
         }}\n\
         \n\
         component Lamp {{\n\
             has {{\n\
                 Mood mood = Mood.Calm;\n\
                 int changes = 0;\n\
             }}\n\
             method update(float dt) {{\n\
                 is mood == Mood.Calm? then {{\n\
                     mood {set} Mood.Busy;\n\
                     changes += 1;\n\
                     Bulb bulb = new Bulb {{}};\n\
                     bulb:rush();\n\
                     is bulb.state == Mood.Busy? then {{\n\
                         World:say(\"busy\");\n\
                     }}\n\
                 }}\n\
             }}\n\
         }}\n\
         pass Lamp;\n\
         end\n"
    )
}

/// The lines of `text` (from 1) where a state is set the old way.
fn old_way_lines(text: &str) -> Vec<u32> {
    (1..)
        .zip(text.lines())
        .filter(|(_, line)| line.contains(" is Mood."))
        .map(|(n, _)| n)
        .collect()
}

/// A project as it was saved before alpha 74: the engine's files in
/// `solis/`, and `main.lst` holding `main`.
pub(super) fn project(name: &str, main: &str) -> reflect::test_support::TempFolder {
    let folder = reflect::test_support::TempFolder::new(&format!("script-{name}")).unwrap();
    let (files, _) = crate::describe::descriptions_for(
        "Test 1.0",
        "solis",
        &crate::project_names::Names::standard(),
    );
    for (path, text) in files {
        let path = folder.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    std::fs::write(folder.join("main.lst"), main).unwrap();
    folder
}

#[test]
fn an_old_project_setting_a_state_with_is_is_refused_showing_the_new_way() {
    let old = lamps("is");
    let folder = project("old-is", &old);
    let found = check(&gather(&folder));
    let lines = old_way_lines(&old);
    assert_eq!(lines.len(), 3, "{old}");
    let at = |problem: &CodeProblem| (problem.file.clone(), problem.line, problem.message.clone());
    assert_eq!(
        found.iter().map(at).collect::<Vec<_>>(),
        [
            "state = Mood.Calm;",
            "state = Mood.Busy;",
            "mood = Mood.Busy;"
        ]
        .iter()
        .zip(lines)
        .map(|(new, line)| (
            "main.lst".to_string(),
            line,
            format!("a state is set with =, so write {new}")
        ))
        .collect::<Vec<_>>(),
        "Lust's own words, at each line, with nothing of Solis's in the way"
    );
}

#[test]
fn the_same_project_with_equals_runs() {
    let folder = project("equals", &lamps("="));
    assert_eq!(check(&gather(&folder)), Vec::<CodeProblem>::new());
}

/// Runs the lamps on `shards` shards, `lamps` of them, for three ticks:
/// the checksum after each.
fn run_lamps(shards: usize, lamps_on: u64) -> (Game, TestWorld, Vec<u64>) {
    let names: Vec<String> = (1..=lamps_on).map(|n| format!("L{n}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut world = TestWorld::with(&names);
    let mut game = unhurried(shards);
    game.load(&compiled(&lamps("=")), &mut world).unwrap();
    let empty = ComponentData::default();
    let all = (1..=lamps_on).map(|n| (id(n), "Lamp", &empty));
    assert!(game.sync_many(all, &mut world).is_empty());
    let mut sums = Vec::new();
    for _ in 0..3 {
        let ticked = game.tick(1.0 / 60.0, &mut world);
        assert!(ticked.failures.is_empty(), "{:?}", ticked.failures);
        sums.push(game.checksum());
    }
    (game, world, sums)
}

#[test]
fn a_state_set_with_equals_changes_once_and_its_class_s_own_state_too() {
    let (mut game, world, _) = run_lamps(1, 2);
    for n in 1..=2 {
        let values = game.values(id(n), "Lamp").unwrap();
        assert_eq!(values.get("mood"), Some(&Value::Text("Busy".into())));
        assert_eq!(values.get("changes"), Some(&Value::Int(1)), "once");
    }
    assert_eq!(world.said, ["busy", "busy"], "the bulb's own state was set");
}

#[test]
fn states_set_with_equals_give_the_same_game_on_any_number_of_shards() {
    let (_, one_world, one) = run_lamps(1, 7);
    let (_, three_world, three) = run_lamps(3, 7);
    assert_eq!(one, three);
    assert_eq!(one_world.said, three_world.said);
}
