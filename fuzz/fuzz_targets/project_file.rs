//! A project file (`project.solis`): refused or read.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = asset_db::Project::read_file(text);
    }
});
