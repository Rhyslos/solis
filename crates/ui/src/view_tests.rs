//! A Lust view laid out with Lust's rules (decision 0087).

use super::*;

/// Text as wide as 10 design pixels a letter, 20 a line.
struct Fixed;

impl Measure for Fixed {
    fn measure(&mut self, _: usize, text: &str, _: f32) -> [f32; 2] {
        let lines = text.split('\n').count() as f32;
        let widest = text
            .split('\n')
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0);
        [widest as f32 * 10.0, lines * 20.0]
    }
}

fn node(kind: &str, styles: &[(&str, Style)], children: Vec<Node>) -> Node {
    Node {
        kind: kind.into(),
        styles: styles
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
        children,
        ..Node::default()
    }
}

fn text(t: &str, styles: &[(&str, Style)]) -> Node {
    Node {
        text: t.into(),
        ..node("text", styles, vec![])
    }
}

fn n(v: f64) -> Style {
    Style::Number(v)
}

fn on() -> Style {
    Style::Flag(true)
}

/// Where each element went: [x, y, w, h], those in a canvas's group
/// (which cuts them off) put where they are on the screen.
fn rects(elements: &[Element]) -> Vec<[f32; 4]> {
    flat(elements)
        .iter()
        .map(|e| [e.offset[0], e.offset[1], e.size[0], e.size[1]])
        .collect()
}

/// The elements, a canvas's group's own in its place, moved where they
/// are on the screen.
fn flat(elements: &[Element]) -> Vec<Element> {
    let mut out = Vec::new();
    for e in elements {
        if e.clips {
            for mut c in flat(&e.children) {
                c.offset = [c.offset[0] + e.offset[0], c.offset[1] + e.offset[1]];
                out.push(c);
            }
        } else {
            out.push(e.clone());
        }
    }
    out
}

#[test]
fn a_column_stacks_its_pieces_with_padding_and_gaps() {
    let view = node(
        "view",
        &[],
        vec![node(
            "column",
            &[("pad", n(10.0)), ("gap", n(5.0))],
            vec![text("ab", &[]), text("abc", &[])],
        )],
    );
    let out = lay_out(&view, [400.0, 300.0], &mut Fixed);
    assert_eq!(
        rects(&out),
        vec![[10.0, 10.0, 20.0, 20.0], [10.0, 35.0, 30.0, 20.0]]
    );
    let ElementKind::Text { text, .. } = &out[1].kind else {
        panic!("text");
    };
    assert_eq!(text, "abc");
}

#[test]
fn a_full_screen_stack_pins_its_child_to_a_corner() {
    // The HUD's way to anchor: a stack as big as the screen, told where.
    let view = node(
        "view",
        &[],
        vec![node(
            "stack",
            &[
                ("width", Style::Text("fill".into())),
                ("height", Style::Text("fill".into())),
                ("bottom", on()),
                ("right", on()),
                ("pad", n(48.0)),
            ],
            vec![text("Ammo", &[])],
        )],
    );
    let out = lay_out(&view, [1920.0, 1080.0], &mut Fixed);
    assert_eq!(
        rects(&out),
        vec![[1920.0 - 48.0 - 40.0, 1080.0 - 48.0 - 20.0, 40.0, 20.0]]
    );
    // On an ultrawide screen it moves out to the edge.
    let out = lay_out(&view, [2520.0, 1080.0], &mut Fixed);
    assert_eq!(out[0].offset[0], 2520.0 - 48.0 - 40.0);
}

#[test]
fn a_row_shares_its_room_and_centres_up_and_down() {
    let view = node(
        "view",
        &[],
        vec![node(
            "row",
            &[("gap", n(10.0)), ("height", n(100.0))],
            vec![
                text("a", &[]),
                node("space", &[], vec![]),
                node(
                    "box",
                    &[("width", n(30.0)), ("height", n(40.0)), ("red5", on())],
                    vec![],
                ),
            ],
        )],
    );
    let out = lay_out(&view, [200.0, 100.0], &mut Fixed);
    // The text, then the box pushed to the right edge, both in the middle.
    assert_eq!(
        rects(&out),
        vec![[0.0, 40.0, 10.0, 20.0], [170.0, 30.0, 30.0, 40.0]]
    );
    assert_eq!(out[1].fill, Fill::Solid(Colour::rgb(0xff, 0x6b, 0x6b)));
}

#[test]
fn a_canvas_places_shapes_where_they_say() {
    let view = node(
        "view",
        &[],
        vec![node(
            "canvas",
            &[("width", n(200.0)), ("height", n(100.0))],
            vec![
                node(
                    "box",
                    &[
                        ("x", n(10.0)),
                        ("y", n(20.0)),
                        ("width", n(40.0)),
                        ("height", n(60.0)),
                        ("round", n(6.0)),
                    ],
                    vec![],
                ),
                node(
                    "circle",
                    &[
                        ("x", n(150.0)),
                        ("y", n(50.0)),
                        ("radius", n(10.0)),
                        ("blue", on()),
                    ],
                    vec![],
                ),
                node(
                    "line",
                    &[
                        ("x", n(0.0)),
                        ("y", n(100.0)),
                        ("toX", n(200.0)),
                        ("toY", n(20.0)),
                        ("thick", n(3.0)),
                    ],
                    vec![],
                ),
            ],
        )],
    );
    let out = flat(&lay_out(&view, [400.0, 300.0], &mut Fixed));
    assert_eq!(
        rects(&out),
        vec![
            [10.0, 20.0, 40.0, 60.0],
            [140.0, 40.0, 20.0, 20.0],
            [0.0, 20.0, 200.0, 80.0]
        ]
    );
    assert_eq!(out[0].kind, ElementKind::Box { round: 6.0 });
    assert_eq!(out[1].kind, ElementKind::Ellipse);
    assert_eq!(
        out[2].kind,
        ElementKind::Line {
            width: 3.0,
            rising: true
        }
    );
}

#[test]
fn a_progress_bar_fills_its_share() {
    let mut bar = node(
        "progress",
        &[("to", n(20.0)), ("width", n(300.0)), ("green", on())],
        vec![],
    );
    bar.value = Some(5.0);
    let out = lay_out(&node("view", &[], vec![bar]), [400.0, 300.0], &mut Fixed);
    let ElementKind::Bar { value, round, .. } = out[0].kind else {
        panic!("a bar");
    };
    assert_eq!(value, 0.25);
    assert_eq!(round, 4.0, "a pill, 8 tall");
    assert_eq!(out[0].fill, Fill::Solid(Colour::rgb(0x2f, 0x9e, 0x44)));
}

#[test]
fn text_wraps_at_the_room_it_has_and_bold_picks_the_second_font() {
    let view = node(
        "view",
        &[],
        vec![node(
            "column",
            &[("width", n(50.0))],
            vec![text("ab cd ef", &[("bold", on())])],
        )],
    );
    let out = lay_out(&view, [400.0, 300.0], &mut Fixed);
    let ElementKind::Text { text, font, .. } = &out[0].kind else {
        panic!("text");
    };
    assert_eq!(text, "ab cd\nef");
    assert_eq!(*font, 1);
    assert_eq!(out[0].size, [50.0, 40.0]);
}

#[test]
fn a_background_and_opacity_carry_down() {
    let view = node(
        "view",
        &[],
        vec![node(
            "column",
            &[
                ("background", Style::Text("#000".into())),
                ("opacity", n(0.5)),
                ("pad", n(4.0)),
            ],
            vec![text("a", &[("white", on())])],
        )],
    );
    let out = lay_out(&view, [100.0, 100.0], &mut Fixed);
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].fill, Fill::Solid(Colour([0.0, 0.0, 0.0, 0.5])));
    assert_eq!(out[1].fill, Fill::Solid(Colour([1.0, 1.0, 1.0, 0.5])));
    // A style that's off (bold done, when not done) is left off.
    let off = node(
        "view",
        &[],
        vec![text("a", &[("bold", Style::Flag(false))])],
    );
    let ElementKind::Text { font, .. } = lay_out(&off, [100.0, 100.0], &mut Fixed)[0].kind else {
        panic!("text");
    };
    assert_eq!(font, 0);
}

#[test]
fn the_design_size_comes_from_the_view() {
    let view = node("view", &[("width", n(480.0)), ("height", n(270.0))], vec![]);
    assert_eq!(design_of(&view), [480.0, 270.0]);
    assert_eq!(design_of(&node("view", &[], vec![])), [1920.0, 1080.0]);
    assert_eq!(
        design_of(&node("view", &[("height", n(-3.0))], vec![])),
        [1920.0, 1080.0]
    );
}

#[test]
fn hostile_trees_are_kept_within_limits() {
    // Deep: cut at the deepest a screen goes.
    let mut deep = text("x", &[]);
    for _ in 0..500 {
        deep = node("column", &[("pad", n(1.0))], vec![deep]);
    }
    let out = lay_out(&node("view", &[], vec![deep]), [100.0, 100.0], &mut Fixed);
    assert!(out.len() <= MAX_ELEMENTS);
    // Wide: no more than the most elements.
    let wide = node(
        "row",
        &[],
        (0..30_000).map(|_| node("box", &[], vec![])).collect(),
    );
    let out = lay_out(&node("view", &[], vec![wide]), [100.0, 100.0], &mut Fixed);
    assert!(out.len() <= MAX_ELEMENTS);
    // Odd numbers: nothing infinite or not-a-number comes out.
    let odd = node(
        "column",
        &[
            ("pad", n(f64::NAN)),
            ("width", n(f64::INFINITY)),
            ("gap", n(-1e30)),
            ("size", n(1e30)),
            ("opacity", n(-4.0)),
        ],
        vec![
            text("a", &[]),
            node("progress", &[("from", n(5.0)), ("to", n(5.0))], vec![]),
        ],
    );
    for e in lay_out(&node("view", &[], vec![odd]), [1e30, f32::NAN], &mut Fixed) {
        assert!(
            e.offset.iter().chain(&e.size).all(|v| v.is_finite()),
            "{e:?}"
        );
    }
    // Unknown pieces are laid out as columns and draw nothing of their own.
    let odd = node("dropdown", &[], vec![text("in it", &[])]);
    assert_eq!(
        lay_out(&node("view", &[], vec![odd]), [100.0, 100.0], &mut Fixed).len(),
        1
    );
}

#[test]
fn colour_words_and_hex_are_lust_s() {
    assert_eq!(colour("white"), Some(0xffffff));
    assert_eq!(colour("red5"), Some(0xff6b6b));
    assert_eq!(colour("#0f0"), Some(0x00ff00));
    assert_eq!(colour("#12345"), None);
    assert_eq!(colour("nope"), None);
}

#[test]
fn random_trees_never_crash_or_give_odd_numbers() {
    // A fuzz test of sorts: many trees made from a fixed seed, every kind
    // and style mixed in, laid out at odd sizes.
    let kinds = [
        "view", "column", "row", "stack", "space", "text", "progress", "canvas", "box", "circle",
        "line", "grid", "", "button",
    ];
    let words = [
        "pad",
        "padX",
        "gap",
        "width",
        "height",
        "grow",
        "minWidth",
        "maxHeight",
        "size",
        "bold",
        "center",
        "right",
        "bottom",
        "middle",
        "spread",
        "round",
        "pill",
        "border",
        "opacity",
        "x",
        "y",
        "toX",
        "toY",
        "radius",
        "thick",
        "from",
        "to",
        "red5",
        "nowrap",
        "background",
    ];
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    let mut next = move |n: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % n as u64) as usize
    };
    fn grow(
        next: &mut dyn FnMut(usize) -> usize,
        depth: usize,
        kinds: &[&str],
        words: &[&str],
    ) -> Node {
        let values = [
            Style::Flag(true),
            Style::Flag(false),
            Style::Number(0.0),
            Style::Number(-50.0),
            Style::Number(37.5),
            Style::Number(1e9),
            Style::Number(f64::NAN),
            Style::Text("fill".into()),
            Style::Text("blue".into()),
            Style::Text("#zzz".into()),
        ];
        let styles = (0..next(6))
            .map(|_| {
                (
                    words[next(words.len())].to_string(),
                    values[next(values.len())].clone(),
                )
            })
            .collect();
        let children = if depth < 6 {
            (0..next(5))
                .map(|_| grow(next, depth + 1, kinds, words))
                .collect()
        } else {
            Vec::new()
        };
        Node {
            kind: kinds[next(kinds.len())].to_string(),
            text: ["", "a b c", "long words wrap here\nand here", "é😀"][next(4)].to_string(),
            value: [None, Some(5.0), Some(f64::INFINITY)][next(3)],
            styles,
            children,
            action: [None, Some(0), Some(9_999)][next(3)],
        }
    }
    for _ in 0..300 {
        let tree = grow(&mut next, 0, &kinds, &words);
        let size = [
            [0.0, 0.0],
            [1920.0, 1080.0],
            [5.0, 3000.0],
            [f32::INFINITY, -1.0],
        ][next(4)];
        for e in lay_out(&tree, size, &mut Fixed) {
            assert!(
                e.offset.iter().chain(&e.size).all(|v| v.is_finite()),
                "{e:?}"
            );
        }
    }
}

fn button(label: &str, action: usize, styles: &[(&str, Style)]) -> Node {
    Node {
        text: label.into(),
        action: Some(action),
        ..node("button", styles, vec![])
    }
}

#[test]
fn buttons_are_laid_out_as_lust_s_and_can_be_hit() {
    let view = node(
        "view",
        &[],
        vec![node(
            "column",
            &[("gap", n(10.0))],
            vec![
                button("Resume", 0, &[("blue", on())]),
                button("Quit", 1, &[("disabled", on())]),
            ],
        )],
    );
    let (out, clickable) = lay_out_with(&view, [400.0, 300.0], &mut Fixed, Looks::default());
    // Its padding round the words, filled with its colour, the words white.
    assert_eq!(out[0].size, [88.0, 34.0], "its words and padding");
    assert_eq!(out[0].fill, Fill::Solid(Colour::rgb(0x1c, 0x7e, 0xd6)));
    let ElementKind::Text { text, align, .. } = &out[1].kind else {
        panic!("its words");
    };
    assert_eq!((text.as_str(), *align), ("Resume", Edge::Middle));
    assert_eq!(out[1].fill, Fill::Solid(Colour::WHITE));
    // Only the one that isn't disabled can be clicked; it's found there.
    assert_eq!(clickable.len(), 1);
    assert_eq!(hit(&clickable, [40.0, 17.0]), Some(0));
    assert_eq!(hit(&clickable, [30.0, 60.0]), None, "the disabled one");
    assert_eq!(hit(&clickable, [200.0, 200.0]), None);
}

#[test]
fn hover_and_focus_change_how_a_button_looks() {
    let hover_red = (
        "hover",
        Style::Nested(vec![("background".into(), Style::Text("red5".into()))]),
    );
    let view = node(
        "view",
        &[],
        vec![button("Go", 0, &[hover_red]), button("Stay", 1, &[])],
    );
    let plain = lay_out_with(&view, [400.0, 300.0], &mut Fixed, Looks::default()).0;
    let hovered = lay_out_with(
        &view,
        [400.0, 300.0],
        &mut Fixed,
        Looks {
            hover: Some(0),
            focus: None,
        },
    )
    .0;
    assert_eq!(hovered[0].fill, Fill::Solid(Colour::rgb(0xff, 0x6b, 0x6b)));
    assert_ne!(plain[0].fill, hovered[0].fill);
    // Without a hover look of its own, a little darker.
    let darker = lay_out_with(
        &view,
        [400.0, 300.0],
        &mut Fixed,
        Looks {
            hover: Some(1),
            focus: None,
        },
    )
    .0;
    assert_ne!(darker[2].fill, plain[2].fill);
    // The focus ring: one more element, round the focused button.
    let focused = lay_out_with(
        &view,
        [400.0, 300.0],
        &mut Fixed,
        Looks {
            hover: None,
            focus: Some(1),
        },
    )
    .0;
    assert_eq!(focused.len(), plain.len() + 1);
}

#[test]
fn a_view_with_a_button_anywhere_inside_has_actions() {
    let plain = node("column", &[], vec![text("Hi", &[])]);
    assert!(!has_actions(&plain));
    let deep = node(
        "column",
        &[],
        vec![node("row", &[], vec![button("Go", 0, &[])])],
    );
    assert!(has_actions(&deep));
}

#[test]
fn a_screen_s_button_is_found_where_its_picture_is_drawn() {
    use crate::layout::{clickable_at, picture_area};
    // A 2D game's 320 × 180 picture on a 1000 × 700 panel: whole steps of
    // 3, with bars round it.
    let area = picture_area([320, 180], [1000, 700], true);
    assert!(area.whole);
    assert_eq!(
        (area.scale, area.size, area.origin),
        (3.0, [960, 540], [20, 80])
    );
    let filling = picture_area([320, 180], [1000, 700], false);
    assert!(!filling.whole && filling.size[0] == 1000);
    // Without whole steps it fills; a screen smaller than the picture
    // shrinks it.
    let small = picture_area([320, 180], [100, 100], true);
    assert!(small.size[0] <= 100 && small.size[1] <= 100);
    let view = node(
        "stack",
        &[
            ("width", Style::Text("fill".into())),
            ("height", Style::Text("fill".into())),
        ],
        vec![button(
            "Go",
            0,
            &[
                ("width", Style::Text("fill".into())),
                ("height", Style::Text("fill".into())),
            ],
        )],
    );
    let screen = crate::Screen {
        name: "Menu".into(),
        design: [320.0, 180.0],
        scale: crate::ScaleRule::WholeSteps,
        elements: Vec::new(),
        view: Some(view),
        looks: Looks::default(),
    };
    let at = |p: [f32; 2]| clickable_at(&screen, area.size, Some(3), p, &mut Fixed);
    assert_eq!(at([480.0, 270.0]), Some(0));
    assert_eq!(at([-1.0, 10.0]), None);
    assert_eq!(at([961.0, 10.0]), None);
    assert_eq!(at([f32::NAN, 10.0]), None);
}

#[test]
fn the_focus_moves_to_the_nearest_button_that_way() {
    let at = |action: usize, x: f32, y: f32| Clickable {
        rect: [x, y, x + 100.0, y + 40.0],
        action,
    };
    // A 3 × 2 grid, numbered across then down.
    let grid: Vec<Clickable> = (0..6)
        .map(|i| at(i, (i % 3) as f32 * 120.0, (i / 3) as f32 * 60.0))
        .collect();
    assert_eq!(
        next_focus(&grid, None, [1.0, 0.0]),
        Some(0),
        "the first to start"
    );
    assert_eq!(
        next_focus(&grid, Some(99), [0.0, 1.0]),
        Some(0),
        "gone: the first"
    );
    assert_eq!(next_focus(&grid, Some(0), [1.0, 0.0]), Some(1));
    assert_eq!(next_focus(&grid, Some(1), [0.0, 1.0]), Some(4));
    assert_eq!(next_focus(&grid, Some(4), [-1.0, 0.0]), Some(3));
    assert_eq!(
        next_focus(&grid, Some(2), [1.0, 0.0]),
        Some(2),
        "nothing that way"
    );
    assert_eq!(next_focus(&grid, Some(3), [0.0, -1.0]), Some(0));
    assert_eq!(next_focus(&grid, Some(3), [f32::NAN, 1.0]), Some(3));
    assert_eq!(next_focus(&grid, Some(3), [0.0, 0.0]), Some(3));
    // An odd layout: a wide button under two small ones goes up to the
    // nearer of them, and back down to it from either.
    let odd = vec![
        at(0, 0.0, 0.0),
        at(1, 300.0, 0.0),
        Clickable {
            rect: [0.0, 100.0, 200.0, 140.0],
            action: 2,
        },
    ];
    assert_eq!(next_focus(&odd, Some(2), [0.0, -1.0]), Some(0));
    assert_eq!(next_focus(&odd, Some(1), [0.0, 1.0]), Some(2));
    assert_eq!(next_focus(&[], None, [1.0, 0.0]), None);
}

#[test]
fn a_canvas_cuts_off_its_shapes_at_its_edges() {
    let shape = |kind: &str, x: f64, y: f64| {
        node(
            kind,
            &[
                ("x", n(x)),
                ("y", n(y)),
                ("width", n(40.0)),
                ("height", n(40.0)),
                ("background", Style::Text("red5".into())),
            ],
            vec![],
        )
    };
    let view = node(
        "view",
        &[],
        vec![node(
            "column",
            &[("pad", n(10.0))],
            vec![node(
                "canvas",
                &[("width", n(100.0)), ("height", n(50.0))],
                vec![shape("box", 80.0, 30.0)],
            )],
        )],
    );
    let screen = crate::Screen {
        name: "Canvas".into(),
        design: [400.0, 300.0],
        scale: crate::ScaleRule::Keep,
        elements: Vec::new(),
        view: Some(view),
        looks: Looks::default(),
    };
    let shapes = crate::layout(&screen, [400, 300], None);
    let boxed = shapes
        .iter()
        .find(|s| s.rect[0] > 80.0)
        .unwrap_or_else(|| panic!("the box: {shapes:?}"));
    assert_eq!(boxed.rect, [90.0, 40.0, 130.0, 80.0]);
    assert_eq!(boxed.clip, [10.0, 10.0, 110.0, 60.0], "the canvas's box");
    // Laid out inside part of a screen, the cut moves with it.
    let moved = crate::layout_at(&screen, [5, 7], [400, 300], None, None);
    let boxed = moved.iter().find(|s| s.rect[0] > 85.0).unwrap();
    assert_eq!(boxed.clip, [15.0, 17.0, 115.0, 67.0]);
    // Outside a canvas nothing is cut.
    let plain = crate::Screen {
        view: Some(node("column", &[], vec![shape("box", 0.0, 0.0)])),
        ..screen
    };
    assert!(
        crate::layout(&plain, [400, 300], None)
            .iter()
            .all(|s| s.clip == crate::UNCLIPPED)
    );
}
