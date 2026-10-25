//! A sidecar (`*.solmeta`): refused or read, then written and read back.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(sidecar) = asset_db::Sidecar::parse(text) {
        assert!(asset_db::Sidecar::parse(&sidecar.to_text()).is_ok());
    }
});

/// Panics Solis catches on purpose (a library's, such as the glTF
/// parser's, which Solis turns into a refused file) aren't failures: only
/// one that escapes Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
