//! Typing out (stage 1, text effects): when letters show, pauses and
//! speeds, the same letters at the same moments at any frame rate, and
//! the moments a typing sound plays on.

use super::*;
use crate::markup::parse;

#[test]
fn letters_show_one_after_another_at_the_speed() {
    let t = TypeOut::of(&parse("abcd"), 10.0);
    let at: Vec<f64> = t.moments.iter().map(|m| m.at).collect();
    assert_eq!(at, [0.0, 0.1, 0.2, 0.30000000000000004]);
    assert_eq!(t.shown(0.0), 1, "the first at once");
    assert_eq!(t.shown(0.15), 2);
    assert_eq!(t.shown(5.0), 4);
    assert!(!t.finished(0.2) && t.finished(0.31));
}

#[test]
fn speed_zero_shows_everything_at_once() {
    let t = TypeOut::of(&parse("all of it"), 0.0);
    assert!(!t.waits());
    assert_eq!(t.shown(0.0), 9);
    assert!(t.finished(0.0));
    let nonsense = TypeOut::of(&parse("odd"), f32::NAN);
    assert!(!nonsense.waits(), "not a number is at once");
}

#[test]
fn a_pause_waits_and_a_speed_changes_the_pace() {
    let t = TypeOut::of(&parse("ab[pause 1]c[speed 2]de[speed 0]fg"), 10.0);
    let at: Vec<f64> = t
        .moments
        .iter()
        .map(|m| (m.at * 1000.0).round() / 1000.0)
        .collect();
    // a 0, b 0.1, then a second's pause: c 1.2; twice as fast: d 1.3,
    // e 1.35; at once: f and g with e's next moment.
    assert_eq!(at, [0.0, 0.1, 1.2, 1.3, 1.35, 1.4, 1.4]);
}

#[test]
fn a_pause_at_the_end_counts_before_it_s_finished() {
    let t = TypeOut::of(&parse("hi[pause 2]"), 10.0);
    assert_eq!(t.shown(0.2), 2);
    assert!(!t.finished(1.0), "still pausing");
    assert!(t.finished(2.3));
}

#[test]
fn whole_letters_show_even_where_markup_was() {
    let m = parse("[wave]hey[/wave] you");
    let t = TypeOut::of(&m, 4.0);
    assert!(t.shows(0, 0.0));
    assert!(!t.shows(1, 0.0));
    assert!(t.shows(1, 0.25));
    assert!(t.shows(99, 0.0), "past the end (\"…\") shows");
    assert!(t.shows(6, f64::NAN), "no start: whole");
}

/// The letters shown on each frame of `fps` frames a second for `seconds`,
/// by the UI clock: whole ticks of 1/60 s plus how far into the next, as
/// the editor's game clock counts them.
fn frames(t: &TypeOut, fps: u32, seconds: f64) -> Vec<(f64, usize)> {
    let tick = 1.0 / 60.0;
    let (mut ticks, mut carry) = (0u64, 0.0f64);
    let mut out = Vec::new();
    for _ in 0..(seconds * f64::from(fps)) as usize {
        carry += 1.0 / f64::from(fps);
        while carry >= tick {
            carry -= tick;
            ticks += 1;
        }
        let now = (ticks as f64 + carry / tick) * tick;
        out.push((now, t.shown(now)));
    }
    out
}

#[test]
fn the_same_letters_show_at_the_same_moments_at_30_and_144_frames_a_second() {
    let t = TypeOut::of(
        &parse("Oh! A new face. [pause 0.37]Have you seen my [speed 2]roses?"),
        29.0,
    );
    let slow = frames(&t, 30, 3.0);
    let fast = frames(&t, 144, 3.0);
    // Every moment both draw a frame at shows the same letters.
    let mut shared = 0;
    for (now, shown) in &slow {
        // (A letter due exactly then could fall either side of a rounding.)
        let on_a_moment = t.moments.iter().any(|m| (m.at - now).abs() < 1e-6);
        if on_a_moment {
            continue;
        }
        if let Some((_, other)) = fast.iter().find(|(n, _)| (n - now).abs() < 1e-9) {
            assert_eq!(shown, other, "at {now}");
            shared += 1;
        }
    }
    assert!(shared >= 10, "{shared} moments shared");
    // And each letter first shows on the first frame at or after its moment.
    for (fps, run) in [(30, &slow), (144, &fast)] {
        for m in t.moments.iter().filter(|m| m.at > 0.0) {
            let first = run.iter().find(|(now, _)| t.shows(m.byte, *now)).unwrap().0;
            assert!(
                first + 1e-9 >= m.at && first < m.at + 1.0 / f64::from(fps) + 1e-9,
                "{fps}: {first} for {}",
                m.at
            );
        }
    }
}

#[test]
fn a_sound_plays_on_letters_or_on_words() {
    let t = TypeOut::of(&parse("ab cd"), 10.0);
    // Letters at 0, 0.1, (space 0.2), 0.3, 0.4.
    assert!(t.sounds_between(-1.0, 0.0, SoundEvery::Letter));
    assert!(
        !t.sounds_between(0.15, 0.25, SoundEvery::Letter),
        "a space is silent"
    );
    assert!(
        t.sounds_between(0.25, 0.31, SoundEvery::Word),
        "c starts a word"
    );
    assert!(!t.sounds_between(0.35, 0.45, SoundEvery::Word), "d doesn't");
    assert!(t.sounds_between(0.35, 0.45, SoundEvery::Letter));
}

#[test]
fn an_icon_is_a_letter_that_types_out() {
    let t = TypeOut::of(&parse("5 [icon=Picture.Ui.Coin]"), 10.0);
    assert_eq!(t.moments.len(), 3);
    assert!(t.moments[2].letter && t.moments[2].word);
}

#[test]
fn starts_give_how_long_each_text_has_typed() {
    let s = Started {
        at: vec![1.0, f64::NEG_INFINITY],
    };
    assert_eq!(s.elapsed(0, 3.5), 2.5);
    assert!(s.elapsed(0, 0.5) == 0.0, "never before it began");
    assert!(s.elapsed(1, 3.0).is_nan(), "skipped: whole");
    assert!(s.elapsed(7, 3.0).is_nan(), "no start: whole");
}
