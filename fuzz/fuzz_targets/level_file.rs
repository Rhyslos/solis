//! A level or module file: refused or read, its modules expanded
//! (it may place itself), and written and read back. Every trigger zone
//! read is within its limits, and its freeform outline is checked and cut
//! into convex pieces without panicking.
#![no_main]
use std::sync::Arc;
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    fuzz_support::quiet_caught_panics();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(registry) = level::builtin::registry() else {
        return;
    };
    let Ok(file) = level::text::read_file(text, &registry) else {
        return;
    };
    // Every module it names is this file again: loops and depth must be
    // refused, never followed.
    let shared = Arc::new(file.clone());
    let lookup = |_| Some(Arc::clone(&shared));
    let _ = level::module::Expander::new(&lookup).expand(&file);
    for (_, zone) in file.level.with::<level::builtin::TriggerZone>() {
        assert!(zone.points.len() <= mesh::outline::MAX_OUTLINE_POINTS);
        if zone.outline().is_ok() {
            assert!(!mesh::outline::convex_pieces(&zone.points).is_empty());
        }
    }
    let written = level::text::write(&file.level, &registry, &|_| None);
    let _ = level::text::read(&written, &registry);
});
