use super::*;
use crate::presets::PRESETS;
use crate::sample::hud;
use crate::screen::{Anchor, Edge, Element, ElementKind, ScaleRule, Screen};

fn rect_of(shapes: &[Shape], i: usize) -> [f32; 4] {
    shapes[i].rect
}

fn close(a: [f32; 4], b: [f32; 4]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.01)
}

#[test]
fn corners_stay_in_their_corners_on_every_screen() {
    for preset in PRESETS {
        let [w, h] = preset.size.map(|v| v as f32);
        let shapes = layout(&hud(ScaleRule::Grow, 0.5), preset.size, None);
        let s = h / 1080.0;
        // Day: 32, 28 from the top left; money: from the top right.
        assert!(
            close(
                rect_of(&shapes, 0),
                [32.0 * s, 28.0 * s, 388.0 * s, 128.0 * s]
            ),
            "{}",
            preset.name
        );
        let money = shapes
            .iter()
            .find(|x| (x.rect[2] - (w - 32.0 * s)).abs() < 0.01);
        assert!(money.is_some(), "money in the top right on {}", preset.name);
        // The hotbar's slots are centred across.
        let slots: Vec<&Shape> = shapes.iter().rev().take(10).collect();
        let left = slots.iter().map(|x| x.rect[0]).fold(f32::MAX, f32::min);
        let right = slots.iter().map(|x| x.rect[2]).fold(f32::MIN, f32::max);
        assert!(
            ((left + right) / 2.0 - w / 2.0).abs() < 0.5,
            "{}",
            preset.name
        );
        assert!(
            (h - slots[0].rect[3] - 32.0 * s).abs() < 0.01,
            "bottom gap on {}",
            preset.name
        );
    }
}

#[test]
fn each_rule_scales_as_it_says() {
    let screen = |rule| Screen {
        elements: vec![Element::new("b", ElementKind::Box { round: 0.0 }).sized([100.0, 50.0])],
        ..Screen::new("t", [1920.0, 1080.0], rule)
    };
    let width = |rule, size, step| {
        let r = layout(&screen(rule), size, step)[0].rect;
        r[2] - r[0]
    };
    // Grow: by the height, so an ultrawide of the same height is the same.
    assert!((width(ScaleRule::Grow, [2560, 1440], None) - 100.0 * 1440.0 / 1080.0).abs() < 1e-3);
    assert_eq!(width(ScaleRule::Grow, [2560, 1080], None), 100.0);
    // Keep: the same pixels.
    assert_eq!(width(ScaleRule::Keep, [3840, 2160], None), 100.0);
    // Whole steps: the game picture's step, or the biggest that fits.
    assert_eq!(width(ScaleRule::WholeSteps, [1920, 1080], Some(4)), 400.0);
    assert_eq!(width(ScaleRule::WholeSteps, [3840, 2160], None), 200.0);
    assert_eq!(
        width(ScaleRule::WholeSteps, [1000, 600], None),
        100.0,
        "at least 1"
    );
}

#[test]
fn whole_steps_land_on_the_game_s_pixels() {
    let shapes = layout(&hud(ScaleRule::WholeSteps, 0.37), [1920, 1080], Some(4));
    assert!(!shapes.is_empty());
    for s in &shapes {
        assert!(s.crisp);
        for v in s.rect {
            assert_eq!(v % 4.0, 0.0, "{:?}", s.rect);
        }
    }
    // The same HUD at 5 times (1440p) is the same in game pixels.
    let five = layout(&hud(ScaleRule::WholeSteps, 0.37), [2560, 1440], Some(5));
    assert_eq!(
        five[0].rect.map(|v| v / 5.0),
        shapes[0].rect.map(|v| v / 4.0)
    );
}

#[test]
fn stretch_fills_with_gaps_and_middle_moves_from_the_middle() {
    let mut screen = Screen::new("t", [1000.0, 1000.0], ScaleRule::Keep);
    screen.elements = vec![
        Element::new("bar", ElementKind::Box { round: 4.0 })
            .at(Anchor::new(Edge::Stretch, Edge::Start), [10.0, 5.0])
            .sized([20.0, 30.0]),
        Element::new("dot", ElementKind::Ellipse)
            .at(Anchor::MIDDLE, [50.0, -10.0])
            .sized([20.0, 20.0]),
        // Gaps bigger than the screen: squeezed to nothing, never inside out.
        Element::new("squeezed", ElementKind::Box { round: 0.0 })
            .at(Anchor::new(Edge::Stretch, Edge::Stretch), [900.0, 900.0])
            .sized([900.0, 900.0]),
    ];
    let shapes = layout(&screen, [800, 600], None);
    assert_eq!(shapes[0].rect, [10.0, 5.0, 780.0, 35.0]);
    assert_eq!(shapes[1].rect, [440.0, 280.0, 460.0, 300.0]);
    assert_eq!(shapes[1].kind, ShapeKind::Ellipse);
    let s = shapes[2].rect;
    assert!(s[0] <= s[2] && s[1] <= s[3], "{s:?}");
}

#[test]
fn a_bar_fills_its_value_inside_its_border() {
    for (value, want) in [
        (0.5, Some(160.0)),
        (0.0, None),
        (7.0, Some(300.0)),
        (f32::NAN, None),
    ] {
        let mut screen = hud(ScaleRule::Keep, value);
        screen.elements.retain(|e| e.name == "energy");
        let shapes = layout(&screen, [1920, 1080], None);
        match want {
            None => assert_eq!(shapes.len(), 1, "{value}"),
            Some(right) => {
                // 32 in, a 2-pixel border, 296 inside.
                let full = shapes[1].rect;
                assert_eq!(full[0], 34.0);
                assert!(
                    (full[2] - (34.0 + 296.0 * value.min(1.0))).abs() < 0.01,
                    "{value}: {full:?}"
                );
                let _ = right;
            }
        }
    }
}

#[test]
fn hidden_and_faded_elements_and_nonsense_numbers() {
    let mut screen = hud(ScaleRule::Grow, 0.5);
    screen.find_mut("money").unwrap().shown = false;
    let all = layout(&hud(ScaleRule::Grow, 0.5), [1920, 1080], None).len();
    assert_eq!(
        layout(&screen, [1920, 1080], None).len(),
        all - 2,
        "money and its coin"
    );
    let day = screen.find_mut("day").unwrap();
    day.opacity = 0.5;
    day.offset = [f32::NAN, f32::INFINITY];
    day.size = [-5.0, f32::NEG_INFINITY];
    let shapes = layout(&screen, [1920, 1080], None);
    assert!(
        (shapes[0].colours[0].0[3] - 0.375).abs() < 1e-6,
        "0.75 at half"
    );
    assert!(shapes[1].colours[0].0[3] <= 0.5, "the sun inside fades too");
    for s in &shapes {
        assert!(s.rect.iter().all(|v| v.is_finite()), "{s:?}");
    }
    assert!(
        layout(&screen, [0, 0], None)
            .iter()
            .all(|s| s.rect.iter().all(|v| v.is_finite()))
    );
}

#[test]
fn too_many_or_too_deep_stops_without_trouble() {
    let mut deep = Element::new("0", ElementKind::Box { round: 0.0 }).sized([1.0, 1.0]);
    for i in 1..200 {
        deep = Element::new(&i.to_string(), ElementKind::Box { round: 0.0 })
            .sized([1.0, 1.0])
            .holding(vec![deep]);
    }
    let mut screen = Screen::new("t", [10.0, 10.0], ScaleRule::Keep);
    screen.elements = vec![deep];
    assert_eq!(
        layout(&screen, [10, 10], None).len(),
        crate::screen::MAX_DEPTH
    );
    screen.elements = (0..20_000)
        .map(|i| Element::new(&i.to_string(), ElementKind::Ellipse))
        .collect();
    assert_eq!(
        layout(&screen, [10, 10], None).len(),
        crate::screen::MAX_ELEMENTS
    );
}

#[test]
fn anchors_say_where_in_words() {
    assert_eq!(Anchor::BOTTOM_LEFT.words(), "bottom left");
    assert_eq!(Anchor::MIDDLE.words(), "middle");
    assert_eq!(Anchor::BOTTOM.words(), "bottom");
    assert_eq!(
        Anchor::new(Edge::Stretch, Edge::End).words(),
        "bottom stretched across"
    );
    assert_eq!(
        Anchor::new(Edge::Stretch, Edge::Stretch).words(),
        "stretched to fill"
    );
    assert_eq!(ScaleRule::WholeSteps.words(), "Whole steps");
    assert_eq!(crate::presets::custom([5, 99_999]), [64, 8192]);
}

proptest::proptest! {
    #[test]
    fn any_numbers_lay_out_finite(
        x in proptest::num::f32::ANY, y in proptest::num::f32::ANY,
        w in proptest::num::f32::ANY, h in proptest::num::f32::ANY,
        sw in 0u32..10_000, sh in 0u32..10_000, step in proptest::option::of(0u32..64),
        rule in 0usize..3, across in 0usize..4, down in 0usize..4,
    ) {
        let edges = [Edge::Start, Edge::Middle, Edge::End, Edge::Stretch];
        let rules = [ScaleRule::Grow, ScaleRule::Keep, ScaleRule::WholeSteps];
        let mut screen = Screen::new("t", [w, h], rules[rule]);
        screen.elements = vec![Element::new("e", ElementKind::Bar { round: x, value: y, background: crate::Colour::WHITE })
            .at(Anchor::new(edges[across], edges[down]), [x, y]).sized([w, h])];
        for s in layout(&screen, [sw, sh], step) {
            proptest::prop_assert!(s.rect.iter().chain([&s.round]).all(|v| v.is_finite()));
            proptest::prop_assert!(s.rect[0] <= s.rect[2] && s.rect[1] <= s.rect[3]);
        }
    }
}

#[test]
fn laid_out_in_part_of_the_screen_moves_every_shape_there() {
    let screen = hud(ScaleRule::WholeSteps, 0.5);
    let plain = layout(&screen, [1920, 1080], Some(4));
    let moved = crate::layout_at(&screen, [320, 0], [1920, 1080], Some(4), None);
    assert_eq!(plain.len(), moved.len());
    assert_eq!(moved[0].rect[0], plain[0].rect[0] + 320.0);
    assert_eq!(moved[0].rect[1], plain[0].rect[1]);
}

#[test]
fn the_player_s_ui_size_grows_the_whole_screen() {
    let mut screen = Screen::new("t", [100.0, 100.0], ScaleRule::Grow);
    screen.elements = vec![
        Element::new("box", ElementKind::Box { round: 0.0 })
            .sized([10.0, 10.0])
            .at(Anchor::TOP_LEFT, [10.0, 10.0])
            .filled(Fill::Solid(Colour::WHITE)),
    ];
    let at = |s: &Screen| layout(s, [200, 200], None)[0].rect;
    assert_eq!(at(&screen), [20.0, 20.0, 40.0, 40.0]);
    screen.ui_size = 1.5;
    assert_eq!(at(&screen), [30.0, 30.0, 60.0, 60.0]);
    screen.ui_size = 99.0;
    assert_eq!(at(&screen), [40.0, 40.0, 80.0, 80.0], "at most twice");
    screen.ui_size = f32::NAN;
    assert_eq!(
        at(&screen),
        [20.0, 20.0, 40.0, 40.0],
        "not a number: as designed"
    );
    // Pixel art stays on whole steps.
    screen.scale = ScaleRule::WholeSteps;
    screen.ui_size = 1.5;
    assert_eq!(screen_scale(&screen, [200.0, 200.0], Some(1)), 2.0);
}

#[test]
fn the_screen_s_look_reaches_every_shape_in_screen_pixels() {
    use crate::screen::ScreenLook;
    let mut screen = Screen::new("t", [100.0, 100.0], ScaleRule::Grow);
    let blue = Colour::rgb(0, 0, 255);
    let mut own = Element::new("own", ElementKind::Box { round: 0.0 })
        .sized([10.0, 10.0])
        .filled(Fill::Solid(Colour::WHITE));
    own.glow = Some((2.0, blue));
    screen.elements = vec![
        Element::new("box", ElementKind::Box { round: 0.0 })
            .sized([20.0, 20.0])
            .filled(Fill::Solid(blue)),
        own,
    ];
    screen.look = ScreenLook {
        glow: 4.0,
        outline: 1.0,
        outline_colour: Colour::rgb(255, 0, 0),
        pixel: 2.0,
    };
    let shapes = layout(&screen, [300, 300], None);
    assert_eq!(shapes.len(), 2);
    assert_eq!(
        shapes[0].glow,
        (12.0, blue),
        "4 design pixels, three times, its own colour"
    );
    assert_eq!(shapes[0].outline, (3.0, Colour::rgb(255, 0, 0)));
    assert_eq!(shapes[0].pixel, 6.0);
    assert_eq!(shapes[1].glow, (6.0, blue), "its own glow first");
    // As it was before: none of it.
    screen.look = ScreenLook::default();
    let plain = layout(&screen, [300, 300], None);
    assert_eq!(plain[0].glow.0, 0.0);
    assert_eq!(plain[0].outline.0, 0.0);
    assert_eq!(plain[0].pixel, 0.0);
}

#[test]
fn review_findings_stay_fixed() {
    use crate::screen::ScreenLook;
    // A text's letters' halos all come before its letters, so one letter's
    // outline never covers the letter before it.
    let inter = include_bytes!("../../editor/assets/fonts/Inter-Regular.ttf");
    let (mut fonts, _) = crate::text::Fonts::new(vec![inter.to_vec()]);
    let mut screen = Screen::new("t", [400.0, 100.0], ScaleRule::Keep);
    screen.elements = vec![
        Element::new(
            "words",
            ElementKind::Text {
                text: "Hello".into(),
                size: 30.0,
                font: 0,
                align: Edge::Start,
            },
        )
        .sized([300.0, 50.0])
        .filled(Fill::Solid(Colour::WHITE)),
    ];
    screen.look = ScreenLook {
        outline: 2.0,
        ..ScreenLook::default()
    };
    let shapes = layout_with(&screen, [400, 100], None, Some(&mut fonts));
    let n = shapes.len();
    assert!(n >= 10 && n % 2 == 0, "{n}");
    for s in &shapes[..n / 2] {
        assert_eq!(s.colours[0], Colour::NONE, "a halo alone");
        assert_eq!(s.outline.0, 2.0);
    }
    for s in &shapes[n / 2..] {
        assert_eq!(s.colours[0], Colour::WHITE, "the letter");
        assert_eq!(s.outline.0, 0.0, "without its halo again");
    }
    // A gradient whose first stop is see-through glows in its other one.
    let blue = Colour::rgb(0, 0, 255);
    screen.elements = vec![
        Element::new("bar", ElementKind::Box { round: 0.0 })
            .sized([10.0, 10.0])
            .filled(Fill::Across(Colour::NONE, blue)),
    ];
    screen.look = ScreenLook {
        glow: 4.0,
        ..ScreenLook::default()
    };
    assert_eq!(layout(&screen, [400, 100], None)[0].glow.1, blue);
}
