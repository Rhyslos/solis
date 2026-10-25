//! A sidecar (`*.solmeta`): refused or read, then written and read back.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(sidecar) = asset_db::Sidecar::parse(text) {
        assert!(asset_db::Sidecar::parse(&sidecar.to_text()).is_ok());
    }
});
