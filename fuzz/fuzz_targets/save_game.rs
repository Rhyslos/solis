//! A save game, loaded into a fresh editor: refused with why, or loaded
//! and played on a few ticks, never a crash.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let info = editor::workbench::AppInfo {
        name: "Fuzz".into(),
        version: "0.0.0".into(),
    };
    let mut w = editor::workbench::Workbench::new(info, None);
    w.load_game(text, "fuzz.solsave");
    for _ in 0..3 {
        w.play_for(1.0 / 60.0);
    }
});
