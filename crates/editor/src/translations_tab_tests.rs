//! The Translations tab: the lines side by side with missing ones marked,
//! typing a cell writing its file, undo writing it back, + Key, another
//! language shown, and Missing only.

use super::*;
use crate::test_support::*;
use egui_kittest::kittest::Queryable;
use level::languages::Languages;

fn nb() -> Language {
    Language::new("nb").unwrap()
}

/// A project in English and Norwegian, its Translations tab open.
pub(crate) fn with_the_tab() -> Workbench {
    let mut w = Workbench::new(info());
    let mut languages = Languages::default();
    languages.add("nb").unwrap();
    w.set_locale(
        languages,
        vec![
            (
                "locale/en/shop.ftl".into(),
                "Gardener-FirstMeeting = Oh! A new face in the village.\nMenu-Continue = Continue\n".into(),
            ),
            (
                "locale/nb/shop.ftl".into(),
                "Gardener-FirstMeeting = Å! Et nytt fjes i landsbyen.\n".into(),
            ),
        ],
    );
    w.open_translations(None);
    w
}

#[test]
fn the_lines_are_side_by_side_with_the_missing_marked() {
    let mut h = harness(with_the_tab());
    for _ in 0..4 {
        h.run();
    }
    assert!(
        h.state()
            .0
            .shell
            .drawn_at(crate::panel::Panel::Translations)
            .is_some(),
        "the Translations tab is in view"
    );
    h.get_by_label("1 missing");
    let tab = h.state().0.open.translations_tab.as_ref().unwrap();
    assert_eq!(tab.editor.file.rows.len(), 2);
    assert_eq!(tab.editor.file.rows[1].id, "Menu.Continue");
    assert_eq!(tab.editor.file.rows[1].cells[1], Cell::Text(String::new()));
}

#[test]
fn typing_a_cell_writes_its_file_and_undo_writes_it_back() {
    let mut w = with_the_tab();
    let ctx = egui::Context::default();
    w.refresh_translations_tab(&ctx);
    let tab = w.open.translations_tab.as_mut().unwrap();
    let before = tab.editor.file.clone();
    tab.editor.set_text((1, 2), "Fortsett").unwrap();
    let lines = typed(&before, &tab.editor.file, (tab.main, tab.other));
    assert_eq!(
        lines,
        [(nb(), "Menu.Continue".to_owned(), "Fortsett".to_owned())]
    );
    w.translations_asked(TranslationsOut {
        lines,
        ..TranslationsOut::default()
    });
    assert_eq!(
        w.translations().text(nb(), "Menu.Continue", &[]),
        "Fortsett"
    );
    w.refresh_translations_tab(&ctx);
    w.translations_undo(false, true);
    assert_eq!(w.translations().missing(nb()), ["Menu.Continue"], "undone");
    let saved = w.take_outbox();
    assert!(
        saved
            .iter()
            .filter(|a| matches!(a, crate::workbench::WorkbenchAction::SaveLocaleFile(..)))
            .count()
            >= 2
    );
}

#[test]
fn a_key_cant_be_changed_but_a_new_one_can_be_made() {
    let mut w = with_the_tab();
    let ctx = egui::Context::default();
    w.refresh_translations_tab(&ctx);
    let tab = w.open.translations_tab.as_mut().unwrap();
    assert!(
        tab.editor
            .set_text((0, 0), "Menu.Go")
            .unwrap_err()
            .contains("never changed")
    );
    w.translations_asked(TranslationsOut {
        new_key: Some("Menu.Quit".into()),
        ..TranslationsOut::default()
    });
    assert!(w.translations().has("Menu.Quit"));
    w.refresh_translations_tab(&ctx);
    assert_eq!(
        w.open
            .translations_tab
            .as_ref()
            .unwrap()
            .editor
            .file
            .rows
            .len(),
        3
    );
}

#[test]
fn missing_only_and_another_language() {
    let mut w = with_the_tab();
    let ctx = egui::Context::default();
    w.open.translations_tab.as_mut().unwrap().missing_only = true;
    w.refresh_translations_tab(&ctx);
    let rows: Vec<String> = w
        .open
        .translations_tab
        .as_ref()
        .unwrap()
        .editor
        .file
        .rows
        .iter()
        .map(|r| r.id.clone())
        .collect();
    assert_eq!(rows, ["Menu.Continue"]);
    w.translations_asked(TranslationsOut {
        show: Some(Language::english()),
        ..TranslationsOut::default()
    });
    assert_eq!(
        w.open.translations_tab.as_ref().unwrap().other,
        Language::english()
    );
}
