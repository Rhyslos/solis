//! The Properties panel: the selected entity's name and components, with
//! fields drawn from each component's schema (decision 0019). It never
//! changes the area itself: it returns what the user asked for, and the
//! workbench applies it (as undo steps, decision 0022).

use crate::document::Document;
use crate::inspector::{self, FieldChange};
use crate::models::ModelInfo;
use crate::panels::empty_note;
use crate::properties_cards as cards;
use crate::properties_header as head;
use crate::tokens::{self, size, text};
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
    /// Turn hints off (the Hints card's "Turn hints off").
    HintsOff,
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

/// While the game plays: game code's values as they are now (for the
/// one entity shown, while Properties is in front), and the fields the
/// user changed since Play, each with its value from before.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WhilePlaying<'a> {
    /// The selected entity's game code values now, by component.
    pub values: Option<&'a BTreeMap<String, reflect::ComponentData>>,
    /// What the user changed since Play.
    pub yours: &'a crate::play::BackToBefore,
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
        "Brush" => "brushes.md",
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
    playing: Option<WhilePlaying<'_>>,
    (help, hints): (Help<'_>, Option<&[String]>),
) -> Vec<PropertyRequest> {
    let mut requests = Vec::new();
    let mut strip = head::header_strip(ui);
    let Some((id, entity)) = document
        .selection
        .primary()
        .and_then(|id| document.area().get(id).map(|e| (id, e)))
    else {
        strip.label(
            RichText::new("Properties")
                .font(text::header())
                .color(tokens::text_emphasis()),
        );
        empty_note(
            ui,
            "Nothing selected. Pick something in the viewport or the Area column.",
        );
        if let Some(hints) = hints {
            egui::Frame::new()
                .inner_margin(egui::Margin::same(size::CARD_GAP as i8))
                .show(ui, |ui| {
                    if cards::hints_card(ui, hints) {
                        requests.push(PropertyRequest::HintsOff);
                    }
                });
        }
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
    let menu = head::EntityMenu {
        selected: &selected,
        copied: !copied.is_empty(),
        move_here: !many && entity.has("Transform"),
        unpack: whole_model,
    };
    requests.extend(head::header(&mut strip, document, (id, entity), &menu));
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin::same(size::CARD_GAP as i8))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = size::CARD_GAP;
                    if !many {
                        if let Some(part) = part {
                            head::assembly_note(ui, id, part, &mut requests);
                        }
                        if let Some(playing) = playing {
                            head::playing_note(ui, playing.yours.len());
                        }
                    }
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
                    let card = Card {
                        document,
                        selected: &selected,
                        part,
                        assets: (models, sheets),
                        copied,
                        playing: playing.filter(|_| !many),
                        help,
                    };
                    let slots = cards::across(ui.available_width(), names.len());
                    for row in names.chunks(slots) {
                        cards::card_row(ui, row[0], (row.len(), slots), |ui, i, background| {
                            card.draw(ui, row[i], background, &mut requests);
                        });
                    }
                    if let Some(hints) = hints
                        && cards::hints_card(ui, hints)
                    {
                        requests.push(PropertyRequest::HintsOff);
                    }
                });
        });
    requests
}

/// What drawing a component's card needs.
struct Card<'a> {
    document: &'a Document,
    /// The selected entities; the first is the one Properties shows.
    selected: &'a [EntityId],
    part: Option<&'a AssemblyPart>,
    assets: (
        &'a BTreeMap<AssetId, ModelInfo>,
        &'a crate::sprite_sheets::SpriteSheets,
    ),
    copied: &'a [String],
    /// While playing (one entity): game code's values as they are now,
    /// and what the user changed since Play.
    playing: Option<WhilePlaying<'a>>,
    help: Help<'a>,
}

impl Card<'_> {
    /// Draws `component`'s card contents: its title row, for game code
    /// the file it's written in, then its fields.
    fn draw(
        &self,
        ui: &mut Ui,
        component: &str,
        background: &egui::Response,
        requests: &mut Vec<PropertyRequest>,
    ) {
        let id = self.selected[0];
        let many = self.selected.len() > 1;
        let area = self.document.area();
        let Some(entity) = area.get(id) else {
            return;
        };
        let Some(schema) = self.document.registry.get(component) else {
            cards::missing_card(ui, id, component, requests);
            return;
        };
        let data = self
            .playing
            .and_then(|p| p.values)
            .and_then(|r| r.get(component))
            .unwrap_or(&entity.components[component]);
        // The fields changed since Play, with their values from before.
        let yours: BTreeMap<&str, Option<&Value>> = self
            .playing
            .into_iter()
            .flat_map(|p| p.yours.iter())
            .filter(|((e, c, _), _)| *e == id && c == component)
            .map(|((_, _, field), was)| (field.as_str(), was.as_ref()))
            .collect();
        let place = self.help.places.and_then(|p| p.get(component));
        let has = cards::MenuHas {
            name: component,
            can_paste: self.copied.iter().any(|c| c == component),
            code: place.is_some(),
            help: place.is_none()
                && !self.help.manual.is_empty()
                && manual_page(component).is_some(),
        };
        match cards::title_row(ui, has, &schema.help, background) {
            Some(cards::CardAsk::Help) => {
                if let Some(page) = manual_page(component) {
                    let address = format!("{}/{page}", self.help.manual);
                    ui.ctx().open_url(egui::OpenUrl::new_tab(address));
                }
            }
            Some(cards::CardAsk::Menu(cards::CardMenu::OpenCode)) => {
                if let Some((file, _)) = place {
                    requests.push(PropertyRequest::OpenCode(file.clone()));
                }
            }
            Some(cards::CardAsk::Menu(row)) => {
                requests.extend(header_request(row, self.selected, component));
            }
            None => {}
        }
        if let Some((file, _)) = place
            && cards::code_line(ui, component, file)
        {
            requests.push(PropertyRequest::OpenCode(file.clone()));
        }
        let assembly = self
            .part
            .filter(|_| !many)
            .and_then(|p| p.expected.as_ref())
            .and_then(|e| e.components.get(component));
        let others: Vec<&reflect::ComponentData> = self.selected[1..]
            .iter()
            .filter_map(|&e| area.get(e))
            .filter_map(|d| d.components.get(component))
            .collect();
        for change in inspector::fields(
            ui,
            schema,
            data,
            (assembly, &yours),
            &others,
            self.assets,
            (area, &self.document.asset_users()),
        ) {
            let component = component.to_owned();
            requests.push(match change {
                FieldChange::Set { field, value, part } if many => PropertyRequest::SetFieldMany {
                    entities: self.selected.to_vec(),
                    component,
                    field,
                    value,
                    part,
                },
                FieldChange::Set { field, value, .. } => PropertyRequest::SetField {
                    entity: id,
                    component,
                    field,
                    value,
                },
                FieldChange::Revert(field) => PropertyRequest::RevertField {
                    entity: id,
                    component,
                    field,
                },
                FieldChange::Apply(field) => PropertyRequest::ApplyField {
                    entity: id,
                    component,
                    field,
                },
            });
        }
        if component == "Model" {
            cards::model_rows_to_come(ui);
        }
    }
}

/// The request for a row of `component`'s card menu, on the `selected`
/// entities (the first is the one Properties shows); none for opening
/// its code, which needs the file.
pub(crate) fn header_request(
    row: cards::CardMenu,
    selected: &[EntityId],
    component: &str,
) -> Option<PropertyRequest> {
    use cards::CardMenu as M;
    let entities = selected.to_vec();
    let component = component.to_string();
    Some(match row {
        M::Remove if entities.len() == 1 => PropertyRequest::RemoveComponent {
            entity: entities[0],
            component,
        },
        M::Remove => PropertyRequest::RemoveComponentMany {
            entities,
            component,
        },
        M::Copy => PropertyRequest::CopyComponents {
            entity: entities[0],
            only: Some(component),
        },
        M::Paste => PropertyRequest::PasteComponents {
            entities,
            only: Some(component),
        },
        M::Reset => PropertyRequest::ResetComponent {
            entities,
            component,
        },
        M::OpenCode => return None,
    })
}

#[cfg(test)]
#[path = "properties_tests.rs"]
mod tests;
