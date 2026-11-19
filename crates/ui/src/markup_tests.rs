//! Markup in a screen's text (stage 1, text effects): every word read,
//! mistakes found where they are and shown as written, and round trips.

use super::*;

fn run(from: u32, to: u32, effect: Effect) -> Run {
    Run { from, to, effect }
}

#[test]
fn plain_text_is_left_as_it_is() {
    let m = parse("Hello, traveller.");
    assert_eq!(m.plain, "Hello, traveller.");
    assert!(m.is_plain() && m.mistakes.is_empty());
}

#[test]
fn each_word_is_read_into_plain_text_and_what_it_does() {
    let m = parse(
        "Oh! [wave]hello[/wave] [shake]there[/shake] [colour=#ff8800]red[/colour] [b]big[/b] [i]lean[/i]",
    );
    assert_eq!(m.plain, "Oh! hello there red big lean");
    assert!(m.mistakes.is_empty(), "{:?}", m.mistakes);
    assert_eq!(
        m.runs,
        [
            run(4, 9, Effect::Wave),
            run(10, 15, Effect::Shake),
            run(16, 19, Effect::Colour(0xff8800)),
            run(20, 23, Effect::Bold),
            run(24, 28, Effect::Italic),
        ]
    );
}

#[test]
fn colours_are_the_screen_s_words_or_hex() {
    let m = parse("[colour=yellow5]a[/colour][colour=#abc]b[/colour][colour teal7]c[/colour]");
    assert!(m.mistakes.is_empty(), "{:?}", m.mistakes);
    let colours: Vec<Effect> = m.runs.iter().map(|r| r.effect).collect();
    assert_eq!(
        colours,
        [
            Effect::Colour(0xfcc419),
            Effect::Colour(0xaabbcc),
            Effect::Colour(0x1098ad)
        ]
    );
}

#[test]
fn icons_stand_in_the_text_as_one_character() {
    let m = parse("Costs 5 [icon=Picture.Ui.Coin] each");
    assert_eq!(m.plain, format!("Costs 5 {ICON} each"));
    assert_eq!(
        m.icons,
        [Icon {
            at: 8,
            name: "Picture.Ui.Coin".into()
        }]
    );
}

#[test]
fn pauses_and_speeds_come_before_the_letter_after_them() {
    let m = parse("Well[pause 0.5]... [speed 2]quickly[speed 1] now");
    assert_eq!(m.plain, "Well... quickly now");
    assert_eq!(
        m.beats,
        [
            (4, Beat::Pause(0.5)),
            (8, Beat::Speed(2.0)),
            (15, Beat::Speed(1.0))
        ]
    );
}

#[test]
fn two_brackets_write_one() {
    let m = parse("Press [[E]] to open [[ or close");
    assert_eq!(m.plain, "Press [E]] to open [ or close");
    assert!(m.mistakes.is_empty());
}

#[test]
fn effects_nest_and_overlap() {
    let m = parse("[wave]a[b]b[/wave]c[/b]");
    assert_eq!(m.plain, "abc");
    assert_eq!(m.runs, [run(0, 2, Effect::Wave), run(1, 3, Effect::Bold)]);
    assert!(m.mistakes.is_empty());
}

#[test]
fn an_unknown_word_is_a_mistake_where_it_is_and_shown_as_written() {
    let m = parse("Hi [glow]there[/glow]");
    assert_eq!(m.plain, "Hi [glow]there[/glow]", "never hidden");
    assert_eq!(m.mistakes.len(), 2);
    assert_eq!(m.mistakes[0].at, 3);
    assert!(
        m.mistakes[0].message.contains("isn't a text effect"),
        "{}",
        m.mistakes[0].message
    );
    assert!(
        m.mistakes[0].message.contains("[["),
        "says how to write a ["
    );
    assert!(m.mistakes[1].message.contains("closes nothing"));
}

#[test]
fn a_word_never_closed_is_a_mistake_and_runs_to_the_end() {
    let m = parse("a [wave]long line");
    assert_eq!(m.plain, "a long line");
    assert_eq!(m.runs, [run(2, 11, Effect::Wave)]);
    assert_eq!(m.mistakes.len(), 1);
    assert!(m.mistakes[0].message.contains("never closed"));
    assert_eq!(
        m.mistakes[0].at, 17,
        "said at the end, where [/wave] is missing"
    );
}

#[test]
fn a_bracket_with_no_end_is_a_mistake() {
    let m = parse("look [here");
    assert_eq!(m.plain, "look [here");
    assert_eq!(m.mistakes.len(), 1);
    assert_eq!(m.mistakes[0].at, 5);
}

#[test]
fn mistakes_say_what_to_write() {
    let says = |text: &str, words: &str| {
        let m = parse(text);
        assert_eq!(m.mistakes.len(), 1, "{text}: {:?}", m.mistakes);
        assert!(
            m.mistakes[0].message.contains(words),
            "{text}: {}",
            m.mistakes[0].message
        );
    };
    says("[color=red]x", "[colour=");
    says("[colour=gold]x", "isn't a colour");
    says("[icon=Coin]", "Picture.Ui.Coin");
    says("[pause]", "seconds");
    says("[pause 999]", "seconds");
    says("[speed fast]", "[speed 2]");
    says("[button=Action.Use]", "later milestone");
    says("[wave 3]x", "takes nothing");
    says("[bold]x", "[b]");
    says("[/i]", "closes nothing");
}

#[test]
fn a_text_with_its_own_stand_in_character_has_no_icon() {
    let m = parse(&format!("odd {ICON} [b]x[/b]"));
    assert!(m.icons.is_empty());
    assert!(!m.plain.contains(ICON));
}

#[test]
fn too_many_words_are_shown_as_written() {
    let text = "[b]x[/b]".repeat(MAX_WORDS);
    let m = parse(&text);
    assert_eq!(m.runs.len(), MAX_WORDS / 2);
    assert!(m.plain.contains("[b]"), "the rest as written");
    assert_eq!(m.mistakes.len(), 1);
}

#[test]
fn deep_effects_stop_at_the_limit() {
    let text = "[b]".repeat(40) + "x";
    let m = parse(&text);
    assert!(m.runs.len() <= 16);
    assert!(!m.mistakes.is_empty());
}

#[test]
fn written_back_it_reads_the_same() {
    for text in [
        "plain",
        "Oh! [wave]hello[/wave] [colour=#ff8800]red [b]roses[/b][/colour]?",
        "[wave]a[b]b[/wave]c[/b]",
        "[b][i]both[/i][/b] and [i][b]both[/b][/i]",
        "Costs 5 [icon=Picture.Ui.Coin] [[each]",
        "Well[pause 0.5]... [speed 2]fast[speed 1][pause 1]",
        "[shake]shaking [icon=Picture.Ui.Skull][/shake]",
        "",
    ] {
        let m = parse(text);
        assert!(m.mistakes.is_empty(), "{text}: {:?}", m.mistakes);
        let again = parse(&write(&m));
        assert_eq!(again, m, "{text} → {}", write(&m));
    }
}

proptest::proptest! {
    #[test]
    fn any_text_reads_without_panicking_and_writes_back_the_same(
        text in "(\\[|\\]|/|=|wave|shake|colour|b|i|icon|Picture\\.Ui\\.Coin|pause|speed| |0\\.5|2|#fc0|red|a|é|\n|\u{FFFC}){0,40}"
    ) {
        let m = parse(&text);
        proptest::prop_assert!(m.runs.iter().all(|r| r.from < r.to && r.to as usize <= m.plain.len()));
        proptest::prop_assert!(m.runs.iter().all(|r| m.plain.is_char_boundary(r.from as usize) && m.plain.is_char_boundary(r.to as usize)));
        proptest::prop_assert_eq!(m.icons.len(), m.plain.matches(ICON).count());
        proptest::prop_assert!(m.mistakes.iter().all(|e| e.at as usize <= text.chars().count()));
        // What it means is kept by writing it back.
        let again = parse(&write(&m));
        proptest::prop_assert_eq!(&again.plain, &m.plain);
        proptest::prop_assert_eq!(&again.runs, &m.runs);
        proptest::prop_assert_eq!(&again.icons, &m.icons);
        proptest::prop_assert_eq!(&again.beats, &m.beats);
    }
}
