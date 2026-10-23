//! The Library panel: the project's assets
//! ([decision 0021](../../../docs/decisions/0021-asset-database-and-models.md)).
//! The app keeps the list up to date from the asset database; the panel
//! only shows it and asks for an asset to be placed.

use crate::icons::{self, Icon};
use crate::theme::{self, Accent};
use egui::{RichText, Ui};
use reflect::AssetId;
use std::path::PathBuf;

/// The open project, as the editor shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInfo {
    /// Its name.
    pub name: String,
    /// Its folder.
    pub root: PathBuf,
}

/// What kind of asset a Library item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibraryKind {
    /// A model.
    #[default]
    Model,
    /// An assembly.
    Assembly,
}

/// One asset in the Library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryItem {
    /// Its ID.
    pub id: AssetId,
    /// What kind of asset.
    pub kind: LibraryKind,
    /// Its name (the file's, without the extension).
    pub name: String,
    /// Where it is inside the project's assets, for the tooltip.
    pub path: String,
}

/// The Library panel's contents. Returns the asset to place, if one was
/// double-clicked.
pub(crate) fn library(
    ui: &mut Ui,
    project: Option<&ProjectInfo>,
    items: &[LibraryItem],
    accent: Accent,
) -> Option<AssetId> {
    let mut place = None;
    egui::Frame::new()
        .inner_margin(theme::panel_margin())
        .show(ui, |ui| {
            let Some(project) = project else {
                ui.label(
                    RichText::new(
                        "No project is open. File > New project… makes one: a folder whose \
                         models are kept here, with IDs that survive renaming.",
                    )
                    .color(theme::TEXT_WEAK),
                );
                return;
            };
            if items.is_empty() {
                ui.label(
                    RichText::new(format!(
                        "{} has no models yet. Import one (File > Import model…), drop one onto \
                         the window, or put .glb and .gltf files in its assets folder.",
                        project.name
                    ))
                    .color(theme::TEXT_WEAK),
                );
                return;
            }
            egui::ScrollArea::vertical().auto_shrink(false).show_rows(
                ui,
                21.0,
                items.len(),
                |ui, range| {
                    for item in &items[range] {
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 21.0),
                            egui::Sense::click(),
                        );
                        if response.hovered() {
                            ui.painter().rect_filled(rect, 3.0, theme::HOVER);
                        }
                        let icon = egui::Rect::from_min_size(
                            egui::pos2(rect.left() + 4.0, rect.center().y - 6.0),
                            egui::vec2(12.0, 12.0),
                        );
                        let (glyph, color) = match item.kind {
                            LibraryKind::Model => (Icon::Model, accent.colors().light),
                            LibraryKind::Assembly => (Icon::Area, theme::FOLDER),
                        };
                        icons::icon(ui.painter(), icon, glyph, color);
                        ui.painter().text(
                            egui::pos2(rect.left() + 22.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            &item.name,
                            egui::TextStyle::Body.resolve(ui.style()),
                            ui.visuals().text_color(),
                        );
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &item.name)
                        });
                        let response = response.on_hover_text(format!(
                            "{}\nDouble-click to place it where the camera looks.",
                            item.path
                        ));
                        // A + on the right does the same as a double-click.
                        let plus = egui::Rect::from_min_size(
                            egui::pos2(rect.right() - 22.0, rect.top()),
                            egui::vec2(20.0, rect.height()),
                        );
                        let button = ui
                            .put(
                                plus,
                                egui::Button::new(RichText::new("+").color(theme::TEXT_WEAK))
                                    .frame(false),
                            )
                            .on_hover_text("Place it where the camera looks");
                        button.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                true,
                                format!("Place {}", item.name),
                            )
                        });
                        if response.double_clicked() || button.clicked() {
                            place = Some(item.id);
                        }
                    }
                },
            );
        });
    place
}
