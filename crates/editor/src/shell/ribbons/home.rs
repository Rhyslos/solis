//! Home's ribbon (the approved additions, round 1,
//! `04-home-ribbon-stopped.png`): Edit, Create, Snapping, Lighting,
//! Project. The Play group follows on every tab.

use super::group;
use crate::icons::Icon;
use crate::shell::ShellCommand as C;
use crate::shell::ribbon::{Group, Shape, big, big_list, row, row_list, switch, unbuilt_at};
use crate::unbuilt::{Entry, RowKind, UnbuiltId};
use crate::workbench::Workbench;

pub(super) fn groups(w: &Workbench) -> Vec<Group> {
    let snapping = w.viewport.snapping;
    vec![
        group!(
            Some("Edit"),
            [
                big(Icon::Undo, "Undo", C::Undo),
                big(Icon::Redo, "Redo", C::Redo),
            ]
        ),
        group!(
            Some("Create"),
            [
                big(Icon::Empty, "Empty", C::AddEntity),
                big_list(
                    Icon::Model,
                    "Model",
                    C::ImportModel,
                    vec![
                        Entry::Built(("Import model…", C::ImportModel)),
                        Entry::Unbuilt(UnbuiltId::HomeCreateShapes),
                    ],
                ),
                big(Icon::Sprite, "Sprite", C::AddSprite),
                big_list(
                    Icon::Light,
                    "Light",
                    C::AddPointLight,
                    vec![
                        Entry::Built(("Point light", C::AddPointLight)),
                        Entry::Built(("Spotlight", C::AddSpotlight)),
                    ],
                ),
                big(Icon::Camera, "Camera", C::AddCamera),
                unbuilt_at(
                    Shape::Big(Icon::Volume),
                    Entry::Unbuilt(UnbuiltId::HomeCreateTrigger),
                ),
                row("Tile map", C::NewTileMap),
                row_list(
                    "More",
                    vec![
                        Entry::Built(("Reverb zone", C::AddReverbZone)),
                        Entry::Built(("Assembly from selection", C::CreateAssembly)),
                    ],
                ),
            ]
        ),
        // Three rows leave no room for a label.
        group!(
            None,
            [
                switch(format!("Grid · {}", snapping.grid_step), C::SnapToGrid),
                switch(format!("Angle · {}°", snapping.angle_step), C::SnapAngles),
                switch("Scale · 10%", C::SnapScale),
            ]
        ),
        group!(
            Some("Lighting"),
            [
                unbuilt_at(
                    Shape::Big(Icon::Sun),
                    Entry::Unbuilt(UnbuiltId::HomeLightingBake),
                ),
                unbuilt_at(
                    Shape::Row(RowKind::List),
                    Entry::Unbuilt(UnbuiltId::HomeLightingQuality),
                ),
                unbuilt_at(
                    Shape::Row(RowKind::Plain),
                    Entry::Unbuilt(UnbuiltId::HomeLightingLightmapView),
                ),
            ]
        ),
        group!(
            Some("Project"),
            [
                big(Icon::Save, "Save", C::SaveArea),
                big(Icon::Build, "Build", C::ExportPlaytest),
            ]
        ),
    ]
}
