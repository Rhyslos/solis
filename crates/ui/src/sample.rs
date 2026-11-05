//! A sample HUD, the mock-up's (decision 0083): the day box pinned top
//! left, money top right, the energy bar bottom left and a hotbar of ten
//! slots bottom middle. The editor shows it with the Console's `ui test`
//! until screens can be made in Lust, and the tests lay it out.

use crate::screen::{Anchor, Colour, Element, ElementKind, Fill, ScaleRule, Screen};

/// A text element: `words` in `colour`, `size` design pixels, filling its
/// parent's box with `left` design pixels' room on the left.
fn label(name: &str, words: &str, size: f32, colour: Colour, left: f32, font: usize) -> Element {
    Element::new(
        name,
        ElementKind::Text {
            text: words.into(),
            size,
            font,
            align: crate::Edge::Start,
        },
    )
    .at(
        Anchor::new(crate::Edge::Stretch, crate::Edge::Stretch),
        [left, 0.0],
    )
    .filled(Fill::Solid(colour))
}

/// The wood-and-parchment look of the mock-up.
fn panel(name: &str, round: f32) -> Element {
    Element::new(name, ElementKind::Box { round })
        .filled(Fill::Solid(Colour::rgb(30, 24, 16).with_alpha(0.75)))
        .bordered(2.0, Colour::rgb(201, 164, 106))
}

/// The sample HUD, under `rule`. Designed at 1920 × 1080; under
/// [`ScaleRule::WholeSteps`], at 480 × 270 game pixels (a quarter).
pub fn hud(rule: ScaleRule, energy: f32) -> Screen {
    let (design, k) = match rule {
        ScaleRule::WholeSteps => ([480.0, 270.0], 0.25),
        _ => ([1920.0, 1080.0], 1.0),
    };
    let s = |v: [f32; 2]| v.map(|x| x * k);
    let mut screen = Screen::new("HUD", design, rule);
    let day = panel("day", 10.0 * k)
        .at(Anchor::TOP_LEFT, s([32.0, 28.0]))
        .sized(s([356.0, 100.0]))
        .holding(vec![
            Element::new("sun", ElementKind::Ellipse)
                .at(
                    Anchor::new(crate::Edge::Start, crate::Edge::Middle),
                    s([22.0, 0.0]),
                )
                .sized(s([40.0, 40.0]))
                .filled(Fill::Solid(Colour::rgb(255, 217, 106))),
            label(
                "season",
                "Spring 3",
                34.0 * k,
                Colour::rgb(255, 233, 184),
                80.0 * k,
                1,
            )
            .sized([0.0, 46.0 * k]),
            label(
                "time",
                "Mon · 08:40",
                24.0 * k,
                Colour::rgb(240, 217, 168),
                80.0 * k,
                0,
            )
            .at(
                Anchor::new(crate::Edge::Stretch, crate::Edge::Stretch),
                s([80.0, 50.0]),
            ),
        ]);
    let money = panel("money", 10.0 * k)
        .at(Anchor::TOP_RIGHT, s([32.0, 28.0]))
        .sized(s([264.0, 68.0]))
        .holding(vec![
            Element::new("coin", ElementKind::Ellipse)
                .at(
                    Anchor::new(crate::Edge::Start, crate::Edge::Middle),
                    s([20.0, 0.0]),
                )
                .sized(s([28.0, 28.0]))
                .filled(Fill::Solid(Colour::rgb(255, 217, 106))),
            label(
                "amount",
                "1,250 g",
                30.0 * k,
                Colour::rgb(255, 217, 106),
                60.0 * k,
                1,
            ),
        ]);
    let energy = Element::new(
        "energy",
        ElementKind::Bar {
            round: 8.0 * k,
            value: energy,
            background: Colour::rgb(30, 24, 16).with_alpha(0.75),
        },
    )
    .at(Anchor::BOTTOM_LEFT, s([32.0, 96.0]))
    .sized(s([300.0, 32.0]))
    .filled(Fill::Across(
        Colour::rgb(127, 211, 106),
        Colour::rgb(184, 232, 106),
    ))
    .bordered(2.0, Colour::rgb(201, 164, 106));
    let energy_label = label("energy label", "ENERGY", 20.0 * k, Colour::WHITE, 0.0, 1)
        .at(Anchor::BOTTOM_LEFT, s([34.0, 132.0]))
        .sized(s([200.0, 28.0]));
    let slots: Vec<Element> = (0..10)
        .map(|i| {
            let mut slot = panel(&format!("slot {}", i + 1), 8.0 * k)
                .at(Anchor::TOP_LEFT, s([i as f32 * 92.0, 0.0]))
                .sized(s([84.0, 84.0]));
            if i == 1 {
                slot.border = Some((3.0, Colour::rgb(255, 242, 196)));
            }
            slot
        })
        .collect();
    let hotbar = Element::new("hotbar", ElementKind::Group)
        .at(Anchor::BOTTOM, s([0.0, 32.0]))
        .sized(s([912.0, 84.0]))
        .holding(slots);
    screen.elements = vec![day, money, energy, energy_label, hotbar];
    screen
}
