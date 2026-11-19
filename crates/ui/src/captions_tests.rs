//! The caption screen: lines shown for a time or until stopped, by kind
//! as the player chose, the newest few, laid out low in the middle on a
//! box as dark as asked.

use super::*;
use crate::layout::{ShapeKind, layout_with};
use crate::text::Fonts;

const INTER: &[u8] = include_bytes!("../../editor/assets/fonts/Inter-Regular.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../../editor/assets/fonts/Inter-SemiBold.ttf");

fn both() -> Look {
    Look {
        captions: true,
        ..Look::default()
    }
}

#[test]
fn a_line_shows_for_its_time_or_until_stopped() {
    let mut c = Captions::default();
    let creak = c.show(Kind::Sound, "", "[The door creaks]", 1.0, Some(2.0));
    let voice = c.show(Kind::Subtitle, "Gardener", "Oh! A new face.", 1.0, None);
    c.tick(2.9);
    assert_eq!(c.shown(&both()).len(), 2);
    c.tick(3.0);
    assert_eq!(
        c.shown(&both()).iter().map(|l| l.id).collect::<Vec<_>>(),
        [voice]
    );
    c.tick(1_000.0);
    assert_eq!(c.lines().len(), 1, "kept until told to stop");
    c.stop(voice);
    c.stop(creak);
    assert!(c.lines().is_empty());
    assert!(c.screen(&both()).is_none(), "nothing to show: no screen");
}

#[test]
fn the_players_choices_pick_what_shows() {
    let mut c = Captions::default();
    c.show(Kind::Sound, "", "[Footsteps]", 0.0, Some(3.0));
    c.show(Kind::Subtitle, "Ada", "Hello.", 0.0, Some(3.0));
    let off = Look {
        captions: false,
        subtitles: false,
        ..Look::default()
    };
    assert!(c.screen(&off).is_none(), "captions off: no caption");
    let subtitles = Look::default();
    let shown: Vec<&str> = c
        .shown(&subtitles)
        .iter()
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(shown, ["Hello."], "captions for sounds start off");
    assert_eq!(c.shown(&both()).len(), 2);
}

#[test]
fn the_same_words_twice_are_kept_up_and_only_the_newest_show() {
    let mut c = Captions::default();
    let a = c.show(Kind::Sound, "", "[Footsteps]", 0.0, Some(1.0));
    let b = c.show(Kind::Sound, "", "[Footsteps]", 0.5, Some(1.0));
    assert_eq!(a, b);
    c.tick(1.2);
    assert_eq!(c.lines().len(), 1, "kept up for longer");
    for n in 0..40 {
        c.show(Kind::Sound, "", &format!("[Sound {n}]"), 2.0, Some(5.0));
    }
    assert_eq!(c.lines().len(), MAX_KEPT);
    let shown: Vec<&str> = c.shown(&both()).iter().map(|l| l.text.as_str()).collect();
    assert_eq!(shown, ["[Sound 37]", "[Sound 38]", "[Sound 39]"]);
    let long = c.show(Kind::Sound, "", &"x".repeat(MAX_CHARS * 2), 0.0, None);
    assert!(
        c.lines()
            .iter()
            .any(|l| l.id == long && l.text.len() == MAX_CHARS)
    );
}

#[test]
fn the_screen_lays_its_lines_out_low_in_the_middle_as_big_and_dark_as_asked() {
    let (mut fonts, _) = Fonts::new(vec![INTER.to_vec(), INTER_BOLD.to_vec()]);
    let mut c = Captions::default();
    c.show(
        Kind::Subtitle,
        "Gardener",
        "Oh! A new face in the village.",
        0.0,
        None,
    );
    let lay = |look: &Look, fonts: &mut Fonts| {
        fonts.begin_frame();
        layout_with(&c.screen(look).unwrap(), [1920, 1080], None, Some(fonts))
    };
    let shapes = lay(&both(), &mut fonts);
    let letters: Vec<_> = shapes
        .iter()
        .filter(|s| s.kind == ShapeKind::Letter)
        .collect();
    assert!(letters.len() > 20, "{} letters", letters.len());
    let top = letters.iter().map(|s| s.rect[1]).fold(f32::MAX, f32::min);
    let left = letters.iter().map(|s| s.rect[0]).fold(f32::MAX, f32::min);
    let right = letters.iter().map(|s| s.rect[2]).fold(f32::MIN, f32::max);
    assert!(top > 900.0, "low on the screen: {top}");
    assert!(
        (left + right) / 2.0 > 860.0 && (left + right) / 2.0 < 1060.0,
        "in the middle"
    );
    let shade = shapes.iter().find(|s| s.kind == ShapeKind::Box).unwrap();
    assert!(
        (shade.colours[0].0[3] - 0.6).abs() < 0.01,
        "as dark as asked"
    );
    assert!(
        shade.rect[0] < left && shade.rect[2] > right,
        "behind the words"
    );
    let big = lay(
        &Look {
            size: 2.0,
            ..both()
        },
        &mut fonts,
    );
    let tall = |s: &[crate::layout::Shape]| {
        let l: Vec<_> = s.iter().filter(|s| s.kind == ShapeKind::Letter).collect();
        l.iter().map(|s| s.rect[3] - s.rect[1]).fold(0.0, f32::max)
    };
    assert!(tall(&big) > tall(&shapes) * 1.6, "twice as big");
    let clear = lay(
        &Look {
            background: 0.0,
            ..both()
        },
        &mut fonts,
    );
    assert!(
        clear
            .iter()
            .filter(|s| s.kind == ShapeKind::Box)
            .all(|s| s.colours[0].0[3] < 0.01)
    );
}
