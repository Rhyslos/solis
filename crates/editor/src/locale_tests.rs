//! The game's text in the editor: the translations made from the files
//! and the tables, made again only when either changes; a line set or
//! taken out written into its file and saved; a table's main text left to
//! its table; the spreadsheet taken in; the check's problems; and captions
//! of sounds heard shown on the caption screen.

use super::*;
use crate::test_support::*;

/// A workbench whose project is in English and Norwegian, with the shop's
/// words.
fn shop() -> Workbench {
    let mut w = Workbench::new(info());
    let mut languages = Languages::default();
    languages.add("nb").unwrap();
    w.set_locale(
        languages,
        vec![
            (
                "locale/en/shop.ftl".into(),
                "Shop-Welcome = Welcome!\nShop-Bye = Bye\n".into(),
            ),
            (
                "locale/nb/shop.ftl".into(),
                "Shop-Welcome = Velkommen!\n".into(),
            ),
        ],
    );
    w
}

fn nb() -> Language {
    Language::new("nb").unwrap()
}

#[test]
fn the_translations_are_made_once_until_the_text_changes() {
    let mut w = shop();
    let first = w.translations();
    assert!(Arc::ptr_eq(&first, &w.translations()), "made once");
    assert_eq!(first.text(nb(), "Shop.Welcome", &[]), "Velkommen!");
    assert_eq!(first.missing(nb()), ["Shop.Bye"]);
    w.set_line(nb(), "Shop.Bye", "Ha det").unwrap();
    let after = w.translations();
    assert!(!Arc::ptr_eq(&first, &after), "made again");
    assert_eq!(after.text(nb(), "Shop.Bye", &[]), "Ha det");
    let saved: Vec<WorkbenchAction> = w.take_outbox();
    assert!(
        saved
            .iter()
            .any(|a| matches!(a, WorkbenchAction::SaveLocaleFile(p, t)
            if p == "locale/nb/shop.ftl" && t.contains("Shop-Bye = Ha det"))),
        "{saved:?}"
    );
}

#[test]
fn a_line_that_isnt_fluent_is_refused_and_empty_takes_it_out() {
    let mut w = shop();
    assert!(w.set_line(nb(), "Shop.Bye", "Ha { det").is_err());
    w.set_line(nb(), "Shop.Welcome", "").unwrap();
    assert_eq!(w.translations().missing(nb()), ["Shop.Welcome", "Shop.Bye"]);
}

#[test]
fn a_new_language_and_a_new_main_one_are_saved() {
    let mut w = shop();
    let mut languages = w.languages().clone();
    languages.add("de").unwrap();
    languages.set_main("nb");
    w.change_languages(languages.clone());
    assert_eq!(w.translations().main(), nb());
    assert!(
        w.take_outbox()
            .contains(&WorkbenchAction::SaveLanguages(languages))
    );
}

#[test]
fn a_spreadsheet_is_taken_in_and_written_out() {
    let mut w = shop();
    w.import_translations("key,en,nb\r\nShop.Bye,Bye,Ha det\r\n");
    assert_eq!(w.translations().text(nb(), "Shop.Bye", &[]), "Ha det");
    w.export_translations();
    let out = w.take_outbox();
    assert!(
        out.iter()
            .any(|a| matches!(a, WorkbenchAction::ExportTranslations(t)
        if t.starts_with("\"key\",\"en\",\"nb\"") && t.contains("\"Ha det\""))),
        "{out:?}"
    );
}

#[test]
fn the_check_lists_whats_missing_and_unused() {
    let mut w = shop();
    let problems = w.translation_problems();
    assert!(
        problems.iter().any(|p| p
            .message
            .starts_with("Norsk is missing 1 of 2 lines: Shop.Bye.")),
        "{problems:?}"
    );
    assert!(
        problems
            .iter()
            .any(|p| p.message.contains("nothing uses: Shop.Welcome, Shop.Bye")),
        "{problems:?}"
    );
}

#[test]
fn the_names_game_code_gets() {
    let mut w = shop();
    let (names, notes) = w.text_names();
    assert!(notes.is_empty());
    assert_eq!(names.keys, ["Shop.Welcome", "Shop.Bye"]);
    assert_eq!(names.languages, ["En", "Nb"]);
}
