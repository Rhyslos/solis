//! This copy of the program: its build number, whether the installer put
//! it here, where updates are downloaded, and starting an installer.

use std::path::{Path, PathBuf};

/// The file the installer leaves beside the program, so the program knows
/// it was installed (and so may update itself). A copy run from a zip or
/// built from source doesn't have it, and is left alone.
pub const INSTALLED_MARKER: &str = "installed.txt";

/// This copy's build number: the GitHub build that made it, set through
/// the `BUILD_NUMBER` variable when it was compiled. None for a copy built
/// any other way (from source, say), which never updates itself.
pub fn this_build() -> Option<u64> {
    parse_build(option_env!("BUILD_NUMBER")?)
}

/// A build number from text: digits only, not zero.
pub fn parse_build(text: &str) -> Option<u64> {
    let text = text.trim();
    if text.is_empty() || text.len() > 18 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok().filter(|n| *n > 0)
}

/// Whether the program at `exe` was put there by the installer.
pub fn is_installed(exe: &Path) -> bool {
    exe.parent()
        .is_some_and(|folder| folder.join(INSTALLED_MARKER).is_file())
}

/// The folder updates are downloaded to: `%LOCALAPPDATA%\<app_id>\updates`
/// on Windows (the system's temporary folder elsewhere).
pub fn updates_folder(app_id: &str) -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(std::env::temp_dir);
    base.join(app_id).join("updates")
}

/// The arguments the installer is started with: no windows or questions,
/// closing the program if it's still open, and (when `relaunch`) starting
/// it again afterwards (the installer's own `RELAUNCH` setting).
pub fn installer_arguments(relaunch: bool) -> Vec<&'static str> {
    let mut args = vec![
        "/VERYSILENT",
        "/SUPPRESSMSGBOXES",
        "/NORESTART",
        "/CLOSEAPPLICATIONS",
    ];
    if relaunch {
        args.push("/RELAUNCH=1");
    }
    args
}

/// Starts the downloaded installer at `installer` and returns at once; the
/// program should close straight after, so the installer can replace it.
pub fn start_installer(installer: &Path, relaunch: bool) -> Result<(), String> {
    if !installer.is_file() {
        return Err(format!("the update at {} is gone", installer.display()));
    }
    std::process::Command::new(installer)
        .args(installer_arguments(relaunch))
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("couldn't start the update: {e}"))
}
