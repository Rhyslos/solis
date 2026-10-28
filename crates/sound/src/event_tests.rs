//! Tests for sound events: reading and writing `.solsound` files, and
//! picking plays.

use crate::event::*;
use proptest::prelude::*;
use reflect::{AssetId, Id};

fn id(n: u64) -> AssetId {
    AssetId(Id::from_raw(n).unwrap())
}

fn footstep() -> SoundEvent {
    SoundEvent {
        takes: (1..=5).map(id).collect(),
        volume_db: -3.0,
        volume_range_db: 2.0,
        pitch_st: 0.5,
        pitch_range_st: 1.0,
        bus: "Effects".into(),
        space: Space::Placed,
        near_m: 2.0,
        far_m: 30.0,
        fade: Fade::Even,
        at_once: 8,
        priority: Priority::High,
        looping: true,
    }
}

#[test]
fn an_event_round_trips_through_its_file() {
    let event = footstep();
    let text = write(&event);
    assert!(text.starts_with("-- A sound event."));
    assert!(text.contains("\nsound version 1;\n"));
    assert!(text.contains("\nbus = \"Effects\";\n"));
    let read = read(&text).unwrap();
    assert_eq!(read.event, event);
    assert!(read.notes.is_empty(), "{:?}", read.notes);
    assert_eq!(write(&read.event), text, "written the same again");
}

#[test]
fn an_empty_event_takes_the_defaults() {
    let read = read("sound version 1;\n").unwrap();
    assert_eq!(read.event, SoundEvent::default());
    assert!(read.notes.is_empty());
}

#[test]
fn values_out_of_range_are_brought_in_with_a_note() {
    let text =
        "sound version 1;\nvolume = 40;\npitch = -99;\nat_once = 0;\nnear = 50;\nfar = 10;\n";
    let read = read(text).unwrap();
    assert_eq!(read.event.volume_db, VOLUME_RANGE_DB.1);
    assert_eq!(read.event.pitch_st, PITCH_RANGE_ST.0);
    assert_eq!(read.event.at_once, 1);
    assert_eq!(read.event.near_m, read.event.far_m, "near never past far");
    assert_eq!(read.notes.len(), 4, "{:?}", read.notes);
    assert!(read.notes[0].starts_with("line 2: volume = 40"));
}

#[test]
fn unknown_settings_and_twice_listed_takes_are_noted_not_refused() {
    let text = format!(
        "sound version 1;\ntakes = [\"{0}\", \"{0}\"];\nreverb = 3;\n",
        id(1)
    );
    let read = read(&text).unwrap();
    assert_eq!(read.event.takes, vec![id(1)]);
    assert!(read.notes.iter().any(|n| n.contains("listed twice")));
    assert!(
        read.notes
            .iter()
            .any(|n| n.contains("\"reverb\" isn't a sound event setting"))
    );
}

#[test]
fn wrong_kinds_of_value_are_refused_with_their_line() {
    let err = |body: &str| read(&format!("sound version 1;\n{body}\n")).unwrap_err();
    for (body, says) in [
        ("volume = \"loud\";", "volume should be a number"),
        ("loop = 1;", "loop should be true or false"),
        (
            "placement = \"Nearby\";",
            "\"In the world\" or \"Everywhere\"",
        ),
        ("fade = \"Fast\";", "\"Realistic\", \"Even\" or \"None\""),
        ("priority = 3;", "priority should be text"),
        ("at_once = 2.5;", "a whole number"),
        ("takes = \"1\";", "a list of sound files' IDs"),
        ("takes = [1];", "IDs in quotes"),
        ("takes = [\"not an id!\"];", "a take's ID"),
        ("bus = \"\";", "a bus's name"),
    ] {
        let e = err(body);
        assert_eq!(e.line, 2, "{body}");
        assert!(e.message.contains(says), "{body}: {}", e.message);
    }
    let many: Vec<String> = (1..=MAX_TAKES as u64 + 1)
        .map(|n| format!("\"{}\"", id(n)))
        .collect();
    let e = err(&format!("takes = [{}];", many.join(", ")));
    assert!(e.message.contains("up to 32"), "{}", e.message);
    assert!(
        read("area version 1;")
            .unwrap_err()
            .message
            .contains("isn't a sound file")
    );
    assert!(
        read(&" ".repeat(MAX_BYTES + 1))
            .unwrap_err()
            .message
            .contains("KB")
    );
}

#[test]
fn a_pick_is_one_of_the_takes_within_the_ranges() {
    let event = footstep();
    for tick in 0..2000 {
        let p = pick(&event, None, 7, tick, 0).unwrap();
        assert!(event.takes.contains(&p.take));
        assert!((p.volume_db - event.volume_db).abs() <= event.volume_range_db + 1e-4);
        assert!((p.pitch_st - event.pitch_st).abs() <= event.pitch_range_st + 1e-4);
    }
    assert!(
        pick(&SoundEvent::default(), None, 7, 0, 0).is_none(),
        "no takes"
    );
}

#[test]
fn the_same_take_never_plays_twice_in_a_row_when_there_is_another() {
    let event = footstep();
    let mut last = None;
    let mut seen = std::collections::HashSet::new();
    for tick in 0..500 {
        let p = pick(&event, last, 1, tick, 0).unwrap();
        assert_ne!(Some(p.take), last);
        seen.insert(p.take);
        last = Some(p.take);
    }
    assert_eq!(seen.len(), 5, "every take gets played");
    let one = SoundEvent {
        takes: vec![id(9)],
        ..SoundEvent::default()
    };
    assert_eq!(
        pick(&one, Some(id(9)), 1, 0, 0).unwrap().take,
        id(9),
        "one take plays again"
    );
}

#[test]
fn the_same_seed_tick_and_order_pick_the_same_play() {
    let event = footstep();
    let a = pick(&event, None, 42, 100, 3).unwrap();
    assert_eq!(pick(&event, None, 42, 100, 3).unwrap(), a);
    let differs = (0..20).any(|order| pick(&event, None, 42, 100, order).unwrap() != a);
    assert!(differs, "the order changes the pick");
}

#[test]
fn volume_and_pitch_become_factors() {
    let p = Pick {
        take: id(1),
        volume_db: -6.0206,
        pitch_st: 12.0,
    };
    assert!((p.amplitude() - 0.5).abs() < 1e-4);
    assert!((p.speed() - 2.0).abs() < 1e-9);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Any text at all is read or refused, never a crash.
    #[test]
    fn any_text_is_read_or_refused(text in "\\PC{0,400}") {
        let _ = read(&format!("sound version 1;\n{text}"));
        let _ = read(&text);
    }

    /// Anything read writes back to the same event.
    #[test]
    fn what_is_read_round_trips(volume in -100.0_f32..100.0, range in -5.0_f32..50.0, n in 0_i64..200, looping: bool) {
        let text = format!("sound version 1;\nvolume = {volume:?};\nvolume_range = {range:?};\nat_once = {n};\nloop = {looping};\n");
        let first = read(&text).unwrap().event;
        let again = read(&write(&first)).unwrap();
        prop_assert_eq!(&again.event, &first);
        prop_assert!(again.notes.is_empty());
    }
}
