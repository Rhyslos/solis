//! The program's name and identity, in one place.

/// The name, shown in the title bar and the About box.
pub const APP_NAME: &str = "Solis";

/// The stable identifier used for the settings folder.
///
/// Changing it makes the editor start with fresh settings.
pub const APP_ID: &str = "solis";

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
    fn the_id_is_safe_for_folder_names() {
        assert!(
            APP_ID
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        );
    }
}
