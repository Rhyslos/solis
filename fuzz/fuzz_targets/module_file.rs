//! A module file (`.module.lst`, or an assembly's older `.solassembly`):
//! refused or read, its copies expanded (it may place itself), and
//! written and read back as the same module.
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
    let Ok(file) = level::module_file::read(text, &registry) else {
        return;
    };
    // Every module it places is this file again: loops and depth must be
    // refused, never followed.
    let shared = Arc::new(file.clone());
    let lookup = |_| Some(Arc::clone(&shared));
    let _ = level::module::Expander::new(&lookup).expand(&file);
    if let Ok(written) = level::module_file::write(&file, &registry) {
        let again = level::module_file::read(&written, &registry);
        assert_eq!(again.as_ref().ok(), Some(&file), "a module written reads back");
    }
});
