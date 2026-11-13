//! A model file (glb or glTF): refused with why, or imported within the
//! limits, its numbers finite.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    fuzz_support::quiet_caught_panics();
    let limits = assets::Limits {
        max_file_bytes: 1 << 22,
        ..assets::Limits::default()
    };
    if let Ok(model) = assets::load_bytes(data, None, "Fuzz", &limits) {
        assert!(model.is_finite(), "the importer only makes finite numbers");
    }
});
