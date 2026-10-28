//! An animator file (`.solanim`, Phase 9, D3b): refused or read, and
//! what's read is within the limits, writes back to itself, and runs
//! without panicking whatever game code sets.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(read) = animation::animator_file::read(text) else {
        return;
    };
    use animation::animator_file as f;
    use animation::{Animator, ParamValue};
    let file = &read.file;
    assert!(file.states.len() <= f::MAX_STATES);
    assert!(file.transitions.len() <= f::MAX_TRANSITIONS);
    assert!(file.parameters.len() <= f::MAX_PARAMETERS);
    for s in &file.states {
        assert!((f::SPEED_RANGE.0..=f::SPEED_RANGE.1).contains(&s.speed));
        assert!(s.place.iter().all(|p| p.abs() <= f::MAX_PLACE));
    }
    for t in &file.transitions {
        assert!(t.when.len() <= f::MAX_CONDITIONS);
    }
    let again = f::read(&f::write(file)).expect("what's written reads");
    assert_eq!(&again.file, file);
    // Run it with every parameter set every way, finished and not.
    let animator = Animator::new(file);
    let mut run = animator.start("");
    for tick in 0..16u8 {
        for (name, _) in animator.parameters().to_vec() {
            for value in [
                ParamValue::Number(f32::from(tick) - 4.0),
                ParamValue::Switch(tick % 2 == 0),
                ParamValue::Trigger(true),
                ParamValue::Direction([f32::from(tick % 3) - 1.0, 1.0]),
            ] {
                let _ = animator.set(&mut run, &name, value);
            }
        }
        animator.step(&mut run, tick % 3 == 0);
        let _ = animator.clip(&run, "walk");
        assert!((0.0..=16.0).contains(&animator.speed(&run)));
    }
});

/// Panics Solis catches on purpose aren't failures: only one that escapes
/// Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
