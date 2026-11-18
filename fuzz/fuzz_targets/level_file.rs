//! A level or prefab file: refused or read, its prefabs expanded
//! (it may place itself), and written and read back.
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
    // Every prefab it names is this file again: loops and depth must be
    // refused, never followed.
    let shared = Arc::new(file.clone());
    let lookup = |_| Some(Arc::clone(&shared));
    let _ = level::prefab::Expander::new(&lookup).expand(&file);
    let written = level::text::write(&file.level, &registry, &|_| None);
    let _ = level::text::read(&written, &registry);
});
