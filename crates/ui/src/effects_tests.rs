//! Text effects on laid-out letters (stage 1, text effects): waves and
//! shakes move letters but never the layout, colours, bold and slant,
//! icons inside text, typing out, and 2,000 letters within the frame.

use super::*;
use crate::layout::{ShapeKind, layout_with};
use crate::screen::{ScaleRule, Screen};
use crate::text::Fonts;
use crate::typing::Started;
use crate::view::{Node, Style};
use std::sync::Arc;

const INTER: &[u8] = include_bytes!("../../editor/assets/fonts/Inter-Regular.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../../editor/assets/fonts/Inter-SemiBold.ttf");

fn fonts() -> Fonts {
    Fonts::new(vec![INTER.to_vec(), INTER_BOLD.to_vec()]).0
}

/// A view of one text, 30 design pixels tall, in a 600-wide column.
fn view_of(words: &str) -> Node {
    let text = Node {
        kind: "text".into(),
        text: words.into(),
        styles: vec![("size".into(), Style::Number(30.0))],
        ..Node::default()
    };
    Node {
        kind: "view".into(),
        children: vec![Node {
            kind: "column".into(),
            styles: vec![("width".into(), Style::Number(600.0))],
            children: vec![text],
            ..Node::default()
        }],
        ..Node::default()
    }
}

/// A screen showing `words`, its texts typing out at `speed`, with a
/// 32 × 16 picture `Picture.Coin`.
fn screen_of(words: &str, speed: f32) -> Screen {
    let mut pictures = crate::Pictures::new();
    pictures
        .add("Picture.Coin", [32, 16], &[200; 32 * 16 * 4])
        .unwrap();
    let mut screen = Screen::new("Talk", [800.0, 400.0], ScaleRule::Keep);
    screen.view = Some(view_of(words));
    screen.pictures = Some(Arc::new(pictures));
    screen.look.typing_speed = speed;
    screen
}

fn letters(shapes: &[crate::Shape]) -> Vec<&crate::Shape> {
    shapes
        .iter()
        .filter(|s| s.kind == ShapeKind::Letter)
        .collect()
}

fn at(fonts: &mut Fonts, screen: &Screen, now: f64) -> Vec<crate::Shape> {
    fonts.set_clock(now);
    fonts.begin_frame();
    layout_with(screen, [800, 400], None, Some(fonts))
}

#[test]
fn a_wave_lifts_letters_by_turns_and_a_shake_jumps_them() {
    let effects = TextEffects {
        runs: vec![
            Run {
                from: 0,
                to: 4,
                effect: Effect::Wave,
            },
            Run {
                from: 4,
                to: 8,
                effect: Effect::Shake,
            },
        ],
        ..TextEffects::default()
    };
    let wave: Vec<f32> = (0..4)
        .map(|i| letter_look(&effects, i, i as usize, 0.3).moved[1])
        .collect();
    assert!(wave.iter().all(|y| y.abs() <= WAVE_HEIGHT + 1e-6));
    assert!(
        wave.windows(2).all(|w| w[0] != w[1]),
        "each letter further along"
    );
    assert_ne!(
        letter_look(&effects, 0, 0, 0.3),
        letter_look(&effects, 0, 0, 0.5),
        "it moves"
    );
    let shake = |now| letter_look(&effects, 5, 5, now).moved;
    assert_eq!(
        shake(0.01),
        shake(0.02),
        "the same jump within a 24th of a second"
    );
    assert_ne!(shake(0.01), shake(0.2));
    assert!(shake(0.3).iter().all(|v| v.abs() <= SHAKE_REACH));
    assert_eq!(
        letter_look(&effects, 9, 9, 0.3),
        LetterLook::default(),
        "past the runs"
    );
}

#[test]
fn the_innermost_colour_wins_and_bold_and_slant_are_said() {
    let m = crate::markup::parse("[colour=red][b]a[colour=blue]b[/colour][/b][i]c[/i][/colour]");
    let e = TextEffects::of(&m, 0, None).unwrap();
    assert_eq!(letter_look(&e, 0, 0, 0.0).colour, Some(0xe03131));
    assert!(letter_look(&e, 0, 0, 0.0).bold);
    assert_eq!(letter_look(&e, 1, 1, 0.0).colour, Some(0x1c7ed6));
    assert!(letter_look(&e, 2, 2, 0.0).slant);
}

#[test]
fn plain_text_carries_no_effects() {
    assert!(TextEffects::of(&crate::markup::parse("just words"), 0, None).is_none());
    assert!(TextEffects::of(&crate::markup::parse("just words"), 0, Some(0.0)).is_none());
    assert!(TextEffects::of(&crate::markup::parse("typed"), 0, Some(20.0)).is_some());
}

#[test]
fn waves_and_shakes_never_move_the_layout() {
    let mut f = fonts();
    let plain = screen_of(
        "Have you seen my red roses? They wiggle when it rains.",
        0.0,
    );
    let marked = screen_of(
        "Have you seen my [shake]red roses[/shake]? They [wave]wiggle[/wave] when it rains.",
        0.0,
    );
    // Every piece's box is where it is without the markup, at any time.
    let boxes = |s: &Screen, f: &mut Fonts| crate::layout::boxes(s, [800, 400], None, f);
    let still = boxes(&plain, &mut f);
    for now in [0.0, 0.37, 1.9] {
        f.set_clock(now);
        assert_eq!(boxes(&marked, &mut f), still, "at {now}");
    }
    // The letters outside the effects are exactly where plain text puts
    // them; inside, only moved.
    let a = at(&mut f, &plain, 0.0);
    let b = at(&mut f, &marked, 0.61);
    let (a, b) = (letters(&a), letters(&b));
    assert_eq!(a.len(), b.len());
    let moved = a.iter().zip(&b).filter(|(x, y)| x.rect != y.rect).count();
    assert!(
        moved > 0 && moved <= "red roses".len() - 1 + "wiggle".len(),
        "{moved}"
    );
    for (x, y) in a.iter().zip(&b) {
        let w = |r: [f32; 4]| [r[2] - r[0], r[3] - r[1]];
        assert!(
            (w(x.rect)[0] - w(y.rect)[0]).abs() < 1e-3,
            "the same size, only moved"
        );
    }
    // And the paragraph itself is one kept layout, the same each frame.
    let kept = f.paragraphs_kept();
    let _ = at(&mut f, &marked, 0.9);
    assert_eq!(
        f.paragraphs_kept(),
        kept,
        "laid out once, not again each frame"
    );
}

#[test]
fn coloured_bold_and_slanted_letters_are_drawn_so() {
    let mut f = fonts();
    let s = screen_of("[colour=#ff8800]red[/colour] [b]big[/b] [i]lean[/i]", 0.0);
    let shapes = at(&mut f, &s, 0.0);
    let l = letters(&shapes);
    assert_eq!(l.len(), 10);
    assert_eq!(l[0].colours[0], crate::Colour::rgb(0xff, 0x88, 0x00));
    assert_ne!(
        l[3].colours[0], l[0].colours[0],
        "the rest in the text's colour"
    );
    assert_eq!(l[3].letter[0], 0.0, "the game's own bold, not thickened");
    assert!(
        l[6].letter[1] > 0.0 && l[0].letter[1] == 0.0,
        "only [i] leans"
    );
    // Bold is the game's bold, which is wider: the line is wider.
    let plain = at(&mut f, &screen_of("red big lean", 0.0), 0.0);
    let pl = letters(&plain);
    let gap = |v: &[&crate::Shape]| v[5].rect[0] - v[3].rect[0];
    assert!(gap(&l) > gap(&pl), "bold letters take more room");
}

#[test]
fn an_icon_is_a_picture_the_height_of_its_text_in_line_with_the_words() {
    let mut f = fonts();
    let s = screen_of("Costs 5 [icon=Picture.Coin] each", 0.0);
    let shapes = at(&mut f, &s, 0.0);
    let pictures: Vec<_> = shapes
        .iter()
        .filter(|s| s.kind == ShapeKind::Picture)
        .collect();
    assert_eq!(pictures.len(), 1);
    let p = pictures[0].rect;
    let (w, h) = (p[2] - p[0], p[3] - p[1]);
    assert!((h - 30.0).abs() < 0.5, "as tall as the text: {h}");
    assert!(
        (w - 60.0).abs() < 0.5,
        "as wide as the picture's shape (twice as wide as tall): {w}"
    );
    let l = letters(&shapes);
    let five = l[5].rect;
    let e = l[6].rect;
    assert!(
        p[0] > five[2] && e[0] > p[2],
        "between its words: {five:?} {p:?} {e:?}"
    );
    // A missing picture is the grey box with its name, as for an image.
    let gone = screen_of("[icon=Picture.Gone]", 0.0);
    let shapes = at(&mut f, &gone, 0.0);
    assert!(shapes.iter().any(|s| s.kind == ShapeKind::Box));
}

#[test]
fn a_typing_text_shows_its_letters_as_its_time_comes() {
    let mut f = fonts();
    let mut s = screen_of("Hello there", 10.0);
    // Without a start (the UI editor's canvas), every letter shows.
    assert_eq!(letters(&at(&mut f, &s, 0.0)).len(), 10);
    s.typing = Some(Arc::new(Started { at: vec![2.0] }));
    let count = |f: &mut Fonts, s: &Screen, now| letters(&at(f, s, now)).len();
    assert_eq!(count(&mut f, &s, 2.0), 1, "H at once");
    assert_eq!(count(&mut f, &s, 2.45), 5, "Hello");
    assert_eq!(count(&mut f, &s, 2.55), 5, "the space shows nothing");
    assert_eq!(count(&mut f, &s, 9.0), 10);
    // Skipped: whole at once.
    s.typing = Some(Arc::new(Started {
        at: vec![f64::NEG_INFINITY],
    }));
    assert_eq!(count(&mut f, &s, 2.0), 10);
    // Typing never moves the letters shown, nor the layout.
    s.typing = Some(Arc::new(Started { at: vec![0.0] }));
    let half = at(&mut f, &s, 0.25);
    let whole = at(&mut f, &s, 5.0);
    let (half, whole) = (letters(&half), letters(&whole));
    for (a, b) in half.iter().zip(&whole) {
        assert_eq!(a.rect, b.rect);
    }
}

#[test]
fn only_text_pieces_type_out_and_each_has_its_number() {
    let view = Node {
        kind: "view".into(),
        children: vec![
            Node {
                kind: "text".into(),
                text: "first".into(),
                ..Node::default()
            },
            Node {
                kind: "button".into(),
                text: "Go".into(),
                ..Node::default()
            },
            Node {
                kind: "text".into(),
                text: "[wave]second[/wave]".into(),
                ..Node::default()
            },
        ],
        ..Node::default()
    };
    assert_eq!(
        crate::view::texts_of(&view),
        ["first", "[wave]second[/wave]"]
    );
    let looks = crate::view::Looks {
        typing_speed: 20.0,
        ..Default::default()
    };
    let (elements, _) =
        crate::view::lay_out_with(&view, [400.0, 300.0], &mut crate::view::NoFonts, looks);
    let effects: Vec<(String, Option<u32>, bool)> = elements
        .iter()
        .filter_map(|e| match &e.kind {
            crate::ElementKind::Text { text, effects, .. } => Some((
                text.clone(),
                effects.as_ref().map(|e| e.number),
                effects.as_ref().is_some_and(|e| e.typing.is_some()),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(
        effects,
        [
            ("first".to_string(), Some(0), true),
            ("Go".to_string(), None, false),
            ("second".to_string(), Some(1), true),
        ]
    );
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_2000_letters_waving_shaking_and_typing_out() {
    // Stage 1's text effects: a screen of 2,000 letters, half waving and
    // half shaking, typing out, laid out and drawn each frame within the
    // frame's share for text.
    const TARGET_MS: f64 = 4.0;
    let line = "[wave]Waves roll over the sea[/wave] and [shake]the old boat shakes[/shake]. ";
    let words = line.repeat(52);
    let mut f = fonts();
    let mut s = screen_of(&words, 2400.0);
    s.view = Some(Node {
        kind: "view".into(),
        children: vec![Node {
            kind: "column".into(),
            styles: vec![("width".into(), Style::Number(1800.0))],
            children: vec![Node {
                kind: "text".into(),
                text: words.clone(),
                styles: vec![("size".into(), Style::Number(16.0))],
                ..Node::default()
            }],
            ..Node::default()
        }],
        ..Node::default()
    });
    s.typing = Some(Arc::new(Started { at: vec![0.0] }));
    let shown = letters(&at(&mut f, &s, 100.0)).len();
    println!("{shown} letters");
    assert!(shown >= 2000, "{shown} letters");
    let mut best = f64::MAX;
    for frame in 1..=20 {
        // Still typing: about 3,600 characters at 2,400 a second.
        let now = 1.2 + f64::from(frame) / 60.0;
        let start = std::time::Instant::now();
        let shapes = at(&mut f, &s, now);
        best = best.min(start.elapsed().as_secs_f64() * 1000.0);
        assert!(!shapes.is_empty());
    }
    let verdict = if best <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: 2,000 letters waving, shaking and typing out {best:.3} ms (target {TARGET_MS} ms) {verdict}"
    );
    assert!(best <= TARGET_MS, "{best:.3} ms");
}
