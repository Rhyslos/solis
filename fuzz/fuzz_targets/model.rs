//! A model file (glb or glTF): refused with why, or imported within the
//! limits, its numbers finite.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
    let limits = assets::Limits {
        max_file_bytes: 1 << 22,
        ..assets::Limits::default()
    };
    if let Ok(model) = assets::load_bytes(data, None, "Fuzz", &limits) {
        assert!(model.is_finite(), "the importer only makes finite numbers");
    }
});

/// Panics Solis catches on purpose (a library's, such as the glTF
/// parser's, which Solis turns into a refused file) aren't failures: only
/// one that escapes Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
