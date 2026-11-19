//! The Translations tab (stage 1, translations; the mock-up is
//! `docs/design/translations/02-the-translations-tab.png`): every line of
//! the game's text, side by side in the main language and one other
//! chosen at the top, missing ones marked, with the search, Missing only,
//! + Key, and the spreadsheet out and back in.
//!
//! The grid is the data tables' (rule 21: the shared piece), its keys
//! and spreadsheet keys the same: the lines are shown as a table of two
//! text columns whose IDs are the keys and can't be changed. Typing in a
//! cell writes its language's Fluent file; undo and redo work on the
//! grid's table and write the files again.

use crate::table_editor::{Sheet, TableEditor};
use crate::tokens::{self, Accent, size, text};
use crate::widgets::{self, button::Button, field};
use crate::workbench::Workbench;
use egui::{RichText, Ui};
use level::table::{Cell, Column, ColumnKind, Row, Table};
use std::sync::Arc;
use translations::{Language, Translations};

/// The Translations tab's own state.
#[derive(Debug, Clone)]
pub(crate) struct TranslationsTab {
    /// The lines as the grid shows them.
    pub(crate) editor: TableEditor,
    /// The main language, as the grid was last made.
    pub(crate) main: Language,
    /// The language beside the main one.
    pub(crate) other: Language,
    /// Whether only the lines `other` lacks are shown.
    pub(crate) missing_only: bool,
    /// A new key being typed (+ Key), and why it can't be one.
    pub(crate) new_key: Option<(String, Option<String>)>,
    /// What the grid was made from: the translations, the language and
    /// Missing only.
    made_from: Option<(usize, Language, bool)>,
}

impl TranslationsTab {
    /// The tab showing `other` beside the main language.
    pub(crate) fn new(other: Language) -> Self {
        Self {
            editor: TableEditor::new("Translations".into(), Table::new("Line")),
            main: Language::english(),
            other,
            missing_only: false,
            new_key: None,
            made_from: None,
        }
    }
}

/// What the tab asked for after a frame.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct TranslationsOut {
    /// Lines typed in: each language, key and text.
    pub(crate) lines: Vec<(Language, String, String)>,
    /// Another language was chosen to show.
    pub(crate) show: Option<Language>,
    /// A new key to make, with its main-language text.
    pub(crate) new_key: Option<String>,
    /// Something to say in the Console.
    pub(crate) said: Option<String>,
    /// Export spreadsheet was pressed.
    pub(crate) export: bool,
    /// Import was pressed.
    pub(crate) import: bool,
}

impl TranslationsOut {
    /// Adds what another frame of the tab asked for.
    pub(crate) fn take(&mut self, other: Self) {
        self.lines.extend(other.lines);
        self.show = other.show.or(self.show);
        self.new_key = other.new_key.or(self.new_key.take());
        self.said = other.said.or(self.said.take());
        self.export |= other.export;
        self.import |= other.import;
    }
}

/// The lines as a table: each key a row (its ID), its text in the main
/// language and in `other` (empty where `other` has none); only the
/// missing ones with `missing_only`.
pub(crate) fn sheet_of(all: &Translations, other: Language, missing_only: bool) -> Table {
    let main = all.main();
    let mut table = Table::new("Line");
    table.columns = vec![
        Column::new("main", ColumnKind::Text),
        Column::new("other", ColumnKind::Text),
    ];
    for key in all.keys() {
        let theirs = all.line(other, &key).map(|l| l.text).unwrap_or_default();
        if missing_only && !theirs.is_empty() {
            continue;
        }
        let ours = all.line(main, &key).map(|l| l.text).unwrap_or_default();
        table.rows.push(Arc::new(Row {
            id: key,
            cells: vec![Cell::Text(ours), Cell::Text(theirs)],
        }));
    }
    table
}

/// Each column's heading: the keys, the main language, and `other`.
fn heads(all: &Translations, other: Language) -> Vec<(String, String)> {
    let main = all.main();
    vec![
        ("Key".into(), String::new()),
        (main.name(), "main".into()),
        (
            other.name(),
            format!("{}/{}", translations::FOLDER, other.code()),
        ),
    ]
}

/// Draws the Translations tab filling `ui`.
pub(crate) fn translations_tab(
    ui: &mut Ui,
    tab: &mut TranslationsTab,
    keys: bool,
    accent: Accent,
) -> TranslationsOut {
    let mut out = TranslationsOut::default();
    egui::Frame::new()
        .inner_margin(crate::theme::panel_margin())
        .show(ui, |ui| {
            bar(ui, tab, &mut out);
            ui.add_space(size::GAP);
            let note_h = 34.0;
            let grid_h =
                (ui.available_height() - note_h - size::GAP).max(crate::table_view::ROW_H * 3.0);
            let before = tab.editor.file.clone();
            ui.allocate_ui(egui::vec2(ui.available_width(), grid_h), |ui| {
                let grid = crate::table_view::grid_only(
                    ui,
                    crate::table_view::TableUi {
                        editor: &mut tab.editor,
                        names: None,
                        keys,
                        accent,
                    },
                );
                if grid.said.is_some() {
                    out.said = grid.said;
                }
            });
            out.lines
                .extend(typed(&before, &tab.editor.file, (tab.main, tab.other)));
            ui.add_space(size::GAP);
            note(ui, tab.other);
        });
    out
}

/// The lines whose text differs between `before` and `after`: the main
/// language's (the first column) and `other`'s (the second), each its
/// language, key and text.
fn typed(
    before: &Table,
    after: &Table,
    (main, other): (Language, Language),
) -> Vec<(Language, String, String)> {
    let mut out = Vec::new();
    for (a, b) in before.rows.iter().zip(&after.rows) {
        if Arc::ptr_eq(a, b) || a == b {
            continue;
        }
        for (column, (was, now)) in a.cells.iter().zip(&b.cells).enumerate() {
            if was != now
                && let Cell::Text(now) = now
            {
                let language = if column == 0 { main } else { other };
                out.push((language, b.id.clone(), now.clone()));
            }
        }
    }
    out
}

/// The bar over the grid.
fn bar(ui: &mut Ui, tab: &mut TranslationsTab, out: &mut TranslationsOut) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = size::CARD_INNER_GAP;
        let missing = tab
            .editor
            .file
            .rows
            .iter()
            .filter(|r| r.cells.get(1) == Some(&Cell::Text(String::new())))
            .count();
        ui.label(
            RichText::new("Beside the main language:")
                .font(text::ui())
                .color(tokens::text_meta()),
        );
        let shown = format!("{} ▾", tab.other.name());
        let choose = Button::plain(&shown).named("The language shown").show(ui);
        egui::Popup::menu(&choose).show(|ui| {
            let languages: Vec<Language> = ui
                .memory(|m| m.data.get_temp(egui::Id::new("translations languages")))
                .unwrap_or_default();
            for language in languages {
                let label = format!("{} · {}", language.name(), language.code());
                if widgets::menu::choice(ui, &label, language == tab.other).clicked() {
                    out.show = Some(language);
                    ui.close();
                }
            }
        });
        if !tab.missing_only && missing > 0 {
            ui.label(
                RichText::new(format!("{missing} missing"))
                    .font(text::header())
                    .color(tokens::ERROR_TEXT),
            );
        }
        field::search_field(
            ui,
            "Search keys and text",
            &mut tab.editor.search,
            "Search keys and text",
            200.0,
        );
        let mut only = tab.missing_only;
        if widgets::checkbox::checkbox(ui, &mut only, "Missing only").changed() {
            tab.missing_only = only;
        }
        let add = Button::plain("+ Key").named("Add a key").show(ui);
        if add.clicked() {
            tab.new_key = Some((String::new(), None));
        }
        if Button::plain("Export spreadsheet").show(ui).clicked() {
            out.export = true;
        }
        if Button::plain("Import").show(ui).clicked() {
            out.import = true;
        }
    });
    new_key_row(ui, tab, out);
}

/// While a new key is typed: its field, why it can't be one, and Enter.
fn new_key_row(ui: &mut Ui, tab: &mut TranslationsTab, out: &mut TranslationsOut) {
    let Some((typed, why)) = &mut tab.new_key else {
        return;
    };
    let mut finished = false;
    let done = &mut finished;
    ui.add_space(size::GAP);
    ui.horizontal(|ui| {
        let field = field::text_field(ui, "New key", typed, "Shop.Welcome", 220.0);
        if !field.has_focus() && !field.lost_focus() {
            field.request_focus();
        }
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
        let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let words = why.clone().unwrap_or_else(|| {
            "A group and a name, such as Shop.Welcome: Enter makes it, Esc doesn't".into()
        });
        let colour = if why.is_some() {
            tokens::WARNING
        } else {
            tokens::text_meta()
        };
        ui.label(RichText::new(words).font(text::ui()).color(colour));
        if escape {
            *done = true;
        } else if enter || field.lost_focus() && !typed.trim().is_empty() {
            let key = typed.trim().to_owned();
            match translations::keys::key_problem(&key) {
                Some(problem) => *why = Some(problem),
                None => {
                    out.new_key = Some(key);
                    *done = true;
                }
            }
        }
    });
    if finished {
        tab.new_key = None;
    }
}

/// Under the grid: where typing writes.
fn note(ui: &mut Ui, other: Language) {
    let words = format!(
        "Typing in a cell writes {}/{}/*.ftl. A table's main text is its table's. Plurals and \
         values: {{ $count -> [one] … *[other] … }}",
        translations::FOLDER,
        other.code()
    );
    crate::file_pages::sentence(ui, &words);
}

impl Workbench {
    /// Opens the Translations tab beside the level's view, showing
    /// `other` beside the main language (the first other language when
    /// none is given).
    pub(crate) fn open_translations(&mut self, other: Option<Language>) {
        let all = self.translations();
        let languages = all.languages();
        let other = other
            .or_else(|| languages.get(1).copied())
            .unwrap_or_else(|| all.main());
        match &mut self.open.translations_tab {
            Some(tab) => tab.other = other,
            None => self.open.translations_tab = Some(TranslationsTab::new(other)),
        }
        self.show_panel(crate::panel::Panel::Translations);
    }

    /// Makes the tab's grid again when the text, the language shown or
    /// Missing only changed (not while a cell is typed in), keeping the
    /// cursor and the search.
    pub(crate) fn refresh_translations_tab(&mut self, ctx: &egui::Context) {
        if self.open.translations_tab.is_none() {
            return;
        }
        let all = self.translations();
        ctx.memory_mut(|m| {
            m.data
                .insert_temp(egui::Id::new("translations languages"), all.languages());
        });
        let Some(tab) = &mut self.open.translations_tab else {
            return;
        };
        let now = (Arc::as_ptr(&all) as usize, tab.other, tab.missing_only);
        if tab.made_from == Some(now) || tab.editor.typing.is_some() {
            return;
        }
        let made = sheet_of(&all, tab.other, tab.missing_only);
        tab.main = all.main();
        tab.editor.sheet = Sheet::Lines {
            heads: heads(&all, tab.other),
        };
        if made != tab.editor.file {
            if tab
                .made_from
                .is_some_and(|(_, l, m)| l == tab.other && m == tab.missing_only)
            {
                // The text changed (typed here, or on disk): undo keeps
                // working on the grid's table.
                tab.editor.file = made;
            } else {
                tab.editor.taken_in(made);
            }
        }
        tab.made_from = Some(now);
    }

    /// Acts on what the Translations tab asked for.
    pub(crate) fn translations_asked(&mut self, out: TranslationsOut) {
        let main = self.translations().main();
        for (language, key, text) in out.lines {
            if let Err(why) = self.set_line(language, &key, &text) {
                self.log(crate::console::LogLevel::Warning, format!("{key}: {why}"));
            }
        }
        if let Some(key) = out.new_key {
            let all = self.translations();
            if all.has(&key) {
                self.log(
                    crate::console::LogLevel::Warning,
                    format!("There's already a line called {key}."),
                );
            } else if let Err(why) = self.set_line(main, &key, &key) {
                self.log(crate::console::LogLevel::Warning, why);
            }
        }
        if let Some(said) = out.said {
            self.log(crate::console::LogLevel::Warning, said);
        }
        if let Some(language) = out.show
            && let Some(tab) = &mut self.open.translations_tab
        {
            tab.other = language;
        }
        if out.export {
            self.export_translations();
        }
        if out.import {
            self.open
                .outbox
                .push(crate::workbench::WorkbenchAction::ReadTranslations);
        }
    }

    /// Undoes (or redoes) the Translations tab's last change, writing the
    /// lines it changes back into their files.
    pub(crate) fn translations_undo(&mut self, redo: bool, undo: bool) {
        let Some(tab) = &mut self.open.translations_tab else {
            return;
        };
        let before = tab.editor.file.clone();
        let done = if redo {
            tab.editor.redo()
        } else if undo {
            tab.editor.undo()
        } else {
            false
        };
        if !done {
            return;
        }
        let lines = typed(&before, &tab.editor.file, (tab.main, tab.other));
        self.translations_asked(TranslationsOut {
            lines,
            ..TranslationsOut::default()
        });
    }
}

#[cfg(test)]
#[path = "translations_tab_tests.rs"]
mod tests;
