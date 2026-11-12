//! LIBRARY TOOLS · Picture's ribbon (the approved additions, round 3,
//! `03-picture-crops.png`): Crop (Every tile, Delete) and Grid (the
//! grid's size, and the zoom). No Play group, as the design's Mesh
//! ribbon.

use super::group;
use crate::icons::Icon;
use crate::shell::ShellCommand as C;
use crate::shell::ribbon::{Group, big, fact, item, row_list};
use crate::workbench::Workbench;

/// The zooms the picture can be shown at, in whole times.
pub(crate) const ZOOMS: [u8; 6] = [1, 2, 4, 7, 10, 16];

pub(super) fn groups(w: &Workbench) -> Vec<Group> {
    let editor = w.open.atlas_editor.as_ref();
    let [gw, gh] = editor.map_or([0, 0], |e| e.atlas.grid);
    let zoom = editor.map_or(1.0, |e| e.zoom);
    let zooms = ZOOMS
        .into_iter()
        .map(|z| item(format!("{}%", u32::from(z) * 100), C::CropsZoom(z)))
        .collect();
    vec![
        group!(
            Some("Crop"),
            [
                big(Icon::Grid, "Every tile", C::CropEveryTile),
                big(Icon::Trash, "Delete", C::CropDelete),
            ]
        ),
        group!(
            Some("Grid"),
            [
                fact(format!("Grid: {gw} \u{d7} {gh} px")),
                row_list(format!("Zoom: {:.0}%", zoom * 100.0), zooms),
            ]
        ),
    ]
}
