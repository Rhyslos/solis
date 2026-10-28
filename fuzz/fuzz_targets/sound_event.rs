//! A sound event file (`.solsound`, Phase 7, S2): refused or read, and an
//! event read is always within its limits and writes back to itself.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(read) = sound::event::read(text) else {
        return;
    };
    let e = &read.event;
    assert!(e.takes.len() <= sound::event::MAX_TAKES);
    assert!(e.volume_db.is_finite() && e.pitch_st.is_finite());
    assert!(e.near_m <= e.far_m && e.far_m <= sound::event::MAX_DISTANCE_M);
    assert!((1..=sound::event::MAX_AT_ONCE).contains(&e.at_once));
    let again = sound::event::read(&sound::event::write(e)).expect("what's written reads");
    assert_eq!(&again.event, e);
    for tick in 0..4 {
        if let Some(pick) = sound::event::pick(e, None, 1, tick, 0) {
            assert!(e.takes.contains(&pick.take));
            assert!(pick.amplitude().is_finite() && pick.speed() > 0.0);
        }
    }
});

/// Panics Solis catches on purpose aren't failures: only one that escapes
/// Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
