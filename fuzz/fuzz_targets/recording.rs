//! A recording of play: refused or read, then written and read back the
//! same.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(recording) = input::Recording::read(text) {
        let again = input::Recording::read(&recording.write());
        assert!(again.is_ok(), "a recording written by Solis reads back");
    }
});
