//! Project settings › Tags (stage 1, tags; `docs/design/tags/01-project-settings-tags.png`):
//! the project's tags, each with what uses it ("Used by 14 entities and 2
//! modules") and a ⋯ menu; **+ New tag**, which starts a new row's name;
//! a double-click on a name to rename it everywhere; and Delete, which
//! asks first when anything uses the tag, saying what. A name is checked
//! as it's typed, and a refused one says why. Beside them, a card saying
//! how game code asks for tags.

use super::UsedBy;
use crate::file_pages::{plain, sentence, with_side};
use crate::tokens::{self, size, text};
use crate::widgets::{self, button::Button, dialog::Answer};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Align, Layout, RichText, Ui};
use reflect::TagId;

/// A tag's name being typed: a new tag's (no ID yet) or a rename's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Naming {
    /// The tag renamed, or none for a new one.
    pub(crate) id: Option<TagId>,
    /// What's typed.
    pub(crate) typed: String,
    /// Whether the field should take the keyboard (its first frame).
    pub(crate) focus: bool,
}

/// The Tags page's own state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TagsPage {
    /// The name being typed, if one is.
    pub(crate) naming: Option<Naming>,
    /// The tag whose Delete waits for an answer.
    pub(crate) deleting: Option<TagId>,
}

/// How tall a tag's row is.
const ROW_H: f32 = 52.0;
/// The width of the names' column.
const NAME_W: f32 = 200.0;
/// The width of a name's field while it's typed.
const FIELD_W: f32 = 160.0;

/// What a row asked for.
enum RowAsk {
    Rename(TagId),
    Delete(TagId),
}

impl Workbench {
    /// Draws Project settings › Tags.
    pub(crate) fn tags_page(&mut self, ui: &mut Ui) {
        if self.open.project.is_none() {
            plain(
                ui,
                "Tags belong to a project: open or make one first.",
                text::ui(),
                tokens::text_meta(),
            );
            return;
        }
        // Where the saved levels use each tag is counted on a worker,
        // once each time the page is shown.
        if !self.open.tag_use_asked {
            self.open.tag_use_asked = true;
            self.open.outbox.push(WorkbenchAction::CountTagUse);
        }
        let mut page = std::mem::take(&mut self.file_pages.tags);
        egui::ScrollArea::vertical()
            .id_salt("tags page")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                with_side(
                    ui,
                    &mut (&mut *self, &mut page),
                    |ui, (w, page)| w.tag_rows(ui, page),
                    |ui, _| game_code_card(ui),
                );
            });
        self.delete_question(ui, &mut page);
        self.file_pages.tags = page;
    }

    /// The section's capitals and sentence, a row per tag, the new tag's
    /// row while it's typed, and + New tag.
    fn tag_rows(&mut self, ui: &mut Ui, page: &mut TagsPage) {
        widgets::rows::section_caps(ui, "Tags");
        about_tags(ui);
        ui.add_space(size::GAP);
        let tags: Vec<(TagId, String)> = self
            .open
            .tags
            .iter()
            .map(|t| (t.id, t.name.clone()))
            .collect();
        let mut asked = None;
        for (id, name) in &tags {
            let used = self.used_by(*id);
            asked = asked.or(self.tag_row(ui, page, (*id, name), &used));
        }
        if page.naming.as_ref().is_some_and(|n| n.id.is_none()) {
            self.new_tag_row(ui, page);
        }
        ui.add_space(size::SECTION_GAP);
        let new = Button::plain("+ New tag")
            .named("New tag")
            .show(ui)
            .on_hover_text("Adds a tag to the project: type its name, and Enter keeps it");
        if new.clicked() {
            page.naming = Some(Naming {
                id: None,
                typed: String::new(),
                focus: true,
            });
        }
        match asked {
            Some(RowAsk::Rename(id)) => {
                page.naming = Some(Naming {
                    id: Some(id),
                    typed: self.open.tags.name_of(id).unwrap_or_default().to_owned(),
                    focus: true,
                });
            }
            Some(RowAsk::Delete(id)) => {
                if self.used_by(id).is_empty() {
                    self.delete_tag(id);
                } else {
                    page.deleting = Some(id);
                }
            }
            None => {}
        }
    }

    /// One tag's row: its name (a field while it's renamed), what uses it
    /// (or, while renamed, why the name typed can't be), and its ⋯ menu.
    fn tag_row(
        &mut self,
        ui: &mut Ui,
        page: &mut TagsPage,
        (id, name): (TagId, &str),
        used: &UsedBy,
    ) -> Option<RowAsk> {
        let mut asked = None;
        let renaming = page.naming.as_ref().is_some_and(|n| n.id == Some(id));
        row(ui, |name_ui, middle, more_ui| {
            if renaming {
                let note = self.name_field(name_ui, page);
                note_words(middle, &note);
            } else {
                let shown = plain(name_ui, name, text::setting_name(), tokens::text_emphasis())
                    .interact(egui::Sense::click())
                    .on_hover_text("Double-click to rename it everywhere");
                if shown.double_clicked() {
                    asked = Some(RowAsk::Rename(id));
                }
                note_words(middle, &(used.words(), tokens::text_meta()));
            }
            let more =
                widgets::button::icon(more_ui, crate::icons::Icon::More, &format!("{name}'s menu"))
                    .on_hover_text("Rename, delete");
            egui::Popup::menu(&more).show(|ui| {
                if widgets::menu::item(ui, "Rename", "").clicked() {
                    asked = Some(RowAsk::Rename(id));
                    ui.close();
                }
                if widgets::menu::item(ui, "Delete", "").clicked() {
                    asked = Some(RowAsk::Delete(id));
                    ui.close();
                }
            });
        });
        asked
    }

    /// The row of a tag being made: its name's field and what Enter does.
    fn new_tag_row(&mut self, ui: &mut Ui, page: &mut TagsPage) {
        row(ui, |name_ui, middle, more_ui| {
            let note = self.name_field(name_ui, page);
            note_words(middle, &note);
            more_ui
                .add_enabled_ui(false, |ui| {
                    widgets::button::icon(ui, crate::icons::Icon::More, "New tag's menu")
                })
                .response
                .on_disabled_hover_text("Keep the new tag's name first (Enter)");
        });
    }

    /// The field of the name being typed: Enter keeps it (or says why it
    /// can't), Esc puts it back, and leaving the field keeps a name that
    /// can be kept. Returns what to say beside it, and its colour.
    fn name_field(&mut self, ui: &mut Ui, page: &mut TagsPage) -> (String, egui::Color32) {
        let Some(naming) = page.naming.as_mut() else {
            return (String::new(), tokens::text_meta());
        };
        let field = widgets::field::text_field(ui, "Tag name", &mut naming.typed, "Name", FIELD_W);
        if naming.focus {
            field.request_focus();
            naming.focus = false;
        }
        let typed = naming.typed.trim().to_owned();
        let id = naming.id;
        let problem = self.open.tags.problem(&typed, id);
        let (enter, escape) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        });
        if escape {
            page.naming = None;
            return (String::new(), tokens::text_meta());
        }
        let left = field.lost_focus();
        if (enter || left) && problem.is_none() {
            let kept = match id {
                Some(id) => self.rename_tag(id, &typed),
                None => self.make_tag(&typed).map(drop),
            };
            match kept {
                Ok(()) => page.naming = None,
                Err(why) => return (why, tokens::WARNING),
            }
            return (String::new(), tokens::text_meta());
        }
        if left && !enter {
            // Left with a name that can't be kept: nothing changes.
            page.naming = None;
            return (String::new(), tokens::text_meta());
        }
        if enter {
            // Kept typing: the field keeps the keyboard.
            if let Some(naming) = page.naming.as_mut() {
                naming.focus = true;
            }
        }
        match (problem, id) {
            (Some(why), _) if !typed.is_empty() => (why, tokens::WARNING),
            (_, None) => ("New: press Enter to keep it".into(), tokens::text_meta()),
            (_, Some(_)) => (
                "Enter renames it everywhere; Esc keeps its name".into(),
                tokens::text_meta(),
            ),
        }
    }

    /// Delete's question, while one waits: what uses the tag, and that
    /// it comes off them all.
    fn delete_question(&mut self, ui: &Ui, page: &mut TagsPage) {
        let Some(id) = page.deleting else {
            return;
        };
        let Some(name) = self.open.tags.name_of(id).map(str::to_owned) else {
            page.deleting = None;
            return;
        };
        let used = self.used_by(id);
        let mut what = Vec::new();
        if used.entities > 0 {
            let levels = used.levels.join(", ");
            what.push(match used.entities {
                1 => format!("1 entity (in {levels})"),
                n => format!("{n} entities (in {levels})"),
            });
        }
        if !used.modules.is_empty() {
            let names = used.modules.join(", ");
            what.push(match used.modules.len() {
                1 => format!("the module {names}"),
                n => format!("{n} modules ({names})"),
            });
        }
        let words = format!(
            "{name} is on {}. Deleting it takes it off all of them, and game code naming \
             Tag.{name} becomes a mistake until it's changed.",
            what.join(" and ")
        );
        let answers = [
            Answer::right("Cancel"),
            Answer::main("Delete").hint("Takes the tag out of the project"),
        ];
        let chosen = widgets::dialog::dialog(
            ui.ctx(),
            "delete tag",
            &format!("Delete the tag {name}?"),
            |ui| {
                ui.add(egui::Label::new(words).wrap());
            },
            &answers,
        );
        match chosen {
            Some(1) => {
                self.delete_tag(id);
                page.deleting = None;
            }
            Some(_) => page.deleting = None,
            None => {}
        }
    }
}

/// A tag's row: `fill` is handed the names' column, the middle and the
/// room for the ⋯ at the right; a hairline goes under.
fn row(ui: &mut Ui, fill: impl FnOnce(&mut Ui, &mut Ui, &mut Ui)) {
    let across = ui.available_rect_before_wrap().x_range();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(across.span(), ROW_H), egui::Sense::hover());
    let name_rect =
        egui::Rect::from_min_max(rect.min, egui::pos2(rect.left() + NAME_W, rect.bottom()));
    let more_rect = egui::Rect::from_min_max(
        egui::pos2(rect.right() - size::ICON_BUTTON - size::GAP, rect.top()),
        rect.max,
    );
    let middle_rect = egui::Rect::from_min_max(
        egui::pos2(name_rect.right(), rect.top()),
        egui::pos2(more_rect.left(), rect.bottom()),
    );
    let child = |ui: &mut Ui, r: egui::Rect, layout: Layout| {
        ui.new_child(egui::UiBuilder::new().max_rect(r).layout(layout))
    };
    let mut name_ui = child(ui, name_rect, Layout::left_to_right(Align::Center));
    let mut middle_ui = child(ui, middle_rect, Layout::left_to_right(Align::Center));
    let mut more_ui = child(ui, more_rect, Layout::right_to_left(Align::Center));
    fill(&mut name_ui, &mut middle_ui, &mut more_ui);
    tokens::hline(
        ui.painter(),
        across,
        rect.bottom() - size::LINE,
        tokens::line(),
    );
}

/// The middle's words, in their colour.
fn note_words(ui: &mut Ui, (words, ink): &(String, egui::Color32)) {
    ui.add(
        egui::Label::new(RichText::new(words).font(text::ui()).color(*ink))
            .truncate()
            .selectable(false),
    );
}

/// The section's sentence, game code's name for a tag in code's letters.
fn about_tags(ui: &mut Ui) {
    let mut job = egui::text::LayoutJob::default();
    let words = |font: egui::FontId| egui::TextFormat {
        font_id: font,
        color: tokens::text_meta(),
        line_height: Some(text::SETTING_HINT_LINE_H),
        ..egui::TextFormat::default()
    };
    job.append(
        "Labels you give entities, such as Enemy or Collectible, so game code can find them: \
         \u{201c}all enemies within 3 m\u{201d}, \u{201c}the nearest collectible\u{201d}. Game code \
         names them as ",
        0.0,
        words(text::setting_hint()),
    );
    job.append("Tag.Enemy", 0.0, words(text::mono_small()));
    job.append(
        ", so a misspelt tag is caught before the game runs.",
        0.0,
        words(text::setting_hint()),
    );
    job.wrap.max_width = ui.available_width();
    ui.add(egui::Label::new(job).selectable(false).wrap());
}

/// The card beside the tags: how game code asks for them.
fn game_code_card(ui: &mut Ui) {
    ui.add_space(size::SECTION_GAP);
    widgets::card::card(ui, "In game code", |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = size::ROW_GAP;
        let code = |ui: &mut Ui, line: &str| {
            plain(ui, line, text::mono(), tokens::text_primary());
        };
        let quiet = |ui: &mut Ui, line: &str| {
            plain(ui, line, text::ui(), tokens::text_meta());
        };
        code(ui, "entity:hasTag(Tag.Enemy)");
        code(ui, "World:within(Tag.Enemy, place, 3)");
        quiet(ui, "every enemy within 3 m, nearest first");
        code(ui, "World:nearest(Tag.Collectible, place)");
        quiet(ui, "the nearest one, or nothing");
        sentence(
            ui,
            "The same in Lust, Rust and Lua. Tags are saved in tags.lst with the project.",
        );
    });
}
