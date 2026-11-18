//! The Trigger zone card in Properties, and the card saying what game
//! code hears (the approved mock-up `docs/design/triggers/`):
//! - **Shape:** Box | Circle (Box | Sphere in a 3D project), and Freeform
//!   in 2D, which starts drawing in the view;
//! - **Size:** a box's width and height (and depth in 3D), a round zone's
//!   radius, or a freeform zone's count of points and **Edit points**;
//! - **Notices**, **Fires** (Every time | Once), **Only tags** greyed until
//!   tags exist, and **Inside now**: "Shown while playing", or who is
//!   inside while the game plays.

use crate::inspector;
use crate::tokens::{self, text};
use crate::widgets::{self, field::Axis};
use egui::{RichText, Ui};
use level::builtin::{Fires, NOTICES, TriggerZone, ZoneForm};
use reflect::{Component, ComponentData, Value};

/// What the card needs that the zone's values don't say.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ZoneFacts {
    /// Whether the project shows 2D (Circle and Freeform) or 3D (Sphere
    /// and depth).
    pub(crate) flat: bool,
    /// While playing: the names of who is inside the zone now.
    pub(crate) inside: Option<Vec<String>>,
}

/// What a change on the card asks for.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ZoneAsk {
    /// A field of the zone's set to a value.
    Set(&'static str, Value),
    /// Freeform chosen: draw its outline in the view.
    Draw,
    /// Edit points: edit the freeform outline's points in the view.
    EditPoints,
}

/// The width of a number field for one of a size's numbers.
const NUMBER_W: f32 = 84.0;

/// Draws the card's rows for the zone in `data`.
pub(crate) fn rows(ui: &mut Ui, data: &ComponentData, facts: &ZoneFacts) -> Vec<ZoneAsk> {
    let zone = TriggerZone::from_data(data);
    let mut asks = Vec::new();
    shape_row(ui, &zone, facts.flat, &mut asks);
    size_row(ui, &zone, facts.flat, &mut asks);
    inspector::row(
        ui,
        "Notices",
        "What the zone notices: characters, and bodies that move.",
        |ui| {
            widgets::field::dropdown_button(ui, "Notices", NOTICES[0], ui.available_width());
        },
    );
    inspector::row(
        ui,
        "Fires",
        "Every time: whenever something walks in or out. Once: the first time something walks \
         in, then never again, even after loading a save.",
        |ui| {
            let mut fires = zone.fires;
            let choices = [(Fires::EveryTime, "Every time"), (Fires::Once, "Once")];
            if widgets::segmented::segmented(ui, "Fires", &mut fires, &choices) {
                asks.push(ZoneAsk::Set("fires", Value::Text(fires.name().into())));
            }
        },
    );
    inspector::row(
        ui,
        "Only tags",
        "Comes with tags: notice only some kinds of thing.",
        |ui| {
            let width = ui.available_width();
            ui.add_enabled_ui(false, |ui| {
                widgets::field::text_field(
                    ui,
                    "Only tags",
                    &mut String::new(),
                    "Comes with tags",
                    width,
                )
            })
            .response
            .on_disabled_hover_text("Comes with tags: a zone will notice only some kinds of thing");
        },
    );
    inside_row(ui, facts);
    asks
}

/// Shape: Box | Circle | Freeform in 2D, Box | Sphere in 3D.
fn shape_row(ui: &mut Ui, zone: &TriggerZone, flat: bool, asks: &mut Vec<ZoneAsk>) {
    inspector::row(
        ui,
        "Shape",
        "A box; a circle in 2D or a sphere in 3D; or, in 2D, a freeform outline drawn point by \
         point.",
        |ui| {
            let mut form = zone.form;
            let mut choices = vec![
                (ZoneForm::Box, ZoneForm::Box.shown(flat)),
                (ZoneForm::Round, ZoneForm::Round.shown(flat)),
            ];
            if flat {
                choices.push((ZoneForm::Freeform, ZoneForm::Freeform.shown(flat)));
            }
            if widgets::segmented::segmented(ui, "Shape", &mut form, &choices) {
                asks.push(match form {
                    ZoneForm::Freeform => ZoneAsk::Draw,
                    other => ZoneAsk::Set("shape", Value::Text(other.name().into())),
                });
            }
        },
    );
}

/// Size: the box's sides, the radius, or the points and Edit points.
fn size_row(ui: &mut Ui, zone: &TriggerZone, flat: bool, asks: &mut Vec<ZoneAsk>) {
    let help = match zone.form {
        ZoneForm::Box => "The box's size in metres. Its handles in the view size it too.",
        ZoneForm::Round => "The radius in metres. Its handles in the view size it too.",
        ZoneForm::Freeform => "How many points its outline has; Edit points (or Tab) changes them.",
    };
    let label = if zone.form == ZoneForm::Round {
        "Radius"
    } else {
        "Size"
    };
    inspector::row(ui, label, help, |ui| match zone.form {
        ZoneForm::Box => {
            let mut size = zone.size;
            let axes: &[(usize, Axis)] = if flat {
                &[(0, Axis::W), (1, Axis::H)]
            } else {
                &[(0, Axis::X), (1, Axis::Y), (2, Axis::Z)]
            };
            ui.horizontal(|ui| {
                for &(i, axis) in axes {
                    let mut value = f64::from(size[i]);
                    let name = format!("Size {}", ["X", "Y", "Z"][i]);
                    if widgets::field::number_field(
                        ui,
                        &name,
                        &mut value,
                        Some(axis),
                        0.05,
                        NUMBER_W,
                    )
                    .changed()
                    {
                        size[i] = value as f32;
                        asks.push(ZoneAsk::Set("size", Value::vec3(size)));
                    }
                }
            });
        }
        ZoneForm::Round => {
            let mut radius = f64::from(zone.radius);
            if widgets::field::number_field(ui, "Radius", &mut radius, None, 0.05, NUMBER_W)
                .changed()
            {
                asks.push(ZoneAsk::Set("radius", Value::Float(radius)));
            }
        }
        ZoneForm::Freeform => {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{} points", zone.points.len()))
                        .font(text::row())
                        .color(tokens::text_primary()),
                );
                if widgets::button::Button::plain("Edit points")
                    .show(ui)
                    .clicked()
                {
                    asks.push(ZoneAsk::EditPoints);
                }
            });
        }
    });
}

/// Inside now: who is inside while playing, as chips.
fn inside_row(ui: &mut Ui, facts: &ZoneFacts) {
    inspector::row(
        ui,
        "Inside now",
        "Who is inside the zone, while the game plays.",
        |ui| match &facts.inside {
            None => {
                ui.label(
                    RichText::new("Shown while playing")
                        .font(text::row())
                        .color(tokens::text_label()),
                );
            }
            Some(names) if names.is_empty() => {
                ui.label(
                    RichText::new("No one")
                        .font(text::row())
                        .color(tokens::text_label()),
                );
            }
            Some(names) => {
                ui.horizontal_wrapped(|ui| {
                    for name in names {
                        widgets::chip::chip(ui, widgets::chip::ChipKind::Large, name, true, None);
                    }
                });
            }
        },
    );
}

/// The card saying what game code hears, with **New code for this
/// zone…**. Returns whether that was clicked.
pub(crate) fn hears(ui: &mut Ui, three_d_too: bool) -> bool {
    let title = |ui: &mut Ui, words: &str| {
        ui.label(
            RichText::new(words)
                .font(text::card_name())
                .color(tokens::text_primary()),
        );
    };
    let weak = |ui: &mut Ui, words: &str| {
        ui.add(
            egui::Label::new(
                RichText::new(words)
                    .font(text::row())
                    .color(tokens::text_label()),
            )
            .wrap(),
        );
    };
    let code = |ui: &mut Ui, words: &str| {
        ui.label(
            RichText::new(words)
                .font(text::mono())
                .color(tokens::text_primary()),
        );
    };
    title(ui, "What game code hears");
    weak(ui, "On this zone:");
    code(ui, "Entered(who)  and  Exited(who)");
    weak(ui, "On whoever walked in:");
    code(ui, "EnteredZone(zone)  and  LeftZone(zone)");
    weak(
        ui,
        if three_d_too {
            "The same zone, events and settings in 2D and 3D: built once, for both."
        } else {
            "Something that goes in and out within one tick isn't counted. What's inside is \
             kept in saves, so loading never says \u{201c}entered\u{201d} twice."
        },
    );
    widgets::button::Button::plain("New code for this zone\u{2026}")
        .show(ui)
        .on_hover_text(
            "Makes a code file named after the zone, with empty Entered and Exited, adds its \
             component to the zone, and opens it",
        )
        .clicked()
}
