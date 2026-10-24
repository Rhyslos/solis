//! The turret sample, Solis's first game (Phase 4): a turret written in
//! Lust shoots at a drone that has health (Lust), armour (Rust) and sparks
//! (Lua), all talking through events. **Help > Make the sample game…**
//! writes it into a folder as a project.
//!
//! Its Rust component, Armour, is compiled into Solis from the sample's
//! own `armour.rs`, so the file the project shows is exactly what runs. It
//! stands in for a game's Rust until Phase 10 builds that, and is offered
//! only in a project whose assets have `armour.rs` (the sample).

use asset_db::Project;
use std::path::Path;

/// The sample's Rust component, from the very file the sample shows (its
/// `pub` is a game's own, so it's allowed here).
#[allow(unreachable_pub)]
mod armour {
    include!("../../../samples/turret/assets/armour.rs");
}

/// The sample's name, as its project file has it.
pub const NAME: &str = "Turret sample";

/// The area the sample opens on.
pub const AREA: &str = "turret.solarea";

/// The sample's files, inside its assets.
pub const FILES: &[(&str, &str)] = &[
    (
        "main.lst",
        include_str!("../../../samples/turret/assets/main.lst"),
    ),
    (
        "sparks.lua",
        include_str!("../../../samples/turret/assets/sparks.lua"),
    ),
    (
        "armour.rs",
        include_str!("../../../samples/turret/assets/armour.rs"),
    ),
    (
        AREA,
        include_str!("../../../samples/turret/assets/turret.solarea"),
    ),
];

/// Makes `root` the turret sample: a new project with the sample's files.
/// Refuses a folder that's already a project.
pub fn make(root: &Path) -> Result<(), asset_db::ProjectError> {
    let project = Project::create(root, NAME)?;
    for (path, text) in FILES {
        asset_db::write_atomically(&project.assets().join(path), text.as_bytes())?;
    }
    Ok(())
}

/// The Rust components built into Solis that the project in `assets`
/// uses: Armour, for the turret sample (its assets have `armour.rs`).
/// With a note if that file was changed, since changes to it don't run
/// until Phase 10.
pub fn rust_components(assets: &Path) -> (Vec<script::RustKind>, Option<String>) {
    let Ok(text) = std::fs::read_to_string(assets.join("armour.rs")) else {
        return (Vec::new(), None);
    };
    let built_in = FILES
        .iter()
        .find(|(path, _)| *path == "armour.rs")
        .map(|(_, t)| *t);
    let note = (Some(text.replace("\r\n", "\n").as_str()) != built_in).then(|| {
        "armour.rs was changed. Rust changes don't run yet: until Phase 10 builds a game's own \
         Rust, the Armour built into Solis is used."
            .to_string()
    });
    (vec![armour::armour()], note)
}

#[cfg(test)]
#[path = "sample_game_tests.rs"]
mod tests;
