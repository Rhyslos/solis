//! The Properties panel: the selected entity's name and components, with
//! fields drawn from each component's schema (decision 0019). It never
//! changes the area itself: it returns what the user asked for, and the
//! workbench applies it (as undo steps, decision 0022).

use crate::document::Document;
use crate::inspector::{self, FieldChange};
use crate::models::ModelInfo;
use crate::panels::empty_note;
use crate::properties_cards as cards;
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, Entry, UnbuiltId};
use crate::widgets;
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
    (help, hints): (Help<'_>, Option<&[String]>),
) -> Vec<PropertyRequest> {
    let mut requests = Vec::new();
    let mut strip = header_strip(ui);
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
    let menu = EntityMenu {
        selected: &selected,
        copied: !copied.is_empty(),
        move_here: !many && entity.has("Transform"),
        unpack: whole_model,
    };
    requests.extend(header(&mut strip, document, (id, entity), &menu));
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin::same(size::CARD_GAP as i8))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = size::CARD_GAP;
                    if !many {
                        if let Some(part) = part {
                            assembly_note(ui, id, part, &mut requests);
                        }
                        if running.is_some_and(|r| !r.is_empty()) {
                            playing_note(ui);
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
                        running: running.filter(|_| !many),
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
    /// Game code's values as they are now, while playing (one entity).
    running: Option<&'a BTreeMap<String, reflect::ComponentData>>,
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
            missing_card(ui, id, component, requests);
            return;
        };
        let data = self
            .running
            .and_then(|r| r.get(component))
            .unwrap_or(&entity.components[component]);
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
            && code_line(ui, component, file)
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
        for change in inspector::fields(ui, schema, data, assembly, &others, self.assets, area) {
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
            model_rows_to_come(ui);
        }
    }
}

/// The Model card's rows the design has and Solis doesn't yet: a model
/// has one setting today, the model itself (found while mocking up,
/// round 2).
fn model_rows_to_come(ui: &mut Ui) {
    let rows: Vec<Entry<()>> = vec![
        Entry::Unbuilt(UnbuiltId::HomeObjectModelMaterial),
        Entry::Unbuilt(UnbuiltId::HomeObjectModelShadows),
    ];
    for entry in unbuilt::filter(rows) {
        if let Entry::Unbuilt(part) = entry {
            unbuilt::field_row(ui, part);
        }
    }
}

/// A card for a component whose game code isn't there: what it is, and
/// Remove.
fn missing_card(ui: &mut Ui, id: EntityId, component: &str, requests: &mut Vec<PropertyRequest>) {
    ui.label(
        RichText::new(format!("Missing component: {component}"))
            .font(text::card_name())
            .color(tokens::WARNING),
    )
    .on_hover_text(
        "Its script or plugin isn't there. Its values are kept as they are until you remove it.",
    );
    if widgets::button::Button::plain("Remove").show(ui).clicked() {
        requests.push(PropertyRequest::RemoveComponent {
            entity: id,
            component: component.to_owned(),
        });
    }
}

/// A game code card's second line: the file it's written in, quiet and
/// in mono, and **Open code** at the right (only Lust opens in its
/// program; other code is shown in its folder, and the button says so).
/// Returns whether the button was clicked.
fn code_line(ui: &mut Ui, component: &str, file: &str) -> bool {
    let shown = file.rsplit_once('/').map_or(file, |(_, n)| n);
    let opens = crate::library::LibraryKind::Code.opens_in_its_program(file);
    let (words, hint) = if opens {
        (
            "Open code",
            format!("Opens {file}, where {component} is written, in your code editor"),
        )
    } else {
        (
            "Show in folder",
            format!("Shows {file}, where {component} is written, in its folder"),
        )
    };
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(shown)
                .font(text::mono_small())
                .color(tokens::text_meta()),
        )
        .on_hover_text(file);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button = widgets::button::Button::plain(words)
                .icon(crate::icons::Icon::External)
                .show(ui);
            button.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    true,
                    format!("Open {component}'s code"),
                )
            });
            button.on_hover_text(hint).clicked()
        })
        .inner
    })
    .inner
}

/// While the game plays: an accent-mixed strip under the header saying
/// what a change here does (`04-properties-playing.png`).
fn playing_note(ui: &mut Ui) {
    egui::Frame::new()
        .fill(tokens::select_row(crate::interface_look::in_use().accent))
        .corner_radius(egui::CornerRadius::same(size::radius::CONTROL))
        .inner_margin(egui::Margin::symmetric(
            size::FIELD_PAD as i8,
            size::ROW_GAP as i8,
        ))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(
                egui::Label::new(
                    RichText::new(
                        "Playing: game code's values are shown as they are now. A change here \
                         reaches the game, and Stop asks whether to keep it; what the game \
                         changed goes back.",
                    )
                    .font(text::ui())
                    .color(tokens::text_primary()),
                )
                .wrap(),
            );
        });
}

/// The 30 px strip at the top of Properties, in a column header's place,
/// with its hairline under it: a child to draw the header in.
fn header_strip(ui: &mut Ui) -> Ui {
    let across = ui.available_rect_before_wrap().x_range();
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(across.span(), size::COLUMN_HEADER_H),
        egui::Sense::hover(),
    );
    tokens::hline(
        ui.painter(),
        across,
        rect.bottom() - size::LINE,
        tokens::line(),
    );
    ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(egui::vec2(size::ROW_PAD, 0.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    )
}

/// Properties' header: the kind's icon, the name (typed over to rename),
/// what it is and what it's under, then at the right **+ Component** and
/// the entity's ⋯. With several selected, how many instead of the name.
fn header(
    ui: &mut Ui,
    document: &Document,
    (id, entity): (EntityId, &area::EntityData),
    menu: &EntityMenu<'_>,
) -> Vec<PropertyRequest> {
    let mut requests = Vec::new();
    let many = menu.selected.len() > 1;
    {
        ui.spacing_mut().item_spacing.x = size::GAP;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            entity_menu(ui, menu, &mut requests);
            if let Some(component) = add_component_menu(ui, document, menu.selected) {
                requests.push(if many {
                    PropertyRequest::AddComponentMany {
                        entities: menu.selected.to_vec(),
                        component,
                    }
                } else {
                    PropertyRequest::AddComponent {
                        entity: id,
                        component,
                    }
                });
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let (spot, _) = ui.allocate_exact_size(
                    egui::vec2(size::ICON_SMALL, size::ICON_SMALL),
                    egui::Sense::hover(),
                );
                let accent = crate::interface_look::accent_fill();
                crate::icons::icon(ui.painter(), spot, crate::icons::Icon::CubeOutline, accent);
                if many {
                    ui.label(
                        RichText::new(format!(
                            "{} entities selected: what they all have is shown, and a change \
                             applies to each.",
                            menu.selected.len()
                        ))
                        .font(text::ui())
                        .color(tokens::text_label()),
                    );
                    return;
                }
                let mut name = entity.name.clone();
                let width = name_width(ui, &name);
                let field = ui.add(
                    egui::TextEdit::singleline(&mut name)
                        .frame(egui::Frame::NONE)
                        .font(text::card_name())
                        .text_color(tokens::text_emphasis())
                        .clip_text(false)
                        .desired_width(width),
                );
                field.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Name")
                });
                if field.on_hover_text("Type to rename it").changed() {
                    requests.push(PropertyRequest::Rename { entity: id, name });
                }
                let what = kind_words(document, entity);
                ui.add(
                    egui::Label::new(
                        RichText::new(what)
                            .font(text::ui())
                            .color(tokens::text_meta()),
                    )
                    .truncate()
                    .selectable(false),
                );
            });
        });
    }
    requests
}

/// How wide the name's field is: the name's own width, at least a
/// little, so the kind follows it as in the design.
fn name_width(ui: &Ui, name: &str) -> f32 {
    widgets::width_of(ui, name, text::card_name()) + size::FIELD_PAD
}

/// "Model · Props": what the entity is (its main component's name) and
/// the entity it's under.
fn kind_words(document: &Document, entity: &area::EntityData) -> String {
    let kind = entity
        .components
        .keys()
        .find(|n| n.as_str() != "Transform")
        .map_or("Entity", |n| n.as_str());
    match entity.parent.and_then(|p| document.area().get(p)) {
        Some(parent) => format!("{kind} \u{b7} {}", parent.name),
        None => kind.to_owned(),
    }
}

/// The request for a row of `component`'s card menu, on the `selected`
/// entities (the first is the one Properties shows); none for opening
/// its code, which needs the file.
fn header_request(
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
    let button = widgets::button::icon(ui, crate::icons::Icon::More, "Entity menu")
        .on_hover_text("Move here, copy and paste components, delete");
    egui::Popup::menu(&button).show(|ui| {
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
}

/// The Add component button, filling the width: it opens a list of every
/// registered component one of `selected` doesn't have yet, with a search
/// box at the top to narrow it as you type. Returns the one picked.
fn add_component_menu(ui: &mut Ui, document: &Document, selected: &[EntityId]) -> Option<String> {
    let mut picked = None;
    let button = widgets::button::dashed_as(ui, "+ Component", "Add component")
        .on_hover_text("Gives it another component, such as a Light: type to search");
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
                                    RichText::new(heading).color(tokens::text_meta()).size(11.0),
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
                        ui.label(RichText::new(words).color(tokens::text_meta()));
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
                RichText::new(format!("This part is gone from {}.", part.name)).color(tokens::WARNING),
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
        ui.label(RichText::new(text).color(tokens::text_meta()))
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
