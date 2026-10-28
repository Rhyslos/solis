//! The mixer file, its tree of buses, and ducking.

use crate::mixer::*;

#[test]
fn the_standard_mixer_round_trips() {
    let mixer = Mixer::default();
    let text = write(&mixer);
    let back = read(&text).unwrap();
    assert_eq!(back.mixer, mixer);
    assert!(back.notes.is_empty(), "{:?}", back.notes);
    // One line per bus, so Git sees a change as one line.
    assert_eq!(text.lines().filter(|l| l.starts_with("bus")).count(), 6);
    assert!(
        text.contains(r#"bus2 = ["Music", "Master", 0.0, [["Voice", -8.0, 0.2, 1.0]]];"#),
        "{text}"
    );
}

#[test]
fn the_standard_buses_are_masters_five_with_voice_ducking_music() {
    let m = Mixer::default();
    let names: Vec<&str> = m.buses.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(
        names,
        ["Master", "Music", "Effects", "Voice", "Ambience", "UI"]
    );
    assert!(m.buses[1..].iter().all(|b| b.inside == MASTER));
    assert_eq!(m.buses[1].ducking[0].while_bus, "Voice");
    assert_eq!(m.buses[1].ducking[0].by_db, -8.0);
}

#[test]
fn values_out_of_range_are_brought_in_with_notes() {
    let text = "mixer version 1;\nbus1 = [\"Master\", \"\", 99.0, []];\n\
                bus2 = [\"Music\", \"Master\", -500, [[\"Master\", 4.0, -1.0, 99.0]]];\n";
    let r = read(text).unwrap();
    assert_eq!(r.mixer.buses[0].volume_db, VOLUME_RANGE_DB.1);
    assert_eq!(r.mixer.buses[1].volume_db, VOLUME_RANGE_DB.0);
    let rule = &r.mixer.buses[1].ducking[0];
    assert_eq!(
        (rule.by_db, rule.down_s, rule.back_s),
        (0.0, 0.0, MAX_FADE_S)
    );
    assert!(r.notes.len() >= 4, "{:?}", r.notes);
}

#[test]
fn a_mixer_without_master_is_refused() {
    let e = read("mixer version 1;\nbus1 = [\"Music\", \"\", 0.0, []];\n").unwrap_err();
    assert!(e.message.contains("Master"), "{e}");
}

#[test]
fn a_bus_inside_one_written_later_still_lands_after_it() {
    let text = "mixer version 1;\nbus1 = [\"Master\", \"\", 0.0, []];\n\
                bus2 = [\"Guns\", \"Effects\", 0.0, []];\n\
                bus3 = [\"Effects\", \"Master\", 0.0, []];\n";
    let r = read(text).unwrap();
    let names: Vec<&str> = r.mixer.buses.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, ["Master", "Effects", "Guns"]);
    assert!(r.notes.is_empty(), "{:?}", r.notes);
}

#[test]
fn loops_and_unknown_parents_end_up_in_master() {
    let text = "mixer version 1;\nbus1 = [\"Master\", \"Music\", 0.0, []];\n\
                bus2 = [\"A\", \"B\", 0.0, []];\nbus3 = [\"B\", \"A\", 0.0, []];\n\
                bus4 = [\"C\", \"Nowhere\", 0.0, []];\n";
    let r = read(text).unwrap();
    let m = &r.mixer;
    assert_eq!(m.buses[0].name, MASTER);
    assert!(m.buses[0].inside.is_empty());
    for (i, _) in m.buses.iter().enumerate().skip(1) {
        assert!(m.is_within(i, 0), "{:?}", m.buses[i]);
    }
    assert_eq!(m.buses.len(), 4);
    assert!(!r.notes.is_empty());
}

#[test]
fn a_name_twice_and_rules_on_missing_buses_are_skipped() {
    let text = "mixer version 1;\nbus1 = [\"Master\", \"\", 0.0, []];\n\
                bus2 = [\"Music\", \"Master\", 0.0, [[\"Ghost\", -3.0, 0.1, 0.1], [\"Music\", -3.0, 0.1, 0.1]]];\n\
                bus3 = [\"Music\", \"Master\", 0.0, []];\n";
    let r = read(text).unwrap();
    assert_eq!(r.mixer.buses.len(), 2);
    assert!(r.mixer.buses[1].ducking.is_empty());
    assert_eq!(r.notes.len(), 3, "{:?}", r.notes);
}

#[test]
fn deep_nesting_is_capped() {
    let mut text = String::from("mixer version 1;\nbus1 = [\"Master\", \"\", 0.0, []];\n");
    let mut inside = "Master".to_owned();
    for i in 0..12 {
        let name = format!("B{i}");
        text.push_str(&format!(
            "bus{} = [\"{name}\", \"{inside}\", 0.0, []];\n",
            i + 2
        ));
        inside = name;
    }
    let r = read(&text).unwrap();
    for i in 0..r.mixer.buses.len() {
        assert!(r.mixer.depth(i) <= MAX_DEPTH, "{}", r.mixer.buses[i].name);
    }
}

#[test]
fn malformed_lines_are_refused_with_their_line() {
    for (text, line) in [
        ("mixer version 1;\nbus1 = 3;\n", 2),
        ("mixer version 1;\nbus1 = [\"Master\", \"\", 0.0];\n", 2),
        ("mixer version 1;\n\nbus1 = [\"\", \"\", 0.0, []];\n", 3),
        (
            "mixer version 1;\nbus1 = [\"Master\", \"\", \"loud\", []];\n",
            2,
        ),
        (
            "mixer version 1;\nbus1 = [\"Master\", \"\", 0.0, [3]];\n",
            2,
        ),
    ] {
        let e = read(text).unwrap_err();
        assert_eq!(e.line, line, "{text}: {e}");
    }
    assert!(read("mixer version 2;\n").is_err());
    assert!(read(&"x".repeat(MAX_BYTES + 1)).is_err());
}

#[test]
fn adding_renaming_and_removing_keep_the_tree_whole() {
    let mut m = Mixer::default();
    let at = m.add_bus("Effects").unwrap();
    assert_eq!(m.buses[at].name, "Bus");
    assert_eq!(m.buses[at].inside, "Effects");
    let inner = m.add_bus("Bus").unwrap();
    assert!(m.is_within(inner, at));
    assert_eq!(m.buses[inner].name, "Bus 2");
    // Renaming carries through to what's inside and to rules.
    assert!(m.rename_bus(at, "Guns"));
    assert_eq!(m.buses[inner].inside, "Guns");
    assert!(m.rename_bus(m.index_of("Voice").unwrap(), "Speech"));
    assert_eq!(m.buses[1].ducking[0].while_bus, "Speech");
    assert!(!m.rename_bus(at, "Music"), "taken");
    assert!(!m.rename_bus(at, " "), "empty");
    // Removing moves what's inside up, and drops rules listening to it.
    let guns = m.index_of("Guns").unwrap();
    m.remove_bus(guns);
    assert_eq!(m.buses[m.index_of("Bus 2").unwrap()].inside, "Effects");
    m.remove_bus(m.index_of("Speech").unwrap());
    assert!(m.buses[1].ducking.is_empty());
    m.remove_bus(0);
    assert_eq!(m.buses[0].name, MASTER, "Master stays");
    // And it still reads back as written.
    assert_eq!(read(&write(&m)).unwrap().mixer, m);
}

#[test]
fn no_more_buses_than_the_limit() {
    let mut m = Mixer::default();
    while m.add_bus(MASTER).is_some() {}
    assert_eq!(m.buses.len(), MAX_BUSES);
}

#[test]
fn an_unknown_bus_plays_through_master() {
    let m = Mixer::default();
    assert_eq!(m.bus_for("Voice"), 3);
    assert_eq!(m.bus_for("Gone"), 0);
}

#[test]
fn levels_follow_faders_overrides_ducking_mute_and_solo() {
    let mut m = Mixer::default();
    m.buses[1].volume_db = -4.0;
    let none = vec![1.0; m.buses.len()];
    let flat = vec![0.0; m.buses.len()];
    let quiet = Listening::default();
    let l = levels(&m, &none, &flat, &quiet);
    assert_eq!(l[1], -4.0);
    // Game code's setBusVolume: a share of the fader (half is about -6 dB).
    let mut half = none.clone();
    half[1] = 0.5;
    assert!((levels(&m, &half, &flat, &quiet)[1] - (-4.0 - 6.0206)).abs() < 0.01);
    half[1] = 0.0;
    assert_eq!(levels(&m, &half, &flat, &quiet)[1], f32::NEG_INFINITY);
    half[1] = f32::NAN;
    assert_eq!(
        levels(&m, &half, &flat, &quiet)[1],
        -4.0,
        "not a number counts as 1"
    );
    assert_eq!(
        levels(&m, &[], &flat, &quiet)[1],
        -4.0,
        "missing counts as 1"
    );
    // Ducking adds on.
    let mut ducked = flat.clone();
    ducked[1] = -8.0;
    assert_eq!(levels(&m, &none, &ducked, &quiet)[1], -12.0);
    // Mute.
    let muted = Listening {
        muted: vec!["UI".into()],
        ..Listening::default()
    };
    assert_eq!(levels(&m, &none, &flat, &muted)[5], f32::NEG_INFINITY);
    // Solo Voice: Voice and Master heard, the others not.
    let solo = Listening {
        soloed: vec!["Voice".into()],
        ..Listening::default()
    };
    let l = levels(&m, &none, &flat, &solo);
    assert!(l[0].is_finite() && l[3].is_finite());
    assert!(l[1] == f32::NEG_INFINITY && l[2] == f32::NEG_INFINITY);
    // Everything muted from the Game panel.
    let all = Listening {
        all_muted: true,
        ..Listening::default()
    };
    assert_eq!(levels(&m, &none, &flat, &all)[0], f32::NEG_INFINITY);
}

#[test]
fn voice_ducks_music_and_music_comes_back_after_the_last_voice() {
    let m = Mixer::default();
    let mut d = Ducker::default();
    let mut busy = vec![false; m.buses.len()];
    busy[3] = true; // Voice plays.
    // 0.2 s down, in frames of 1/60 s.
    for _ in 0..6 {
        d.step(&m, &busy, 1.0 / 60.0);
    }
    assert!(
        d.now_db[1] < 0.0 && d.now_db[1] > -8.0,
        "on the way: {}",
        d.now_db[1]
    );
    for _ in 0..12 {
        d.step(&m, &busy, 1.0 / 60.0);
    }
    assert_eq!(d.now_db[1], -8.0, "all the way down after 0.2 s");
    assert!(
        d.now_db
            .iter()
            .enumerate()
            .all(|(i, v)| i == 1 || *v == 0.0)
    );
    // Voice stops: 1 s back.
    busy[3] = false;
    for _ in 0..30 {
        d.step(&m, &busy, 1.0 / 60.0);
    }
    assert!(
        d.now_db[1] > -8.0 && d.now_db[1] < 0.0,
        "half way back: {}",
        d.now_db[1]
    );
    for _ in 0..40 {
        d.step(&m, &busy, 1.0 / 60.0);
    }
    assert_eq!(d.now_db[1], 0.0);
}

#[test]
fn a_bus_inside_the_ducking_one_counts_as_it_playing() {
    let mut m = Mixer::default();
    let lines = m.add_bus("Voice").unwrap();
    let mut d = Ducker::default();
    let mut busy = vec![false; m.buses.len()];
    busy[lines] = true;
    d.step(&m, &busy, 1.0);
    assert_eq!(d.now_db[1], -8.0);
}

#[test]
fn ducking_ignores_bad_frame_times() {
    let m = Mixer::default();
    let mut d = Ducker::default();
    let busy = vec![true; m.buses.len()];
    d.step(&m, &busy, f32::NAN);
    d.step(&m, &busy, -1.0);
    assert_eq!(d.now_db[1], 0.0);
    d.step(&m, &busy, f32::INFINITY);
    assert_eq!(
        d.now_db[1], 0.0,
        "an infinite frame isn't a number of seconds"
    );
}
