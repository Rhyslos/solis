//! The project's 2D settings (`pixels.solpix`, Phase 9, D1): refused or
//! read, always within the limits, and written back to themselves.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(read) = area::pixels::read(text) else {
        return;
    };
    let s = read.settings;
    assert_eq!(s, s.sanitized(), "what's read is within the limits");
    assert!(s.pixel_size().is_finite() && s.pixel_size() > 0.0);
    let again = area::pixels::read(&area::pixels::write(&s)).expect("what's written reads");
    assert_eq!(again.settings, s);
    assert!(again.notes.is_empty());
});

/// Panics Solis catches on purpose aren't failures: only one that escapes
/// Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
