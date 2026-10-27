//! A sound file (Phase 7, S1): a WAV or Ogg Vorbis file is refused or
//! read, and a sound read is always within the limits (crates/sound,
//! `read`): a sample rate and channel count it accepts, at least one
//! frame, and every sample a finite number between -1 and 1.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
    let Ok(clip) = sound::read_bytes(data) else {
        return;
    };
    let info = clip.info;
    assert!(
        (sound::read::MIN_SAMPLE_RATE..=sound::read::MAX_SAMPLE_RATE).contains(&info.sample_rate)
    );
    assert!((1..=sound::read::MAX_CHANNELS).contains(&info.channels));
    assert!(info.frames > 0);
    assert_eq!(clip.overview.frames(), info.frames);
    if let sound::Samples::Memory(frames) = &clip.samples {
        assert_eq!(frames.len() as u64, info.frames);
        for f in frames.iter() {
            assert!(f.left.is_finite() && f.right.is_finite());
            assert!((-1.0..=1.0).contains(&f.left) && (-1.0..=1.0).contains(&f.right));
        }
    }
});

/// Panics Solis catches on purpose aren't failures: only one that escapes
/// Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
