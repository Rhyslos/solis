//! Project settings › Languages: its rows, + Language, Make main and
//! Remove, and the player's caption settings, driven as a user would.

use crate::file_pages::Tab;
use crate::test_support::*;
use crate::workbench::{Workbench, WorkbenchAction};
use egui_kittest::kittest::Queryable;
use level::languages::Languages;

/// A project in English, Norwegian and German, on Project settings ›
/// Languages.
pub(crate) fn on_the_page() -> Workbench {
    let mut w = Workbench::new(info());
    w.set_project(Some(crate::ProjectInfo {
        name: "Farm Days".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm".into(),
    }));
    let mut languages = Languages::default();
    languages.add("nb").unwrap();
    languages.add("de").unwrap();
    w.set_locale(
        languages,
        vec![
            (
                "locale/en/shop.ftl".into(),
                "Shop-Welcome = Welcome!\nShop-Bye = Bye\n".into(),
            ),
            (
                "locale/nb/shop.ftl".into(),
                "Shop-Welcome = Velkommen!\nShop-Bye = Ha det\n".into(),
            ),
            (
                "locale/de/shop.ftl".into(),
                "Shop-Welcome = Willkommen!\n".into(),
            ),
        ],
    );
    w.show_settings(Tab::Languages);
    w
}

#[test]
fn each_language_says_how_much_is_translated() {
    let mut h = harness(on_the_page());
    h.run();
    h.get_by_label("English");
    h.get_by_label("main");
    h.get_by_label("2 lines");
    h.get_by_label("All translated");
    h.get_by_label("50% · 1 missing");
}

#[test]
fn make_main_and_remove_change_the_languages() {
    let mut w = on_the_page();
    let mut languages = w.languages().clone();
    languages.set_main("nb");
    w.change_languages(languages);
    assert_eq!(w.translations().main().code(), "nb");
    let mut languages = w.languages().clone();
    assert!(languages.remove("de"));
    w.change_languages(languages.clone());
    let out = w.take_outbox();
    assert!(
        out.contains(&WorkbenchAction::SaveLanguages(languages)),
        "{out:?}"
    );
}

#[test]
fn the_players_caption_settings_are_set_here() {
    let mut h = harness(on_the_page());
    h.run();
    h.get_all_by_label("Captions for sounds")
        .last()
        .unwrap()
        .click();
    h.run();
    assert!(h.state().0.player_settings().captions);
    h.get_by_label("Large").click();
    h.run();
    assert_eq!(h.state().0.player_settings().caption_size, 1.5);
}
