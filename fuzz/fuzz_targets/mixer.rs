//! The project's buses (`mixer.solmix`, Phase 7, S4): refused or read, and
//! buses read always make a sound tree (Master first, every other inside
//! one before it, within the limits), write back to themselves, and give
//! levels and ducking that are numbers.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(read) = sound::mixer::read(text) else {
        return;
    };
    let m = &read.mixer;
    assert!(!m.buses.is_empty() && m.buses.len() <= sound::mixer::MAX_BUSES);
    let (lo, hi) = sound::voices::VOICES_RANGE;
    assert!((lo..=hi).contains(&m.voices));
    assert_eq!(m.buses[0].name, sound::mixer::MASTER);
    assert!(m.buses[0].inside.is_empty());
    let parents = m.parents();
    for (i, bus) in m.buses.iter().enumerate().skip(1) {
        let p = parents[i].expect("every bus is inside one");
        assert!(p < i, "a bus comes after the one it's inside");
        assert!(m.depth(i) <= sound::mixer::MAX_DEPTH);
        assert!(bus.ducking.len() <= sound::mixer::MAX_RULES);
        for rule in &bus.ducking {
            assert!(m.index_of(&rule.while_bus).is_some() && rule.while_bus != bus.name);
            assert!(rule.by_db <= 0.0 && rule.down_s >= 0.0 && rule.back_s >= 0.0);
        }
    }
    let again = sound::mixer::read(&sound::mixer::write(m)).expect("what's written reads");
    assert_eq!(&again.mixer, m);
    let busy = vec![true; m.buses.len()];
    let mut ducker = sound::mixer::Ducker::default();
    let ducked = ducker.step(m, &busy, 0.1).to_vec();
    assert!(ducked.iter().all(|d| d.is_finite() && *d <= 0.0));
    let levels = sound::mixer::levels(m, &[], &ducked, &Default::default());
    assert!(levels.iter().all(|l| !l.is_nan()));
});

/// Panics Solis catches on purpose aren't failures: only one that escapes
/// Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
