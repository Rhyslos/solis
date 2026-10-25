//! The Area panel: the area's entities as a tree. Clicking one selects
//! it and shows it in Properties.

use crate::document::Document;
use crate::icons::{self, Icon};
use crate::models::ModelInfo;
use crate::theme::{self, Accent};
use area::EntityData;
use area::builtin::{Light, LightKind, Model};
use egui::{Align, Color32, Layout, RichText, Ui};
use reflect::{AssetId, EntityId};
use std::collections::BTreeMap;

/// How far each level of the tree is indented.
const INDENT: f32 = 14.0;

/// One row of the Area panel: an icon, a name, a quiet note on the
/// right, highlighted when selected, and a × when `remove` names what it
/// removes. Returns the row's response and whether × was clicked.
pub(crate) fn area_row(
    ui: &mut Ui,
    (icon, icon_color): (Icon, Color32),
    name: &str,
    note: &str,
    selected: bool,
    accent: Accent,
    remove: Option<&str>,
) -> (egui::Response, bool) {
    let (row, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 21.0),
        egui::Sense::click_and_drag(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, name)
    });
    if selected {
        ui.painter()
            .rect_filled(row, 3.0, accent.colors().fill.gamma_multiply(0.45));
    } else if response.hovered() {
        ui.painter().rect_filled(row, 3.0, theme::HOVER);
    }
    let mut row_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(row.shrink2(egui::vec2(4.0, 0.0)))
            .layout(Layout::left_to_right(Align::Center)),
    );
    let ui = &mut row_ui;
    let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
    icons::icon(ui.painter(), icon_rect, icon, icon_color);
    ui.add(egui::Label::new(name).truncate().selectable(false));
    let mut removed = false;
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if let Some(what) = remove {
            let button = ui
                .add(egui::Button::new(RichText::new("×").color(theme::TEXT_WEAK)).frame(false))
                .on_hover_text(format!("Remove {what} from the area"));
            button.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Remove {what}"))
            });
            removed = button.clicked();
        }
        ui.add(egui::Label::new(RichText::new(note).color(theme::TEXT_WEAK)).selectable(false));
    });
    (response, removed)
}

/// An entity's icon and note, from its components.
fn looks(
    entity: &EntityData,
    models: &BTreeMap<AssetId, ModelInfo>,
    accent: Accent,
) -> (Icon, Color32, String) {
    if let Some(light) = entity.get::<Light>() {
        let [r, g, b] = light.color.map(|c| (c * 255.0).round() as u8);
        let color = if light.kind == LightKind::Sun {
            theme::WARNING
        } else {
            Color32::from_rgb(r, g, b)
        };
        let note = match light.kind {
            LightKind::Sun => "",
            LightKind::Point => "Point light",
            LightKind::Spot => "Spotlight",
        };
        return (Icon::Light, color, note.to_string());
    }
    if let Some(model) = entity.get::<Model>() {
        let note = model
            .asset
            .and_then(|r| models.get(&r.asset))
            .map_or_else(String::new, |m| {
                format!("{} tris", crate::models::grouped(m.triangles))
            });
        return (Icon::Model, accent.colors().light, note);
    }
    if entity.has("Environment") {
        return (Icon::AreaFile, accent.colors().light, "sky, fog".into());
    }
    (Icon::Area, theme::TEXT_WEAK, String::new())
}

/// Something the user asked for in the Area panel. Like Properties, the
/// panel never changes the area or the selection itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AreaRequest {
    /// Select only this entity (a click).
    Select(EntityId),
    /// Add it to the selection, or take it out (Ctrl+click).
    Toggle(EntityId),
    /// Select every row from the last one picked to this one
    /// (Shift+click).
    SelectRange(EntityId),
    /// Delete this entity and everything below it.
    Delete(EntityId),
    /// Move an entity (and the rest of the selection, if it's selected)
    /// under another, or to the top level with `None` (drag and drop).
    Reparent {
        /// The entity dragged.
        entity: EntityId,
        /// Where it was dropped.
        onto: Option<EntityId>,
    },
    /// Give an entity a new name (typed after F2 or a double-click).
    Rename {
        /// The entity.
        entity: EntityId,
        /// Its new name.
        name: String,
    },
}

/// Where the name being typed is kept between frames.
fn rename_key() -> egui::Id {
    egui::Id::new("area-panel-rename")
}

/// Starts renaming `id` in the Area panel (F2), with `name` to edit.
pub(crate) fn begin_rename(ctx: &egui::Context, id: EntityId, name: &str) {
    ctx.data_mut(|d| d.insert_temp(rename_key(), (id, name.to_string(), true)));
}

/// The name being typed for an entity, if one is being renamed:
/// (entity, text, whether the box still needs the keyboard focus).
fn renaming(ctx: &egui::Context) -> Option<(EntityId, String, bool)> {
    ctx.data(|d| d.get_temp(rename_key()))
}

fn stop_renaming(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove::<(EntityId, String, bool)>(rename_key()));
}

/// The Area panel: every entity, as a tree. Click to select, Ctrl+click to
/// add or remove, Shift+click for a range, drag onto another to put it
/// under that one, or onto empty space to move it to the top level;
/// double-click to rename. Returns what the user asked for.
pub(crate) fn area_tree(
    ui: &mut Ui,
    document: &Document,
    models: &BTreeMap<AssetId, ModelInfo>,
    accent: Accent,
) -> Vec<AreaRequest> {
    let mut requests = Vec::new();
    let area = document.area();
    let tree = area.hierarchy();
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(6, 4))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    // Depth-first, parents before children; a loop in a
                    // damaged area is cut where it comes round again.
                    let mut seen = std::collections::HashSet::new();
                    let mut todo: Vec<(EntityId, usize)> = tree
                        .get(&None)
                        .map(|top| top.iter().rev().map(|&id| (id, 0)).collect())
                        .unwrap_or_default();
                    while let Some((id, depth)) = todo.pop() {
                        if !seen.insert(id) {
                            continue;
                        }
                        let Some(entity) = area.get(id) else {
                            continue;
                        };
                        ui.horizontal(|ui| {
                            ui.add_space(depth as f32 * INDENT);
                            entity_row(ui, document, models, accent, id, entity, &mut requests);
                        });
                        if let Some(kids) = tree.get(&Some(id)) {
                            todo.extend(kids.iter().rev().map(|&k| (k, depth + 1)));
                        }
                    }
                    ui.add_space(6.0);
                    let note = if models.is_empty() {
                        "To see a model, use File > Import model, or drop a .gltf or .glb \
                         file onto the window. Entity > Add point light adds a lamp."
                    } else {
                        "Areas can't be saved yet: saving arrives later in Phase 2."
                    };
                    ui.label(RichText::new(note).color(theme::TEXT_WEAK).size(11.0));
                    // The rest of the panel: dropping here moves to the top
                    // level.
                    let rest = ui.available_size().max(egui::vec2(0.0, 24.0));
                    let (_, empty) = ui.allocate_exact_size(rest, egui::Sense::hover());
                    if let Some(dragged) = empty.dnd_release_payload::<EntityId>() {
                        requests.push(AreaRequest::Reparent {
                            entity: *dragged,
                            onto: None,
                        });
                    }
                });
        });
    requests
}

/// One entity's row: its name (or the box to rename it), and what clicks,
/// drags and drops on it ask for.
fn entity_row(
    ui: &mut Ui,
    document: &Document,
    models: &BTreeMap<AssetId, ModelInfo>,
    accent: Accent,
    id: EntityId,
    entity: &EntityData,
    requests: &mut Vec<AreaRequest>,
) {
    if let Some((renamed, mut text, focus)) = renaming(ui.ctx()).filter(|(r, ..)| *r == id) {
        let edit = ui.add(egui::TextEdit::singleline(&mut text).desired_width(f32::INFINITY));
        if focus {
            edit.request_focus();
        }
        let (enter, escape) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        });
        if escape {
            stop_renaming(ui.ctx());
        } else if edit.lost_focus() || enter {
            stop_renaming(ui.ctx());
            if text != entity.name {
                requests.push(AreaRequest::Rename {
                    entity: renamed,
                    name: text,
                });
            }
        } else {
            ui.ctx()
                .data_mut(|d| d.insert_temp(rename_key(), (renamed, text, false)));
        }
        return;
    }
    let (icon, color, note) = looks(entity, models, accent);
    let removable = entity.has("Model").then_some(entity.name.as_str());
    let (response, remove) = area_row(
        ui,
        (icon, color),
        &entity.name,
        &note,
        document.selection.contains(id),
        accent,
        removable,
    );
    response.dnd_set_drag_payload(id);
    if let Some(dragged) = response.dnd_release_payload::<EntityId>() {
        if *dragged != id {
            requests.push(AreaRequest::Reparent {
                entity: *dragged,
                onto: Some(id),
            });
        }
    } else if response
        .dnd_hover_payload::<EntityId>()
        .is_some_and(|d| *d != id)
    {
        ui.painter().rect_stroke(
            response.rect,
            3.0,
            egui::Stroke::new(1.5, accent.colors().light),
            egui::StrokeKind::Inside,
        );
    }
    let modifiers = ui.input(|i| i.modifiers);
    if remove {
        requests.push(AreaRequest::Delete(id));
    } else if response.double_clicked() {
        begin_rename(ui.ctx(), id, &entity.name);
    } else if response.clicked() {
        requests.push(if modifiers.command {
            AreaRequest::Toggle(id)
        } else if modifiers.shift {
            AreaRequest::SelectRange(id)
        } else {
            AreaRequest::Select(id)
        });
    }
}
