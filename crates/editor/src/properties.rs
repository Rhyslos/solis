//! The Properties panel: the selected entity's name and components, with
//! fields drawn from each component's schema (decision 0019). It never
//! changes the area itself: it returns what the user asked for, and the
//! workbench applies it (as undo steps, decision 0022).

use crate::document::Document;
use crate::inspector;
use crate::models::ModelInfo;
use crate::panels::placeholder;
use crate::theme;
use egui::{RichText, Ui};
use reflect::{AssetId, EntityId, Value};
use std::collections::BTreeMap;

/// Something the user asked for in Properties.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PropertyRequest {
    /// Set a component's field.
    SetField {
        /// The entity.
        entity: EntityId,
        /// The component's name.
        component: String,
        /// The field's name.
        field: String,
        /// Its new value.
        value: Value,
    },
    /// Rename an entity.
    Rename {
        /// The entity.
        entity: EntityId,
        /// Its new name.
        name: String,
    },
    /// Take a component off an entity.
    RemoveComponent {
        /// The entity.
        entity: EntityId,
        /// The component's name.
        component: String,
    },
    /// Delete an entity and everything below it.
    Delete(EntityId),
    /// Move an entity to where the camera is looking.
    MoveHere(EntityId),
}

/// The Properties panel.
pub(crate) fn properties(
    ui: &mut Ui,
    document: &Document,
    models: &BTreeMap<AssetId, ModelInfo>,
) -> Vec<PropertyRequest> {
    let mut requests = Vec::new();
    let Some((id, entity)) = document
        .selection
        .primary()
        .and_then(|id| document.area().get(id).map(|e| (id, e)))
    else {
        placeholder(ui, "Nothing selected. Pick something in the Area panel.");
        return requests;
    };
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(theme::panel_margin())
                .show(ui, |ui| {
                    let mut name = entity.name.clone();
                    inspector::row(ui, "Name", "What the Area panel calls it.", |ui| {
                        if ui.text_edit_singleline(&mut name).changed() {
                            requests.push(PropertyRequest::Rename { entity: id, name });
                        }
                    });
                    ui.add_space(4.0);
                    // Transform first, as in every engine; the rest by name.
                    let mut names: Vec<&String> = entity.components.keys().collect();
                    names.sort_by_key(|n| (n.as_str() != "Transform", n.as_str()));
                    for component in names {
                        let data = &entity.components[component];
                        ui.separator();
                        match document.registry.get(component) {
                            Some(schema) => {
                                ui.label(RichText::new(component).strong())
                                    .on_hover_text(&schema.help);
                                for change in inspector::fields(ui, schema, data, models) {
                                    requests.push(PropertyRequest::SetField {
                                        entity: id,
                                        component: component.clone(),
                                        field: change.field,
                                        value: change.value,
                                    });
                                }
                            }
                            None => {
                                ui.label(
                                    RichText::new(format!("Missing component: {component}"))
                                        .color(theme::WARNING),
                                )
                                .on_hover_text(
                                    "Its script or plugin isn't there. Its values are kept as \
                                     they are until you remove it.",
                                );
                                if ui.button("Remove").clicked() {
                                    requests.push(PropertyRequest::RemoveComponent {
                                        entity: id,
                                        component: component.clone(),
                                    });
                                }
                            }
                        }
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if entity.has("Transform")
                            && ui
                                .button("Move here")
                                .on_hover_text(
                                    "Moves it to where the camera looks (a spotlight to the \
                                     camera, shining the same way)",
                                )
                                .clicked()
                        {
                            requests.push(PropertyRequest::MoveHere(id));
                        }
                        if ui
                            .button("Delete")
                            .on_hover_text("Deletes it and everything belonging to it")
                            .clicked()
                        {
                            requests.push(PropertyRequest::Delete(id));
                        }
                    });
                });
        });
    requests
}
