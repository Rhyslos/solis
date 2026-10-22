//! The Scene panel: until entities arrive in Phase 2, the sun, the
//! environment, the lamps and the imported models. Clicking one shows it
//! in Properties.

use crate::icons::{self, Icon};
use crate::lighting::{Lamp, Selection};
use crate::models::{ModelId, ModelInfo};
use crate::theme::{self, Accent};
use egui::{Align, Color32, Layout, RichText, Ui};

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
                .on_hover_text(format!("Remove {what} from the viewport"));
            button.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Remove {what}"))
            });
            removed = button.clicked();
        }
        ui.add(egui::Label::new(RichText::new(note).color(theme::TEXT_WEAK)).selectable(false));
    });
    (response, removed)
}

/// The Scene panel's contents until entities arrive: the sun, the lamps
/// and the imported models. Clicking one shows it in Properties.
pub(crate) fn scene_list(
    ui: &mut Ui,
    lamps: &[Lamp],
    models: &[ModelInfo],
    selection: &mut Selection,
    removed: &mut Vec<ModelId>,
    accent: Accent,
) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(6, 4))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            let light = theme::WARNING;
            if scene_row(
                ui,
                (Icon::Light, light),
                "Sun",
                "",
                *selection == Selection::Sun,
                accent,
                None,
            )
            .0
            .clicked()
            {
                *selection = Selection::Sun;
            }
            if scene_row(
                ui,
                (Icon::SceneFile, accent.colors().light),
                "Environment",
                "sky, fog",
                *selection == Selection::Environment,
                accent,
                None,
            )
            .0
            .clicked()
            {
                *selection = Selection::Environment;
            }
            for lamp in lamps {
                let [r, g, b] = lamp.color;
                if scene_row(
                    ui,
                    (Icon::Light, Color32::from_rgb(r, g, b)),
                    &lamp.name,
                    lamp.kind.name(),
                    *selection == Selection::Lamp(lamp.id),
                    accent,
                    None,
                )
                .0
                .clicked()
                {
                    *selection = Selection::Lamp(lamp.id);
                }
            }
            for model in models {
                let (response, remove) = scene_row(
                    ui,
                    (Icon::Model, accent.colors().light),
                    &model.name,
                    &format!("{} tris", crate::models::grouped(model.triangles)),
                    *selection == Selection::Model(model.id),
                    accent,
                    Some(&model.name),
                );
                if remove {
                    removed.push(model.id);
                } else if response.clicked() {
                    *selection = Selection::Model(model.id);
                }
            }
            ui.add_space(6.0);
            let note = if models.is_empty() && lamps.is_empty() {
                "To see a model, use File > Import model, or drop a .gltf or .glb file onto \
                 the window. Entity > Add point light adds a lamp."
            } else {
                "Models and lamps aren't saved yet: scenes arrive in Phase 2."
            };
            ui.label(RichText::new(note).color(theme::TEXT_WEAK).size(11.0));
        });
}
