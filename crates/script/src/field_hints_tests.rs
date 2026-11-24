//! The hint words, read and put on fields.

use super::*;
use reflect::{FieldInfo, FieldKind, Hint, Value};

fn float() -> FieldInfo {
    FieldInfo::new("speed", "Speed", FieldKind::Float, Value::Float(4.0))
}

#[test]
fn every_word_is_read() {
    let (words, mistakes) = read(
        "range 0 10, step 0.5, unit \"m/s\", tip \"Walking \\\"speed\\\"\", group \"Moving\", \
         multiline, hidden",
    );
    assert!(mistakes.is_empty(), "{mistakes:?}");
    assert_eq!(
        words,
        [
            Word::Range(0.0, 10.0),
            Word::Step(0.5),
            Word::Unit("m/s".into()),
            Word::Tip("Walking \"speed\"".into()),
            Word::Group("Moving".into()),
            Word::Multiline,
            Word::Hidden,
        ]
    );
    let (words, _) = read("range -1.5 2e3");
    assert_eq!(words, [Word::Range(-1.5, 2000.0)]);
    assert_eq!(read("").0, []);
    assert_eq!(read(" , ,").0, [], "empty items are nothing");
}

#[test]
fn each_mistake_says_what_to_write_and_leaves_the_rest() {
    let cases = [
        (
            "range 10 0",
            "must be smaller than the second: range 0 10, not range 10 0",
        ),
        ("range 1 1", "must be smaller"),
        ("range 1", "range takes two numbers"),
        ("range a b", "range takes two numbers"),
        ("step 0", "step must be more than 0"),
        ("step -1", "step must be more than 0"),
        ("step", "step takes one number"),
        ("unit m", "unit takes a short unit in quotes"),
        ("tip", "tip takes a sentence in quotes"),
        ("hidden 3", "hidden takes nothing after it"),
        ("speeed 3", "speeed isn't a hint Solis knows: range, step"),
        ("3 range", "each hint starts with its word"),
        ("tip \"no end", "no closing \""),
        ("range 1x 2", "1x isn't a number"),
        ("range 0 1e999", "isn't a number"),
        ("tip @", "@ has no place in a hint"),
        ("range 0 1, range 0 2", "range is given twice"),
    ];
    for (text, said) in cases {
        let (_, mistakes) = read(text);
        assert!(
            mistakes.iter().any(|m| m.contains(said)),
            "{text}: {mistakes:?}"
        );
    }
    let (words, mistakes) = read("speeed 3, unit \"m\"");
    assert_eq!(words, [Word::Unit("m".into())], "the good one is kept");
    assert_eq!(mistakes.len(), 1);
    let long = format!("tip \"{}\"", "a".repeat(MAX_HINT_TEXT + 1));
    assert!(read(&long).1[0].contains("longer than"));
}

#[test]
fn words_fit_only_their_kinds() {
    let mut field = float();
    let mistakes = apply(
        &mut field,
        vec![
            Word::Range(0.0, 10.0),
            Word::Unit("m".into()),
            Word::Multiline,
        ],
        "float",
    );
    assert_eq!(field.hint.range, Some((0.0, 10.0)));
    assert!(field.hint.slider, "a number with a range is a slider");
    assert_eq!(field.hint.unit, " m");
    assert!(!field.hint.multiline);
    assert_eq!(
        mistakes,
        ["multiline is for text (a string), and speed is a float"]
    );

    let mut place = FieldInfo::new("home", "Home", FieldKind::Vec3, Value::floats(&[0.0; 3]));
    assert!(apply(&mut place, vec![Word::Range(-5.0, 5.0)], "vec3").is_empty());
    assert!(!place.hint.slider, "three numbers are dragged, not slid");

    let mut on = FieldInfo::new("on", "On", FieldKind::Bool, Value::Bool(true));
    let mistakes = apply(
        &mut on,
        vec![Word::Step(1.0), Word::Hidden, Word::Tip("t".into())],
        "bool",
    );
    assert_eq!(
        mistakes,
        ["step is for numbers (int, float, vec2 and vec3), and on is a bool"]
    );
    assert!(on.hint.hidden);
    assert_eq!(on.hint.tip, "t");
    assert_eq!(on.hint.help, "t", "pointing at the name says it too");

    let mut count = FieldInfo::new("count", "Count", FieldKind::Int, Value::Int(3));
    let mistakes = apply(&mut count, vec![Word::Step(0.5)], "int");
    assert_eq!(
        mistakes,
        ["count is a whole number, so its step must be one too: step 1"]
    );
    assert!(apply(&mut count, vec![Word::Step(2.0)], "int").is_empty());
    assert_eq!(count.hint.step, Some(2.0));
}

#[test]
fn a_start_outside_the_range_is_said() {
    let mut field = float();
    assert_eq!(outside_range(&field), None, "no range");
    apply(&mut field, vec![Word::Range(0.0, 10.0)], "float");
    assert_eq!(outside_range(&field), None);
    field.default = Value::Float(12.0);
    assert!(
        outside_range(&field)
            .is_some_and(|m| m.contains("speed starts outside its range (0 to 10)"))
    );
    let mut place = FieldInfo::new("p", "P", FieldKind::Vec2, Value::floats(&[1.0, 20.0]));
    apply(&mut place, vec![Word::Range(0.0, 10.0)], "vec2");
    assert!(outside_range(&place).is_some());
}

#[test]
fn no_words_change_nothing() {
    let mut field = float();
    assert!(apply(&mut field, Vec::new(), "float").is_empty());
    assert_eq!(field.hint, Hint::default());
}

proptest::proptest! {
    /// Any text between the brackets reads without a crash, and every
    /// word read fits the limits.
    #[test]
    fn any_hint_text_reads(text in "\\PC{0,120}") {
        let (words, _) = read(&text);
        for word in words {
            match word {
                Word::Range(low, high) => proptest::prop_assert!(low < high),
                Word::Step(step) => proptest::prop_assert!(step > 0.0),
                Word::Unit(t) | Word::Tip(t) | Word::Group(t) => {
                    proptest::prop_assert!(t.chars().count() <= MAX_HINT_TEXT);
                }
                Word::Multiline | Word::Hidden | Word::Toggle | Word::Checkbox => {}
            }
        }
    }
}

/// A yes or no can be shown as a switch (decision 0137): `toggle`, and
/// `checkbox` for the way it is anyway; on another kind, a mistake.
#[test]
fn a_bool_can_ask_for_a_switch_and_nothing_else_can() {
    let (words, mistakes) = read("toggle");
    assert!(mistakes.is_empty());
    let mut open = FieldInfo::new("open", "Open", FieldKind::Bool, reflect::Value::Bool(false));
    assert!(apply(&mut open, words, "bool").is_empty());
    assert!(open.hint.toggle);
    let (words, _) = read("checkbox");
    assert!(apply(&mut open, words, "bool").is_empty());
    assert!(!open.hint.toggle);
    let (words, _) = read("toggle");
    let mut speed = float();
    let said = apply(&mut speed, words, "float");
    assert_eq!(
        said,
        ["toggle is for a yes or no (a bool), and speed is a float"]
    );
    assert_eq!(read("toggle 3").1, ["toggle takes nothing after it"]);
}
