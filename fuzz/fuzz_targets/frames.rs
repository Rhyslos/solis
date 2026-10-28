//! A sprite's clips file (`.solframes`, Phase 9, D3): refused or read,
//! and what's read is within the limits, writes back to itself, and plays
//! without panicking on any sheet.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(read) = animation::frames_file::read(text) else {
        return;
    };
    use animation::frames_file as f;
    let file = &read.file;
    assert!(file.tags.len() + file.clips.len() <= f::MAX_CLIPS);
    for t in &file.tags {
        assert!((f::SPEED_RANGE.0..=f::SPEED_RANGE.1).contains(&t.speed));
        assert!(t.events.len() <= f::MAX_TAG_EVENTS);
    }
    for c in &file.clips {
        assert!(c.frames.len() <= f::MAX_FRAMES && c.repeats <= f::MAX_REPEATS);
        assert!(c.frames.iter().all(|fr| (1..=f::MAX_MS).contains(&fr.ms)));
        let clip = animation::Clip::new(
            c.name.clone(),
            c.frames.clone(),
            c.direction,
            c.repeats,
            c.speed,
            false,
        );
        let _ = clip.at(0.37);
        let mut out = Vec::new();
        clip.events_between(0.0, 2.5, &mut out);
    }
    let again = f::read(&f::write(file)).expect("what's written reads");
    assert_eq!(&again.file, file);
});

/// Panics Solis catches on purpose aren't failures: only one that escapes
/// Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
