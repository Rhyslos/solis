//! An area or assembly file: refused or read, its assemblies expanded
//! (it may place itself), and written and read back.
#![no_main]
use std::sync::Arc;
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(registry) = area::builtin::registry() else {
        return;
    };
    let Ok(file) = area::text::read_file(text, &registry) else {
        return;
    };
    // Every assembly it names is this file again: loops and depth must be
    // refused, never followed.
    let shared = Arc::new(file.clone());
    let lookup = |_| Some(Arc::clone(&shared));
    let _ = area::assembly::Expander::new(&lookup).expand(&file);
    let written = area::text::write(&file.area, &registry, &|_| None);
    let _ = area::text::read(&written, &registry);
});
