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
    let (row, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 21.0), egui::Sense::click());
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AreaRequest {
    /// Select only this entity.
    Select(EntityId),
    /// Delete this entity and everything below it.
    Delete(EntityId),
}

/// The Area panel: every entity, as a tree. Returns what the user asked
/// for.
pub(crate) fn area_tree(
    ui: &mut Ui,
    document: &Document,
    models: &BTreeMap<AssetId, ModelInfo>,
    accent: Accent,
) -> Vec<AreaRequest> {
    let mut requests = Vec::new();
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(6, 4))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                // Depth-first, parents before children.
                let mut todo: Vec<(EntityId, usize)> = document
                    .area()
                    .children(None)
                    .into_iter()
                    .rev()
                    .map(|id| (id, 0))
                    .collect();
                let mut shown = 0;
                while let Some((id, depth)) = todo.pop() {
                    shown += 1;
                    if shown > document.area().len() {
                        break;
                    }
                    let Some(entity) = document.area().get(id) else {
                        continue;
                    };
                    let (icon, color, note) = looks(entity, models, accent);
                    let removable = entity.has("Model").then_some(entity.name.as_str());
                    ui.horizontal(|ui| {
                        ui.add_space(depth as f32 * INDENT);
                        let (response, remove) = area_row(
                            ui,
                            (icon, color),
                            &entity.name,
                            &note,
                            document.selection.contains(id),
                            accent,
                            removable,
                        );
                        if remove {
                            requests.push(AreaRequest::Delete(id));
                        } else if response.clicked() {
                            requests.push(AreaRequest::Select(id));
                        }
                    });
                    for child in document.area().children(Some(id)).into_iter().rev() {
                        todo.push((child, depth + 1));
                    }
                }
                ui.add_space(6.0);
                let note = if models.is_empty() {
                    "To see a model, use File > Import model, or drop a .gltf or .glb file onto \
                     the window. Entity > Add point light adds a lamp."
                } else {
                    "Areas can't be saved yet: saving arrives later in Phase 2."
                };
                ui.label(RichText::new(note).color(theme::TEXT_WEAK).size(11.0));
            });
        });
    requests
}
