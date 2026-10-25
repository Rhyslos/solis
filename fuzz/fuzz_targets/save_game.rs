//! A save game, loaded into a fresh editor: refused with why, or loaded
//! and played on a few ticks, never a crash.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
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

/// Panics Solis catches on purpose (a library's, such as the glTF
/// parser's, which Solis turns into a refused file) aren't failures: only
/// one that escapes Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
