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
    let made = f.atlas.revision;
    assert_eq!(made, 6, "six different letters, each made once");
    let again = f.place(0, "Spring", 48.0, [0.0, 0.0], None);
    assert_eq!(f.atlas.revision, made, "nothing made again at another size");
    // Twice the size: twice as big, the same place in the atlas.
    let (a, b) = (first[0], again[0]);
    assert_eq!(a.atlas, b.atlas);
    let wa = a.rect[2] - a.rect[0];
    let wb = b.rect[2] - b.rect[0];
    assert!((wb - 2.0 * wa).abs() < 0.01, "{wa} {wb}");
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
fn a_full_atlas_says_so_and_carries_on() {
    let mut f = fonts();
    // Every letter Inter has, big: more than fit.
    let all: String = (0x20u32..0x2500).filter_map(char::from_u32).collect();
    let _ = f.place(0, &all, 20.0, [0.0, 0.0], None);
    assert!(f.full);
    let _ = f.place(0, "still fine", 20.0, [0.0, 0.0], None);
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
