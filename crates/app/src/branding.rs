//! The program's name and identity, in one place.

/// The name, shown in the title bar and the About box.
pub const APP_NAME: &str = "Solis";

/// The stable identifier used for the settings folder.
///
/// Changing it makes the editor start with fresh settings.
pub const APP_ID: &str = "solis";

/// The file that marks a game's project folder, one per game, as
/// `project.godot` does for Godot (decision 0014). Other programs look for
/// it too: Aventail 3D finds a Solis project by it.
pub const PROJECT_FILE: &str = "project.solis";

/// The extension of scene files: `level.solscene` (decision 0014).
pub const SCENE_EXTENSION: &str = "solscene";

/// The extension of prefab files: `crate.solprefab` (decision 0014).
pub const PREFAB_EXTENSION: &str = "solprefab";

/// The extension of material files: `stone.solmat` (decision 0014).
pub const MATERIAL_EXTENSION: &str = "solmat";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_not_empty() {
        assert!(!APP_NAME.trim().is_empty());
        assert!(!APP_ID.trim().is_empty());
    }

    #[test]
    fn file_names_are_plain_and_distinct() {
        let extensions = [SCENE_EXTENSION, PREFAB_EXTENSION, MATERIAL_EXTENSION];
        for ext in extensions {
            assert!(!ext.is_empty());
            assert!(!ext.starts_with('.'), "{ext} is written without its dot");
            assert!(ext.chars().all(|c| c.is_ascii_lowercase()), "{ext}");
        }
        for (i, a) in extensions.iter().enumerate() {
            for b in &extensions[i + 1..] {
                assert_ne!(a, b);
            }
        }
        // The project file is a fixed name, not an extension, and isn't
        // mistaken for one of the others.
        let project_extension = PROJECT_FILE.rsplit_once('.').map(|(_, ext)| ext);
        assert!(project_extension.is_some_and(|ext| !extensions.contains(&ext)));
        assert!(!PROJECT_FILE.contains(['/', '\\']));
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
