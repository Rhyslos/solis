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
    };
    let s = wild.sanitized();
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
