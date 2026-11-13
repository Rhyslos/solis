//! GitHub's answer about the latest release, and the checksum file beside
//! the installer (decision 0043): refused or read, and a release read is
//! always one of the repository's own downloads. The input is the answer,
//! a 0x01 byte, then the checksum file.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    fuzz_support::quiet_caught_panics();
    let (answer, checksum) = match data.iter().position(|b| *b == 1) {
        Some(at) => (&data[..at], &data[at + 1..]),
        None => (data, &[][..]),
    };
    if let Ok(text) = std::str::from_utf8(answer)
        && let Ok(release) = updater::release::read_release(text, REPO, SETUP)
    {
        assert!(updater::release::is_release_download(
            &release.installer.url,
            REPO
        ));
        assert!(updater::release::is_release_download(
            &release.checksum.url,
            REPO
        ));
        assert!(release.installer.size <= updater::release::MAX_INSTALLER_SIZE);
    }
    if let Ok(text) = std::str::from_utf8(checksum) {
        let _ = updater::release::read_checksum(text, SETUP);
    }
});

const REPO: &str = "Rhyslos/solis";
const SETUP: &str = "solis-setup.exe";
