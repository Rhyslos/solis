//! Tags as chips (stage 1, tags; `docs/design/tags/02-entity-tags.png`):
//! the row under an entity's name in Properties, and a trigger zone's
//! Only tags. Each tag is a chip with an × that takes it off, then
//! **+ Tag ▾**, a list of the project's tags to pick from with a box to
//! search it, and **New tag…**, which makes one and gives it. A prefab's
//! copy whose tags differ from its prefab's has the changed dot before
//! the row's name, and ↶ after the chips puts the prefab's back (R5).

use super::TagAsk;
use crate::tokens::{self, size, text};
use crate::widgets;
use egui::{RichText, Ui};
use level::tags::TagList;
use reflect::TagId;

/// The widest the + Tag list is.
const LIST_W: f32 = 180.0;

/// What a row of chips shows.
pub(crate) struct TagRow<'a> {
    /// The row's name ("Tags", "Only tags").
    pub(crate) label: &'a str,
    /// What hovering the name says.
    pub(crate) help: &'a str,
    /// The project's tags.
    pub(crate) tags: &'a TagList,
    /// The tags shown, in order: those every picked entity carries.
    pub(crate) shown: &'a [TagId],
    /// A prefab's copy whose tags differ from its prefab's.
    pub(crate) changed: bool,
}

/// Draws the row; what was asked for.
pub(crate) fn chips(ui: &mut Ui, row: &TagRow<'_>) -> Vec<TagAsk> {
    let mut asks = Vec::new();
    crate::field_rows::row_marked(ui, (row.label, row.help), (row.changed, false, false), |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(size::ROW_GAP, size::ROW_GAP);
            for &id in row.shown {
                let Some(name) = row.tags.name_of(id) else {
                    continue;
                };
                if widgets::chip::tag_chip(ui, name, ui.is_enabled())
                    .on_hover_text(format!("Takes {name} off"))
                    .clicked()
                {
                    asks.push(TagAsk::Remove(id));
                }
            }
            if let Some(ask) = add_menu(ui, row) {
                asks.push(ask);
            }
            if row.changed
                && ui
                    .add(
                        egui::Button::new(
                            RichText::new("\u{21b6}")
                                .font(text::mono_small())
                                .color(tokens::text_meta()),
                        )
                        .frame(false),
                    )
                    .on_hover_text("Back to the prefab's tags: this copy follows the prefab here again")
                    .clicked()
            {
                asks.push(TagAsk::Revert);
            }
        });
    });
    asks
}

/// **+ Tag ▾** and its list: the project's tags not shown yet, narrowed
/// by the box at the top as you type, and New tag…. Enter takes the
/// first one listed, or makes the tag typed.
fn add_menu(ui: &mut Ui, row: &TagRow<'_>) -> Option<TagAsk> {
    let button = widgets::button::dashed_as(ui, "+ Tag \u{25be}", "Add tag")
        .on_hover_text("Gives it one of the project's tags, or a new one: type to search");
    let search_id = button.id.with("tag search");
    let opened = button.clicked();
    let mut picked = None;
    egui::Popup::menu(&button)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_width(LIST_W);
            let mut typed: String = ui.data(|d| d.get_temp(search_id)).unwrap_or_default();
            if opened {
                typed.clear();
            }
            let field = ui.add(
                egui::TextEdit::singleline(&mut typed)
                    .hint_text("Search tags")
                    .desired_width(LIST_W),
            );
            field.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Search tags")
            });
            if opened {
                field.request_focus();
            }
            ui.data_mut(|d| d.insert_temp(search_id, typed.clone()));
            let wanted = typed.trim().to_lowercase();
            let offered: Vec<(TagId, &str)> = row
                .tags
                .iter()
                .filter(|t| !row.shown.contains(&t.id))
                .filter(|t| t.name.to_lowercase().contains(&wanted))
                .map(|t| (t.id, t.name.as_str()))
                .collect();
            for (id, name) in &offered {
                if ui
                    .add(egui::Button::new(*name).frame(false).min_size(egui::vec2(LIST_W, 0.0)))
                    .clicked()
                {
                    picked = Some(TagAsk::Add(*id));
                }
            }
            if offered.is_empty() {
                let words = if row.tags.is_empty() {
                    "The project has no tags yet."
                } else if wanted.is_empty() {
                    "It has every tag there is."
                } else {
                    "No tag it lacks is called that."
                };
                ui.label(RichText::new(words).color(tokens::text_meta()));
            }
            ui.separator();
            let name = typed.trim().to_owned();
            let exists = row.tags.id_of(&name).is_some();
            let problem = (!name.is_empty() && !exists)
                .then(|| row.tags.problem(&name, None))
                .flatten();
            let new_words = if name.is_empty() || exists {
                "New tag\u{2026}".to_owned()
            } else {
                format!("New tag \u{201c}{name}\u{201d}")
            };
            let new = ui
                .add(
                    egui::Button::new(RichText::new(new_words).color(tokens::text_label()))
                        .frame(false),
                )
                .on_hover_text("Makes a tag of the name typed above, in the project, and gives it");
            if new.clicked() {
                if name.is_empty() {
                    field.request_focus();
                } else if problem.is_none() && !exists {
                    picked = Some(TagAsk::New(name.clone()));
                }
            }
            if let Some(why) = &problem {
                ui.label(RichText::new(why).color(tokens::WARNING).size(text::SMALL_SIZE));
            } else if new.hovered() && name.is_empty() {
                ui.label(
                    RichText::new("Type its name in the box above.")
                        .color(tokens::text_meta())
                        .size(text::SMALL_SIZE),
                );
            }
            // Enter takes the first one listed, or makes the tag typed.
            if picked.is_none()
                && field.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
            {
                picked = match offered.first() {
                    Some((id, _)) if !wanted.is_empty() => Some(TagAsk::Add(*id)),
                    _ if !name.is_empty() && problem.is_none() && !exists => {
                        Some(TagAsk::New(name.clone()))
                    }
                    _ => None,
                };
            }
            if picked.is_some() {
                ui.close();
            }
        });
    picked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_row_draws_its_chips_and_the_add_button() {
        let mut tags = TagList::default();
        let enemy = tags.add("Enemy").unwrap();
        let flying = tags.add("Flying").unwrap();
        let shown = [enemy, flying, TagId::random()];
        let ctx = egui::Context::default();
        let mut asks = Vec::new();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                asks = chips(
                    ui,
                    &TagRow {
                        label: "Tags",
                        help: "",
                        tags: &tags,
                        shown: &shown,
                        changed: true,
                    },
                );
            });
        });
        assert!(asks.is_empty());
    }
}
