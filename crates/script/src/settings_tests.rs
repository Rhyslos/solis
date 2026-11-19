//! The player's settings (decision 0091).

use super::*;

#[test]
fn changes_are_kept_in_their_ranges() {
    let s = PlayerSettings::default();
    assert_eq!(s.with(Setting::TextSize(1.5)).text_size, 1.5);
    assert_eq!(s.with(Setting::TextSize(9.0)).text_size, 3.0);
    assert_eq!(
        s.with(Setting::TextSize(f64::NAN)).text_size,
        1.0,
        "not a number: nothing"
    );
    assert_eq!(s.with(Setting::UiSize(0.1)).ui_size, 0.5);
    assert_eq!(s.with(Setting::UiSize(f64::INFINITY)).ui_size, 1.0);
    assert!(s.with(Setting::Fullscreen(true)).fullscreen);
    assert_eq!(
        s.with(Setting::Resolution(1, 99_999)).resolution,
        [320, 16384]
    );
}

#[test]
fn settings_read_from_anywhere_are_made_sane() {
    let wild = PlayerSettings {
        text_size: f64::NAN,
        ui_size: 40.0,
        fullscreen: true,
        resolution: [0, 1080],
        language: Some(translations::Language::new("nb").unwrap()),
        subtitles: false,
        captions: true,
        caption_size: f64::INFINITY,
        caption_background: -3.0,
    };
    let s = wild.sanitized();
    assert_eq!(s.caption_size, 1.0, "not a number: as it starts");
    assert_eq!(s.caption_background, 0.0);
    assert!(!s.subtitles && s.captions);
    assert_eq!(s.language.map(|l| l.option()).as_deref(), Some("Nb"));
    assert_eq!(s.text_size, 1.0);
    assert_eq!(s.ui_size, 2.0);
    assert!(s.fullscreen);
    assert_eq!(s.resolution, [320, 1080]);
}

#[test]
fn text_size_is_in_percent_and_resolutions_step_round() {
    let s = PlayerSettings::default().with(Setting::TextSize(1.5));
    assert_eq!(s.text_percent(), 150);
    let mut r = PlayerSettings::default();
    let mut seen = Vec::new();
    for _ in 0..RESOLUTIONS.len() {
        r = r.with(Setting::Resolution(
            r.next_resolution()[0],
            r.next_resolution()[1],
        ));
        seen.push(r.resolution);
    }
    assert_eq!(seen.last(), Some(&[1280, 720]), "back to the first");
    let odd = PlayerSettings::default().with(Setting::Resolution(1000, 700));
    assert_eq!(
        odd.next_resolution(),
        [1280, 720],
        "an unusual size: the first"
    );
}

#[test]
fn the_players_language_falls_back_to_the_main_one() {
    let l = |c: &str| translations::Language::new(c).unwrap();
    let project = [l("en"), l("nb")];
    let chose = PlayerSettings::default().with(Setting::Language(l("nb")));
    assert_eq!(chose.language_of(&project), l("nb"));
    let other = PlayerSettings::default().with(Setting::Language(l("ja")));
    assert_eq!(other.language_of(&project), l("en"));
    let s = PlayerSettings::default()
        .with(Setting::CaptionSize(5.0))
        .with(Setting::CaptionBackground(0.25))
        .with(Setting::Captions(true))
        .with(Setting::Subtitles(false));
    assert_eq!(
        (
            s.caption_size,
            s.caption_background,
            s.captions,
            s.subtitles
        ),
        (2.0, 0.25, true, false)
    );
}
