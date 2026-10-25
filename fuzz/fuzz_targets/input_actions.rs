//! A project's input actions (`input.solinput`): refused or read, then
//! written and read back the same.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(bindings) = input::Bindings::read(text) {
        let again = input::Bindings::read(&bindings.write());
        assert_eq!(again.ok().as_ref(), Some(&bindings), "round trip");
    }
});
