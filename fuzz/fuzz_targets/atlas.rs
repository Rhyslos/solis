//! An atlas file (`.atlas.lst`, Atlas Crop, decision 0089): refused or
//! read, and what's read is within the limits, each crop's name good and
//! used once, its box inside the biggest picture, and writes back to
//! itself.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    use animation::atlas as a;
    let Ok(read) = a::read(text) else {
        return;
    };
    let atlas = &read.atlas;
    assert!(atlas.crops.len() <= a::MAX_CROPS);
    assert!(atlas.grid.iter().all(|&g| (1..=a::MAX_SIDE).contains(&g)));
    for (i, crop) in atlas.crops.iter().enumerate() {
        assert!(a::good_name(&crop.name));
        assert!(!atlas.crops[..i].iter().any(|c| c.name.eq_ignore_ascii_case(&crop.name)));
        assert!(crop.rect[2] >= 1 && crop.rect[3] >= 1);
        assert!(crop.rect.iter().all(|&v| v <= a::MAX_SIDE));
    }
    let again = a::read(&a::write(atlas)).expect("what's written reads back");
    assert_eq!(&again.atlas, atlas);
});

/// Panics Solis catches on purpose aren't failures: only one that escapes
/// Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
