//! Properties for the screen's piece picked on the canvas (decision
//! 0089): its words; its size and place (width and height, how what's
//! inside it lines up on a 3 × 3 grid, padding, gap); its look (colour,
//! corners, opacity, text size, bold); a hover look; whether it's
//! disabled; and what it does when clicked, shown as code. Every change
//! is one undo step, and the rest of its `[ ]` can be typed as written.

use crate::screen_doc::ScreenDoc;
use crate::screen_ties::{Kind, Readable, ScreenGame, TieTo};
use crate::theme;
use egui::{RichText, Ui};

/// The colours offered, Lust's own words.
const COLOURS: &[&str] = &[
    "white", "black", "gray1", "gray3", "gray5", "gray7", "gray9", "red", "orange", "yellow",
    "green", "teal", "blue", "indigo", "purple", "pink", "red5", "orange5", "yellow5", "green5",
    "teal5", "blue5", "indigo5", "purple5",
];

/// The words that line things up: across, then down.
const ACROSS: [&str; 3] = ["left", "center", "right"];
const DOWN: [&str; 3] = ["top", "middle", "bottom"];

/// Properties for the piece picked; gives whether the screen changed.
pub(crate) fn screen_properties(ui: &mut Ui, doc: &mut ScreenDoc, game: &ScreenGame) -> bool {
    let readable = &game.readable[..];
    let Some(path) = doc.picked.first().cloned() else {
        ui.label(RichText::new(&doc.screen).strong());
        ui.label(
            RichText::new("Pick a piece on the canvas or in Elements to change it.")
                .color(theme::TEXT_WEAK),
        );
        return false;
    };
    let Some(piece) = doc.piece(&path).cloned() else {
        return false;
    };
    let text = doc.text.clone();
    let mut changed = false;
    let many = doc.picked.len() > 1;
    egui::Grid::new("screen piece")
        .num_columns(2)
        .spacing([10.0, 6.0])
        .show(ui, |ui| {
            ui.label("Element");
            ui.horizontal(|ui| {
                ui.label(RichText::new(&piece.kind).strong());
                ui.label(
                    RichText::new(format!("{}, line {}", doc.path, piece.line))
                        .color(theme::TEXT_WEAK),
                );
                if many {
                    ui.label(
                        RichText::new(format!("and {} more", doc.picked.len() - 1))
                            .color(theme::TEXT_WEAK),
                    );
                }
            });
            ui.end_row();
            // Its words.
            if let Some(arg) = &piece.arg {
                ui.label("Words");
                match &arg.words {
                    Some(words) if !many => {
                        let id = egui::Id::new(("screen words", &path));
                        let mut typed: String =
                            ui.data(|d| d.get_temp(id)).unwrap_or_else(|| words.clone());
                        let field = ui.text_edit_singleline(&mut typed);
                        if field.changed() {
                            ui.data_mut(|d| d.insert_temp(id, typed.clone()));
                        }
                        if field.lost_focus() && typed != *words {
                            changed |= doc.set_words(&typed);
                            ui.data_mut(|d| d.remove::<String>(id));
                        }
                    }
                    _ => {
                        ui.label(
                            RichText::new(format!("{} (code)", &text[arg.span.clone()]))
                                .monospace()
                                .color(theme::TEXT_WEAK),
                        );
                    }
                }
                ui.end_row();
            }
        });
    if !many {
        changed |= ties(ui, doc, readable, &piece, &text);
    }
    section(ui, "SIZE AND PLACE");
    egui::Grid::new("screen piece size")
        .num_columns(2)
        .spacing([10.0, 6.0])
        .show(ui, |ui| {
            for name in ["width", "height"] {
                ui.label(if name == "width" { "Width" } else { "Height" });
                changed |= size_field(ui, doc, &text, &piece, name);
                ui.end_row();
            }
            ui.label("Align");
            changed |= align_grid(ui, doc, &piece);
            ui.end_row();
            ui.label("Padding");
            changed |= number_field(ui, doc, &text, &piece, "pad", 8.0);
            ui.end_row();
            if piece.children.is_some() {
                ui.label("Gap");
                changed |= number_field(ui, doc, &text, &piece, "gap", 8.0);
                ui.end_row();
            }
            if doc
                .block()
                .and_then(|b| b.at(&path[..path.len() - 1]))
                .is_some_and(|p| p.kind == "canvas")
            {
                for (name, label) in [("x", "X"), ("y", "Y")] {
                    ui.label(label);
                    changed |= number_field(ui, doc, &text, &piece, name, 0.0);
                    ui.end_row();
                }
            }
        });
    section(ui, "LOOK");
    egui::Grid::new("screen piece look")
        .num_columns(2)
        .spacing([10.0, 6.0])
        .show(ui, |ui| {
            ui.label("Colour");
            changed |= colour_field(ui, doc, &text, &piece);
            ui.end_row();
            ui.label("Corners");
            changed |= number_field(ui, doc, &text, &piece, "round", 8.0);
            ui.end_row();
            ui.label("Opacity");
            changed |= number_field(ui, doc, &text, &piece, "opacity", 1.0);
            ui.end_row();
            if matches!(piece.kind.as_str(), "text" | "button") {
                ui.label("Text size");
                changed |= number_field(ui, doc, &text, &piece, "size", 15.0);
                ui.end_row();
                ui.label("Bold");
                let mut bold = piece.has("bold");
                if ui.checkbox(&mut bold, "").changed() {
                    changed |= if bold {
                        doc.set_style("bold", None)
                    } else {
                        doc.remove_styles(&["bold"])
                    };
                }
                ui.end_row();
            }
            ui.label("Hover");
            match piece.style_text(&text, "hover") {
                Some(look) => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(look).monospace());
                        if ui
                            .small_button("Remove")
                            .on_hover_text("No hover look")
                            .clicked()
                        {
                            changed |= doc.remove_styles(&["hover"]);
                        }
                    });
                }
                None => {
                    if ui
                        .button("+ Add a hover look")
                        .on_hover_text("Darkens it a little while the pointer is over it")
                        .clicked()
                    {
                        changed |= doc.set_style("hover", Some("[opacity 0.85]"));
                    }
                }
            }
            ui.end_row();
            ui.label("Disabled");
            let mut off = piece.has("disabled");
            if ui
                .checkbox(&mut off, "")
                .on_hover_text("Greyed, and clicks do nothing")
                .changed()
            {
                changed |= if off {
                    doc.set_style("disabled", None)
                } else {
                    doc.remove_styles(&["disabled"])
                };
            }
            ui.end_row();
        });
    if let Some(action) = &piece.action {
        section(ui, "WHEN CLICKED");
        let body = text[action.clone()].trim();
        if let (Some(now), false) = (doc.sends(), many) {
            let events = &game.events;
            ui.horizontal(|ui| {
                ui.label("Sends");
                egui::ComboBox::from_id_salt(("screen sends", &path))
                    .selected_text(now.clone().unwrap_or_else(|| "nothing".into()))
                    .height(520.0)
                    .show_ui(ui, |ui| {
                        for (e, file) in events {
                            if ui
                                .selectable_label(now.as_deref() == Some(e), e)
                                .on_hover_text(format!("Written in {file}"))
                                .clicked()
                                && now.as_deref() != Some(e)
                            {
                                changed |= doc.set_sends(e, file);
                            }
                        }
                    })
                    .response
                    .on_hover_text("The event sent to the screen's entity when it's clicked");
            });
            ui.label(
                RichText::new(if events.is_empty() {
                    "Declare an event in your game's code (event Pause();) to choose it here."
                } else {
                    "The game's events. The entity's code reacts with on Pause() { }."
                })
                .color(theme::TEXT_WEAK)
                .small(),
            );
        } else {
            ui.label(
                RichText::new(if body.is_empty() { "nothing yet" } else { body })
                    .monospace()
                    .color(theme::TEXT_WEAK),
            );
            ui.label(
                RichText::new("What it does is written in its { }: Open as code to change it.")
                    .color(theme::TEXT_WEAK)
                    .small(),
            );
        }
    }
    // Everything in its [ ], as written.
    section(ui, "ALL ITS SETTINGS");
    let written = piece
        .styles
        .as_ref()
        .map(|s| text[s.span.start + 1..s.span.end - 1].to_string())
        .unwrap_or_default();
    let id = egui::Id::new(("screen styles", &path));
    let mut typed: String = ui
        .data(|d| d.get_temp(id))
        .unwrap_or_else(|| written.clone());
    let field = ui.add(
        egui::TextEdit::singleline(&mut typed)
            .code_editor()
            .desired_width(f32::INFINITY)
            .hint_text("gap 8, center"),
    );
    if field.changed() {
        ui.data_mut(|d| d.insert_temp(id, typed.clone()));
    }
    if field.lost_focus() && typed != written && !many {
        ui.data_mut(|d| d.remove::<String>(id));
        changed |= set_all(doc, &typed);
    }
    ui.label(
        RichText::new("As written in its [ ]: anything Lust has, such as gap 8, center or nowrap.")
            .color(theme::TEXT_WEAK)
            .small(),
    );
    changed
}

/// What the piece picked can be tied to, with a menu of the screen's
/// entity's components and their fields for each.
fn ties(
    ui: &mut Ui,
    doc: &mut ScreenDoc,
    readable: &[Readable],
    piece: &ui::source::ElementSource,
    text: &str,
) -> bool {
    let mut targets: Vec<(&str, Kind, TieTo)> = Vec::new();
    match piece.kind.as_str() {
        "text" | "button" => targets.push(("Words", Kind::Words, TieTo::Words)),
        "progress" | "slider" => targets.push(("Value", Kind::Number, TieTo::Value)),
        _ => {}
    }
    if !matches!(piece.kind.as_str(), "text" | "button") {
        for side in ["width", "height"] {
            let written = piece.style_text(text, side).map(str::trim);
            // A number, or a bar already tied: `360 * health.hp / health.maxHp`.
            let full = written.and_then(|v| {
                v.parse::<f32>()
                    .ok()
                    .or_else(|| v.split_once(" * ")?.0.trim().parse().ok())
            });
            if let Some(full) = full {
                let label = if side == "width" {
                    "Fill across"
                } else {
                    "Fill up"
                };
                targets.push((
                    label,
                    Kind::Number,
                    TieTo::Bar {
                        side: side.into(),
                        full,
                        max: String::new(),
                    },
                ));
            }
        }
    }
    targets.push(("Opacity", Kind::Number, TieTo::Style("opacity".into())));
    section(ui, "TIED TO THE GAME");
    let mut changed = false;
    if readable.is_empty() {
        ui.label(
            RichText::new(
                "Give the entity this screen is shown for a component, and its fields can be tied here.",
            )
            .color(theme::TEXT_WEAK)
            .small(),
        );
        return false;
    }
    egui::Grid::new("screen ties")
        .num_columns(2)
        .spacing([10.0, 6.0])
        .show(ui, |ui| {
            for (label, kind, to) in &targets {
                ui.label(*label);
                let now = match to {
                    TieTo::Words | TieTo::Value => piece
                        .arg
                        .as_ref()
                        .filter(|a| a.words.is_none())
                        .map(|a| text[a.span.clone()].to_string()),
                    TieTo::Style(n) => piece
                        .style_text(text, n)
                        .filter(|v| v.contains('.'))
                        .map(str::to_string),
                    TieTo::Bar { side, .. } => piece
                        .style_text(text, side)
                        .filter(|v| v.contains(" * ") && v.contains('.'))
                        .map(|v| v.trim().to_string()),
                };
                let shown = now.clone().unwrap_or_else(|| "Tie to…".into());
                ui.menu_button(shown, |ui| {
                    for r in readable {
                        let fields: Vec<_> = r
                            .fields
                            .iter()
                            .filter(|f| f.1 == *kind || *kind == Kind::Words)
                            .collect();
                        if fields.is_empty() {
                            continue;
                        }
                        ui.menu_button(&r.component, |ui| {
                            for (field, _, start) in &fields {
                                match to {
                                    TieTo::Bar { side, full, .. } => {
                                        ui.menu_button(format!("{field} out of"), |ui| {
                                            // Its starting value as the most:
                                            // health starts full.
                                            let most = crate::canvas_tools::num(*start as f32);
                                            if *start > 0.0
                                                && ui
                                                    .button(format!("{most} (its start)"))
                                                    .clicked()
                                            {
                                                let to = TieTo::Bar {
                                                    side: side.clone(),
                                                    full: *full,
                                                    max: most.clone(),
                                                };
                                                changed |= doc.tie(r, field, &to);
                                                ui.close();
                                            }
                                            for (max, k, _) in &r.fields {
                                                if *k == Kind::Number
                                                    && max != field
                                                    && ui.button(max).clicked()
                                                {
                                                    let to = TieTo::Bar {
                                                        side: side.clone(),
                                                        full: *full,
                                                        max: max.clone(),
                                                    };
                                                    changed |= doc.tie(r, field, &to);
                                                    ui.close();
                                                }
                                            }
                                        });
                                    }
                                    _ => {
                                        if ui.button(field).clicked() {
                                            changed |= doc.tie(r, field, to);
                                            ui.close();
                                        }
                                    }
                                }
                            }
                        });
                    }
                })
                .response
                .on_hover_text("Shows one of the entity's values, read again after every tick");
                ui.end_row();
            }
        });
    changed
}

/// A number being dragged is kept between frames (so a drag adds up),
/// and saved once, when the drag ends or a typed number is entered.
fn kept_while_dragged(
    ui: &Ui,
    drag: &egui::Response,
    id: egui::Id,
    n: f32,
    save: impl FnOnce() -> bool,
) -> bool {
    if drag.dragged() || drag.has_focus() {
        ui.data_mut(|d| d.insert_temp(id, n));
    }
    let done = drag.drag_stopped()
        || (drag.changed() && !drag.dragged() && !drag.has_focus())
        || drag.lost_focus();
    if done {
        ui.data_mut(|d| d.remove::<f32>(id));
        return save();
    }
    false
}

/// A grey heading.
fn section(ui: &mut Ui, title: &str) {
    ui.add_space(8.0);
    ui.label(
        RichText::new(title)
            .small()
            .strong()
            .color(theme::TEXT_WEAK),
    );
}

/// Replaces the first piece picked's whole `[ ]` with `typed`, if it reads.
fn set_all(doc: &mut ScreenDoc, typed: &str) -> bool {
    let Some(path) = doc.picked.first().cloned() else {
        return false;
    };
    let Some(piece) = doc.piece(&path).cloned() else {
        return false;
    };
    let typed = typed.trim();
    let text = match &piece.styles {
        Some(list) if typed.is_empty() => {
            let mut start = list.span.start;
            while start > 0 && doc.text.as_bytes()[start - 1] == b' ' {
                start -= 1;
            }
            ui::source::replaced(&doc.text, start..list.span.end, "")
        }
        Some(list) => ui::source::replaced(&doc.text, list.span.clone(), &format!("[{typed}]")),
        None if typed.is_empty() => return false,
        None => {
            let after = piece
                .arg
                .as_ref()
                .map_or(piece.span.start + piece.kind.len(), |a| a.span.end);
            ui::source::replaced(&doc.text, after..after, &format!(" [{typed}]"))
        }
    };
    // Only if it still reads.
    if !ui::source::parse(&text).is_ok_and(|s| s.iter().any(|s| s.name == doc.screen)) {
        return false;
    }
    let changed = doc.change(text);
    doc.behind |= changed;
    changed
}

/// A width or height: a number, `fill`, or auto (nothing).
fn size_field(
    ui: &mut Ui,
    doc: &mut ScreenDoc,
    text: &str,
    piece: &ui::source::ElementSource,
    name: &str,
) -> bool {
    let now = piece.style_text(text, name);
    let mut changed = false;
    ui.horizontal(|ui| {
        let mode = match now {
            None => "auto",
            Some("") | Some("fill") => "fill",
            Some(_) => "number",
        };
        let id = egui::Id::new(("screen size", name, piece.span.start));
        let written: f32 = now.and_then(|v| v.trim().parse().ok()).unwrap_or(100.0);
        let mut n: f32 = ui.data(|d| d.get_temp(id)).unwrap_or(written);
        let drag = ui.add_enabled(
            mode == "number",
            egui::DragValue::new(&mut n).range(0.0..=8192.0).speed(1.0),
        );
        changed |= kept_while_dragged(ui, &drag, id, n, || {
            doc.set_style(name, Some(&format!("{}", n.round())))
        });
        for (m, label, hint) in [
            ("number", "set", "A size in the screen's pixels"),
            ("fill", "fill", "As big as the room it has"),
            ("auto", "auto", "As big as what's in it"),
        ] {
            if ui
                .selectable_label(mode == m, label)
                .on_hover_text(hint)
                .clicked()
                && mode != m
            {
                changed |= match m {
                    "number" => doc.set_style(name, Some(&format!("{}", n.round()))),
                    "fill" => doc.set_style(name, Some("fill")),
                    _ => doc.remove_styles(&[name]),
                };
            }
        }
    });
    changed
}

/// A number setting, or none (its default shown faint).
fn number_field(
    ui: &mut Ui,
    doc: &mut ScreenDoc,
    text: &str,
    piece: &ui::source::ElementSource,
    name: &str,
    default: f32,
) -> bool {
    let now = piece.style_text(text, name);
    let written: Option<f32> = now.and_then(|v| v.trim().parse().ok());
    let code = now.is_some() && written.is_none() && !now.is_some_and(str::is_empty);
    let mut changed = false;
    ui.horizontal(|ui| {
        if code {
            ui.label(RichText::new(now.unwrap_or_default()).monospace());
            ui.label(RichText::new("(code)").color(theme::TEXT_WEAK));
            return;
        }
        let id = egui::Id::new(("screen number", name, piece.span.start));
        let mut n: f32 = ui
            .data(|d| d.get_temp(id))
            .unwrap_or(written.unwrap_or(default));
        let range = if name == "opacity" {
            0.0..=1.0
        } else {
            -8192.0..=8192.0
        };
        let speed = if name == "opacity" { 0.01 } else { 1.0 };
        let drag = ui.add(egui::DragValue::new(&mut n).range(range).speed(speed));
        changed |= kept_while_dragged(ui, &drag, id, n, || {
            let v = if name == "opacity" {
                format!("{:.2}", n)
            } else {
                format!("{}", n.round())
            };
            doc.set_style(name, Some(&v))
        });
        if written.is_some() {
            if ui
                .small_button("↺")
                .on_hover_text("Back to its default")
                .clicked()
            {
                changed |= doc.remove_styles(&[name]);
            }
        } else {
            ui.label(RichText::new("default").color(theme::TEXT_WEAK).small());
        }
    });
    changed
}

/// A colour from Lust's words: a button's or a shape's fill, text's ink,
/// a container's background.
fn colour_field(
    ui: &mut Ui,
    doc: &mut ScreenDoc,
    text: &str,
    piece: &ui::source::ElementSource,
) -> bool {
    // Where its colour is written.
    let (style, alone) = match piece.kind.as_str() {
        "text" => ("color", false),
        "button" | "box" | "circle" | "line" | "progress" => ("", true),
        _ => ("background", false),
    };
    let now: Option<String> = if alone {
        piece
            .styles
            .as_ref()
            .and_then(|s| {
                s.items
                    .iter()
                    .rev()
                    .find(|i| i.value.is_none() && ui::view::colour(&i.name).is_some())
            })
            .map(|i| i.name.clone())
    } else {
        piece.style_text(text, style).map(str::to_string)
    };
    let mut changed = false;
    ui.horizontal(|ui| {
        if let Some(c) = now.as_deref().and_then(ui::view::colour) {
            let [_, r, g, b] = c.to_be_bytes();
            let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(rect, 3.0, egui::Color32::from_rgb(r, g, b));
        }
        egui::ComboBox::from_id_salt(("screen colour", piece.span.start))
            .height(520.0)
            .selected_text(now.clone().unwrap_or_else(|| "default".into()))
            .show_ui(ui, |ui| {
                if ui.selectable_label(now.is_none(), "default").clicked() {
                    changed |= if alone {
                        let names: Vec<&str> = COLOURS.to_vec();
                        doc.remove_styles(&names)
                    } else {
                        doc.remove_styles(&[style])
                    };
                }
                for name in COLOURS {
                    let on = now.as_deref() == Some(*name);
                    if ui.selectable_label(on, *name).clicked() && !on {
                        changed |= if alone {
                            let mut old: Vec<&str> = COLOURS.to_vec();
                            if let Some(n) = now.as_deref() {
                                old.push(n);
                            }
                            doc.restyle(&old, &[(name, None)])
                        } else {
                            doc.set_style(style, Some(name))
                        };
                    }
                }
            });
    });
    changed
}

/// The 3 × 3 grid of how what's inside lines up.
fn align_grid(ui: &mut Ui, doc: &mut ScreenDoc, piece: &ui::source::ElementSource) -> bool {
    let across = ACROSS.iter().position(|w| piece.has(w));
    let down = DOWN.iter().position(|w| piece.has(w));
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
            for (d, dw) in DOWN.iter().enumerate() {
                ui.horizontal(|ui| {
                    for (a, aw) in ACROSS.iter().enumerate() {
                        let on = across.unwrap_or(0) == a
                            && down.unwrap_or(0) == d
                            && (across.is_some() || down.is_some());
                        let (rect, r) =
                            ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::click());
                        ui.painter().rect(
                            rect,
                            2.0,
                            if on {
                                egui::Color32::from_rgb(0x24, 0x68, 0xd8)
                            } else {
                                egui::Color32::TRANSPARENT
                            },
                            egui::Stroke::new(1.0, theme::TEXT_WEAK),
                            egui::StrokeKind::Inside,
                        );
                        let named = format!("{aw}, {dw}");
                        r.widget_info(|| {
                            egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, &named)
                        });
                        if r.on_hover_text(named).clicked() {
                            let all: Vec<&str> =
                                ACROSS.iter().chain(DOWN.iter()).copied().collect();
                            changed |= doc.restyle(&all, &[(aw, None), (dw, None)]);
                        }
                    }
                });
            }
        });
        let words = match (across, down) {
            (None, None) => "top left (default)".to_string(),
            _ => format!(
                "{}, {}",
                ACROSS[across.unwrap_or(0)],
                DOWN[down.unwrap_or(0)]
            ),
        };
        ui.label(RichText::new(words).color(theme::TEXT_WEAK));
    });
    changed
}

#[cfg(test)]
#[path = "screen_props_tests.rs"]
mod tests;
