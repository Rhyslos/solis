//! GitHub's answer about the latest release, and the checksum file beside
//! the installer (decision 0043): refused or read, and a release read is
//! always one of the repository's own downloads. The input is the answer,
//! a 0x01 byte, then the checksum file.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
    let (answer, checksum) = match data.iter().position(|b| *b == 1) {
        Some(at) => (&data[..at], &data[at + 1..]),
        None => (data, &[][..]),
    };
    if let Ok(text) = std::str::from_utf8(answer)
        && let Ok(release) = updater::release::read_release(text, REPO, SETUP)
    {
        assert!(updater::release::is_release_download(&release.installer.url, REPO));
        assert!(updater::release::is_release_download(&release.checksum.url, REPO));
        assert!(release.installer.size <= updater::release::MAX_INSTALLER_SIZE);
    }
    if let Ok(text) = std::str::from_utf8(checksum) {
        let _ = updater::release::read_checksum(text, SETUP);
    }
});

const REPO: &str = "Rhyslos/solis";
const SETUP: &str = "solis-setup.exe";

/// Panics Solis catches on purpose aren't failures: only one that escapes
/// Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
