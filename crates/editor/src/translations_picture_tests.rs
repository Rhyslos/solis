//! Translations in approved pictures (stage 1, translations; the mock-up
//! `docs/design/translations/`): Project settings › Languages with three
//! languages and the player's caption card, and the Translations tab with
//! a missing line. Drawn for real at 1440 × 900 and cropped to what they
//! show.

use crate::file_pages::Tab;
use crate::interface_look::InterfaceLook;
use crate::picture_support::{Picture, drawn, editor_window, matches_approved};
use crate::workbench::Workbench;
use level::languages::Languages;

const SIZE: [f32; 2] = [1440.0, 900.0];

/// The mock-up's project: Farm Days in English, Norsk and Deutsch, with
/// the gardener's, the items' and the menu's words.
fn farm_days() -> Workbench {
    let mut w = Workbench::new(crate::test_support::info());
    w.set_project(Some(crate::ProjectInfo {
        name: "Farm Days".into(),
        root: "/projects/farm-days".into(),
        assets: "/projects/farm-days".into(),
    }));
    let mut languages = Languages::default();
    languages.add("nb").unwrap();
    languages.add("de").unwrap();
    let english = "Gardener-FirstMeeting = Oh! A new face in the village.\n\
                   Items-IronSword-name = Iron sword\n\
                   Menu-Continue = Continue\n\
                   Shop-Welcome = Welcome, { $name }!\n";
    let norsk = "Gardener-FirstMeeting = Å! Et nytt fjes i landsbyen.\n\
                 Items-IronSword-name = Jernsverd\n\
                 Shop-Welcome = Velkommen, { $name }!\n";
    let deutsch = "Menu-Continue = Weiter\n";
    w.set_locale(
        languages,
        vec![
            ("locale/en/farm.ftl".into(), english.into()),
            ("locale/nb/farm.ftl".into(), norsk.into()),
            ("locale/de/farm.ftl".into(), deutsch.into()),
        ],
    );
    w.open.outbox.clear();
    w
}

/// Draws `w` and crops a part.
fn part_of(w: Workbench, min: [u32; 2], size: [u32; 2]) -> Option<Picture> {
    let mut h = editor_window(w, InterfaceLook::default(), SIZE)?;
    for _ in 0..6 {
        h.run();
    }
    Some(drawn(&mut h).cropped(min, size))
}

#[test]
fn the_languages_tab_of_project_settings() {
    let mut w = farm_days();
    w.show_settings(Tab::Languages);
    let Some(picture) = part_of(w, [220, 30], [1220, 600]) else {
        return;
    };
    matches_approved("languages-page", &picture);
}

#[test]
fn the_translations_tab_with_a_missing_line() {
    let mut w = farm_days();
    w.open_translations(Some(translations::Language::new("nb").unwrap()));
    let Some(picture) = part_of(w, [250, 98], [870, 540]) else {
        return;
    };
    matches_approved("translations-tab", &picture);
}
