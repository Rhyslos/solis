//! The application: opens the window, owns the frame loop and wires the
//! engine and the editor together.
//!
//! Layer 3 (app). Holds no feature logic. The program's entry point is
//! `main.rs`; this library part holds what the entry point and its tests
//! share. See this crate's README for its purpose and limits.

pub mod branding;
pub mod cli;
pub mod gpu;
pub mod import;
pub mod window;

/// The text shown by `--version`: the name and version.
pub fn version_banner() -> String {
    format!("{} {}", branding::APP_NAME, env!("CARGO_PKG_VERSION"))
}

/// The name and version, for the editor.
pub fn app_info() -> editor::AppInfo {
    editor::AppInfo {
        name: branding::APP_NAME.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_contains_name_and_version() {
        let banner = version_banner();
        assert!(banner.starts_with(branding::APP_NAME));
        assert!(banner.ends_with(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn app_info_comes_from_branding() {
        assert_eq!(app_info().name, branding::APP_NAME);
    }
}
