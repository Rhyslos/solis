use super::*;
use crate::screen::{Anchor, Colour, Edge, Element, ElementKind, Fill, ScaleRule, Screen};

const INTER: &[u8] = include_bytes!("../../editor/assets/fonts/Inter-Regular.ttf");

fn fonts() -> Fonts {
    let (fonts, notes) = Fonts::new(vec![INTER.to_vec()]);
    assert!(notes.is_empty(), "{notes:?}");
    fonts
}

#[test]
fn a_letter_is_made_once_into_the_atlas() {
    let mut f = fonts();
    let first = f.place(0, "Spring", 24.0, [10.0, 10.0], None);
    assert_eq!(first.len(), 6);
    assert_eq!(f.letters.len(), 6, "six different letters, each made once");
    let made = f.atlas.revision;
    let again = f.place(0, "Spring", 48.0, [0.0, 0.0], None);
    assert_eq!(f.atlas.revision, made, "nothing made again at another size");
    assert_eq!(f.letters.len(), 6);
    // Twice the size: twice as big, the same place in the atlas.
    let (a, b) = (first[0], again[0]);
    assert_eq!(a.atlas, b.atlas);
    let wa = a.rect[2] - a.rect[0];
    let wb = b.rect[2] - b.rect[0];
    assert!((wb - 2.0 * wa).abs() < 0.01, "{wa} {wb}");
}

#[test]
fn no_two_font_sets_share_an_atlas_revision() {
    // The renderer keeps one letters' atlas on the card and sends another
    // when the revision differs: two sets of fonts with as many letters
    // made used to have the same one, and the second drew its text from
    // the first's letters.
    let (mut a, mut b) = (fonts(), fonts());
    assert_ne!(a.atlas.revision, b.atlas.revision, "empty");
    let _ = a.place(0, "Spring", 24.0, [0.0, 0.0], None);
    let _ = b.place(0, "Winter", 24.0, [0.0, 0.0], None);
    assert_ne!(a.atlas.revision, b.atlas.revision, "six letters each");
    // A copy changes apart from its original.
    let mut c = a.clone();
    let _ = c.place(0, "z", 24.0, [0.0, 0.0], None);
    let _ = a.place(0, "z", 24.0, [0.0, 0.0], None);
    assert_ne!(a.atlas.revision, c.atlas.revision);
}

#[test]
fn letters_run_left_to_right_and_lines_down() {
    let mut f = fonts();
    let one = f.place(0, "ab", 32.0, [0.0, 0.0], None);
    assert!(one[1].rect[0] > one[0].rect[0]);
    let two = f.place(0, "a\nb", 32.0, [0.0, 0.0], None);
    assert!(two[1].rect[1] > two[0].rect[3] - 4.0, "b on the next line");
    let [w, h] = f.measure(0, "ab", 32.0);
    assert!(w > 20.0 && w < 50.0, "{w}");
    let [_, h2] = f.measure(0, "a\nb", 32.0);
    assert!((h2 - 2.0 * h).abs() < 0.01);
    // A space has no letter, but moves the pen.
    let spaced = f.place(0, "a b", 32.0, [0.0, 0.0], None);
    assert_eq!(spaced.len(), 2);
    assert!(spaced[1].rect[0] > one[1].rect[0]);
}

#[test]
fn measuring_reads_each_character_from_the_font_once() {
    // The second review pass: every word measured read the font's tables
    // again. Now the common characters are read with the font, and each
    // other one the first time it's measured, then remembered.
    let mut f = fonts();
    let text = "Spring → spring ←→";
    let first = f.measure(0, text, 24.0);
    assert_eq!(f.fonts[0].remembered(), [2, 0], "the two arrows");
    assert_eq!(f.measure(0, text, 24.0), first, "the same the second time");
    assert_eq!(f.fonts[0].remembered(), [2, 0], "nothing read again");
    assert_eq!(f.place(0, text, 24.0, [0.0, 0.0], None).len(), 15);
    assert_eq!(f.fonts[0].remembered(), [2, 0]);
    // As wide as the font says, letter by letter (Inter kerns in another
    // table, so no pairs are looked up).
    use skrifa::MetadataProvider as _;
    let face = skrifa::FontRef::new(INTER).unwrap();
    let (charmap, advances) = (
        face.charmap(),
        face.glyph_metrics(
            skrifa::instance::Size::unscaled(),
            skrifa::instance::LocationRef::default(),
        ),
    );
    let units: f32 = text
        .chars()
        .map(|c| advances.advance_width(charmap.map(c).unwrap()).unwrap())
        .sum();
    let scale = f.fonts[0].at(24.0).px(1.0);
    assert!((first[0] - units * scale).abs() < 0.01, "{first:?} {units}");
}

#[test]
fn what_a_font_remembers_has_a_limit() {
    // Text of every character there is can't grow it forever.
    let mut f = fonts();
    let every: Vec<String> = ('\u{800}'..='\u{2_ffff}')
        .collect::<Vec<char>>()
        .chunks(4000)
        .map(|line| line.iter().collect())
        .collect();
    let every = every.join("\n");
    let _ = f.measure(0, &every, 12.0);
    assert!(f.fonts[0].remembered()[0] <= crate::font::MAX_REMEMBERED);
    let wide = f.measure(0, "Wy→", 30.0);
    let _ = f.measure(0, &every, 12.0);
    assert!(f.fonts[0].remembered()[0] <= crate::font::MAX_REMEMBERED);
    // What it measures stays the same after starting again.
    assert_eq!(f.measure(0, "Wy→", 30.0), wide);
}

#[test]
fn the_distance_image_is_inside_out_and_edge_in_the_middle() {
    let mut f = fonts();
    let placed = f.place(0, "I", 48.0, [0.0, 0.0], None)[0];
    let [l, t, r, b] = placed.atlas.map(u32::from);
    let at = |x: u32, y: u32| f.atlas.pixels[(y * ATLAS_SIDE + x) as usize];
    // The middle of the I is inside (more than 128), its corner outside.
    assert!(at((l + r) / 2, (t + b) / 2) > 140);
    assert!(at(l, t) < 40);
}

#[test]
fn crisp_text_lands_on_whole_steps() {
    let mut f = fonts();
    for p in f.place(0, "Spring 3", 64.0, [3.0, 7.0], Some(4.0)) {
        assert_eq!(p.rect[0] % 4.0, 0.0);
        assert_eq!(p.rect[1] % 4.0, 0.0);
    }
}

#[test]
fn broken_fonts_and_nonsense_sizes_are_safe() {
    let (mut none, notes) = Fonts::new(vec![b"not a font".to_vec(), vec![0; MAX_FONT_BYTES + 1]]);
    assert_eq!(notes.len(), 2, "{notes:?}");
    assert!(none.is_empty());
    assert!(none.place(0, "hello", 20.0, [0.0, 0.0], None).is_empty());
    assert_eq!(none.measure(0, "hello", 20.0), [0.0, 0.0]);
    let mut f = fonts();
    for size in [f32::NAN, f32::INFINITY, -5.0, 0.0, 1.0e9] {
        for p in f.place(0, "Wy", size, [0.0, 0.0], None) {
            assert!(p.rect.iter().all(|v| v.is_finite()), "{size}");
        }
    }
    // Font 5 of 1: the first.
    assert_eq!(f.place(5, "a", 20.0, [0.0, 0.0], None).len(), 1);
    let long = "x".repeat(MAX_LETTERS * 2);
    assert_eq!(f.place(0, &long, 10.0, [0.0, 0.0], None).len(), MAX_LETTERS);
}

#[test]
fn damaged_fonts_never_crash_and_never_give_a_shape_that_isnt_finite() {
    // A fuzz sweep (rule 9): Inter damaged in many ways, the same ways on
    // every run (a fixed sequence, no chance): bytes changed in its
    // tables' directory and anywhere, and cut short. Each is read, or left
    // out with a note; what's read lays text out with finite boxes.
    let mut state = 0x5eed_u64;
    let mut next = move |below: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % below as u64) as usize
    };
    for case in 0..120 {
        let mut bytes = INTER.to_vec();
        for _ in 0..1 + case % 8 {
            // Half the changes in the first kilobyte: the directory and
            // the small tables it points to first.
            let reach = if next(2) == 0 { 1024 } else { bytes.len() };
            let at = next(reach);
            bytes[at] = next(256) as u8;
        }
        if case % 5 == 0 {
            bytes.truncate(next(bytes.len()));
        }
        let (mut f, notes) = Fonts::new(vec![bytes]);
        assert_eq!(f.len() + notes.len(), 1, "case {case}");
        f.begin_frame();
        for p in f.place(0, "Ag→ fi", 24.0, [0.0, 0.0], None) {
            assert!(p.rect.iter().all(|v| v.is_finite()), "case {case}");
        }
        assert!(f.measure(0, "Ag→ fi", 24.0).iter().all(|v| v.is_finite()));
    }
}

/// Every letter Inter has, big: more than the atlas holds.
fn every_letter() -> String {
    (0x20u32..0x2500).filter_map(char::from_u32).collect()
}

/// The atlas's pixels inside `rect`, row by row.
fn atlas_part(f: &Fonts, rect: [u16; 4]) -> Vec<u8> {
    let side = ATLAS_SIDE as usize;
    let [l, t, r, b] = rect.map(usize::from);
    (t..b)
        .flat_map(|y| f.atlas.pixels[y * side + l..y * side + r].iter().copied())
        .collect()
}

#[test]
fn a_full_atlas_says_so_and_makes_room_the_next_frame() {
    let mut f = fonts();
    f.begin_frame();
    let _ = f.place(0, &every_letter(), 20.0, [0.0, 0.0], None);
    assert!(f.full);
    // That frame showed every letter in the atlas, so nothing is let go
    // yet: an arrow, near the end, still doesn't fit.
    f.begin_frame();
    let placed = f.place(0, "→ still fine", 20.0, [0.0, 0.0], None);
    assert_eq!(placed.len(), "stillfine".chars().count());
    // This frame showed only these letters: the next lets go of the
    // rest, so the arrow gets a place (it used to be lost for good).
    f.begin_frame();
    let placed = f.place(0, "→ still fine", 20.0, [0.0, 0.0], None);
    assert_eq!(
        placed.len(),
        "→stillfine".chars().count(),
        "every letter has a place"
    );
}

#[test]
fn letters_placed_in_a_frame_keep_their_place_when_the_atlas_fills() {
    // A frame's text is laid out a piece at a time, and drawn after: the
    // atlas must not change under letters already placed (it used to be
    // wiped by the next piece of text once it filled).
    let mut f = fonts();
    f.begin_frame();
    let big = f.place(0, &every_letter(), 20.0, [0.0, 0.0], None);
    assert!(f.full);
    let before: Vec<Vec<u8>> = big.iter().map(|p| atlas_part(&f, p.atlas)).collect();
    let revision = f.atlas.revision;
    let score = f.place(0, "Score 120", 24.0, [0.0, 0.0], None);
    assert_eq!(score.len(), "Score120".chars().count());
    let after: Vec<Vec<u8>> = big.iter().map(|p| atlas_part(&f, p.atlas)).collect();
    assert!(
        before == after,
        "the letters placed this frame are where they were"
    );
    assert_eq!(f.atlas.revision, revision);
}

#[test]
fn a_frame_that_needs_more_letters_than_fit_makes_none_again() {
    // Every frame wants more letters than fit: they are left out, and
    // the atlas stays as it is, rather than being made again (and sent
    // to the card again) every frame.
    let mut f = fonts();
    let all = every_letter();
    f.begin_frame();
    let first = f.place(0, &all, 20.0, [0.0, 0.0], None);
    let revision = f.atlas.revision;
    for _ in 0..3 {
        f.begin_frame();
        let again = f.place(0, &all, 20.0, [0.0, 0.0], None);
        assert_eq!(f.atlas.revision, revision, "nothing made again");
        assert!(again == first, "the same letters in the same places");
    }
}

#[test]
fn making_room_keeps_the_letters_last_shown_as_they_were() {
    let mut f = fonts();
    f.begin_frame();
    let _ = f.place(0, &every_letter(), 20.0, [0.0, 0.0], None);
    // A frame showing a score, and an arrow that doesn't fit.
    f.begin_frame();
    let score = f.place(0, "Score 120", 24.0, [0.0, 0.0], None);
    assert!(f.place(0, "→", 24.0, [0.0, 0.0], None).is_empty());
    let before: Vec<Vec<u8>> = score.iter().map(|p| atlas_part(&f, p.atlas)).collect();
    // The next frame lets go of every other letter: the score's are
    // moved together, their images as they were, and the arrow fits.
    f.begin_frame();
    let moved = f.place(0, "Score 120", 24.0, [0.0, 0.0], None);
    let after: Vec<Vec<u8>> = moved.iter().map(|p| atlas_part(&f, p.atlas)).collect();
    assert!(before == after, "copied, not changed");
    for (was, now) in score.iter().zip(&moved) {
        assert_eq!(was.rect, now.rect, "on the screen where they were");
    }
    assert_eq!(f.place(0, "→", 24.0, [0.0, 0.0], None).len(), 1);
}

#[test]
fn text_elements_are_laid_out_as_letters_aligned_in_their_box() {
    let mut f = fonts();
    let text = |align| {
        Element::new(
            "label",
            ElementKind::Text {
                text: "Hi".into(),
                size: 40.0,
                font: 0,
                align,
            },
        )
        .at(Anchor::TOP_LEFT, [100.0, 100.0])
        .sized([400.0, 100.0])
        .filled(Fill::Solid(Colour::WHITE))
    };
    let lay = |f: &mut Fonts, align| {
        let mut screen = Screen::new("t", [1000.0, 1000.0], ScaleRule::Keep);
        screen.elements = vec![text(align)];
        crate::layout_with(&screen, [1000, 1000], None, Some(f))
    };
    let left = lay(&mut f, Edge::Start);
    let middle = lay(&mut f, Edge::Middle);
    let right = lay(&mut f, Edge::End);
    assert_eq!(left.len(), 2);
    assert!(left.iter().all(|s| s.kind == crate::ShapeKind::Letter));
    assert!(
        left[0].rect[0] >= 95.0 && left[0].rect[0] < 110.0,
        "{:?}",
        left[0].rect
    );
    assert!(
        middle[0].rect[0] > 250.0 && middle[0].rect[0] < 300.0,
        "{:?}",
        middle[0].rect
    );
    assert!(
        right[1].rect[2] > 480.0 && right[1].rect[2] <= 506.0,
        "{:?}",
        right[1].rect
    );
    // Vertically in the middle of the box (100 to 200).
    let mid = (left[0].rect[1] + left[0].rect[3]) / 2.0;
    assert!(mid > 130.0 && mid < 170.0, "{mid}");
    // Without fonts, text is left out.
    let mut screen = Screen::new("t", [1000.0, 1000.0], ScaleRule::Keep);
    screen.elements = vec![text(Edge::Start)];
    assert!(crate::layout(&screen, [1000, 1000], None).is_empty());
}
