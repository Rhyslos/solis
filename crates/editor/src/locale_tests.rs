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

/// A tick of the game.
fn tick(w: &mut Workbench) {
    w.run_game((world::TICK + 1e-9) as f32);
}

/// The caption screen's lines now, if it's shown.
fn caption_lines(w: &Workbench) -> Vec<String> {
    w.game_screens()
        .iter()
        .filter(|s| s.name == "Captions")
        .flat_map(|s| {
            let mut words = Vec::new();
            fn walk(n: &ui::view::Node, out: &mut Vec<String>) {
                if n.kind == "text" {
                    out.push(n.text.clone());
                }
                for c in &n.children {
                    walk(c, out);
                }
            }
            if let Some(view) = &s.view {
                walk(view, &mut words);
            }
            words
        })
        .collect()
}

#[test]
fn a_sound_heard_shows_its_caption_while_captions_are_on() {
    let mut w = shop();
    w.set_locale(
        w.languages().clone(),
        vec![(
            "locale/en/captions.ftl".into(),
            "Captions-DoorCreaks = [The door creaks]\nSpeakers-Gardener = Gardener\nGardener-Hello = Hello!\n".into(),
        )],
    );
    w.play();
    tick(&mut w);
    let heard = [
        ("Captions.DoorCreaks".to_owned(), String::new(), 1.0),
        (
            "Gardener.Hello".to_owned(),
            "Speakers.Gardener".to_owned(),
            1.0,
        ),
    ];
    // Captions for sounds start off; subtitles on.
    w.captions_heard(&heard);
    tick(&mut w);
    assert_eq!(caption_lines(&w), ["Gardener:", "Hello!"], "only the voice");
    let mut settings = w.player_settings();
    settings.captions = true;
    w.set_player_settings(settings);
    w.captions_heard(&heard);
    tick(&mut w);
    // The creak was heard first (kept while captions were off), so it's
    // above the voice.
    assert_eq!(
        caption_lines(&w),
        ["[The door creaks]", "Gardener:", "Hello!"]
    );
    // Two seconds at least, then gone.
    for _ in 0..(2.5 / world::TICK) as usize {
        tick(&mut w);
    }
    assert!(caption_lines(&w).is_empty(), "their time is up");
    settings.captions = false;
    settings.subtitles = false;
    w.set_player_settings(settings);
    w.captions_heard(&heard);
    tick(&mut w);
    assert!(caption_lines(&w).is_empty(), "both off: nothing");
}

#[test]
fn a_subtitle_shown_from_rust_stays_until_stopped() {
    let mut w = shop();
    w.play();
    tick(&mut w);
    let line = w.show_subtitle("Mira", "Roses!", None);
    for _ in 0..300 {
        tick(&mut w);
    }
    assert_eq!(caption_lines(&w), ["Mira:", "Roses!"]);
    w.stop_subtitle(line);
    assert!(caption_lines(&w).is_empty());
}
