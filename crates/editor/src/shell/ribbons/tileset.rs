//! LIBRARY TOOLS · Tileset's ribbon (the approved additions, round 3,
//! `02-tileset-editor.png`): Terrain (Add, Remove), Kind (the chosen
//! terrain's kind, its template saved beside the tileset) and Picture
//! (the tile size, and the zoom). No Play group, as the design's Mesh
//! ribbon.

use super::group;
use crate::icons::Icon;
use crate::shell::ShellCommand as C;
use crate::shell::ribbon::{Group, big, fact, item, row, row_list};
use crate::workbench::Workbench;

/// The zooms the picture can be shown at, in whole times.
pub(crate) const ZOOMS: [u8; 6] = [1, 2, 3, 4, 6, 8];

pub(super) fn groups(w: &Workbench) -> Vec<Group> {
    let editor = w.open.tilesets.tab();
    let kind = editor
        .and_then(|e| e.chosen())
        .map_or("—", |terrain| terrain.kind.word());
    let kinds = tiles::TileKind::ALL
        .into_iter()
        .map(|kind| item(kind.word(), C::TerrainKind(kind)))
        .collect();
    let [tw, th] = editor.map_or([0, 0], |e| e.file.tile);
    let zoom = editor.map_or(1.0, |e| e.zoom);
    let zooms = ZOOMS
        .into_iter()
        .map(|z| item(format!("{}%", u32::from(z) * 100), C::TilesetZoom(z)))
        .collect();
    vec![
        group!(
            Some("Terrain"),
            [
                big(Icon::Plus, "Add", C::TerrainAdd),
                big(Icon::Trash, "Remove", C::TerrainRemove),
            ]
        ),
        group!(
            Some("Kind"),
            [
                row_list(format!("Kind: {kind}"), kinds),
                row("Save its template…", C::SaveTemplate),
            ]
        ),
        group!(
            Some("Picture"),
            [
                fact(format!("Tile size: {tw} \u{d7} {th} px")),
                row_list(format!("Zoom: {:.0}%", zoom * 100.0), zooms),
            ]
        ),
    ]
}
