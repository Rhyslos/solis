use super::*;

fn files(main: &str) -> Vec<(String, String)> {
    vec![
        ("solis/components.lst".into(), placeholder("Test 1.0")),
        ("main.lst".into(), main.into()),
    ]
}

fn ours(code: &ComponentCode) -> &str {
    &code
        .files
        .iter()
        .find(|(path, _)| path == "solis/components.lst")
        .expect("the file is written")
        .1
}

#[test]
fn each_component_gets_a_locked_reading_and_a_reader() {
    let code = write(
        "solis",
        &files(
            "component Knight {\n    has {\n        bool hasKey = false;\n        int coins = 3;\n    }\n}\nend\n",
        ),
        "main.lst",
    );
    let text = ours(&code);
    assert!(text.contains("state Component = [Knight];"), "{text}");
    assert!(text.contains("class KnightReading {"), "{text}");
    assert!(text.contains("lock bool hasKey = false;"), "{text}");
    assert!(text.contains("lock int coins = 0;"), "{text}");
    assert!(
        text.contains("method KnightReading entity:knight() native;"),
        "{text}"
    );
    assert!(
        text.ends_with("pass Component, KnightReading;\nend\n"),
        "no values classes since U6b: {text}"
    );
    // A module's class takes the fields, with the component's own plain
    // starting values (crate::made).
    let fields = &code.fields["Knight"];
    let kept: Vec<(&str, &str, Option<&str>)> = fields
        .iter()
        .map(|f| (f.kind.as_str(), f.name.as_str(), f.own.as_deref()))
        .collect();
    assert_eq!(
        kept,
        [
            ("bool", "hasKey", Some("false")),
            ("int", "coins", Some("3"))
        ]
    );
    assert_eq!(code.readable, [("Knight".into(), "knight".into())]);
    assert!(code.notes.is_empty());
}

#[test]
fn a_state_the_file_keeps_to_itself_is_passed_on_its_end_line() {
    let main = "state Mood = [Calm, Angry];\n\
                component Knight {\n    has {\n        Mood mood = Mood.Calm;\n    }\n}\nend\n";
    let code = write("solis", &files(main), "main.lst");
    let theirs = &code.files.iter().find(|(p, _)| p == "main.lst").unwrap().1;
    // The same lines, the last one sharing Mood.
    assert_eq!(theirs.lines().count(), main.lines().count());
    assert!(theirs.ends_with("pass Mood; end\n"), "{theirs}");
    let text = ours(&code);
    assert!(text.contains("fetch \"main\":[Mood];"), "{text}");
    assert!(text.contains("lock Mood mood = Mood.Calm;"), "{text}");
}

#[test]
fn a_field_holding_a_thing_is_left_out() {
    let main = "component Farmer {\n    has {\n        FarmerAnimator anim;\n        int n = 1;\n    }\n}\nend\n";
    let text = write("solis", &files(main), "main.lst");
    let text = ours(&text);
    assert!(!text.contains("anim"), "{text}");
    assert!(text.contains("lock int n = 0;"), "{text}");
}

#[test]
fn a_name_the_game_already_uses_is_a_note_not_a_clash() {
    let main = "class KnightReading {\n    has {\n    }\n}\n\
                component Knight {\n    has {\n        int n = 1;\n    }\n}\nend\n";
    let code = write("solis", &files(main), "main.lst");
    assert!(code.readable.is_empty());
    assert!(
        code.notes[0].contains("has a KnightReading of its own"),
        "{:?}",
        code.notes
    );
    // Still in the state, for World:the.
    assert!(ours(&code).contains("state Component = [Knight];"));
}

#[test]
fn no_components_still_writes_the_state() {
    let code = write("solis", &files("end\n"), "main.lst");
    assert!(
        ours(&code).contains("state Component = ["),
        "{}",
        ours(&code)
    );
    assert!(code.readable.is_empty());
}
