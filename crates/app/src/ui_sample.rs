//! The UI sample (game UI milestone M7): a turret the player fires at a
//! drone, with a HUD, the drone's health bar and a pause menu offering the
//! player's settings, every screen drawn on the UI editor's canvas.
//! **Make the UI sample…** (File > Learn, or the search) writes it into a folder as a project.

use asset_db::Project;
use std::path::Path;

/// The sample's name, as its project file has it.
pub const NAME: &str = "UI sample";

/// The area the sample opens on.
pub const AREA: &str = "ui.solarea";

/// The sample's files, inside its assets.
pub const FILES: &[(&str, &str)] = &[
    (
        "main.lst",
        include_str!("../../../samples/ui/assets/main.lst"),
    ),
    (AREA, include_str!("../../../samples/ui/assets/ui.solarea")),
    (
        "ui/hud.lst",
        include_str!("../../../samples/ui/assets/ui/hud.lst"),
    ),
    (
        "ui/droneHealth.lst",
        include_str!("../../../samples/ui/assets/ui/droneHealth.lst"),
    ),
    (
        "ui/pauseMenu.lst",
        include_str!("../../../samples/ui/assets/ui/pauseMenu.lst"),
    ),
];

/// Makes `root` the UI sample: a new project with the sample's files.
/// Refuses a folder that's already a project, or that has a file where
/// the sample's would go (nothing of the person's is ever replaced).
pub fn make(root: &Path) -> Result<(), asset_db::ProjectError> {
    for (path, _) in FILES {
        let file = root.join(asset_db::ASSETS_FOLDER).join(path);
        if file.exists() {
            return Err(asset_db::ProjectError::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!(
                    "there's already a file called {}, which the sample would replace; pick an \
                     empty folder",
                    file.strip_prefix(root).unwrap_or(&file).display()
                ),
            )));
        }
    }
    let project = Project::create(root, NAME)?;
    for (path, text) in FILES {
        let file = project.assets().join(path);
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder)?;
        }
        asset_db::write_atomically(&file, text.as_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "ui_sample_tests.rs"]
mod tests;
