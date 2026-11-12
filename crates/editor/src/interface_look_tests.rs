//! The saved look: it reads back, old accents are mapped, missing and
//! unknown fields are no trouble, and damage never crashes the reader.

use super::*;

/// A look with nothing left at its standard value.
fn changed() -> InterfaceLook {
    InterfaceLook {
        accent: Accent::Purple,
        scale: Scale::from(150),
        density: Density::Compact,
        ribbon_labels: false,
        hide_unfinished: true,
        shelf_opens_on: ShelfOpensOn::Entity,
    }
}

#[test]
fn the_standard_look_is_the_design_s() {
    let look = InterfaceLook::default();
    assert_eq!(look.accent, Accent::Blue);
    assert_eq!(look.scale.percent(), 100);
    assert_eq!(look.density, Density::Default);
    assert!(look.ribbon_labels, "the ribbon shows its buttons' names");
    assert!(!look.hide_unfinished, "unfinished parts show at first");
    assert_eq!(
        look.shelf_opens_on,
        ShelfOpensOn::AsItIs,
        "the shelf stays as it is, as it always did"
    );
}

#[test]
fn a_saved_look_reads_back() {
    let mut looks = vec![InterfaceLook::default(), changed()];
    for accent in Accent::ALL {
        for scale in Scale::ALL {
            for density in Density::ALL {
                looks.push(InterfaceLook {
                    accent,
                    scale,
                    density,
                    ..InterfaceLook::default()
                });
            }
        }
    }
    for look in looks {
        let text = ron::to_string(&look).unwrap();
        assert_eq!(
            ron::from_str::<InterfaceLook>(&text).unwrap(),
            look,
            "{text}"
        );
    }
    // The scale is saved as a plain number of percent.
    assert!(ron::to_string(&changed()).unwrap().contains("scale:150"));
}

#[test]
fn missing_fields_take_their_standard_and_unknown_ones_are_passed_over() {
    // From an earlier build that had fewer fields.
    assert_eq!(
        ron::from_str::<InterfaceLook>("(accent:Teal)").unwrap(),
        InterfaceLook {
            accent: Accent::Teal,
            ..InterfaceLook::default()
        }
    );
    assert_eq!(
        ron::from_str::<InterfaceLook>("()").unwrap(),
        InterfaceLook::default()
    );
    // From a later build that has more.
    let later = "(accent:Orange,scale:125,contrast:High,density:Comfortable,panels:[1,2])";
    assert_eq!(
        ron::from_str::<InterfaceLook>(later).unwrap(),
        InterfaceLook {
            accent: Accent::Orange,
            scale: Scale::from(125),
            density: Density::Comfortable,
            ..InterfaceLook::default()
        }
    );
}

#[test]
fn a_scale_between_steps_becomes_the_nearest_step() {
    for (saved, read) in [(110, 100), (130, 125), (10, 100), (5000, 200)] {
        let look = ron::from_str::<InterfaceLook>(&format!("(scale:{saved})")).unwrap();
        assert_eq!(look.scale.percent(), read);
    }
}

#[test]
fn what_cannot_be_read_is_an_error_the_app_answers_with_the_standard() {
    for damaged in [
        "",
        "(",
        "(accent:Neon)",
        "(accent:Plum)",
        "(scale:-1)",
        "(scale:\"big\")",
        "(density:Huge)",
        "(ribbon_labels:7)",
        "Blue",
        "[1,2,3]",
    ] {
        let read = ron::from_str::<InterfaceLook>(damaged);
        assert!(read.is_err(), "{damaged:?} read as {read:?}");
        assert_eq!(read.unwrap_or_default(), InterfaceLook::default());
    }
}

#[test]
fn each_earlier_accent_becomes_the_nearest_of_the_five() {
    use AccentBefore as Old;
    let mapping = [
        (Old::Blue, Accent::Blue),
        (Old::PetrolBlue, Accent::Teal),
        (Old::Green, Accent::Teal),
        (Old::Olive, Accent::Teal),
        (Old::Red, Accent::Orange),
        (Old::Copper, Accent::Orange),
        (Old::Orange, Accent::Orange),
        (Old::Amber, Accent::Orange),
        (Old::Gold, Accent::Orange),
        (Old::Plum, Accent::Purple),
    ];
    assert_eq!(mapping.len(), Old::ALL.len());
    for (old, new) in mapping {
        assert!(Old::ALL.contains(&old));
        assert_eq!(old.mapped(), new, "{old:?}");
    }
    // Nothing becomes Grey: it is new.
    assert!(Old::ALL.iter().all(|old| old.mapped() != Accent::Grey));
    // Nine of the ten change colour; only the old standard blue keeps its
    // name, and even its shade is the design's now.
    let kept = Old::ALL
        .iter()
        .filter(|old| format!("{old:?}") == old.mapped().name())
        .count();
    assert_eq!(kept, 2, "Blue and Orange keep their names");
}

#[test]
fn an_earlier_accent_still_reads_under_the_name_it_was_saved_with() {
    for (saved, old) in [
        ("PetrolBlue", AccentBefore::PetrolBlue),
        ("Red", AccentBefore::Red),
        ("Blue", AccentBefore::Blue),
        ("Green", AccentBefore::Green),
        ("Plum", AccentBefore::Plum),
        ("Olive", AccentBefore::Olive),
        ("Copper", AccentBefore::Copper),
        ("Orange", AccentBefore::Orange),
        ("Amber", AccentBefore::Amber),
        ("Gold", AccentBefore::Gold),
    ] {
        assert_eq!(
            ron::from_str::<AccentBefore>(saved).ok(),
            Some(old),
            "{saved}"
        );
    }
    assert!(
        ron::from_str::<AccentBefore>("Teal").is_err(),
        "never saved"
    );
}

#[test]
fn the_first_start_takes_the_accent_from_what_an_earlier_build_saved() {
    use AccentBefore as Old;
    let accent = |chosen, oldest| InterfaceLook::from_before(chosen, oldest).accent;
    // Nothing saved: the standard.
    assert_eq!(
        InterfaceLook::from_before(None, None),
        InterfaceLook::default()
    );
    // The newer key wins over the older.
    assert_eq!(accent(Some(Old::Plum), Some(Old::Red)), Accent::Purple);
    assert_eq!(
        accent(Some(Old::Amber), None),
        Accent::Orange,
        "chosen amber"
    );
    // Under the oldest key amber was the standard of its day: never
    // chosen, so it becomes today's standard. Anything else was chosen.
    assert_eq!(accent(None, Some(Old::Amber)), Accent::Blue);
    assert_eq!(accent(None, Some(Old::Green)), Accent::Teal);
    assert_eq!(accent(None, Some(Old::PetrolBlue)), Accent::Teal);
    // Only the accent is carried over.
    let look = InterfaceLook::from_before(Some(Old::Gold), None);
    assert_eq!(
        look,
        InterfaceLook {
            accent: Accent::Orange,
            ..InterfaceLook::default()
        }
    );
}

/// A small generator of numbers that is the same on every run.
struct Dice(u32);

impl Dice {
    fn roll(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
}

/// `good` with a few bytes changed, cut or added.
fn damage(good: &str, dice: &mut Dice) -> String {
    let mut bytes = good.as_bytes().to_vec();
    for _ in 0..1 + dice.roll() % 4 {
        if bytes.is_empty() {
            break;
        }
        let at = dice.roll() as usize % bytes.len();
        match dice.roll() % 7 {
            0 => bytes[at] = (dice.roll() >> 8) as u8,
            1 => bytes.truncate(at),
            2 => bytes.insert(at, b"(),:\"-9"[dice.roll() as usize % 7]),
            3 => {
                bytes.remove(at);
            }
            4 => {
                let piece: Vec<u8> = bytes[at..].iter().take(6).copied().collect();
                bytes.splice(at..at, piece);
            }
            // Gentler damage, which often still reads: another digit, a
            // switch flipped.
            5 => {
                if let Some(digit) = bytes.iter_mut().find(|b| b.is_ascii_digit()) {
                    *digit = b'0' + (dice.roll() % 10) as u8;
                }
            }
            _ => {
                let text = String::from_utf8_lossy(&bytes).replace("true", "false");
                bytes = text.into_bytes();
            }
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

#[test]
fn damaged_saved_looks_never_crash_and_what_reads_is_always_usable() {
    let mut dice = Dice(0x2545_f491);
    let goods = [
        ron::to_string(&InterfaceLook::default()).unwrap(),
        ron::to_string(&changed()).unwrap(),
        "(accent:Orange,scale:175)".to_string(),
    ];
    let (mut read, mut failed) = (0, 0);
    for round in 0..9000 {
        let text = damage(&goods[round % goods.len()], &mut dice);
        match ron::from_str::<InterfaceLook>(&text) {
            Ok(look) => {
                read += 1;
                assert!(Scale::STEPS.contains(&look.scale.percent()), "{text}");
                assert!(Accent::ALL.contains(&look.accent));
                assert!(look.scale.factor() >= 1.0 && look.scale.factor() <= 2.0);
                // It can be used and saved again.
                let again = ron::to_string(&look).unwrap();
                assert_eq!(ron::from_str::<InterfaceLook>(&again).unwrap(), look);
            }
            Err(_) => failed += 1,
        }
        // The earlier accents' reader meets the same damage.
        let _ = ron::from_str::<AccentBefore>(&text);
    }
    assert!(read > 100, "only {read} damaged looks still read");
    assert!(failed > 100, "only {failed} were refused");
}

#[test]
fn the_look_in_use_is_this_thread_s_own() {
    use_look(changed());
    assert_eq!(in_use(), changed());
    assert_eq!(accent_fill(), Accent::Purple.colour());
    assert_eq!(tokens::size::row_h(), 22.0);
    assert_eq!(tokens::size::shelf_tab_h(), 24.0);
    assert_eq!(selected(), tokens::select_item(Accent::Purple));
    assert_eq!(selected_border(), tokens::open_card_border(Accent::Purple));
    assert_eq!(notice(), tokens::changes_pill(Accent::Purple));
    // A thread where none was chosen has the standard one.
    std::thread::spawn(|| {
        assert_eq!(in_use(), InterfaceLook::default());
        assert_eq!(tokens::size::row_h(), 24.0);
    })
    .join()
    .unwrap();
    use_look(InterfaceLook::default());
}
