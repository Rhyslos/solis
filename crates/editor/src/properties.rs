//! The Properties panel: the selected entity's name and components, with
//! fields drawn from each component's schema (decision 0019). It never
//! changes the area itself: it returns what the user asked for, and the
//! workbench applies it (as undo steps, decision 0022).

use crate::document::Document;
use crate::inspector::{self, FieldChange};
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
    /// Give an entity a component it doesn't have, with its starting
    /// values.
    AddComponent {
        /// The entity.
        entity: EntityId,
        /// The component's name.
        component: String,
    },
    /// Take a component off an entity.
    RemoveComponent {
        /// The entity.
        entity: EntityId,
        /// The component's name.
        component: String,
    },
    /// Put a field back to its assembly's value.
    RevertField {
        /// The entity.
        entity: EntityId,
        /// The component's name.
        component: String,
        /// The field's name.
        field: String,
    },
    /// Make a field's value the assembly's, for every copy.
    ApplyField {
        /// The entity.
        entity: EntityId,
        /// The component's name.
        component: String,
        /// The field's name.
        field: String,
    },
    /// Turn a placed assembly into plain entities.
    Unpack(EntityId),
    /// Delete an entity and everything below it.
    Delete(EntityId),
    /// Move an entity to where the camera is looking.
    MoveHere(EntityId),
}

/// What the selected entity's assembly says about it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssemblyPart {
    /// The assembly's name.
    pub name: String,
    /// The entity as the assembly has it, or `None` if its part is gone
    /// from the assembly.
    pub expected: Option<area::EntityData>,
    /// Whether it's the placed copy's root.
    pub root: bool,
}

/// The Properties panel. `part` is set when the selected entity comes from
/// a placed assembly.
pub(crate) fn properties(
    ui: &mut Ui,
    document: &Document,
    part: Option<&AssemblyPart>,
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
                    if let Some(part) = part {
                        assembly_note(ui, id, part, &mut requests);
                    }
                    ui.add_space(4.0);
                    // Transform first, as in every engine; the rest by name.
                    let mut names: Vec<&String> = entity.components.keys().collect();
                    names.sort_by_key(|n| (n.as_str() != "Transform", n.as_str()));
                    for component in names {
                        let data = &entity.components[component];
                        ui.separator();
                        match document.registry.get(component) {
                            Some(schema) => {
                                if component_header(ui, component, &schema.help) {
                                    requests.push(PropertyRequest::RemoveComponent {
                                        entity: id,
                                        component: component.clone(),
                                    });
                                }
                                let assembly = part
                                    .and_then(|p| p.expected.as_ref())
                                    .and_then(|e| e.components.get(component));
                                for change in inspector::fields(ui, schema, data, assembly, models)
                                {
                                    let (entity, component) = (id, component.clone());
                                    requests.push(match change {
                                        FieldChange::Set { field, value } => {
                                            PropertyRequest::SetField {
                                                entity,
                                                component,
                                                field,
                                                value,
                                            }
                                        }
                                        FieldChange::Revert(field) => {
                                            PropertyRequest::RevertField {
                                                entity,
                                                component,
                                                field,
                                            }
                                        }
                                        FieldChange::Apply(field) => PropertyRequest::ApplyField {
                                            entity,
                                            component,
                                            field,
                                        },
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
                    if let Some(component) = add_component_menu(ui, document, entity) {
                        requests.push(PropertyRequest::AddComponent {
                            entity: id,
                            component,
                        });
                    }
                    ui.add_space(4.0);
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
                        let whole_model = entity.origin.is_none()
                            && entity
                                .get::<area::builtin::Model>()
                                .and_then(|m| m.asset)
                                .filter(|r| r.part.is_none())
                                .and_then(|r| models.get(&r.asset))
                                .is_some_and(|info| !info.nodes.is_empty());
                        if whole_model
                            && ui
                                .button("Unpack")
                                .on_hover_text(
                                    "Splits the model into one entity per part, so each can be \
                                     moved or changed on its own",
                                )
                                .clicked()
                        {
                            requests.push(PropertyRequest::Unpack(id));
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

/// A component's name, with a × on the right to remove it. Returns
/// whether × was clicked.
fn component_header(ui: &mut Ui, name: &str, help: &str) -> bool {
    ui.horizontal(|ui| {
        ui.label(RichText::new(name).strong()).on_hover_text(help);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button = ui
                .add(egui::Button::new(RichText::new("×").color(theme::TEXT_WEAK)).frame(false))
                .on_hover_text(format!("Remove {name}"));
            button.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Remove {name}"))
            });
            button.clicked()
        })
        .inner
    })
    .inner
}

/// The Add component menu: every registered component the entity doesn't
/// have yet, by name. Returns the one picked.
fn add_component_menu(
    ui: &mut Ui,
    document: &Document,
    entity: &area::EntityData,
) -> Option<String> {
    let mut picked = None;
    ui.menu_button("Add component", |ui| {
        let mut any = false;
        for schema in document.registry.iter() {
            if entity.has(&schema.name) {
                continue;
            }
            any = true;
            if ui
                .button(&schema.name)
                .on_hover_text(&schema.help)
                .clicked()
            {
                picked = Some(schema.name.clone());
                ui.close();
            }
        }
        if !any {
            ui.label(RichText::new("It has every component there is.").color(theme::TEXT_WEAK));
        }
    })
    .response
    .on_hover_text("Gives it another component, such as a Light");
    picked
}

/// Which assembly the entity comes from, with Unpack on a copy's root, or
/// a warning when its part is gone from the assembly.
fn assembly_note(
    ui: &mut Ui,
    id: EntityId,
    part: &AssemblyPart,
    requests: &mut Vec<PropertyRequest>,
) {
    ui.horizontal(|ui| {
        if part.expected.is_none() && !part.root {
            ui.label(
                RichText::new(format!("This part is gone from {}.", part.name)).color(theme::WARNING),
            )
            .on_hover_text(
                "Its changes are kept, in case the part comes back. Remove it if you don't need \
                 them.",
            );
            if ui.button("Remove").clicked() {
                requests.push(PropertyRequest::Delete(id));
            }
            return;
        }
        let text = if part.root {
            format!("A copy of {}", part.name)
        } else {
            format!("Part of {}", part.name)
        };
        ui.label(RichText::new(text).color(theme::TEXT_WEAK))
            .on_hover_text("Bold settings differ from the assembly: ↺ puts one back, ↑ makes it the assembly's");
        if part.root
            && ui
                .button("Unpack")
                .on_hover_text("Turns this copy into plain entities, no longer following the assembly")
                .clicked()
        {
            requests.push(PropertyRequest::Unpack(id));
        }
    });
}
