//! The Map tab (Phase 9, D4), beside Properties while a tile map is
//! selected, as the project lead chose it (mock-up A): the map's layers
//! stacked with a little picture of each, shown or hidden in the editor and
//! drawn under or over sprites; its tileset's terrains as big swatches with
//! how many cells use each; its cell values; and the cell under the
//! pointer in the viewport, every layer and value. Changes go back as
//! [`MapRequest`]s; the workbench makes each one undo step.

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
}

/// A terrain's colour, or the missing colour.
fn terrain_colour(set: Option<&Tileset>, code: u8) -> Color32 {
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

/// A small picture of a layer: a grid of samples of its cells.
fn layer_thumb(ui: &mut Ui, map: &TileMap, layer: usize, set: Option<&Tileset>) {
    let (rect, _) = ui.allocate_exact_size(vec2(30.0, 20.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 2.0, theme::FIELD);
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

/// The Map tab. Gives back what was asked for.
pub(crate) fn map_tab(ui: &mut Ui, m: &MapUi<'_>) -> Vec<MapRequest> {
    let mut asked = Vec::new();
    let map = m.map;
    ui.horizontal(|ui| {
        ui.label(RichText::new(&m.name).strong());
        ui.label(
            RichText::new(format!("{} × {} cells", map.size()[0], map.size()[1]))
                .color(theme::TEXT_WEAK),
        );
    });
    ui.add_space(6.0);
    // Layers, top first as they're drawn.
    ui.horizontal(|ui| {
        ui.label(RichText::new("Layers").color(theme::TEXT_WEAK));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let add = ui
                .add_enabled(
                    map.layers().len() < tiles::map::MAX_LAYERS,
                    egui::Button::new("+"),
                )
                .on_hover_text("Add a layer on top");
            add.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Add a layer")
            });
            if add.clicked() {
                asked.push(MapRequest::AddLayer);
            }
        });
    });
    for (i, layer) in map.layers().iter().enumerate().rev() {
        egui::Frame::new()
            .fill(if i == 0 {
                Color32::from_rgb(0x3a, 0x2e, 0x18)
            } else {
                theme::FIELD
            })
            .corner_radius(4.0)
            .inner_margin(4.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    layer_thumb(ui, map, i, m.tileset);
                    if let Some(name) = name_field(ui, ("layer", layer.id), &layer.name, 96.0) {
                        asked.push(MapRequest::SetLayer(i, name, layer.over));
                    }
                    egui::ComboBox::from_id_salt(("over", layer.id))
                        .selected_text(if layer.over {
                            "over sprites"
                        } else {
                            "under sprites"
                        })
                        .width(104.0)
                        .show_ui(ui, |ui| {
                            for (over, text) in [(false, "under sprites"), (true, "over sprites")] {
                                if ui.selectable_label(layer.over == over, text).clicked()
                                    && layer.over != over
                                {
                                    asked.push(MapRequest::SetLayer(i, layer.name.clone(), over));
                                }
                            }
                        });
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
                    ui.menu_button("…", |ui| {
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
                    });
                });
            });
    }
    ui.add_space(8.0);
    // Terrains, with how many cells use each (every layer together).
    ui.horizontal(|ui| {
        ui.label(RichText::new("Terrains").color(theme::TEXT_WEAK));
        ui.label(
            RichText::new("(cells using each)")
                .small()
                .color(theme::TEXT_WEAK),
        );
    });
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
                            ui.allocate_exact_size(vec2(50.0, 50.0), egui::Sense::hover());
                        ui.painter().rect_filled(
                            rect,
                            3.0,
                            terrain_colour(Some(set), t.code as u8),
                        );
                        ui.painter().text(
                            rect.min + vec2(4.0, 3.0),
                            egui::Align2::LEFT_TOP,
                            counts[usize::from(t.code as u8)].to_string(),
                            egui::FontId::proportional(10.0),
                            Color32::from_white_alpha(190),
                        );
                        r.on_hover_text(format!(
                            "{}: written \"{}\" in the map file",
                            t.name, t.code
                        ));
                        ui.label(RichText::new(format!("{} {}", t.name, t.code)).small());
                    });
                }
            });
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
    ui.add_space(8.0);
    // Cell values.
    ui.horizontal(|ui| {
        ui.label(RichText::new("Cell values").color(theme::TEXT_WEAK));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let add = ui
                .add_enabled(
                    map.values().len() < tiles::map::MAX_VALUES,
                    egui::Button::new("+"),
                )
                .on_hover_text("Add a value every cell has (a switch to start)");
            add.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Add a value")
            });
            if add.clicked() {
                asked.push(MapRequest::AddValue);
            }
        });
    });
    for (i, v) in map.values().iter().enumerate() {
        ui.horizontal(|ui| {
            if let Some(name) = name_field(ui, ("value", v.id), &v.name, 80.0) {
                asked.push(MapRequest::RenameValue(i, name));
            }
            egui::ComboBox::from_id_salt(("kind", v.id))
                .selected_text(v.kind.word())
                .width(70.0)
                .show_ui(ui, |ui| {
                    for kind in [
                        ValueKind::Number,
                        ValueKind::Switch,
                        ValueKind::Choice(Vec::new()),
                    ] {
                        let same = std::mem::discriminant(&kind) == std::mem::discriminant(&v.kind);
                        if ui.selectable_label(same, kind.word()).clicked() && !same {
                            asked.push(MapRequest::SetValueKind(i, kind));
                        }
                    }
                });
            if let ValueKind::Choice(options) = &v.kind {
                let now = options.join(", ");
                let id = ui.id().with(("options", v.id));
                let mut text: String = ui.data(|d| d.get_temp(id)).unwrap_or_else(|| now.clone());
                let r = ui
                    .add(
                        egui::TextEdit::singleline(&mut text)
                            .desired_width(110.0)
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
                .color(theme::TEXT_WEAK),
        );
    }
    ui.add_space(8.0);
    // The cell under the pointer.
    match m.hovered {
        None => {
            ui.label(
                RichText::new("Point at a cell in the viewport to see it here.")
                    .small()
                    .color(theme::TEXT_WEAK),
            );
        }
        Some([x, y]) => {
            ui.label(RichText::new(format!("Cell · x {x}, y {y}")).color(theme::TEXT_WEAK));
            egui::Frame::new()
                .fill(theme::FIELD)
                .corner_radius(4.0)
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    for (i, layer) in map.layers().iter().enumerate() {
                        crate::inspector::row(ui, &layer.name, "", |ui| match map.code(i, x, y) {
                            Some(0) | None => {
                                ui.label(RichText::new("—").color(theme::TEXT_WEAK));
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
    ui.add_space(6.0);
    ui.label(
        RichText::new(if m.playing {
            "While playing, the map is the game's: watch a cell change as the game changes it."
        } else {
            "Changes save at once; Ctrl+Z over this tab undoes them. Painting cells comes with the tile painter."
        })
        .small()
        .color(theme::TEXT_WEAK),
    );
    asked
}

/// A tileset's Properties (clicked in the Library): its terrains, each
/// with its colour, name and code. Gives back the tileset changed, if it
/// was.
pub(crate) fn tileset_properties(
    ui: &mut Ui,
    path: &str,
    set: &Tileset,
    notes: &[String],
) -> Option<Tileset> {
    let mut changed: Option<Tileset> = None;
    let name = path.rsplit_once('/').map_or(path, |(_, n)| n);
    ui.horizontal(|ui| {
        ui.label(RichText::new("▦").color(theme::kind::SPRITE));
        ui.label(RichText::new(name).strong());
    });
    ui.label(RichText::new("Terrains (what a cell can be)").color(theme::TEXT_WEAK));
    for (i, t) in set.terrains.iter().enumerate() {
        ui.horizontal(|ui| {
            let mut c = t.colour.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
            if ui.color_edit_button_srgb(&mut c).changed() {
                let mut s = changed.clone().unwrap_or_else(|| set.clone());
                s.terrains[i].colour = c.map(|v| f32::from(v) / 255.0);
                changed = Some(s);
            }
            if let Some(n) = name_field(ui, ("terrain", t.id), &t.name, 110.0)
                && set.by_name(&n).is_none()
            {
                let mut s = changed.clone().unwrap_or_else(|| set.clone());
                s.terrains[i].name = n;
                changed = Some(s);
            }
            let id = ui.id().with(("code", t.id));
            let mut code: String = ui
                .data(|d| d.get_temp(id))
                .unwrap_or_else(|| t.code.to_string());
            let r = ui
                .add(
                    egui::TextEdit::singleline(&mut code)
                        .desired_width(18.0)
                        .char_limit(1),
                )
                .on_hover_text("The one character the map file writes for it");
            if r.has_focus() {
                ui.data_mut(|d| d.insert_temp(id, code.clone()));
            } else {
                ui.data_mut(|d| d.remove::<String>(id));
                if let Some(c) = code.chars().next()
                    && r.lost_focus()
                    && c != t.code
                    && tiles::tileset::usable_code(c)
                    && set.by_code(c).is_none()
                {
                    let mut s = changed.clone().unwrap_or_else(|| set.clone());
                    s.terrains[i].code = c;
                    changed = Some(s);
                }
            }
            let x = ui
                .small_button("×")
                .on_hover_text("Take this terrain away (maps keep its code, drawn pink)");
            x.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    true,
                    format!("Remove {}", t.name),
                )
            });
            if x.clicked() {
                let mut s = changed.clone().unwrap_or_else(|| set.clone());
                let gone = s.terrains.remove(i).id;
                s.pieces.retain(|p| p.terrain != gone);
                s.rules.retain(|r| r.terrain != gone);
                changed = Some(s);
            }
        });
    }
    let can_add = set.terrains.len() < tiles::tileset::MAX_TERRAINS;
    if ui
        .add_enabled(can_add, egui::Button::new("+ Terrain"))
        .clicked()
        && let Some(code) = set.free_code()
    {
        let mut s = changed.clone().unwrap_or_else(|| set.clone());
        let name = (1..)
            .map(|n| {
                if n == 1 {
                    "terrain".to_owned()
                } else {
                    format!("terrain{n}")
                }
            })
            .find(|n| set.by_name(n).is_none())
            .unwrap_or_default();
        s.terrains.push(tiles::Terrain {
            id: reflect::Id::random(),
            name,
            code,
            colour: [0.5; 3],
            kind: tiles::TileKind::Plain,
        });
        changed = Some(s);
    }
    ui.label(
        RichText::new("Its name, the character the map file writes for it, and its colour. The first fills new maps. Double-click the tileset in the Library to give its terrains art in the Tileset tab.")
            .small()
            .color(theme::TEXT_WEAK),
    );
    for note in notes {
        ui.label(RichText::new(note).small().color(theme::WARNING));
    }
    changed
}

/// A map file's Properties (clicked in the Library): its tileset and
/// size. Gives back the tileset and size chosen, if changed.
pub(crate) fn map_file_properties(
    ui: &mut Ui,
    path: &str,
    map: &TileMap,
    tilesets: &[(String, reflect::AssetId)],
    notes: &[String],
) -> Option<(Option<reflect::AssetId>, [u32; 2])> {
    let mut out = None;
    let name = path.rsplit_once('/').map_or(path, |(_, n)| n);
    ui.horizontal(|ui| {
        ui.label(RichText::new("▦").color(theme::kind::SPRITE));
        ui.label(RichText::new(name).strong());
    });
    crate::inspector::row(ui, "Tileset", "The terrains its cells can be.", |ui| {
        let shown = match map.tileset {
            None => "None".to_owned(),
            Some(id) => tilesets
                .iter()
                .find(|(_, i)| *i == id)
                .map_or_else(|| format!("Missing tileset {id}"), |(n, _)| n.clone()),
        };
        egui::ComboBox::from_id_salt(("map tileset", path))
            .selected_text(shown)
            .show_ui(ui, |ui| {
                for (n, id) in tilesets {
                    if ui.selectable_label(map.tileset == Some(*id), n).clicked()
                        && map.tileset != Some(*id)
                    {
                        out = Some((Some(*id), map.size()));
                    }
                }
                if tilesets.is_empty() {
                    ui.label("No tilesets yet: Library > Import > New tileset.");
                }
            });
    });
    crate::inspector::row(
        ui,
        "Size",
        "Its width and height in cells; cells past a smaller size are taken away.",
        |ui| {
            let [w, h] = map.size();
            let max = f64::from(tiles::map::MAX_SIDE);
            let a = crate::frames_view::settled_whole_drag(
                ui,
                &format!("map w {path}"),
                f64::from(w),
                1.0..=max,
                0.2,
            );
            ui.label(RichText::new("×").color(theme::TEXT_WEAK));
            let b = crate::frames_view::settled_whole_drag(
                ui,
                &format!("map h {path}"),
                f64::from(h),
                1.0..=max,
                0.2,
            );
            ui.label(RichText::new("cells").color(theme::TEXT_WEAK));
            if a.is_some() || b.is_some() {
                out = Some((
                    map.tileset,
                    [a.map_or(w, |v| v as u32), b.map_or(h, |v| v as u32)],
                ));
            }
        },
    );
    ui.label(
        RichText::new(format!(
            "{} layers · {} values. Select an entity placing it to change its layers and values in the Map tab.",
            map.layers().len(),
            map.values().len()
        ))
        .small()
        .color(theme::TEXT_WEAK),
    );
    for note in notes {
        ui.label(RichText::new(note).small().color(theme::WARNING));
    }
    out
}

#[cfg(test)]
#[path = "map_tab_tests.rs"]
mod tests;
