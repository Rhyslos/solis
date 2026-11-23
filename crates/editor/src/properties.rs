//! The Properties panel: the selected entity's name and components, with
//! fields drawn from each component's schema (decision 0019). It never
//! changes the level itself: it returns what the user asked for, and the
//! workbench applies it (as undo steps, decision 0022).

use crate::document::Document;
use crate::inspector::{self, FieldChange};
use crate::panels::empty_note;
use crate::properties_cards as cards;
use crate::properties_header as head;
use crate::tokens::size;
use egui::Ui;
use reflect::{EntityId, Value};
use std::collections::BTreeMap;

/// Something the user asked for in Properties.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PropertyRequest {
    /// Play the sound event at this path in the project, to hear it (a
    /// sound field's ▶, decision 0127). Changes nothing.
    PlaySound(String),
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
    /// Open the user guide at a manual page's chapter (`lights`).
    OpenGuide(&'static str),
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
    /// Put a field back to its module's value.
    RevertField {
        /// The entity.
        entity: EntityId,
        /// The component's name.
        component: String,
        /// The field's name.
        field: String,
    },
    /// Make a field's value the module's, for every copy.
    ApplyField {
        /// The entity.
        entity: EntityId,
        /// The component's name.
        component: String,
        /// The field's name.
        field: String,
    },
    /// Turn a placed module into plain entities (Unlink).
    Unpack(EntityId),
    /// Open this module on its own page.
    OpenModule(reflect::AssetId),
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
    /// Draw a trigger zone's freeform outline in the view (its card's
    /// Shape › Freeform; stage 1, triggers).
    ZoneDraw(EntityId),
    /// Edit a freeform trigger zone's points in the view (its card's
    /// Edit points).
    ZoneEditPoints(EntityId),
    /// Make a code file for a trigger zone, add its component to the
    /// zone, and open it (New code for this zone…).
    NewZoneCode(EntityId),
    /// Change a trigger zone's Only tags (stage 1, tags).
    ZoneOnly {
        /// The zone.
        entity: EntityId,
        /// What to do.
        ask: crate::tags::TagAsk,
    },
    /// Stretch a brush about its middle to a size, in metres (the Brush
    /// card's Size; Tools 1a, B3).
    BrushSize {
        /// The entity.
        entity: EntityId,
        /// Its new size.
        size: glam::Vec3,
    },
    /// Change the tags of entities (the chips under the name; stage 1),
    /// as one undo step.
    Tags {
        /// The entities.
        entities: Vec<EntityId>,
        /// What to do.
        ask: crate::tags::TagAsk,
    },
    /// Put a component's fields back to their starting values.
    ResetComponent {
        /// The entities.
        entities: Vec<EntityId>,
        /// The component's name.
        component: String,
    },
}

/// What the selected entity's module says about it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModulePart {
    /// The module.
    pub asset: reflect::AssetId,
    /// The module's name.
    pub name: String,
    /// The entity as the module has it, or `None` if its part is gone
    /// from the module.
    pub expected: Option<level::EntityData>,
    /// Whether it's the placed copy's root.
    pub root: bool,
}

/// Where Properties sends people to read more about a component: the
/// user guide's chapter for the engine's own (`manual_page`), and where
/// each game code component is written (its file inside the assets, and
/// line), by name.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Help<'a> {
    /// Each game code component's file and line.
    pub places: Option<&'a BTreeMap<String, (String, u32)>>,
    /// What the Brush card shows that a brush's values don't say outright
    /// (its counts and size, what's picked while it is edited; Tools 1a,
    /// B3), for the entity Properties shows.
    pub brush: Option<&'a crate::brushing::card::BrushFacts>,
    /// What the Trigger zone card shows that a zone's values don't say
    /// (which shapes the project offers, who is inside while playing;
    /// stage 1), for the entity Properties shows.
    pub zone: Option<&'a crate::zones::card::ZoneFacts>,
    /// The project's tags (stage 1), for the chips under the name; none
    /// where tags aren't shown.
    pub tags: Option<&'a level::tags::TagList>,
    /// Whether the entity shown is a module's copy whose tags differ from
    /// its module's.
    pub tags_changed: bool,
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
        "TriggerZone" => "trigger-zones.md",
        "Tags" => "tags.md",
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
/// a placed module; `copied` names the components whose values were
/// copied (for Paste). With several entities selected, it shows the
/// components they all have, and a change applies to all of them.
pub(crate) fn properties(
    ui: &mut Ui,
    document: &Document,
    part: Option<&ModulePart>,
    (models, sheets, library): crate::inspector::Known<'_>,
    copied: &[String],
    playing: Option<WhilePlaying<'_>>,
    (help, hints): (Help<'_>, Option<&[String]>),
) -> Vec<PropertyRequest> {
    let mut requests = Vec::new();
    let Some((id, entity)) = document
        .selection
        .primary()
        .and_then(|id| document.level().get(id).map(|e| (id, e)))
    else {
        // The slot's own header says Properties (decision 0118): no strip
        // of its own over the note.
        empty_note(
            ui,
            "Nothing selected. Pick something in the viewport or the Level column.",
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
    let mut strip = head::header_strip(ui);
    // The one Properties shows first, then the rest of the selection.
    let mut selected = vec![id];
    selected.extend(
        document
            .selection
            .iter()
            .filter(|&e| e != id && document.level().contains(e)),
    );
    let many = selected.len() > 1;
    let whole_model = !many
        && entity.origin.is_none()
        && entity
            .get::<level::builtin::Model>()
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
                    if let Some(tags) = help.tags {
                        let changed = help.tags_changed && !many;
                        tags_row(ui, document, (tags, changed), &selected, &mut requests);
                    }
                    if !many {
                        if let Some(part) = part {
                            head::module_note(ui, id, part, &mut requests);
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
                        // Tags are the chips under the name, never a card.
                        .filter(|n| {
                            n.as_str() != <level::builtin::Tags as reflect::Component>::NAME
                        })
                        .filter(|n| {
                            !many
                                || (document.registry.get(n).is_some()
                                    && selected.iter().all(|&e| {
                                        document.level().get(e).is_some_and(|d| d.has(n))
                                    }))
                        })
                        .collect();
                    names.sort_by_key(|n| (n.as_str() != "Transform", n.as_str()));
                    let card = Card {
                        document,
                        selected: &selected,
                        part,
                        assets: (models, sheets, library),
                        copied,
                        playing: playing.filter(|_| !many),
                        help,
                    };
                    for name in &names {
                        cards::card_row(ui, name, (1, 1), |ui, _, background| {
                            card.draw(ui, name, background, &mut requests);
                        });
                    }
                    if !many
                        && let Some(facts) = help.zone
                        && entity.has(<level::builtin::TriggerZone as reflect::Component>::NAME)
                    {
                        cards::card_row(ui, "zone hears", (1, 1), |ui, _, _| {
                            if crate::zones::card::hears(ui, !facts.flat) {
                                requests.push(PropertyRequest::NewZoneCode(id));
                            }
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

/// The tags under the name (stage 1): those every selected entity
/// carries, as chips, with + Tag; a change applies to each.
fn tags_row(
    ui: &mut Ui,
    document: &Document,
    (tags, changed): (&level::tags::TagList, bool),
    selected: &[EntityId],
    requests: &mut Vec<PropertyRequest>,
) {
    let level = document.level();
    let mut shared: Vec<reflect::TagId> = level
        .get(selected[0])
        .map(|e| crate::tags::carried(tags, e))
        .unwrap_or_default();
    for &other in &selected[1..] {
        let theirs = level
            .get(other)
            .map(|e| crate::tags::carried(tags, e))
            .unwrap_or_default();
        shared.retain(|t| theirs.contains(t));
    }
    let help = if selected.len() > 1 {
        "The tags every selected entity carries. A change applies to each."
    } else {
        "Labels game code finds it by (Project settings \u{203a} Tags): hasTag, World:within, \
         World:nearest."
    };
    let row = crate::tags::chips::TagRow {
        label: "Tags",
        help,
        tags,
        shown: &shared,
        changed,
    };
    for ask in crate::tags::chips::chips(ui, &row) {
        requests.push(PropertyRequest::Tags {
            entities: selected.to_vec(),
            ask,
        });
    }
}

/// What drawing a component's card needs.
struct Card<'a> {
    document: &'a Document,
    /// The selected entities; the first is the one Properties shows.
    selected: &'a [EntityId],
    part: Option<&'a ModulePart>,
    assets: crate::inspector::Known<'a>,
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
        let level = self.document.level();
        let Some(entity) = level.get(id) else {
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
            help: place.is_none() && manual_page(component).is_some(),
        };
        match cards::title_row(ui, has, &schema.help, background) {
            Some(cards::CardAsk::Help) => {
                if let Some(page) = manual_page(component) {
                    requests.push(PropertyRequest::OpenGuide(page.trim_end_matches(".md")));
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
        if cards::is_folded(ui, component) {
            return;
        }
        if let Some((file, _)) = place
            && cards::code_line(ui, component, file)
        {
            requests.push(PropertyRequest::OpenCode(file.clone()));
        }
        let module = self
            .part
            .filter(|_| !many)
            .and_then(|p| p.expected.as_ref())
            .and_then(|e| e.components.get(component));
        let others: Vec<&reflect::ComponentData> = self.selected[1..]
            .iter()
            .filter_map(|&e| level.get(e))
            .filter_map(|d| d.components.get(component))
            .collect();
        // A component of the kind the project hides is shown, never lost,
        // but not edited (R6): a quiet note says how to edit it.
        let kind = crate::kinds::component(component);
        let editable = crate::kinds::shown(kind);
        if let Some(note) = kind.note().filter(|_| !editable) {
            cards::kind_note(ui, &note);
        }
        if component == <level::builtin::TriggerZone as reflect::Component>::NAME
            && !many
            && let Some(facts) = self.help.zone
        {
            ui.add_enabled_ui(editable, |ui| {
                for ask in crate::zones::card::rows(ui, data, facts) {
                    use crate::zones::card::ZoneAsk;
                    requests.push(match ask {
                        ZoneAsk::Set(field, value) => PropertyRequest::SetField {
                            entity: id,
                            component: component.to_owned(),
                            field: field.to_owned(),
                            value,
                        },
                        ZoneAsk::Draw => PropertyRequest::ZoneDraw(id),
                        ZoneAsk::EditPoints => PropertyRequest::ZoneEditPoints(id),
                        ZoneAsk::Only(ask) => PropertyRequest::ZoneOnly { entity: id, ask },
                    });
                }
            });
            return;
        }
        if component == <level::builtin::Brush as reflect::Component>::NAME
            && !many
            && let Some(facts) = self.help.brush
        {
            ui.add_enabled_ui(editable, |ui| {
                self.brush_card(ui, (schema, data), facts, requests);
            });
            return;
        }
        // Several zones: Only tags is IDs as saved, so it's shown as the
        // tags' names (when every zone has the same), never as the IDs.
        let zone_only = component == <level::builtin::TriggerZone as reflect::Component>::NAME;
        let without_only;
        let schema = if zone_only {
            let mut fewer = reflect::ComponentSchema::clone(schema);
            fewer.fields.retain(|f| f.name != ONLY_TAGS);
            without_only = fewer;
            &without_only
        } else {
            schema
        };
        let changes = ui
            .add_enabled_ui(editable, |ui| {
                inspector::fields(
                    ui,
                    schema,
                    data,
                    (module, &yours),
                    &others,
                    self.assets,
                    (level, &self.document.asset_users()),
                )
            })
            .inner;
        for change in changes.into_iter().filter(|_| editable) {
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
                FieldChange::Play(path) => PropertyRequest::PlaySound(path),
            });
        }
        if zone_only {
            self.shared_only_tags(ui, data, &others);
        }
        if component == "Model" {
            cards::model_rows_to_come(ui);
        }
    }

    /// Several zones' Only tags, as names, when every zone has the same;
    /// nothing when they differ (each zone's own card changes them).
    fn shared_only_tags(
        &self,
        ui: &mut Ui,
        data: &reflect::ComponentData,
        others: &[&reflect::ComponentData],
    ) {
        let (Some(tags), Some(Value::Text(mine))) = (self.help.tags, data.get(ONLY_TAGS)) else {
            return;
        };
        let same = others
            .iter()
            .all(|o| o.get(ONLY_TAGS) == Some(&Value::Text(mine.clone())));
        if !same {
            return;
        }
        let names: Vec<&str> = level::builtin::Tags::read(mine)
            .ids
            .into_iter()
            .filter_map(|id| tags.name_of(id))
            .collect();
        let words = if names.is_empty() {
            "None: everything it notices".to_owned()
        } else {
            names.join(" \u{b7} ")
        };
        inspector::row(
            ui,
            "Only tags",
            "The tags every picked zone notices. Pick one zone to change them.",
            |ui| {
                ui.label(
                    egui::RichText::new(words)
                        .font(crate::tokens::text::row())
                        .color(crate::tokens::text_primary()),
                );
            },
        );
    }
}

/// A trigger zone's Only tags field, as the level keeps it.
const ONLY_TAGS: &str = "only_tags";

impl Card<'_> {
    /// The Brush card's rows (`brushing::card`), and what changing them
    /// asks for.
    fn brush_card(
        &self,
        ui: &mut Ui,
        brush: (&reflect::ComponentSchema, &reflect::ComponentData),
        facts: &crate::brushing::card::BrushFacts,
        requests: &mut Vec<PropertyRequest>,
    ) {
        use crate::brushing::card::CardChange;
        let entity = self.selected[0];
        let component = <level::builtin::Brush as reflect::Component>::NAME.to_owned();
        let changes =
            crate::brushing::card::rows(ui, brush, facts, self.assets, self.document.level());
        for change in changes {
            requests.push(match change {
                CardChange::Size(size) => PropertyRequest::BrushSize { entity, size },
                CardChange::Field(FieldChange::Set { field, value, .. }) => {
                    PropertyRequest::SetField {
                        entity,
                        component: component.clone(),
                        field,
                        value,
                    }
                }
                CardChange::Field(FieldChange::Revert(field)) => PropertyRequest::RevertField {
                    entity,
                    component: component.clone(),
                    field,
                },
                CardChange::Field(FieldChange::Apply(field)) => PropertyRequest::ApplyField {
                    entity,
                    component: component.clone(),
                    field,
                },
                CardChange::Field(FieldChange::Play(path)) => PropertyRequest::PlaySound(path),
            });
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
