//! The program's name and identity, in one place.
//!
//! (A build made only to try the update question on the splash screen.)

/// The name, shown in the title bar and the About box.
pub const APP_NAME: &str = "Solis";

/// The stable identifier used for the settings folder.
///
/// Changing it makes the editor start with fresh settings.
pub const APP_ID: &str = "solis";

/// The logo, 256 pixels square, for the window and taskbar. Its source
/// is `branding/logo.svg`; the program file's own icon is
/// `branding/logo.ico`, built in by `build.rs` on Windows.
pub const LOGO_PNG: &[u8] = include_bytes!("../../../branding/logo-256.png");

/// The GitHub repository whose releases hold each build, as `owner/name`
/// (updates, decision 0043).
pub const UPDATE_REPO: &str = "Rhyslos/solis";

/// The installer's file name in each release: the ID and `-setup.exe`.
/// The Windows EXE workflow names it the same way.
pub const INSTALLER_NAME: &str = "solis-setup.exe";

/// The folder in each project's assets holding the files that describe the
/// engine to game code (`solis/world.lst`), which games fetch by this name.
/// Changing it breaks existing games' code, like a file extension.
pub const CODE_FOLDER: &str = "solis";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_not_empty() {
        assert!(!APP_NAME.trim().is_empty());
        assert!(!APP_ID.trim().is_empty());
    }

    #[test]
    fn the_logo_is_a_square_picture() {
        let icon = eframe::icon_data::from_png_bytes(LOGO_PNG).unwrap();
        assert_eq!((icon.width, icon.height), (256, 256));
        // Round: the corners are see-through.
        assert_eq!(icon.rgba[3], 0);
    }

    #[test]
    fn the_installer_is_named_after_the_id() {
        assert_eq!(INSTALLER_NAME, format!("{APP_ID}-setup.exe"));
        assert_eq!(UPDATE_REPO.matches('/').count(), 1);
    }

    #[test]
    fn the_id_is_safe_for_folder_names() {
        assert!(
            APP_ID
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        );
    }
}
