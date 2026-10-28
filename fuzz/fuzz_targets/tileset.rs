//! A tileset (`.soltileset`, Phase 9, D4): refused or read, and what's
//! read is within the limits, has each name and code once, and writes
//! back to itself.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(read) = tiles::tileset::read(text) else {
        return;
    };
    use tiles::tileset as t;
    let set = &read.tileset;
    assert!(set.terrains.len() <= t::MAX_TERRAINS);
    for (i, terrain) in set.terrains.iter().enumerate() {
        assert!(t::usable_code(terrain.code), "{:?}", terrain.code);
        assert!(!terrain.name.is_empty() && terrain.name.chars().count() <= t::MAX_NAME);
        assert!(terrain.colour.iter().all(|c| (0.0..=1.0).contains(c)));
        assert_eq!(set.by_code(terrain.code).map(|f| f.id), Some(terrain.id), "{i}");
        assert_eq!(set.by_name(&terrain.name).map(|f| f.id), Some(terrain.id), "{i}");
    }
    let again = t::read(&t::write(set)).expect("what's written reads");
    assert_eq!(&again.tileset, set);
    if let Some(code) = set.free_code() {
        assert!(set.by_code(code).is_none() && t::usable_code(code));
    }
});

/// Panics Solis catches on purpose aren't failures: only one that escapes
/// Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
