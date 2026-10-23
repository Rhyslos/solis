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
    /// Set a field on several entities at once, as one undo step: the
    /// whole value, or one of its three numbers (`part`) on each.
    SetFieldMany {
        /// The entities.
        entities: Vec<EntityId>,
        /// The component's name.
        component: String,
        /// The field's name.
        field: String,
        /// The new value, as the entity Properties shows has it.
        value: Value,
        /// Which of three numbers changed, and to what (degrees for a
        /// rotation).
        part: Option<(usize, f64)>,
    },
    /// Give several entities a component, each one that lacks it.
    AddComponentMany {
        /// The entities.
        entities: Vec<EntityId>,
        /// The component's name.
        component: String,
    },
    /// Take a component off several entities.
    RemoveComponentMany {
        /// The entities.
        entities: Vec<EntityId>,
        /// The component's name.
        component: String,
    },
    /// Copy an entity's components' values (or only one's).
    CopyComponents {
        /// The entity.
        entity: EntityId,
        /// Only this component, or all of them.
        only: Option<String>,
    },
    /// Paste the copied values onto entities (or only one component's),
    /// adding components they lack.
    PasteComponents {
        /// The entities.
        entities: Vec<EntityId>,
        /// Only this component, or every one copied.
        only: Option<String>,
    },
    /// Put a component's fields back to their starting values.
    ResetComponent {
        /// The entities.
        entities: Vec<EntityId>,
        /// The component's name.
        component: String,
    },
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
/// a placed assembly; `copied` names the components whose values were
/// copied (for Paste). With several entities selected, it shows the
/// components they all have, and a change applies to all of them.
pub(crate) fn properties(
    ui: &mut Ui,
    document: &Document,
    part: Option<&AssemblyPart>,
    models: &BTreeMap<AssetId, ModelInfo>,
    copied: &[String],
) -> Vec<PropertyRequest> {
    let mut requests = Vec::new();
    let Some((id, entity)) = document
        .selection
        .primary()
        .and_then(|id| document.area().get(id).map(|e| (id, e)))
    else {
        menu_row(ui, None, copied, &mut requests);
        placeholder(
            ui,
            "Nothing selected. Pick something in the viewport or the Area panel.",
        );
        return requests;
    };
    // The one Properties shows first, then the rest of the selection.
    let mut selected = vec![id];
    selected.extend(
        document
            .selection
            .iter()
            .filter(|&e| e != id && document.area().contains(e)),
    );
    let many = selected.len() > 1;
    menu_row(ui, Some(&selected), copied, &mut requests);
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(theme::panel_margin())
                .show(ui, |ui| {
                    if many {
                        ui.label(
                            RichText::new(format!(
                                "{} entities selected: what they all have is shown, and a                                  change applies to each.",
                                selected.len()
                            ))
                            .color(theme::TEXT_WEAK),
                        );
                    } else {
                        let mut name = entity.name.clone();
                        inspector::row(ui, "Name", "What the Area panel calls it.", |ui| {
                            if ui.text_edit_singleline(&mut name).changed() {
                                requests.push(PropertyRequest::Rename { entity: id, name });
                            }
                        });
                        if let Some(part) = part {
                            assembly_note(ui, id, part, &mut requests);
                        }
                    }
                    ui.add_space(4.0);
                    // Transform first, as in every engine; the rest by name.
                    // With several selected, only what every one has.
                    let mut names: Vec<&String> = entity
                        .components
                        .keys()
                        .filter(|n| {
                            !many
                                || (document.registry.get(n).is_some()
                                    && selected.iter().all(|&e| {
                                        document.area().get(e).is_some_and(|d| d.has(n))
                                    }))
                        })
                        .collect();
                    names.sort_by_key(|n| (n.as_str() != "Transform", n.as_str()));
                    for component in names {
                        let data = &entity.components[component];
                        ui.separator();
                        match document.registry.get(component) {
                            Some(schema) => {
                                let header = component_header(
                                    ui,
                                    component,
                                    &schema.help,
                                    copied.contains(component),
                                );
                                if let Some(action) = header {
                                    requests.push(header_request(
                                        action, &selected, component,
                                    ));
                                }
                                let assembly = part
                                    .filter(|_| !many)
                                    .and_then(|p| p.expected.as_ref())
                                    .and_then(|e| e.components.get(component));
                                let others: Vec<&reflect::ComponentData> = selected[1..]
                                    .iter()
                                    .filter_map(|&e| document.area().get(e))
                                    .filter_map(|d| d.components.get(component))
                                    .collect();
                                for change in
                                    inspector::fields(ui, schema, data, assembly, &others, models)
                                {
                                    let (entity, component) = (id, component.clone());
                                    requests.push(match change {
                                        FieldChange::Set { field, value, part } if many => {
                                            PropertyRequest::SetFieldMany {
                                                entities: selected.clone(),
                                                component,
                                                field,
                                                value,
                                                part,
                                            }
                                        }
                                        FieldChange::Set { field, value, .. } => {
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
                    if let Some(component) = add_component_menu(ui, document, &selected) {
                        requests.push(if many {
                            PropertyRequest::AddComponentMany {
                                entities: selected.clone(),
                                component,
                            }
                        } else {
                            PropertyRequest::AddComponent {
                                entity: id,
                                component,
                            }
                        });
                    }
                    if many {
                        return;
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

/// What a component's header was asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeaderAction {
    Remove,
    Copy,
    Paste,
    Reset,
}

/// The request for a header action on `component` of the `selected`
/// entities (the first is the one Properties shows).
fn header_request(action: HeaderAction, selected: &[EntityId], component: &str) -> PropertyRequest {
    let entities = selected.to_vec();
    let component = component.to_string();
    match action {
        HeaderAction::Remove if entities.len() == 1 => PropertyRequest::RemoveComponent {
            entity: entities[0],
            component,
        },
        HeaderAction::Remove => PropertyRequest::RemoveComponentMany {
            entities,
            component,
        },
        HeaderAction::Copy => PropertyRequest::CopyComponents {
            entity: entities[0],
            only: Some(component),
        },
        HeaderAction::Paste => PropertyRequest::PasteComponents {
            entities,
            only: Some(component),
        },
        HeaderAction::Reset => PropertyRequest::ResetComponent {
            entities,
            component,
        },
    }
}

/// A component's name, with a ⋯ menu (copy, paste, reset) and a × to
/// remove it on the right. Returns what was asked for.
fn component_header(ui: &mut Ui, name: &str, help: &str, can_paste: bool) -> Option<HeaderAction> {
    ui.horizontal(|ui| {
        ui.label(RichText::new(name).strong()).on_hover_text(help);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button = ui
                .add(egui::Button::new(RichText::new("×").color(theme::TEXT_WEAK)).frame(false))
                .on_hover_text(format!("Remove {name}"));
            button.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("Remove {name}"))
            });
            if button.clicked() {
                return Some(HeaderAction::Remove);
            }
            let mut picked = None;
            let menu = ui.menu_button(RichText::new("⋯").color(theme::TEXT_WEAK), |ui| {
                if ui.button("Copy values").clicked() {
                    picked = Some(HeaderAction::Copy);
                    ui.close();
                }
                if ui
                    .add_enabled(can_paste, egui::Button::new("Paste values"))
                    .on_disabled_hover_text(format!("Copy a {name}'s values first"))
                    .clicked()
                {
                    picked = Some(HeaderAction::Paste);
                    ui.close();
                }
                ui.separator();
                if ui
                    .button("Reset to defaults")
                    .on_hover_text("Puts every setting back to its starting value")
                    .clicked()
                {
                    picked = Some(HeaderAction::Reset);
                    ui.close();
                }
            });
            menu.response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("{name} menu"))
            });
            picked
        })
        .inner
    })
    .inner
}

/// The panel's menu row: ≡ and its Edit menu, to copy every component's
/// values of the entity shown and paste them onto the selection.
fn menu_row(
    ui: &mut Ui,
    selected: Option<&[EntityId]>,
    copied: &[String],
    requests: &mut Vec<PropertyRequest>,
) {
    theme::menu_row_frame().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        egui::MenuBar::new().ui(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label("≡");
            ui.menu_button("Edit", |ui| {
                let first = selected.and_then(|s| s.first().copied());
                if ui
                    .add_enabled(first.is_some(), egui::Button::new("Copy components"))
                    .on_hover_text("Copies the values of every component of the entity shown")
                    .clicked()
                    && let Some(entity) = first
                {
                    requests.push(PropertyRequest::CopyComponents { entity, only: None });
                    ui.close();
                }
                let paste = egui::Button::new("Paste components");
                if ui
                    .add_enabled(selected.is_some() && !copied.is_empty(), paste)
                    .on_hover_text(
                        "Gives the selection the copied components' values, adding any it                          lacks",
                    )
                    .clicked()
                    && let Some(entities) = selected
                {
                    requests.push(PropertyRequest::PasteComponents {
                        entities: entities.to_vec(),
                        only: None,
                    });
                    ui.close();
                }
            });
        });
    });
}

/// The Add component menu: every registered component one of `selected`
/// doesn't have yet, by name. Returns the one picked.
fn add_component_menu(ui: &mut Ui, document: &Document, selected: &[EntityId]) -> Option<String> {
    let mut picked = None;
    ui.menu_button("Add component", |ui| {
        let mut any = false;
        for schema in document.registry.iter() {
            let everyone_has = selected
                .iter()
                .all(|&e| document.area().get(e).is_none_or(|d| d.has(&schema.name)));
            if everyone_has {
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
