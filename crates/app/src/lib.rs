//! The application: opens the window, owns the frame loop and wires the
//! engine and the editor together.
//!
//! Layer 3 (app). Holds no feature logic. The program's entry point is
//! `main.rs`; this library part holds what the entry point and its tests
//! share. See this crate's README for its purpose and limits.

pub mod area_file;
pub mod benchmark;
pub mod branding;
mod brush_uploads;
pub mod check_project;
pub mod cli;
mod external;
#[cfg(test)]
mod fixtures_tests;
pub mod game_audio;
pub mod game_code;
pub mod gamepad;
pub mod gpu;
pub mod import;
pub mod known_projects;
pub mod logger;
pub mod play_files;
pub mod playground;
mod playground_sounds;
pub mod playtest;
pub mod project;
pub mod sample_game;
pub mod sound;
pub mod sounds;
pub mod sprites;
pub mod startup;
pub mod text_files;
pub mod thumbnails;
mod to_lust;
pub mod ui_sample;
pub mod updates;
mod upgrades;
pub mod window;

/// The text shown by `--version`: the name and version.
pub fn version_banner() -> String {
    format!("{} {}", branding::APP_NAME, version())
}

/// The version, with the build number when GitHub built this copy
/// (`0.0.1 (build 57)`).
pub fn version() -> String {
    match updater::this_build() {
        Some(build) => format!("{} (build {build})", env!("CARGO_PKG_VERSION")),
        None => env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// The name and version, for the editor.
pub fn app_info() -> editor::AppInfo {
    editor::AppInfo {
        name: branding::APP_NAME.to_string(),
        version: version(),
    }
}

/// Things that aren't ordinary files, each called `name` in a folder of
/// its own under `folder`: a folder and, on Unix, a link to a device
/// (which never ends) and a pipe nobody writes to (which never opens).
/// Windows has neither of those in a project's folder. A reader that
/// hangs on one has lost its check (`reflect::ordinary_file`).
#[cfg(test)]
pub(crate) fn odd_files(folder: &std::path::Path, name: &str) -> Vec<std::path::PathBuf> {
    let inside = |kind: &str| {
        std::fs::create_dir_all(folder.join(kind)).unwrap();
        folder.join(kind).join(name)
    };
    let as_folder = inside("as-folder");
    std::fs::create_dir_all(&as_folder).unwrap();
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut odd = vec![as_folder];
    #[cfg(unix)]
    {
        let device = inside("as-device");
        std::os::unix::fs::symlink("/dev/zero", &device).unwrap();
        let pipe = inside("as-pipe");
        let made = std::process::Command::new("mkfifo").arg(&pipe).status();
        assert!(made.unwrap().success(), "mkfifo");
        odd.extend([device, pipe]);
    }
    odd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_contains_name_and_version() {
        let banner = version_banner();
        assert!(banner.starts_with(branding::APP_NAME));
        assert!(banner.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn app_info_comes_from_branding() {
        assert_eq!(app_info().name, branding::APP_NAME);
    }
}
