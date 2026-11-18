//! Properties' header and what sits under it: the entity's icon, name
//! and kind, **+ Component** and the entity's ⋯ menu (spec 199-201), the
//! note about a prefab's copy, and the note while playing. Split from
//! `properties.rs` so neither file is long (review finding DEBT-08).

use crate::document::Document;
use crate::properties::{PrefabPart, PropertyRequest};
use crate::tokens::{self, size, text};
use crate::widgets;
use egui::{RichText, Ui};
use reflect::EntityId;

/// While the game plays: an accent-mixed strip under the header saying
/// what a change here does, with how many of the user's changes there are
/// so far (`04-properties-playing.png`).
pub(crate) fn playing_note(ui: &mut Ui, yours: usize) {
    egui::Frame::new()
        .fill(tokens::select_row(crate::interface_look::in_use().accent))
        .corner_radius(egui::CornerRadius::same(size::radius::CONTROL))
        .inner_margin(egui::Margin::symmetric(
            size::FIELD_PAD as i8,
            size::ROW_GAP as i8,
        ))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let count = match yours {
                    1 => "1 change so far".to_owned(),
                    n => format!("{n} changes so far"),
                };
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(count)
                            .font(text::ui())
                            .color(tokens::text_primary()),
                    );
                    ui.add(
                        egui::Label::new(
                            RichText::new(
                                "You're playing. What you change now is listed when you press \
                                 Stop, to Keep or Undo all. What the game changes goes back by \
                                 itself.",
                            )
                            .font(text::ui())
                            .color(tokens::text_primary()),
                        )
                        .wrap(),
                    );
                });
            });
        });
}

/// The 30 px strip at the top of Properties, in a column header's place,
/// with its hairline under it: a child to draw the header in.
pub(crate) fn header_strip(ui: &mut Ui) -> Ui {
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
pub(crate) fn header(
    ui: &mut Ui,
    document: &Document,
    (id, entity): (EntityId, &level::EntityData),
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
fn kind_words(document: &Document, entity: &level::EntityData) -> String {
    // A trigger zone says so, whatever code it carries (the mock-up's
    // "Door zone  Trigger zone").
    let zone = entity.has("TriggerZone").then_some("Trigger zone");
    let kind = zone.unwrap_or_else(|| {
        entity
            .components
            .keys()
            .find(|n| n.as_str() != "Transform")
            .map_or("Entity", |n| n.as_str())
    });
    match entity.parent.and_then(|p| document.level().get(p)) {
        Some(parent) => format!("{kind} \u{b7} {}", parent.name),
        None => kind.to_owned(),
    }
}

/// What the entity's ⋯ menu offers.
pub(crate) struct EntityMenu<'a> {
    /// The selected entities (the first is the one Properties shows).
    pub(crate) selected: &'a [EntityId],
    /// Whether some components' values were copied.
    pub(crate) copied: bool,
    /// Whether it can be moved to where the camera looks.
    pub(crate) move_here: bool,
    /// Whether it's a whole model that can be split into parts.
    pub(crate) unpack: bool,
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
                    let builtin = level::builtin::registry().ok();
                    let is_builtin =
                        |name: &str| builtin.as_ref().is_some_and(|r| r.get(name).is_some());
                    for (heading, own) in [("Built in", true), ("Game code", false)] {
                        let mut headed = false;
                        for schema in document.registry.iter() {
                            // Only components the project's kind keeps.
                            if is_builtin(&schema.name) != own
                                || !schema.name.to_lowercase().contains(&wanted)
                                || !crate::kinds::shown(crate::kinds::component(&schema.name))
                            {
                                continue;
                            }
                            let everyone_has = selected.iter().all(|&e| {
                                document.level().get(e).is_none_or(|d| d.has(&schema.name))
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

/// Which prefab the entity comes from (board 07, panel 4): "Prefab:
/// Chicken" in a box at the top, with **Open** (its own page) and, on a
/// copy's root, **Unlink** (plain entities again, as one undo step); or a
/// warning when its part is gone from the prefab.
pub(crate) fn prefab_note(
    ui: &mut Ui,
    id: EntityId,
    part: &PrefabPart,
    requests: &mut Vec<PropertyRequest>,
) {
    if part.expected.is_none() && !part.root {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("This part is gone from {}.", part.name))
                    .color(tokens::WARNING),
            )
            .on_hover_text(
                "Its changes are kept, in case the part comes back. Remove it if you don't need \
                 them.",
            );
            if ui.button("Remove").clicked() {
                requests.push(PropertyRequest::Delete(id));
            }
        });
        return;
    }
    let frame = egui::Frame::new()
        .fill(crate::theme::kind::PREFAB.gamma_multiply(0.12))
        .stroke(egui::Stroke::new(
            1.0,
            crate::theme::kind::PREFAB.gamma_multiply(0.45),
        ))
        .corner_radius(crate::tokens::size::radius::CONTROL)
        .inner_margin(egui::Margin::symmetric(8, 5));
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let (spot, _) = ui.allocate_exact_size(
                egui::vec2(
                    crate::tokens::size::ICON_SMALL,
                    crate::tokens::size::ICON_SMALL,
                ),
                egui::Sense::hover(),
            );
            crate::icons::icon(
                ui.painter(),
                spot,
                crate::icons::Icon::Link,
                crate::theme::kind::PREFAB,
            );
            let lead = if part.root { "Prefab:" } else { "Part of" };
            ui.label(RichText::new(lead).color(tokens::text_body()));
            ui.label(
                RichText::new(&part.name)
                    .strong()
                    .color(tokens::text_emphasis()),
            )
            .on_hover_text(
                "White values come from the prefab. A dot marks one this copy changes; \u{21b6} \
                     puts it back.",
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if part.root
                    && crate::widgets::button::link(ui, "Unlink")
                        .on_hover_text(
                            "Makes this copy plain entities that no longer follow the prefab",
                        )
                        .clicked()
                {
                    requests.push(PropertyRequest::Unpack(id));
                }
                if part.root {
                    ui.label(RichText::new("\u{b7}").color(tokens::text_meta()));
                }
                if crate::widgets::button::link(ui, "Open")
                    .on_hover_text("Opens the prefab on its own page, to change every copy")
                    .clicked()
                {
                    requests.push(PropertyRequest::OpenPrefab(part.asset));
                }
            });
        });
    });
}
