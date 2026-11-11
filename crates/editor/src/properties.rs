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
    /// Open the game code file a component is written in, in the
    /// computer's program for it (Ledger, for Lust): its path inside the
    /// assets.
    OpenCode(String),
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

/// Where Properties sends people to read more: the manual's address, and
/// where each game code component is written (its file inside the assets,
/// and line), by name.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Help<'a> {
    /// The manual's address (`…/docs/manual`); empty for none.
    pub manual: &'a str,
    /// Each game code component's file and line.
    pub places: Option<&'a BTreeMap<String, (String, u32)>>,
}

/// The manual page about one of the engine's components, or the kit's.
pub(crate) fn manual_page(component: &str) -> Option<&'static str> {
    Some(match component {
        "Transform" => "entities.md",
        "Model" => "importing-models.md",
        "Light" => "lights.md",
        "Environment" => "sky-fog-and-exposure.md",
        "Camera"
        | "ThirdPersonCamera"
        | "FirstPersonCamera"
        | "RtsCamera"
        | "FollowCamera2D"
        | "CharacterController" => "characters-and-cameras.md",
        "RigidBody" | "Collider" | "CharacterBody" => "physics.md",
        "Collider2D" | "TopDownController" => "walls-and-walking.md",
        "AudioSource" => "sound-events.md",
        "Listener" => "listeners.md",
        "ReverbZone" => "reverb-zones.md",
        "Sprite" => "sprites.md",
        "Parallax" => "parallax.md",
        "Animator" => "animators.md",
        "TileMap" => "tile-maps.md",
        "Screen" => "screens.md",
        _ => return None,
    })
}

/// A component's icon and its colour, for its card.
fn component_icon(component: &str, game_code: bool) -> Option<(crate::icons::Icon, egui::Color32)> {
    use crate::icons::Icon;
    use theme::kind;
    Some(match component {
        "Model" => (Icon::Model, kind::MODEL),
        "Light" => (Icon::Light, kind::SUN),
        "Environment" => (Icon::AreaFile, kind::ENVIRONMENT),
        "Camera" => (Icon::Camera, kind::CAMERA),
        "AudioSource" | "Listener" | "ReverbZone" => (Icon::Sound, kind::SOUND),
        "Sprite" | "Parallax" => (Icon::Game, kind::SPRITE),
        "Animator" => (Icon::Animator, kind::SOUND_EVENT),
        "TileMap" => (Icon::Layers, kind::AREA),
        "Screen" => (Icon::Game, kind::PICTURE),
        "Transform" => (Icon::Properties, theme::text()),
        _ if game_code => (Icon::Code, kind::CODE),
        _ => return None,
    })
}

/// The Properties panel. `part` is set when the selected entity comes from
/// a placed assembly; `copied` names the components whose values were
/// copied (for Paste). With several entities selected, it shows the
/// components they all have, and a change applies to all of them.
pub(crate) fn properties(
    ui: &mut Ui,
    document: &Document,
    part: Option<&AssemblyPart>,
    (models, sheets): (
        &BTreeMap<AssetId, ModelInfo>,
        &crate::sprite_sheets::SpriteSheets,
    ),
    copied: &[String],
    running: Option<&BTreeMap<String, reflect::ComponentData>>,
    help: Help<'_>,
) -> Vec<PropertyRequest> {
    let mut requests = Vec::new();
    let Some((id, entity)) = document
        .selection
        .primary()
        .and_then(|id| document.area().get(id).map(|e| (id, e)))
    else {
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
    let whole_model = !many
        && entity.origin.is_none()
        && entity
            .get::<area::builtin::Model>()
            .and_then(|m| m.asset)
            .filter(|r| r.part.is_none())
            .and_then(|r| models.get(&r.asset))
            .is_some_and(|info| !info.nodes.is_empty());
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(theme::panel_margin())
                .show(ui, |ui| {
                    // The name (or how many are selected), with the
                    // entity's ⋯ menu: rare actions live there, so the
                    // panel shows values (decision 0085).
                    ui.horizontal(|ui| {
                        let menu = EntityMenu {
                            selected: &selected,
                            copied: !copied.is_empty(),
                            move_here: !many && entity.has("Transform"),
                            unpack: whole_model,
                        };
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            entity_menu(ui, &menu, &mut requests);
                            if many {
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(format!(
                                                    "{} entities selected: what they all have \
                                                     is shown, and a change applies to each.",
                                                    selected.len()
                                                ))
                                                .color(theme::text_weak()),
                                            )
                                            .wrap(),
                                        );
                                    },
                                );
                            } else {
                                let mut name = entity.name.clone();
                                let field = ui.add(
                                    egui::TextEdit::singleline(&mut name)
                                        .desired_width(ui.available_width())
                                        .font(egui::TextStyle::Body),
                                );
                                field.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::TextEdit,
                                        true,
                                        "Name",
                                    )
                                });
                                if field
                                    .on_hover_text("What the Area panel calls it")
                                    .changed()
                                {
                                    requests.push(PropertyRequest::Rename { entity: id, name });
                                }
                            }
                        });
                    });
                    if !many {
                        if let Some(part) = part {
                            assembly_note(ui, id, part, &mut requests);
                        }
                        if running.is_some_and(|r| !r.is_empty()) {
                            ui.label(
                                RichText::new(
                                    "Playing: game code's values are shown as they are now. A \
                                     change here reaches the game; Stop puts every value back.",
                                )
                                .color(theme::text_weak()),
                            );
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
                                    && selected
                                        .iter()
                                        .all(|&e| document.area().get(e).is_some_and(|d| d.has(n))))
                        })
                        .collect();
                    names.sort_by_key(|n| (n.as_str() != "Transform", n.as_str()));
                    for component in names {
                        // While playing, game code's values as they are now.
                        let data = running
                            .filter(|_| !many)
                            .and_then(|r| r.get(component))
                            .unwrap_or(&entity.components[component]);
                        ui.add_space(6.0);
                        card(ui, |ui| match document.registry.get(component) {
                            Some(schema) => {
                                let place = help.places.and_then(|p| p.get(component));
                                let header = component_header(
                                    ui,
                                    component,
                                    &schema.help,
                                    copied.contains(component),
                                    place.map(|(file, _)| file.as_str()),
                                    help.manual,
                                );
                                match header {
                                    Some(HeaderAction::OpenCode) => {
                                        if let Some((file, _)) = place {
                                            requests.push(PropertyRequest::OpenCode(file.clone()));
                                        }
                                    }
                                    Some(action) => {
                                        requests
                                            .extend(header_request(action, &selected, component));
                                    }
                                    None => {}
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
                                for change in inspector::fields(
                                    ui,
                                    schema,
                                    data,
                                    assembly,
                                    &others,
                                    (models, sheets),
                                    document.area(),
                                ) {
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
                        });
                    }
                    ui.add_space(8.0);
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
    /// Open the file it's written in.
    OpenCode,
}

/// The request for a header action on `component` of the `selected`
/// entities (the first is the one Properties shows); none for opening its
/// file, which needs the file.
fn header_request(
    action: HeaderAction,
    selected: &[EntityId],
    component: &str,
) -> Option<PropertyRequest> {
    let entities = selected.to_vec();
    let component = component.to_string();
    Some(match action {
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
        HeaderAction::OpenCode => return None,
    })
}

/// A component's card: a softly lit box with a border, filling the width.
fn card(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .fill(theme::menu_row())
        .stroke(egui::Stroke::new(1.0, theme::button_border()))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add(ui);
        });
}

/// A small text button with no frame, named `name` for screen readers.
fn quiet_button(ui: &mut Ui, text: RichText, name: &str, hover: &str) -> egui::Response {
    let button = ui
        .add(egui::Button::new(text).frame(false))
        .on_hover_text(hover);
    button.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
    button
}

/// A component's header: its icon and name (and, for game code, the file
/// it's written in), then on the right its ⋯ menu (copy, paste, reset,
/// remove) and a way to read more: **?** opens its manual page, or
/// **Open in Ledger ↗** the game code it's written in (**Show in folder ↗**
/// for code that isn't Lust, which is never opened in another program).
/// Returns what was asked for.
fn component_header(
    ui: &mut Ui,
    name: &str,
    help: &str,
    can_paste: bool,
    file: Option<&str>,
    manual: &str,
) -> Option<HeaderAction> {
    ui.horizontal(|ui| {
        if let Some((icon, colour)) = component_icon(name, file.is_some()) {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
            crate::icons::icon(ui.painter(), rect, icon, colour);
        }
        ui.label(RichText::new(name).strong().color(theme::text_strong()))
            .on_hover_text(help);
        if let Some(file) = file {
            let shown = file.rsplit_once('/').map_or(file, |(_, n)| n);
            ui.label(RichText::new(shown).color(theme::text_weak()).size(12.0))
                .on_hover_text(file);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut picked = None;
            let menu = ui.menu_button(RichText::new("⋯").color(theme::text_weak()), |ui| {
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
                if ui
                    .button("Reset to defaults")
                    .on_hover_text("Puts every setting back to its starting value")
                    .clicked()
                {
                    picked = Some(HeaderAction::Reset);
                    ui.close();
                }
                ui.separator();
                let remove = ui.button(format!("Remove {name}"));
                if remove.clicked() {
                    picked = Some(HeaderAction::Remove);
                    ui.close();
                }
            });
            menu.response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, format!("{name} menu"))
            });
            menu.response
                .on_hover_text("Copy, paste or reset its values, or remove it");
            if let Some(file) = file {
                // Only Lust opens in another program; a Lua or Rust file is
                // shown in its folder (the Library's rule), and the button
                // says which.
                let opens = crate::library::LibraryKind::Code.opens_in_its_program(file);
                let (words, hint) = if opens {
                    (
                        "Open in Ledger ↗",
                        format!("Opens {file}, where {name} is written, in its program"),
                    )
                } else {
                    (
                        "Show in folder ↗",
                        format!("Shows {file}, where {name} is written, in its folder"),
                    )
                };
                let open = quiet_button(
                    ui,
                    RichText::new(words)
                        .color(ui.visuals().hyperlink_color)
                        .size(12.0),
                    &format!("Open {name}'s code"),
                    &hint,
                );
                if open.clicked() {
                    picked = Some(HeaderAction::OpenCode);
                }
            } else if let Some(page) = manual_page(name).filter(|_| !manual.is_empty()) {
                let more = quiet_button(
                    ui,
                    RichText::new("?").color(theme::text_weak()),
                    &format!("About {name}"),
                    &format!("Opens the manual's page about {name}"),
                );
                if more.clicked() {
                    ui.ctx()
                        .open_url(egui::OpenUrl::new_tab(format!("{manual}/{page}")));
                }
            }
            picked
        })
        .inner
    })
    .inner
}

/// What the entity's ⋯ menu offers.
struct EntityMenu<'a> {
    /// The selected entities (the first is the one Properties shows).
    selected: &'a [EntityId],
    /// Whether some components' values were copied.
    copied: bool,
    /// Whether it can be moved to where the camera looks.
    move_here: bool,
    /// Whether it's a whole model that can be split into parts.
    unpack: bool,
}

/// The entity's ⋯ menu, beside its name: Move here, Unpack, copying and
/// pasting every component's values, and Delete.
fn entity_menu(ui: &mut Ui, menu: &EntityMenu<'_>, requests: &mut Vec<PropertyRequest>) {
    let one = (menu.selected.len() == 1).then(|| menu.selected[0]);
    let shown = ui.menu_button(RichText::new("⋯").color(theme::text_weak()), |ui| {
        if let Some(id) = one.filter(|_| menu.move_here)
            && ui
                .button("Move here")
                .on_hover_text(
                    "Moves it to where the camera looks (a spotlight to the camera, shining \
                     the same way)",
                )
                .clicked()
        {
            requests.push(PropertyRequest::MoveHere(id));
            ui.close();
        }
        if let Some(id) = one.filter(|_| menu.unpack)
            && ui
                .button("Unpack")
                .on_hover_text(
                    "Splits the model into one entity per part, so each can be moved or \
                     changed on its own",
                )
                .clicked()
        {
            requests.push(PropertyRequest::Unpack(id));
            ui.close();
        }
        if let Some(entity) = one
            && ui
                .button("Copy components")
                .on_hover_text("Copies the values of every component it has")
                .clicked()
        {
            requests.push(PropertyRequest::CopyComponents { entity, only: None });
            ui.close();
        }
        if ui
            .add_enabled(menu.copied, egui::Button::new("Paste components"))
            .on_hover_text("Gives the selection the copied components' values, adding any it lacks")
            .on_disabled_hover_text("Copy an entity's components first")
            .clicked()
        {
            requests.push(PropertyRequest::PasteComponents {
                entities: menu.selected.to_vec(),
                only: None,
            });
            ui.close();
        }
        if let Some(id) = one {
            ui.separator();
            if ui
                .button("Delete")
                .on_hover_text("Deletes it and everything belonging to it")
                .clicked()
            {
                requests.push(PropertyRequest::Delete(id));
                ui.close();
            }
        }
    });
    shown
        .response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Entity menu"));
    shown
        .response
        .on_hover_text("Move here, copy and paste components, delete");
}

/// The Add component button, filling the width: it opens a list of every
/// registered component one of `selected` doesn't have yet, with a search
/// box at the top to narrow it as you type. Returns the one picked.
fn add_component_menu(ui: &mut Ui, document: &Document, selected: &[EntityId]) -> Option<String> {
    let mut picked = None;
    let button = ui.add(
        egui::Button::new(RichText::new("+  Add component…").color(theme::text()))
            .min_size(egui::vec2(ui.available_width(), 26.0)),
    );
    button
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Add component"));
    let button =
        button.on_hover_text("Gives it another component, such as a Light: type to search");
    let search_id = ui.id().with("add component search");
    let opened = button.clicked();
    egui::Popup::menu(&button)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_width(240.0);
            let mut query: String = ui.data(|d| d.get_temp(search_id)).unwrap_or_default();
            if opened {
                query.clear();
            }
            let field = ui.add(
                egui::TextEdit::singleline(&mut query)
                    .hint_text("Search components")
                    .desired_width(240.0),
            );
            field.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Search components")
            });
            if opened {
                field.request_focus();
            }
            ui.data_mut(|d| d.insert_temp(search_id, query.clone()));
            let wanted = query.trim().to_lowercase();
            let mut first = None;
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .max_width(240.0)
                .show(ui, |ui| {
                    let mut any = false;
                    // In two groups: Solis's own, then the game's code (the
                    // ready-made kit's and yours), each by name.
                    let builtin = area::builtin::registry().ok();
                    let is_builtin =
                        |name: &str| builtin.as_ref().is_some_and(|r| r.get(name).is_some());
                    for (heading, own) in [("Built in", true), ("Game code", false)] {
                        let mut headed = false;
                        for schema in document.registry.iter() {
                            if is_builtin(&schema.name) != own
                                || !schema.name.to_lowercase().contains(&wanted)
                            {
                                continue;
                            }
                            let everyone_has = selected.iter().all(|&e| {
                                document.area().get(e).is_none_or(|d| d.has(&schema.name))
                            });
                            if everyone_has {
                                continue;
                            }
                            if !headed {
                                headed = true;
                                if any {
                                    ui.separator();
                                }
                                ui.label(
                                    RichText::new(heading).color(theme::text_weak()).size(11.0),
                                );
                            }
                            any = true;
                            first.get_or_insert_with(|| schema.name.clone());
                            if ui
                                .button(&schema.name)
                                .on_hover_text(&schema.help)
                                .clicked()
                            {
                                picked = Some(schema.name.clone());
                            }
                        }
                    }
                    if !any {
                        let words = if wanted.is_empty() {
                            "It has every component there is.".to_string()
                        } else {
                            format!("No component it lacks is called “{}”.", query.trim())
                        };
                        ui.label(RichText::new(words).color(theme::text_weak()));
                    }
                });
            // Enter takes the first one shown.
            if picked.is_none()
                && !wanted.is_empty()
                && field.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
            {
                picked = first;
            }
            if picked.is_some() {
                ui.close();
            }
        });
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
        ui.label(RichText::new(text).color(theme::text_weak()))
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

#[cfg(test)]
#[path = "properties_tests.rs"]
mod tests;
