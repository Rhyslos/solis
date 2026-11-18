//! A list of game states: kept as changes and read back whole, written
//! and read the same, and damaged files refused, never crashing.

use super::*;

fn parts(pairs: &[(&str, &str)]) -> StateParts {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Three ticks: a crate falls, a drone comes, then goes.
fn sample() -> StateList {
    let mut list = StateList::default();
    list.push(1, 2, parts(&[("game seed", "7"), ("crate place", "0 0 2")]));
    list.push(
        3,
        4,
        parts(&[
            ("game seed", "7"),
            ("crate place", "0 0 1.5"),
            ("drone \"A\tb\"", "x\ny"),
        ]),
    );
    list.push(
        5,
        6,
        parts(&[("game seed", "7"), ("crate place", "0 0 1.5")]),
    );
    list
}

#[test]
fn each_tick_keeps_only_what_changed_and_is_read_back_whole() {
    let list = sample();
    assert_eq!(list.len(), 3);
    assert_eq!(list.ticks[0].changes.len(), 2);
    assert_eq!(list.ticks[1].changes.len(), 2, "{:?}", list.ticks[1]);
    assert_eq!(
        list.ticks[2].changes,
        vec![("drone \"A b\"".to_string(), None)]
    );
    assert_eq!(list.checksums(1), Some((3, 4)));
    assert_eq!(list.checksums(3), None);
    let at_one = list.state_at(1).unwrap();
    assert_eq!(at_one["crate place"], "0 0 1.5");
    assert_eq!(at_one["drone \"A b\""], "x y", "kept on one line");
    assert_eq!(
        list.state_at(2).unwrap(),
        parts(&[("game seed", "7"), ("crate place", "0 0 1.5")])
    );
    assert_eq!(list.state_at(3), None);
}

#[test]
fn a_list_is_written_and_read_back_the_same() {
    let list = sample();
    let text = list.write("made by a test\nsecond line");
    assert!(text.starts_with("solis states 1\n-- made by a test\n-- second line\n"));
    let again = StateList::read(&text).unwrap();
    assert_eq!(again, list);
    assert_eq!(again.write("made by a test\nsecond line"), text);
    // Windows line ends read the same.
    assert_eq!(StateList::read(&text.replace('\n', "\r\n")).unwrap(), list);
    // Read back, it goes on as it would have.
    let mut more = again.clone();
    let mut went_on = list.clone();
    let next = parts(&[("game seed", "8")]);
    more.push(9, 9, next.clone());
    went_on.push(9, 9, next);
    assert_eq!(more, went_on);
}

#[test]
fn differences_name_each_part_that_differs() {
    let mut list = StateList::default();
    let long = "1.5 ".repeat(20);
    list.push(
        1,
        1,
        parts(&[("a", "1"), ("b", "2"), ("c", "3"), ("long", &long)]),
    );
    let actual = parts(&[("a", "1"), ("b", "2.0000001"), ("d", "4"), ("long", &long)]);
    let found = list.differences(0, &actual).unwrap();
    let named: Vec<&str> = found.iter().map(|d| d.part.as_str()).collect();
    assert_eq!(named, ["b", "c", "d"]);
    assert_eq!(found[1].actual, None);
    assert_eq!(found[2].expected, None);
    assert_eq!(found[2].actual.as_deref(), Some("4"));
    assert_eq!(list.differences(1, &actual), None, "past the end");
    // A long value is kept as its checksum, and still compared exactly.
    let kept = list.state_at(0).unwrap()["long"].clone();
    assert!(kept.starts_with('#') && kept.len() == 9, "{kept}");
    let mut changed = actual.clone();
    changed.insert("long".into(), long.replacen("1.5", "1.6", 1));
    let found = list.differences(0, &changed).unwrap();
    let long_one = found.iter().find(|d| d.part == "long").unwrap();
    assert_eq!(long_one.expected.as_deref(), Some(kept.as_str()));
    assert!(
        long_one.actual.as_deref().unwrap().starts_with("1.6"),
        "shown in full"
    );
}

#[test]
fn a_part_gone_and_back_keeps_its_number() {
    let mut list = StateList::default();
    list.push(0, 0, parts(&[("drone", "here"), ("seed", "1")]));
    list.push(0, 0, parts(&[("seed", "1")]));
    list.push(0, 0, parts(&[("drone", "back"), ("seed", "1")]));
    let text = list.write("");
    assert!(text.contains("+ 0 drone\there\n"), "{text}");
    assert!(text.contains("- 0\n"), "{text}");
    assert!(text.contains("= 0\tback\n"), "{text}");
    assert_eq!(StateList::read(&text).unwrap(), list);
}

#[test]
fn damaged_lists_are_refused_never_crashing() {
    assert_eq!(StateList::read(""), Err(StateListError::NotAStateList));
    assert_eq!(
        StateList::read("solis states 2\n"),
        Err(StateListError::NotAStateList)
    );
    assert_eq!(
        StateList::read("solis states 1\n"),
        Ok(StateList::default())
    );
    let tick = "tick 0 w 0000000000000001 g 0000000000000002";
    for (bad, line) in [
        ("solis states 1\n+ 0 a\tb\n".to_string(), 2),
        (format!("solis states 1\n{tick}\n+ 0 a b\n"), 3),
        (format!("solis states 1\n{tick}\n+ 0 \tb\n"), 3),
        (format!("solis states 1\n{tick}\n+ 1 a\tb\n"), 3),
        (format!("solis states 1\n{tick}\n= 0\tb\n"), 3),
        (format!("solis states 1\n{tick}\n- 0\n"), 3),
        (format!("solis states 1\n{tick}\n+ 0 a\tb\n= x\tc\n"), 4),
        (format!("solis states 1\n{tick}\n* a\n"), 3),
        (
            "solis states 1\ntick 1 w 0000000000000001 g 0000000000000002\n".into(),
            2,
        ),
        ("solis states 1\ntick 0 w 01 g 0000000000000002\n".into(), 2),
        (
            "solis states 1\ntick 0 w zzzzzzzzzzzzzzzz g 0000000000000002\n".into(),
            2,
        ),
        ("solis states 1\ntick 0 g 0000000000000002\n".into(), 2),
        (format!("solis states 1\n{tick}\n{tick}\n"), 3),
    ] {
        assert_eq!(
            StateList::read(&bad),
            Err(StateListError::Damaged(line)),
            "{bad}"
        );
    }
    let huge = "x".repeat(MAX_STATE_LIST_BYTES + 1);
    assert_eq!(StateList::read(&huge), Err(StateListError::TooBig));
    // Fuzz: every prefix and many changed bytes of a good file are read
    // or refused, never a crash.
    let good = sample().write("a comment");
    for cut in 0..good.len() {
        if good.is_char_boundary(cut) {
            let _ = StateList::read(&good[..cut]);
        }
    }
    let bytes = good.as_bytes();
    let mut seed = 0x5eed_1234_u64;
    for _ in 0..20_000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut changed = bytes.to_vec();
        for k in 0..1 + (seed >> 60) as usize {
            let at = (seed.rotate_left(k as u32 * 8) as usize) % changed.len();
            changed[at] = (seed >> (8 * (k % 4))) as u8;
        }
        if let Ok(text) = String::from_utf8(changed)
            && let Ok(list) = StateList::read(&text)
        {
            for tick in 0..list.len() {
                let _ = list.state_at(tick);
            }
            let _ = StateList::read(&list.write(""));
        }
    }
}
