//! The Scene panel: the scene's entities as a tree. Clicking one selects
//! it and shows it in Properties.

use crate::document::Document;
use crate::icons::{self, Icon};
use crate::models::ModelInfo;
use crate::theme::{self, Accent};
use egui::{Align, Color32, Layout, RichText, Ui};
use reflect::{AssetId, EntityId};
use scene::EntityData;
use scene::builtin::{Light, LightKind, Model};
use std::collections::BTreeMap;

/// How far each level of the tree is indented.
const INDENT: f32 = 14.0;

/// One row of the Scene panel: an icon, a name, a quiet note on the
/// right, highlighted when selected, and a × when `remove` names what it
/// removes. Returns the row's response and whether × was clicked.
pub(crate) fn scene_row(
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
                .on_hover_text(format!("Remove {what} from the scene"));
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
        return (Icon::SceneFile, accent.colors().light, "sky, fog".into());
    }
    (Icon::Scene, theme::TEXT_WEAK, String::new())
}

/// The Scene panel: every entity, as a tree. Selecting one changes the
/// document's selection; returns the entities whose × was clicked.
pub(crate) fn scene_tree(
    ui: &mut Ui,
    document: &mut Document,
    models: &BTreeMap<AssetId, ModelInfo>,
    accent: Accent,
) -> Vec<EntityId> {
    let mut removed = Vec::new();
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(6, 4))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                // Depth-first, parents before children.
                let mut todo: Vec<(EntityId, usize)> = document
                    .scene
                    .children(None)
                    .into_iter()
                    .rev()
                    .map(|id| (id, 0))
                    .collect();
                let mut shown = 0;
                while let Some((id, depth)) = todo.pop() {
                    shown += 1;
                    if shown > document.scene.len() {
                        break;
                    }
                    let Some(entity) = document.scene.get(id) else {
                        continue;
                    };
                    let (icon, color, note) = looks(entity, models, accent);
                    let removable = entity.has("Model").then_some(entity.name.as_str());
                    ui.horizontal(|ui| {
                        ui.add_space(depth as f32 * INDENT);
                        let (response, remove) = scene_row(
                            ui,
                            (icon, color),
                            &entity.name,
                            &note,
                            document.selection.contains(id),
                            accent,
                            removable,
                        );
                        if remove {
                            removed.push(id);
                        } else if response.clicked() {
                            document.selection.select_only(id);
                        }
                    });
                    for child in document.scene.children(Some(id)).into_iter().rev() {
                        todo.push((child, depth + 1));
                    }
                }
                ui.add_space(6.0);
                let note = if models.is_empty() {
                    "To see a model, use File > Import model, or drop a .gltf or .glb file onto \
                     the window. Entity > Add point light adds a lamp."
                } else {
                    "Scenes can't be saved yet: saving arrives later in Phase 2."
                };
                ui.label(RichText::new(note).color(theme::TEXT_WEAK).size(11.0));
            });
        });
    removed
}
