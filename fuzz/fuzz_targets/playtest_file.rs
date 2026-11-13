//! A playtest file (`playtest.txt` beside an exported game, and a
//! project's `.solis/last export.txt`): refused or read, every part put
//! right (a usable name and area, a window Solis can open), and written
//! back to itself.
#![no_main]
use editor::export_dialog::{MAX_PLAYTEST_BYTES, Playtest, game_name, usable_area};
use script::settings::WINDOW_SIDES;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    fuzz_support::quiet_caught_panics();
    // Both readers read at most this much, as text with bad bytes replaced.
    if data.len() as u64 > MAX_PLAYTEST_BYTES {
        return;
    }
    let Ok(playtest) = Playtest::parse(&String::from_utf8_lossy(data)) else {
        return;
    };
    assert_eq!(
        playtest.name,
        game_name(&playtest.name),
        "the name is put right"
    );
    assert!(
        usable_area(&playtest.area),
        "the area is one inside the assets"
    );
    assert!(
        playtest
            .size
            .iter()
            .all(|side| (WINDOW_SIDES.0..=WINDOW_SIDES.1).contains(side)),
        "the window is one Solis opens"
    );
    assert_eq!(
        Playtest::parse(&playtest.text()),
        Ok(playtest),
        "a playtest file reads back as written"
    );
});
