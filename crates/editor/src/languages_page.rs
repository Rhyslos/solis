//! Project settings › Languages (stage 1, translations; the mock-up is
//! `docs/design/translations/01-project-settings-languages.png`): a row
//! per language with its code, how much of it is translated and its ⋯
//! (Make main, Show in Translations, Remove), the main one's chip; + Language
//! with the languages Solis knows and a code typed; the Translations tab,
//! and the translators' spreadsheet out and back in. Beside them, how
//! game code reads the text, and the player's subtitles and captions as
//! they're used while playing here.

use crate::file_pages::{plain, sentence, with_side};
use crate::tokens::{self, size, text};
use crate::widgets::{self, button::Button};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Align, Layout, RichText, Ui};
use translations::Language;

/// The Languages page's own state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LanguagesPage {
    /// A language's code typed in + Language's list.
    pub(crate) typed: String,
}

/// How tall a language's row is.
const ROW_H: f32 = 52.0;
/// The names' column.
const NAME_W: f32 = 200.0;
/// The bar showing how much is translated.
const METER_W: f32 = 180.0;

/// What a row asked for.
enum RowAsk {
    MakeMain(Language),
    Show(Language),
    Remove(Language),
}

impl Workbench {
    /// Draws Project settings › Languages.
    pub(crate) fn languages_page(&mut self, ui: &mut Ui) {
        if self.open.project.is_none() {
            plain(
                ui,
                "Languages belong to a project: open or make one first.",
                text::ui(),
                tokens::text_meta(),
            );
            return;
        }
        let mut page = std::mem::take(&mut self.file_pages.languages);
        egui::ScrollArea::vertical()
            .id_salt("languages page")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                with_side(
                    ui,
                    &mut (&mut *self, &mut page),
                    |ui, (w, page)| w.language_rows(ui, page),
                    |ui, (w, _)| {
                        game_code_card(ui);
                        w.player_card(ui);
                    },
                );
            });
        self.file_pages.languages = page;
    }

    /// The section's words, a row per language, and the buttons.
    fn language_rows(&mut self, ui: &mut Ui, page: &mut LanguagesPage) {
        widgets::rows::section_caps(ui, "Languages");
        sentence(
            ui,
            "The languages your game's text is written in. The main language is the one you write \
             in: its text is shown wherever another language has no line yet, so the game works \
             before anything is translated. Each language's text is in locale/<code>/*.ftl.",
        );
        ui.add_space(size::GAP);
        let all = self.translations();
        let total = all.keys().len();
        let mut asked = None;
        for (n, language) in all.languages().into_iter().enumerate() {
            let main = n == 0;
            let have = all.count(language);
            asked = asked.or(language_row(ui, language, main, (have, total)));
        }
        ui.add_space(size::SECTION_GAP);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = size::CARD_INNER_GAP;
            self.add_language_button(ui, page);
            if Button::primary("Open the Translations tab")
                .show(ui)
                .clicked()
            {
                self.open_translations(None);
            }
            if Button::plain("Export spreadsheet")
                .show(ui)
                .on_hover_text("Writes translations.csv in the project's folder")
                .clicked()
            {
                self.export_translations();
            }
            if Button::plain("Import")
                .show(ui)
                .on_hover_text("Reads translations.csv back from the project's folder")
                .clicked()
            {
                self.open.outbox.push(WorkbenchAction::ReadTranslations);
            }
        });
        ui.add_space(size::GAP);
        sentence(
            ui,
            "Export writes translations.csv in the project's folder: a key, the main language and \
             a column per language. Import reads it back.",
        );
        match asked {
            Some(RowAsk::MakeMain(language)) => {
                let mut languages = self.languages().clone();
                languages.set_main(language.code());
                self.change_languages(languages);
            }
            Some(RowAsk::Show(language)) => self.open_translations(Some(language)),
            Some(RowAsk::Remove(language)) => {
                let mut languages = self.languages().clone();
                if languages.remove(language.code()) {
                    self.change_languages(languages);
                    self.log(
                        crate::console::LogLevel::Info,
                        format!(
                            "{} is no longer one of the project's languages; its files in {}/{} \
                             are kept.",
                            language.name(),
                            translations::FOLDER,
                            language.code()
                        ),
                    );
                }
            }
            None => {}
        }
    }

    /// The add-a-language button and its list: the languages Solis knows
    /// that the project hasn't, and a field for any code.
    fn add_language_button(&mut self, ui: &mut Ui, page: &mut LanguagesPage) {
        let add = Button::plain("+ Language ▾")
            .named("Add a language")
            .show(ui);
        let mut added = None;
        egui::Popup::menu(&add).show(|ui| {
            widgets::menu::caps(ui, "Add a language");
            let typed = widgets::field::text_field(
                ui,
                "A language's code",
                &mut page.typed,
                "pt-BR",
                160.0,
            );
            if typed.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                added = Some(page.typed.trim().to_owned());
                ui.close();
            }
            let filter = page.typed.trim().to_ascii_lowercase();
            for language in translations::language::known() {
                if self.languages().has(language.code()) {
                    continue;
                }
                let label = format!("{} · {}", language.name(), language.code());
                if !filter.is_empty() && !label.to_ascii_lowercase().contains(&filter) {
                    continue;
                }
                if widgets::menu::item(ui, &label, "").clicked() {
                    added = Some(language.code().to_owned());
                    ui.close();
                }
            }
        });
        if let Some(code) = added {
            page.typed.clear();
            let mut languages = self.languages().clone();
            match languages.add(&code) {
                Ok(()) => self.change_languages(languages),
                Err(why) => self.log(crate::console::LogLevel::Warning, why),
            }
        }
    }

    /// The player's settings for subtitles and captions, and their
    /// language, as used while playing here.
    fn player_card(&mut self, ui: &mut Ui) {
        ui.add_space(size::SECTION_GAP);
        let mut settings = self.player_settings;
        let languages = self.translations().languages();
        widgets::card::card(ui, "Subtitles and captions (the player's settings)", |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = size::ROW_GAP;
            setting(ui, "Subtitles", |ui| {
                widgets::toggle::toggle(ui, &mut settings.subtitles, "Subtitles");
            });
            setting(ui, "Captions for sounds", |ui| {
                widgets::toggle::toggle(ui, &mut settings.captions, "Captions for sounds");
            });
            setting(ui, "Size", |ui| {
                let mut size = if settings.caption_size < 0.9 {
                    0
                } else if settings.caption_size > 1.2 {
                    2
                } else {
                    1
                };
                if widgets::segmented::segmented(
                    ui,
                    "Caption size",
                    &mut size,
                    &[(0, "Small"), (1, "Medium"), (2, "Large")],
                ) {
                    settings.caption_size = [0.75, 1.0, 1.5][size];
                }
            });
            setting(ui, "Background", |ui| {
                let mut strength = settings.caption_background as f32;
                widgets::slider::slider(ui, &mut strength, 0.0..=1.0, "Caption background");
                settings.caption_background = f64::from(strength);
                ui.label(
                    RichText::new(format!("{:.0}%", strength * 100.0))
                        .font(text::ui())
                        .color(tokens::text_body()),
                );
            });
            setting(ui, "Language", |ui| {
                let shown = match settings.language {
                    Some(language) => language.name(),
                    None => format!(
                        "The computer's ({})",
                        settings.language_of(&languages).name()
                    ),
                };
                let pick = Button::plain(&format!("{shown} ▾"))
                    .named("The player's language")
                    .show(ui);
                egui::Popup::menu(&pick).show(|ui| {
                    if widgets::menu::choice(ui, "The computer's", settings.language.is_none())
                        .clicked()
                    {
                        settings.language = None;
                        ui.close();
                    }
                    for language in &languages {
                        let chosen = settings.language == Some(*language);
                        if widgets::menu::choice(ui, &language.name(), chosen).clicked() {
                            settings.language = Some(*language);
                            ui.close();
                        }
                    }
                });
            });
            sentence(
                ui,
                "Used while you play here. A game offers them in its own menu; an exported game \
                 keeps each player's.",
            );
        });
        if settings != self.player_settings {
            self.set_player_settings(settings);
        }
    }
}

/// One setting's row in the card: its name, then its control.
fn setting(ui: &mut Ui, name: &str, control: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            egui::vec2(140.0, 24.0),
            egui::Label::new(
                RichText::new(name)
                    .font(text::ui())
                    .color(tokens::text_label()),
            )
            .selectable(false),
        );
        control(ui);
    });
}

/// A language's row: its name, code and the main one's chip, how much of
/// it is translated, and its ⋯.
fn language_row(
    ui: &mut Ui,
    language: Language,
    main: bool,
    (have, total): (usize, usize),
) -> Option<RowAsk> {
    let mut asked = None;
    let across = ui.available_rect_before_wrap().x_range();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(across.span(), ROW_H), egui::Sense::hover());
    let mut inside = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let ui = &mut inside;
    let (names, _) = ui.allocate_exact_size(egui::vec2(NAME_W, ROW_H), egui::Sense::hover());
    let mut name_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(names)
            .layout(Layout::left_to_right(Align::Center)),
    );
    plain(
        &mut name_ui,
        &language.name(),
        text::setting_name(),
        tokens::text_emphasis(),
    );
    plain(
        &mut name_ui,
        language.code(),
        text::mono_small(),
        tokens::text_meta(),
    );
    if main {
        widgets::pill::pill(&mut name_ui, "main");
    }
    let share = if total == 0 {
        1.0
    } else {
        have as f32 / total as f32
    };
    let (bar, _) = ui.allocate_exact_size(egui::vec2(METER_W, 6.0), egui::Sense::hover());
    let colour = if main {
        tokens::LINK
    } else if share >= 0.9 {
        tokens::SUCCESS_TEXT
    } else {
        tokens::ERROR_TEXT
    };
    ui.painter()
        .rect_filled(bar, egui::CornerRadius::same(3), tokens::field());
    let mut filled = bar;
    filled.set_width(bar.width() * share.clamp(0.0, 1.0));
    ui.painter()
        .rect_filled(filled, egui::CornerRadius::same(3), colour);
    ui.add_space(size::GAP);
    let words = if main {
        (format!("{total} lines"), tokens::text_meta())
    } else if have == total {
        ("All translated".to_owned(), tokens::SUCCESS_TEXT)
    } else {
        (
            format!("{:.0}% · {} missing", share * 100.0, total - have),
            colour,
        )
    };
    ui.label(RichText::new(words.0).font(text::ui()).color(words.1));
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let more = widgets::button::icon(
            ui,
            crate::icons::Icon::More,
            &format!("{}'s menu", language.name()),
        )
        .on_hover_text("Make main, show in Translations, remove");
        egui::Popup::menu(&more).show(|ui| {
            if !main && widgets::menu::item(ui, "Make main", "").clicked() {
                asked = Some(RowAsk::MakeMain(language));
                ui.close();
            }
            if widgets::menu::item(ui, "Show in Translations", "").clicked() {
                asked = Some(RowAsk::Show(language));
                ui.close();
            }
            if !main && widgets::menu::item(ui, "Remove", "").clicked() {
                asked = Some(RowAsk::Remove(language));
                ui.close();
            }
        });
    });
    tokens::hline(
        ui.painter(),
        across,
        rect.bottom() - size::LINE,
        tokens::line(),
    );
    asked
}

/// The card beside the languages: how game code reads the text.
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
        code(ui, "World:text(Text.ShopWelcome)");
        quiet(ui, "a line in the player's language");
        code(ui, "ShopText:welcome(name, gold)");
        quiet(ui, "a line that takes values");
        code(ui, "Screens.language = Language.Nb");
        sentence(
            ui,
            "The same in Lust, Rust and Lua. A misspelt key is a mistake before the game runs.",
        );
    });
}

#[cfg(test)]
#[path = "languages_page_tests.rs"]
mod tests;
