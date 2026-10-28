//! Animator files: read as written, round trips, merges, limits, and
//! damaged files put right or refused with their line.

use super::*;
use proptest::prelude::*;

fn id(n: u64) -> Id {
    Id::from_raw(n).unwrap()
}

fn i(n: u64) -> String {
    id(n).to_string()
}

/// The farmer of the mock-up: facing, moving, speed and swing; Idle,
/// Walk and Use tool; and four transitions.
pub(crate) fn farmer() -> AnimatorFile {
    let by_facing = |stem: &str| Plays::ByDirection {
        by: "facing".into(),
        clips: [Facing::Up, Facing::Right, Facing::Down, Facing::Left]
            .into_iter()
            .map(|f| (f, format!("{stem}_{}", f.word())))
            .collect(),
    };
    let state = |n: u64, name: &str, stem: &str, x: f32| State {
        id: id(n),
        name: name.into(),
        place: [x, 80.0],
        plays: by_facing(stem),
        speed: 1.0,
        speed_by: String::new(),
    };
    let mut walk = state(2, "Walk", "walk", 400.0);
    walk.speed_by = "speed".into();
    let param = |n: u64, name: &str, kind| Parameter {
        id: id(n),
        name: name.into(),
        kind,
    };
    let way = |n: u64, from: Option<u64>, to: u64, when: Vec<Condition>| Transition {
        id: id(n),
        from: from.map(id),
        to: id(to),
        when,
    };
    AnimatorFile {
        sheet: Some(AssetId(Id::from_raw(77).unwrap())),
        entry: Some(id(1)),
        any_place: [120.0, 300.0],
        parameters: vec![
            param(10, "facing", ParameterKind::Direction(Facing::Down)),
            param(11, "moving", ParameterKind::Switch(false)),
            param(12, "speed", ParameterKind::Number(1.0)),
            param(13, "swing", ParameterKind::Trigger),
        ],
        states: vec![
            state(1, "Idle", "idle", 120.0),
            walk,
            state(3, "Use tool", "hoe", 400.0),
        ],
        transitions: vec![
            way(20, Some(1), 2, vec![Condition::Is("moving".into())]),
            way(21, Some(2), 1, vec![Condition::Not("moving".into())]),
            way(22, None, 3, vec![Condition::Is("swing".into())]),
            way(23, Some(3), 2, vec![Condition::Finished]),
        ],
    }
}

#[test]
fn the_farmer_round_trips_and_reads_as_a_person_would_write_it() {
    let file = farmer();
    let text = write(&file);
    assert!(text.starts_with("-- An animator."));
    assert!(text.contains("animator version 1;"));
    assert!(text.contains("by = \"facing\";"));
    assert!(text.contains("clips = [[\"up\", \"hoe_up\"], [\"right\", \"hoe_right\"]"));
    assert!(text.contains("from = \"any\";"));
    assert!(text.contains("when = [\"not moving\"];"));
    let back = read(&text).unwrap();
    assert!(back.notes.is_empty(), "{:?}", back.notes);
    assert_eq!(back.file, file);
    assert_eq!(write(&back.file), text, "written the same again");
}

#[test]
fn the_fixture_reads_without_notes() {
    let text = include_str!("../../../tests/fixtures/animator/v1/farmer.solanim");
    let read = read(text).unwrap();
    assert!(read.notes.is_empty(), "{:?}", read.notes);
    assert_eq!(read.file.states.len(), 3);
    assert_eq!(read.file.entry_state().unwrap().name, "Idle");
}

#[test]
fn conditions_read_and_write_as_words() {
    for (text, c) in [
        ("moving", Condition::Is("moving".into())),
        (" not moving ", Condition::Not("moving".into())),
        ("speed > 0.5", Condition::Above("speed".into(), 0.5)),
        ("speed<2", Condition::Below("speed".into(), 2.0)),
        ("finished", Condition::Finished),
    ] {
        assert_eq!(Condition::of_text(text).unwrap(), c, "{text}");
        assert_eq!(Condition::of_text(&c.text()).unwrap(), c);
    }
    for bad in [
        "",
        "not ",
        "speed > fast",
        "two words",
        "a > inf",
        "finished > 1",
    ] {
        assert!(Condition::of_text(bad).is_err(), "{bad}");
    }
    assert!(parameter_name(&"x".repeat(MAX_PARAMETER_NAME + 1)).is_err());
    assert_eq!(parameter_name(" höst_2 ").unwrap(), "höst_2");
}

#[test]
fn the_nearest_facing_is_chosen() {
    assert_eq!(Facing::nearest([0.1, -3.0]), Facing::Down);
    assert_eq!(Facing::nearest([2.0, 1.9]), Facing::UpRight);
    assert_eq!(Facing::nearest([0.0, 0.0]), Facing::Down);
    assert_eq!(Facing::nearest([f32::NAN, 1.0]), Facing::Down);
    for f in Facing::ALL {
        assert_eq!(Facing::of_word(f.word()), Some(f));
        assert_eq!(Facing::nearest(f.vector()), f);
    }
}

#[test]
fn what_can_t_be_used_is_skipped_with_a_note() {
    let text = format!(
        "animator version 1;\n\
        sheet = \"not an id\";\n\
        entry = \"{e7}\";\n\
        colour = \"red\";\n\
        parameter {s1} {{ name = \"moving\"; kind = \"switch\"; }}\n\
        parameter {s2} {{ name = \"moving\"; kind = \"switch\"; }}\n\
        parameter {s3} {{ name = \"two words\"; kind = \"switch\"; }}\n\
        parameter {s4} {{ name = \"pace\"; kind = \"velocity\"; }}\n\
        parameter {s5} {{ name = \"look\"; kind = \"direction\"; start = \"north\"; }}\n\
        state {s1} {{ name = \"Idle\"; speed = 99.0; spin = 1; }}\n\
        state {s2} {{ name = \"Idle\"; by = \"moving\"; clips = [[\"up\", \"a\"], [\"up\", \"b\"]]; }}\n\
        state {s3} {{ name = \"Idle\"; by = \"moving\"; speed_by = \"moving\"; }}\n\
        state {s1} {{ name = \"again\"; }}\n\
        state x {{ }}\n\
        transition {s1} {{ from = \"{s1}\"; to = \"{s9}\"; }}\n\
        transition {s2} {{ from = \"any\"; to = \"{s1}\"; when = [\"not\", \"moving\", 3]; }}\n\
        transition {s3} {{ from = \"any\"; to = \"{s3}\"; when = [\"not \"]; }}\n\
        bone 1 {{ }}\n",
        e7 = i(7),
        s1 = i(1),
        s2 = i(2),
        s3 = i(3),
        s9 = i(9),
        s4 = i(4),
        s5 = i(5)
    );
    let read = read(&text).unwrap();
    let notes = read.notes.join("\n");
    let f = &read.file;
    assert_eq!(f.sheet, None);
    assert_eq!(f.entry, None, "{notes}");
    assert_eq!(f.parameters.len(), 1);
    assert_eq!(f.states.len(), 2);
    assert_eq!(f.states[0].speed, 16.0, "brought into range");
    assert_eq!(f.states[1].name, "Idle 2", "a free name");
    assert_eq!(f.transitions.len(), 1);
    assert!(f.transitions[0].when.is_empty());
    for expected in [
        "the sheet isn't an asset's ID",
        "\"colour\" isn't an animator setting",
        "a second parameter called \"moving\"",
        "\"two words\" isn't a parameter's name",
        "\"velocity\" isn't a kind of parameter",
        "\"north\" isn't a direction",
        "\"spin\" isn't a state setting",
        "up is given two clips",
        "\"moving\" isn't a direction parameter",
        "isn't a number parameter",
        "a second state with this ID",
        "\"x\" isn't an ID",
        "its to isn't one of the states",
        "its conditions should be text in quotes",
        "\"bone\" isn't a part of an animator",
        "the entry isn't one of its states",
    ] {
        assert!(notes.contains(expected), "{expected} in\n{notes}");
    }
}

#[test]
fn limits_are_kept() {
    let mut text = String::from("animator version 1;\n");
    for n in 1..=(MAX_STATES + 3) {
        text.push_str(&format!("state {} {{ name = \"S\"; }}\n", i(n as u64)));
    }
    for n in 1..=(MAX_PARAMETERS + 2) {
        text.push_str(&format!(
            "parameter {} {{ name = \"p{n}\"; kind = \"trigger\"; }}\n",
            i(n as u64)
        ));
    }
    let when: Vec<String> = (0..MAX_CONDITIONS + 4)
        .map(|_| "\"p1\"".to_string())
        .collect();
    text.push_str(&format!(
        "transition {} {{ from = \"any\"; to = \"{}\"; when = [{}]; }}\n",
        i(1),
        i(1),
        when.join(", ")
    ));
    let read = read(&text).unwrap();
    assert_eq!(read.file.states.len(), MAX_STATES);
    assert_eq!(read.file.parameters.len(), MAX_PARAMETERS);
    assert_eq!(read.file.transitions[0].when.len(), MAX_CONDITIONS);
    assert_eq!(
        read.file.states[MAX_STATES - 1].name,
        format!("S {MAX_STATES}")
    );
    let big = "-".repeat(MAX_BYTES + 1);
    assert!(super::read(&big).unwrap_err().message.contains("KB"));
}

#[test]
fn a_damaged_or_newer_file_is_refused_with_its_line() {
    let err = |t: &str| read(t).unwrap_err();
    assert!(
        err("frames version 1;")
            .message
            .contains("isn't a animator file")
    );
    assert!(err("animator version 2;").message.contains("newer version"));
    let open = err("animator version 1;\n\nstate 1 {\n name = \"A\";\n");
    assert!(open.to_string().contains("isn't closed"), "{open}");
    assert_eq!(err("animator version 1;\nstate 1 { name = ; }").line, 2);
}

#[test]
fn two_people_s_new_states_merge_as_lines() {
    // Each adds a state on their own branch: the blocks are apart, so a
    // three-way merge of the text keeps both (Git sees two additions).
    let base = farmer();
    let mut mine = base.clone();
    mine.states.push(State {
        id: id(4),
        name: "Sit".into(),
        place: [0.0, 300.0],
        plays: Plays::Clip("sit".into()),
        speed: 1.0,
        speed_by: String::new(),
    });
    let mut theirs = base.clone();
    theirs.transitions.push(Transition {
        id: id(24),
        from: Some(id(2)),
        to: id(3),
        when: vec![Condition::Above("speed".into(), 3.0)],
    });
    let (b, m, t) = (write(&base), write(&mine), write(&theirs));
    // Mine adds lines before the transitions; theirs adds lines at the end.
    let merged = {
        let start = m.find(&format!("state {}", i(4))).unwrap();
        let end = m[start..].find("}\n").unwrap() + start + 2;
        let added = &m[start - 1..end];
        let at = t.find("\ntransition").unwrap();
        format!("{}{}{}", &t[..at], added, &t[at..])
    };
    assert!(b.len() < merged.len());
    let read = read(&merged).unwrap();
    assert!(read.notes.is_empty(), "{:?}", read.notes);
    assert_eq!(read.file.states.len(), 4);
    assert_eq!(read.file.transitions.len(), 5);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Whatever is read is written so it reads back the same.
    #[test]
    fn anything_read_writes_back_the_same(
        names in proptest::collection::vec("[ -~]{0,70}", 1..6),
        speeds in proptest::collection::vec(-5.0f32..40.0, 1..6),
        x in -2.0e5f32..2.0e5,
    ) {
        let mut text = String::from("animator version 1;\n");
        for (i, (n, s)) in names.iter().zip(&speeds).enumerate() {
            let n = n.replace(['"', '\\'], "");
            text.push_str(&format!(
                "state {} {{ name = \"{n}\"; speed = {s:?}; place = [{x:?}, 1.0]; }}\n",
                super::tests::i(i as u64 + 1)
            ));
        }
        let first = read(&text).unwrap().file;
        let again = read(&write(&first)).unwrap();
        prop_assert_eq!(&again.file, &first);
    }
}
