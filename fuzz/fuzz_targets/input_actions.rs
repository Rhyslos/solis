//! A project's input actions (`input.solinput`): refused or read, then
//! written and read back the same.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(bindings) = input::Bindings::read(text) {
        let again = input::Bindings::read(&bindings.write());
        assert_eq!(again.ok().as_ref(), Some(&bindings), "round trip");
    }
});

/// Panics Solis catches on purpose (a library's, such as the glTF
/// parser's, which Solis turns into a refused file) aren't failures: only
/// one that escapes Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
