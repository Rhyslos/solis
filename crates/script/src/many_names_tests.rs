//! Programs with more top-level names than Luau gives a function locals
//! (Fixes 1, B1): game code and screens with hundreds of methods, states,
//! classes, objects, fields and events still start, and run as they would
//! with few.

use super::*;
use crate::code::{Code, compile};
use crate::screens::{NoReader, Screens};
use reflect::Value;

/// Far more of each than a game writes, and than Luau allows a function.
const MANY: usize = 300;

fn code(files: &[(&str, String)]) -> Code {
    Code {
        files: files
            .iter()
            .map(|(p, t)| (p.to_string(), t.clone()))
            .collect(),
        ..Code::default()
    }
}

/// A file of `n` methods, `m0` to `m{n-1}`, each giving back its number,
/// all passed.
fn methods(n: usize) -> String {
    let mut text = String::new();
    for i in 0..n {
        text.push_str(&format!("method int m{i}() {{ return {i}; }}\n"));
    }
    let names: Vec<String> = (0..n).map(|i| format!("m{i}")).collect();
    text.push_str(&format!("pass {};\nend\n", names.join(", ")));
    text
}

/// A main file whose `Clock` object counts with `body` once a tick, after
/// `head`.
fn main_with(head: &str, body: &str) -> String {
    format!(
        "{head}\nobject Clock {{\n    has {{ int hour = 0; }}\n    method update(float dt) {{\n        \
         {body}\n    }}\n}}\npass Clock;\nend\n"
    )
}

/// The Clock's hour after `ticks` ticks of the compiled game.
fn hour_after(compiled: &crate::Compiled, ticks: usize) -> Value {
    let mut game = crate::Runtime::new().unwrap();
    game.load(compiled, &mut crate::NoHost).unwrap();
    for _ in 0..ticks {
        game.tick(0.1, &mut crate::NoHost);
    }
    let values = game.object_values();
    let (_, clock) = values.iter().find(|(n, _)| n == "Clock").unwrap();
    clock.get("hour").cloned().unwrap()
}

#[test]
fn a_screen_fetching_a_file_of_hundreds_of_methods_starts() {
    // The report's failure: only the screen fetches the file.
    let main = main_with("", "hour = hour + 1;");
    let hud = format!(
        "fetch \"main\":Clock;\nget \"lib\":m{last};\n\nassembly Hud {{\n    view {{\n        \
         text (Clock.hour + m{last}()).text();\n    }}\n}}\n\npass Hud;\nend\n",
        last = MANY - 1
    );
    let compiled = compile(&code(&[
        ("main.lst", main),
        ("lib.lst", methods(MANY)),
        ("ui/hud.lst", hud),
    ]))
    .unwrap()
    .unwrap();
    let screens = Screens::start(compiled.screens.as_ref().unwrap()).unwrap();
    let view = screens.view("Hud", None, &mut NoReader).unwrap();
    assert_eq!(view.children[0].text, (MANY - 1).to_string());
}

#[test]
fn game_code_with_hundreds_of_methods_starts_and_runs() {
    let main = main_with(
        &format!("get \"lib\":[m1, m{}];", MANY - 1),
        &format!("hour = hour + m1() + m{}();", MANY - 1),
    );
    let compiled = compile(&code(&[("main.lst", main), ("lib.lst", methods(MANY))]))
        .unwrap()
        .unwrap();
    assert_eq!(hour_after(&compiled, 2), Value::Int(2 * MANY as i64));
}

#[test]
fn one_method_calling_hundreds_of_others_starts_and_runs() {
    // Each name a method reaches outside it is one of its upvalues, of
    // which Luau allows 200 too.
    let names: Vec<String> = (0..MANY).map(|i| format!("m{i}")).collect();
    let calls: Vec<String> = (0..MANY).map(|i| format!("hour = hour + m{i}();")).collect();
    let main = main_with(
        &format!("get \"lib\":[{}];", names.join(", ")),
        &calls.join("\n        "),
    );
    let compiled = compile(&code(&[("main.lst", main), ("lib.lst", methods(MANY))]))
        .unwrap()
        .unwrap();
    let sum = (MANY * (MANY - 1) / 2) as i64;
    assert_eq!(hour_after(&compiled, 1), Value::Int(sum));
}

#[test]
fn hundreds_of_states_classes_objects_and_events_start() {
    let mut text = String::new();
    for i in 0..MANY {
        text.push_str(&format!("state Mood{i} = [Calm, Cross];\n"));
        text.push_str(&format!(
            "class Thing{i} {{\n    has {{ int n = {i}; }}\n    method int value() {{ return n; }}\n}}\n"
        ));
        text.push_str(&format!(
            "object Store{i} {{\n    has {{ int n = {i}; }}\n}}\n"
        ));
        text.push_str(&format!("event Told{i}(int n);\n"));
    }
    let last = MANY - 1;
    let main = main_with(
        &text,
        &format!(
            "Thing{last} t = new Thing{last} {{ }};\n        Mood{last} m = Mood{last}.Cross;\n        \
             is m == Mood{last}.Cross? then {{ hour = hour + t:value() + Store{last}.n; }}"
        ),
    );
    let compiled = compile(&code(&[("main.lst", main)])).unwrap().unwrap();
    assert_eq!(hour_after(&compiled, 1), Value::Int(2 * last as i64));
}

#[test]
fn a_component_of_hundreds_of_fields_and_a_state_of_thousands_of_options_start() {
    let fields: String = (0..MANY).map(|i| format!("int f{i} = {i}; ")).collect();
    let options: Vec<String> = (0..4 * MANY).map(|i| format!("Option{i}")).collect();
    let main = format!(
        "state Big = [{}];\ncomponent Wide {{\n    has {{ {fields}Big pick = Big.Option{}; }}\n    \
         method update(float dt) {{ f0 = f0 + f{}; }}\n}}\npass Wide;\nend\n",
        options.join(", "),
        4 * MANY - 1,
        MANY - 1
    );
    let compiled = compile(&code(&[("main.lst", main)])).unwrap().unwrap();
    let wide = compiled.schemas.iter().find(|s| s.name == "Wide").unwrap();
    assert_eq!(wide.fields.len(), MANY + 1);
}

#[test]
fn a_method_called_like_the_runtime_s_own_stays_its_own() {
    // `next` is a word Lust's runtime uses: with hundreds of names it stays
    // a local, so the runtime's `next` is still Lua's.
    let mut lib = methods(MANY);
    lib = lib.replace("pass ", "method int next() { return 7; }\npass next, ");
    let main = main_with(
        &format!("get \"lib\":[next, m{}];", MANY - 1),
        &format!("hour = hour + next() + m{}();", MANY - 1),
    );
    let compiled = compile(&code(&[("main.lst", main), ("lib.lst", lib)]))
        .unwrap()
        .unwrap();
    assert_eq!(hour_after(&compiled, 1), Value::Int(7 + MANY as i64 - 1));
}

#[test]
fn a_program_with_few_names_is_left_as_it_was() {
    let program = "_L.exact = true\nlocal A, b, C\nA = _L.class(\"A\", nil)\n";
    assert_eq!(fitted(program, &BTreeSet::new()), None);
}

#[test]
fn names_past_the_kept_ones_become_globals_and_no_line_moves() {
    let names: Vec<String> = (0..KEPT + 30).map(|i| format!("m{i}")).collect();
    let program = format!(
        "local _K = {{ 1 }}\nlocal {}\nlocal Mood = _L.state(\"Mood\", {{}})\n  local inner = 1\nm0 = function() end\n",
        names.join(", ")
    );
    let taken: BTreeSet<&str> = ["m130"].into_iter().collect();
    let out = fitted(&program, &taken).unwrap();
    assert_eq!(out.lines().count(), program.lines().count());
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "local _K = { 1 }");
    let kept: Vec<&str> = lines[1]
        .strip_prefix("local ")
        .unwrap()
        .split(", ")
        .collect();
    // `_K` and the first KEPT - 1 names, and the one the runtime uses.
    assert_eq!(kept.len(), KEPT);
    assert!(kept.contains(&"m0") && kept.contains(&"m130"));
    assert!(!kept.contains(&"m149"));
    assert_eq!(lines[2], "Mood = _L.state(\"Mood\", {})");
    assert_eq!(lines[3], "  local inner = 1", "inside a function: left");
}

#[test]
fn only_the_program_after_the_runtime_is_changed() {
    let mut names: Vec<String> = (0..KEPT + 5).map(|i| format!("m{i}")).collect();
    names.push("m200".into());
    let code = format!(
        "local _L = (function()\nlocal m3, m200 = 1, 2\nend)()\nlocal {}\n",
        names.join(", ")
    );
    let words = words(&code[..code.find(RUNTIME_END).unwrap()]);
    assert!(words.contains("m200") && words.contains("local"));
    let out = fitted(&code[code.find(RUNTIME_END).unwrap() + RUNTIME_END.len()..], &words).unwrap();
    let forward = out.lines().next().unwrap();
    assert!(forward.ends_with(", m200"), "the runtime's word stays: {forward}");
    assert!(!forward.contains("m124,"), "{forward}");
}

#[test]
fn odd_lines_never_panic_and_never_move() {
    let samples = [
        "",
        "local",
        "local ",
        "local ,",
        "local a,,b",
        "local 1x = 2",
        "local a, b = 1, 2",
        "local function f() end",
        "local é = 1",
        "local x=1",
    ];
    let filler: Vec<String> = (0..KEPT + 1).map(|i| format!("local v{i} = {i}")).collect();
    for sample in samples {
        let program = format!("{}\n{sample}\n", filler.join("\n"));
        let out = fitted(&program, &BTreeSet::new()).unwrap();
        assert_eq!(out.split('\n').count(), program.split('\n').count(), "{sample}");
        assert!(out.contains(sample.trim_start_matches("local ")), "{sample}");
    }
}
