//! The Map tab (Phase 9, D4), beside Properties while a tile map is
//! selected, as the project lead chose it (mock-up A): the map's layers
//! stacked with a little picture of each, shown or hidden in the editor and
//! drawn under or over sprites; its tileset's terrains as big swatches with
//! how many cells use each; its cell values; and the cell under the
//! pointer in the viewport, every layer and value. Changes go back as
//! [`MapRequest`]s; the workbench makes each one undo step.
//!
//! The terrain swatches are the tile painter's palette (D6): click one to
//! paint with it; with Exact on, the terrain's tiles show beneath to
//! choose the one placed.

use crate::theme;
use egui::{Color32, RichText, Ui, vec2};
use std::collections::HashSet;
use tiles::{CellValue, TileMap, Tileset, ValueKind};

/// What the tab shows.
pub(crate) struct MapUi<'a> {
    /// The map file's name.
    pub(crate) name: String,
    /// The map: as the game has it while playing, otherwise its file's.
    pub(crate) map: &'a TileMap,
    /// Its tileset, if read.
    pub(crate) tileset: Option<&'a Tileset>,
    /// The cell under the pointer in the viewport.
    pub(crate) hovered: Option<[u32; 2]>,
    /// Layers hidden in the editor, by ID.
    pub(crate) hidden: &'a HashSet<reflect::Id>,
    /// Whether the game is running (the cell shown follows it).
    pub(crate) playing: bool,
    /// What the tile painter paints with.
    pub(crate) paint: PaintShown<'a>,
}

/// The tile painter's palette as the tab shows it (D6).
pub(crate) struct PaintShown<'a> {
    /// The terrain painted, by code.
    pub(crate) code: Option<u8>,
    /// Exact tiles are placed: the terrain's tiles are shown to choose.
    pub(crate) exact: bool,
    /// The exact tile chosen.
    pub(crate) piece: Option<reflect::Id>,
    /// A copy is held: it's painted instead of a terrain.
    pub(crate) stamp: Option<[u32; 2]>,
    /// The layer painted on, by index.
    pub(crate) layer: usize,
    /// The tileset's picture, to show its tiles.
    pub(crate) picture: Option<crate::tileset_view::Picture<'a>>,
    /// The time animated tiles show.
    pub(crate) ms: u64,
    /// The accent colour, round the swatch painted with.
    pub(crate) accent: Color32,
}

/// A change asked for in the tab.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MapRequest {
    /// A layer on top, under sprites.
    AddLayer,
    /// Take this layer away.
    RemoveLayer(usize),
    /// Move this layer up (drawn later) or down.
    MoveLayer(usize, bool),
    /// Rename this layer, and say whether it's over sprites.
    SetLayer(usize, String, bool),
    /// Show or hide this layer in the editor (by ID; not saved).
    ToggleHidden(reflect::Id),
    /// A new value, a switch.
    AddValue,
    /// Take this value away.
    RemoveValue(usize),
    /// Rename this value.
    RenameValue(usize, String),
    /// Change this value's kind (or a choice's options).
    SetValueKind(usize, ValueKind),
    /// Paint with this terrain (by code).
    PaintWith(u8),
    /// Place this piece as an exact tile.
    PaintPiece(reflect::Id),
    /// Paint on this layer (a click on its row).
    PaintOnLayer(usize),
    /// Make this terrain (by code) solid, or not: saved in its tileset
    /// (decision 0071).
    SetSolid(u8, bool),
}

/// A terrain's colour, or the missing colour.
pub(crate) fn terrain_colour(set: Option<&Tileset>, code: u8) -> Color32 {
    match set.and_then(|t| t.by_code(char::from(code))) {
        Some(t) => {
            let [r, g, b] = t.colour.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
            Color32::from_rgb(r, g, b)
        }
        None => {
            let [r, g, b, _] = crate::tile_view::MISSING;
            Color32::from_rgb(r, g, b)
        }
    }
}

/// The wall mark on a solid terrain's swatch: a small box in the
/// collider colour in its top-right corner (decision 0071).
fn solid_mark(ui: &Ui, swatch: egui::Rect) {
    let mark = egui::Rect::from_min_size(swatch.right_top() + vec2(-17.0, 3.0), vec2(14.0, 14.0));
    let p = ui.painter();
    p.rect_filled(mark, 2.0, Color32::from_black_alpha(200));
    p.rect_stroke(
        mark,
        2.0,
        egui::Stroke::new(1.0, theme::COLLIDER),
        egui::StrokeKind::Inside,
    );
    // Bricks: two rows, the lower one offset.
    let line = egui::Stroke::new(1.0, theme::COLLIDER);
    let (l, r, t, b) = (
        mark.left() + 3.0,
        mark.right() - 3.0,
        mark.top() + 3.0,
        mark.bottom() - 3.0,
    );
    let mid = (t + b) * 0.5;
    p.line_segment([egui::pos2(l, mid), egui::pos2(r, mid)], line);
    p.line_segment(
        [egui::pos2((l + r) * 0.5, t), egui::pos2((l + r) * 0.5, mid)],
        line,
    );
    p.line_segment([egui::pos2(l + 2.0, mid), egui::pos2(l + 2.0, b)], line);
    p.line_segment([egui::pos2(r - 2.0, mid), egui::pos2(r - 2.0, b)], line);
}

/// A small picture of a layer: a grid of samples of its cells.
fn layer_thumb(ui: &mut Ui, map: &TileMap, layer: usize, set: Option<&Tileset>) {
    let (rect, _) = ui.allocate_exact_size(vec2(30.0, 20.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 2.0, theme::field());
    let [w, h] = map.size();
    let (cols, rows) = (6u32, 4u32);
    let step = vec2(rect.width() / cols as f32, rect.height() / rows as f32);
    for r in 0..rows {
        for c in 0..cols {
            let (x, y) = (c * w / cols, r * h / rows);
            if let Some(code) = map.code(layer, x, y).filter(|&c| c != 0) {
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        rect.min + vec2(c as f32 * step.x, r as f32 * step.y),
                        step,
                    ),
                    0.0,
                    terrain_colour(set, code),
                );
            }
        }
    }
}

/// A name field that counts once it's left (or Enter is pressed).
pub(crate) fn name_field(
    ui: &mut Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    now: &str,
    width: f32,
) -> Option<String> {
    let id = ui.id().with(("name field", salt));
    let mut text: String = ui
        .data(|d| d.get_temp(id))
        .unwrap_or_else(|| now.to_owned());
    let r = ui.add(egui::TextEdit::singleline(&mut text).desired_width(width));
    if r.has_focus() {
        ui.data_mut(|d| d.insert_temp(id, text.clone()));
        None
    } else {
        ui.data_mut(|d| d.remove::<String>(id));
        (r.lost_focus() && text != now)
            .then(|| tiles::tileset::name_of(&text).ok())
            .flatten()
    }
}

/// The Map tab (the approved additions, round 3,
/// `01-tile-map-painting-a.png`): four columns, the map's layers, its
/// tileset's terrains as the painter's palette, its cell values, and the
/// cell under the pointer. Gives back what was asked for.
pub(crate) fn map_tab(ui: &mut Ui, m: &MapUi<'_>) -> Vec<MapRequest> {
    use crate::widgets::columns::{Part, Wide, columns};
    let map = m.map;
    let [w, h] = map.size();
    let terrains = m.tileset.map_or_else(
        || "no tileset".to_owned(),
        |set| format!("{} terrains", set.terrains.len()),
    );
    let cell = m
        .hovered
        .map_or_else(String::new, |[x, y]| format!("x {x}, y {y}"));
    let parts = [
        Part::new(
            &m.name,
            format!("Tile map \u{b7} {w} \u{d7} {h} cells"),
            Wide::Fixed(LAYERS_W),
        ),
        Part::new("Terrains", terrains, Wide::Rest),
        Part::new(
            "Cell values",
            map.values().len().to_string(),
            Wide::Fixed(VALUES_W),
        ),
        Part::new("Cell", cell, Wide::Fixed(CELL_W)),
    ];
    let mut asked = Vec::new();
    let mut tools_asked = Vec::new();
    columns(
        ui,
        &parts,
        |ui, index| match index {
            0 => add_button(ui, &mut tools_asked, AddWhat::Layer, map),
            2 => add_button(ui, &mut tools_asked, AddWhat::Value, map),
            _ => {}
        },
        |ui, index| match index {
            0 => layers(ui, m, &mut asked),
            1 => palette(ui, m, &mut asked),
            2 => values(ui, map, &mut asked),
            _ => cell_now(ui, m),
        },
    );
    asked.extend(tools_asked);
    asked
}

/// The layers column's width, and the values' and the cell's (round 3).
const LAYERS_W: f32 = 290.0;
const VALUES_W: f32 = 300.0;
const CELL_W: f32 = 210.0;

/// What a column's + adds.
#[derive(Clone, Copy)]
enum AddWhat {
    Layer,
    Value,
}

/// A column header's +: a layer on top, or a value every cell has.
fn add_button(ui: &mut Ui, asked: &mut Vec<MapRequest>, what: AddWhat, map: &TileMap) {
    let (room, name, hint, most) = match what {
        AddWhat::Layer => (
            map.layers().len() < tiles::map::MAX_LAYERS,
            "Add a layer",
            "Add a layer on top",
            format!("A map has at most {} layers", tiles::map::MAX_LAYERS),
        ),
        AddWhat::Value => (
            map.values().len() < tiles::map::MAX_VALUES,
            "Add a value",
            "Add a value every cell has (a switch to start)",
            format!("A map has at most {} values", tiles::map::MAX_VALUES),
        ),
    };
    let add = ui
        .add_enabled(room, egui::Button::new("+").frame(false))
        .on_hover_text(hint)
        .on_disabled_hover_text(most);
    add.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
    if add.clicked() {
        asked.push(match what {
            AddWhat::Layer => MapRequest::AddLayer,
            AddWhat::Value => MapRequest::AddValue,
        });
    }
}

/// The map's layers, top first as they're drawn: a click on a row paints
/// on that layer (the project lead's answer 31).
fn layers(ui: &mut Ui, m: &MapUi<'_>, asked: &mut Vec<MapRequest>) {
    let map = m.map;
    for (i, layer) in map.layers().iter().enumerate().rev() {
        let painted = m.paint.layer == i;
        // The row's click is put under its controls: it is noted before
        // them, where the row was last drawn (one frame behind).
        let place = ui.id().with(("layer row", layer.id));
        let was: Option<egui::Rect> = ui.data(|d| d.get_temp(place));
        let row = was.map(|rect| ui.interact(rect, place, egui::Sense::click()));
        let drawn = egui::Frame::new()
            .fill(if painted {
                crate::interface_look::selected()
            } else {
                Color32::TRANSPARENT
            })
            .corner_radius(4.0)
            .inner_margin(4.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    layer_thumb(ui, map, i, m.tileset);
                    if let Some(name) = name_field(ui, ("layer", layer.id), &layer.name, 70.0) {
                        asked.push(MapRequest::SetLayer(i, name, layer.over));
                    }
                    let mut over = layer.over;
                    let choices = [(false, "under sprites"), (true, "over sprites")];
                    if crate::widgets::field::dropdown(ui, "Drawn", &mut over, &choices, 92.0) {
                        asked.push(MapRequest::SetLayer(i, layer.name.clone(), over));
                    }
                    let shown = !m.hidden.contains(&layer.id);
                    let eye = ui
                        .add(egui::Button::new(if shown { "◉" } else { "○" }).frame(false))
                        .on_hover_text(if shown {
                            "Shown in the editor: click to hide it here (the game always shows it)"
                        } else {
                            "Hidden in the editor: click to show it"
                        });
                    eye.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::Checkbox,
                            true,
                            shown,
                            format!("Show {}", layer.name),
                        )
                    });
                    if eye.clicked() {
                        asked.push(MapRequest::ToggleHidden(layer.id));
                    }
                    ui.menu_button("⋯", |ui| {
                        if ui
                            .add_enabled(i + 1 < map.layers().len(), egui::Button::new("Move up"))
                            .clicked()
                        {
                            asked.push(MapRequest::MoveLayer(i, true));
                            ui.close();
                        }
                        if ui
                            .add_enabled(i > 0, egui::Button::new("Move down"))
                            .clicked()
                        {
                            asked.push(MapRequest::MoveLayer(i, false));
                            ui.close();
                        }
                        if ui.button("Delete").clicked() {
                            asked.push(MapRequest::RemoveLayer(i));
                            ui.close();
                        }
                    })
                    .response
                    .on_hover_text("Move or delete this layer");
                });
            });
        ui.data_mut(|d| d.insert_temp(place, drawn.response.rect));
        let Some(row) = row else {
            continue;
        };
        let row = row.on_hover_text("Click to paint on this layer");
        row.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Button,
                true,
                painted,
                format!("Paint on {}", layer.name),
            )
        });
        if row.clicked() && !painted {
            asked.push(MapRequest::PaintOnLayer(i));
        }
    }
}

/// The tileset's terrains, with how many cells use each (every layer
/// together): the painter's palette.
fn palette(ui: &mut Ui, m: &MapUi<'_>, asked: &mut Vec<MapRequest>) {
    let map = m.map;
    let mut counts = [0usize; 256];
    for l in 0..map.layers().len() {
        for (total, n) in counts.iter_mut().zip(map.counts(l)) {
            *total += n;
        }
    }
    match m.tileset {
        None => {
            ui.label(
                RichText::new("No tileset: choose one in the map file's Properties (click it in the Library).")
                    .color(theme::WARNING),
            );
        }
        Some(set) => {
            ui.horizontal_wrapped(|ui| {
                for t in &set.terrains {
                    ui.vertical(|ui| {
                        let (rect, r) =
                            ui.allocate_exact_size(vec2(50.0, 50.0), egui::Sense::click());
                        let code = t.code as u8;
                        ui.painter()
                            .rect_filled(rect, 3.0, terrain_colour(Some(set), code));
                        let painted = m.paint.stamp.is_none()
                            && m.paint
                                .code
                                .filter(|&c| set.by_code(char::from(c)).is_some())
                                .or_else(|| set.terrains.first().map(|t| t.code as u8))
                                == Some(code);
                        if painted {
                            ui.painter().rect_stroke(
                                rect.expand(2.0),
                                4.0,
                                egui::Stroke::new(2.0, m.paint.accent),
                                egui::StrokeKind::Outside,
                            );
                        }
                        r.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::Button,
                                true,
                                painted,
                                format!("Paint with {}", t.name),
                            )
                        });
                        if r.clicked() {
                            asked.push(MapRequest::PaintWith(code));
                        }
                        ui.painter().text(
                            rect.min + vec2(4.0, 3.0),
                            egui::Align2::LEFT_TOP,
                            counts[usize::from(t.code as u8)].to_string(),
                            egui::FontId::proportional(10.0),
                            Color32::from_white_alpha(190),
                        );
                        if t.solid {
                            solid_mark(ui, rect);
                        }
                        // Right-click: the terrain's settings.
                        r.context_menu(|ui| {
                            ui.label(RichText::new(&t.name).strong());
                            let mut solid = t.solid;
                            if ui
                                .checkbox(&mut solid, "Solid")
                                .on_hover_text(
                                    "Nothing with a 2D collider passes into its cells: players, \
                                     NPCs and anything else that moves. Saved in the tileset, so \
                                     every map using it agrees.",
                                )
                                .changed()
                            {
                                asked.push(MapRequest::SetSolid(code, solid));
                                ui.close();
                            }
                        });
                        r.on_hover_text(format!(
                            "{}: written \"{}\" in the map file{}. Click to paint with it; \
                             right-click for its settings.",
                            t.name,
                            t.code,
                            if t.solid { ", solid" } else { "" }
                        ));
                        ui.label(RichText::new(format!("{} {}", t.name, t.code)).small());
                    });
                }
            });
            if let Some([w, h]) = m.paint.stamp {
                ui.label(
                    RichText::new(format!(
                        "Painting with a copy of {w} × {h} cells: X and Y flip it, Z turns it, Esc puts it away."
                    ))
                    .small()
                    .color(theme::text_weak()),
                );
            } else if m.paint.exact {
                exact_tiles(ui, set, m, asked);
            }
            let missing: usize = (1..256)
                .filter(|&c| set.by_code(char::from(c as u8)).is_none())
                .map(|c| counts[c])
                .sum();
            if missing > 0 {
                ui.label(
                    RichText::new(format!(
                        "{missing} cells have a code the tileset hasn't (drawn pink)."
                    ))
                    .color(theme::WARNING),
                );
            }
        }
    }
}

/// The values every cell has: name, kind, and a choice's options.
fn values(ui: &mut Ui, map: &TileMap, asked: &mut Vec<MapRequest>) {
    for (i, v) in map.values().iter().enumerate() {
        ui.horizontal(|ui| {
            if let Some(name) = name_field(ui, ("value", v.id), &v.name, 70.0) {
                asked.push(MapRequest::RenameValue(i, name));
            }
            let kinds = [
                ValueKind::Number,
                ValueKind::Switch,
                ValueKind::Choice(Vec::new()),
            ];
            let words: Vec<&str> = kinds.iter().map(ValueKind::word).collect();
            let list = crate::widgets::field::List {
                shown: v.kind.word(),
                note: None,
                width: 70.0,
            };
            if let Some(k) = crate::widgets::field::dropdown_of_names(ui, "Kind", &words, list) {
                asked.push(MapRequest::SetValueKind(i, kinds[k].clone()));
            }
            if let ValueKind::Choice(options) = &v.kind {
                let now = options.join(", ");
                let id = ui.id().with(("options", v.id));
                let mut text: String = ui.data(|d| d.get_temp(id)).unwrap_or_else(|| now.clone());
                let r = ui
                    .add(
                        egui::TextEdit::singleline(&mut text)
                            .desired_width(80.0)
                            .hint_text("wheat, corn"),
                    )
                    .on_hover_text("Its options, by name, with commas between");
                if r.has_focus() {
                    ui.data_mut(|d| d.insert_temp(id, text.clone()));
                } else {
                    ui.data_mut(|d| d.remove::<String>(id));
                    if r.lost_focus() && text != now {
                        let mut new: Vec<String> = Vec::new();
                        for o in text
                            .split(',')
                            .filter_map(|o| tiles::tileset::name_of(o).ok())
                        {
                            if !new.contains(&o) && new.len() < tiles::map::MAX_OPTIONS {
                                new.push(o);
                            }
                        }
                        asked.push(MapRequest::SetValueKind(i, ValueKind::Choice(new)));
                    }
                }
            }
            let x = ui
                .small_button("×")
                .on_hover_text("Take this value away from every cell");
            x.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    true,
                    format!("Remove {}", v.name),
                )
            });
            if x.clicked() {
                asked.push(MapRequest::RemoveValue(i));
            }
        });
    }
    if map.values().is_empty() {
        ui.label(
            RichText::new("None yet. A value is something every cell has: crop (a choice), growth (a number), watered (a switch).")
                .small()
                .color(theme::text_weak()),
        );
    }
}

/// The cell under the pointer: every layer's terrain and every value.
fn cell_now(ui: &mut Ui, m: &MapUi<'_>) {
    let map = m.map;
    match m.hovered {
        None => {
            ui.label(
                RichText::new("Point at a cell in the viewport to see it here.")
                    .small()
                    .color(theme::text_weak()),
            );
        }
        Some([x, y]) => {
            egui::Frame::new()
                .fill(theme::field())
                .corner_radius(4.0)
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    for (i, layer) in map.layers().iter().enumerate() {
                        crate::inspector::row(ui, &layer.name, "", |ui| match map.code(i, x, y) {
                            Some(0) | None => {
                                ui.label(RichText::new("—").color(theme::text_weak()));
                            }
                            Some(code) => {
                                let name = m
                                    .tileset
                                    .and_then(|t| t.by_code(char::from(code)))
                                    .map_or_else(
                                        || format!("\"{}\" (not in the tileset)", char::from(code)),
                                        |t| t.name.clone(),
                                    );
                                ui.label(RichText::new("■").color(terrain_colour(m.tileset, code)));
                                ui.label(name);
                            }
                        });
                    }
                    for (i, v) in map.values().iter().enumerate() {
                        let shown = match (map.value(i, x, y), &v.kind) {
                            (Some(CellValue::Number(n)), _) => n.to_string(),
                            (Some(CellValue::Switch(true)), _) => "✓".to_owned(),
                            (Some(CellValue::Switch(false)), _) => "—".to_owned(),
                            (Some(CellValue::Choice(Some(o))), ValueKind::Choice(options)) => {
                                options.get(usize::from(o)).cloned().unwrap_or_default()
                            }
                            _ => "—".to_owned(),
                        };
                        crate::inspector::row(ui, &v.name, "", |ui| ui.label(shown));
                    }
                });
        }
    }
    if m.playing {
        ui.label(
            RichText::new(
                "While playing, the map is the game's: watch a cell change as the game changes it.",
            )
            .small()
            .color(theme::text_weak()),
        );
    }
}

/// The painted terrain's tiles, to choose the one an exact tile shows.
fn exact_tiles(ui: &mut Ui, set: &Tileset, m: &MapUi<'_>, asked: &mut Vec<MapRequest>) {
    let Some(code) = m
        .paint
        .code
        .filter(|&c| set.by_code(char::from(c)).is_some())
        .or_else(|| set.terrains.first().map(|t| t.code as u8))
    else {
        return;
    };
    let Some(terrain) = set.by_code(char::from(code)) else {
        return;
    };
    let chosen = crate::tile_paint::exact_piece(set, code, m.paint.piece);
    ui.label(
        RichText::new(format!(
            "Exact tiles of {}: click one to place it as it is",
            terrain.name
        ))
        .small()
        .color(theme::text_weak()),
    );
    let pieces: Vec<usize> = (0..set.pieces.len())
        .filter(|&i| set.pieces[i].terrain == terrain.id)
        .collect();
    if pieces.is_empty() {
        ui.label(
            RichText::new("It has no art yet: give it tiles in the Tileset tab (double-click the tileset in the Library).")
                .small()
                .color(theme::WARNING),
        );
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
        for i in pieces {
            let piece = &set.pieces[i];
            let (rect, r) = ui.allocate_exact_size(vec2(28.0, 28.0), egui::Sense::click());
            match &m.paint.picture {
                Some(p) => crate::tileset_view::paint_piece(ui, rect, set, p, i, m.paint.ms),
                None => {
                    ui.painter()
                        .rect_filled(rect, 2.0, terrain_colour(Some(set), code));
                }
            }
            let on = chosen == Some(piece.id);
            if on {
                ui.painter().rect_stroke(
                    rect.expand(1.0),
                    2.0,
                    egui::Stroke::new(2.0, m.paint.accent),
                    egui::StrokeKind::Outside,
                );
            }
            let name = format!(
                "Tile at column {}, row {} (slot {})",
                piece.at[0] + 1,
                piece.at[1] + 1,
                piece.slot
            );
            r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, &name));
            if r.on_hover_text(&name).clicked() {
                asked.push(MapRequest::PaintPiece(piece.id));
            }
        }
    });
}

#[cfg(test)]
#[path = "map_tab_tests.rs"]
mod tests;
