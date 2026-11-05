//! Game-wide objects in save games (decision 0080): kept, given back on a
//! load so the game goes on as it would have, and what the code no longer
//! has is said.

use super::tests::{BUSY, busy, program, run_to, sums};
use crate::console::LogLevel;

/// The busy game with a clock that counts ticks, which Busy reads.
fn clocked() -> String {
    BUSY.replace(
        "int request = 0;\n",
        "int request = 0;\n            int seen = 0;\n",
    )
    .replace(
        "ticks = ticks + 1;\n",
        "ticks = ticks + 1;\n            seen = Clock.ticks;\n",
    )
    .replace(
        "pass Busy;",
        "object Clock {\n    has {\n        int ticks = 0;\n        string label = \"dawn\";\n    }\n\
         \x20   method update(float dt) {\n        ticks = ticks + 1;\n    }\n}\npass Busy, Clock;",
    )
}

#[test]
fn a_save_keeps_the_objects_and_the_loaded_game_goes_on_alike() {
    let mut w = busy("Harbour");
    w.set_program(Some(program(&clocked())));
    w.play();
    run_to(&mut w, 20);
    let text = w.save_text().unwrap();
    assert!(text.contains("\nobjects "), "{text}");
    assert!(text.contains("Clock.ticks = 20;"), "{text}");
    run_to(&mut w, 30);
    let at_30 = sums(&mut w);
    w.load_game(&text, "quick");
    assert!(
        !w.log_lines().iter().any(|l| l.level == LogLevel::Error),
        "{:?}",
        w.log_lines().last().map(|l| &l.text)
    );
    let game = w.playing.as_mut().unwrap().game.as_mut().unwrap();
    assert_eq!(
        game.object_values()[0].1.get("ticks"),
        Some(&reflect::Value::Int(20))
    );
    run_to(&mut w, 30);
    assert_eq!(sums(&mut w), at_30, "the loaded game went on alike");
}

#[test]
fn objects_the_code_no_longer_has_are_said_and_left_out() {
    let mut w = busy("Harbour");
    w.set_program(Some(program(&clocked())));
    w.play();
    run_to(&mut w, 10);
    let text = w.save_text().unwrap();
    w.stop();
    // The clock's gone from the code, and Busy no longer reads it.
    w.set_program(Some(program(BUSY)));
    w.play();
    w.load_game(&text, "quick");
    let said: Vec<&str> = w.log_lines().iter().map(|l| l.text.as_str()).collect();
    assert!(
        said.iter()
            .any(|t| t.contains("the game's code has no object Clock any more")),
        "{said:?}"
    );
    assert!(!w.log_lines().iter().any(|l| l.level == LogLevel::Error));
    run_to(&mut w, 20);
}

#[test]
fn a_damaged_objects_section_starts_the_objects_afresh() {
    let objects = super::objects_of("not an area", &[]);
    assert!(objects.is_err());
    assert!(super::objects_of("", &[]).unwrap().is_empty());
    let mut clock = reflect::ComponentData::default();
    clock.set("ticks", reflect::Value::Int(4));
    clock.set("hour", reflect::Value::Float(6.25));
    clock.set("phase", reflect::Value::Text("Day".into()));
    let values = vec![("Clock".to_string(), clock)];
    let text = super::objects_text(&values);
    // Read back with the object's kinds, exactly.
    let schema = reflect::ComponentSchema {
        name: "Clock".into(),
        help: String::new(),
        fields: vec![
            reflect::FieldInfo::new(
                "ticks",
                "Ticks",
                reflect::FieldKind::Int,
                reflect::Value::Int(0),
            ),
            reflect::FieldInfo::new(
                "hour",
                "Hour",
                reflect::FieldKind::Float,
                reflect::Value::Float(0.0),
            ),
            reflect::FieldInfo::new(
                "phase",
                "Phase",
                reflect::FieldKind::Choice(vec!["Night".into(), "Day".into()]),
                reflect::Value::Text("Night".into()),
            ),
        ],
    };
    assert_eq!(super::objects_of(&text, &[schema]).unwrap(), values);
}
