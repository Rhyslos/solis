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

/// Where each element went: [x, y, w, h].
fn rects(elements: &[Element]) -> Vec<[f32; 4]> {
    elements
        .iter()
        .map(|e| [e.offset[0], e.offset[1], e.size[0], e.size[1]])
        .collect()
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
    let out = lay_out(&view, [400.0, 300.0], &mut Fixed);
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
        "line", "grid", "",
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
