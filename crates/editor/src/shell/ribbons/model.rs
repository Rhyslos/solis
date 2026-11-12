//! LIBRARY TOOLS · Model's ribbon (round 3, `11-unbuilt-model-viewer`):
//! the model viewer isn't built, so every control is an unfinished part
//! but one, Place, which places the model the viewer shows.

use super::group;
use crate::icons::Icon;
use crate::shell::ShellCommand as C;
use crate::shell::ribbon::{Group, Shape, big, unbuilt_at};
use crate::unbuilt::{Entry, RowKind, UnbuiltId};

pub(super) fn groups() -> Vec<Group> {
    let unbuilt_big = |icon, part| unbuilt_at(Shape::Big(icon), part);
    let row = |kind, part| unbuilt_at(Shape::Row(kind), part);
    vec![
        group!(
            Some("View"),
            [
                unbuilt_big(Icon::Sun, Entry::Unbuilt(UnbuiltId::MeshViewLit)),
                unbuilt_big(Icon::Empty, Entry::Unbuilt(UnbuiltId::MeshViewUnlit)),
                unbuilt_big(Icon::Model, Entry::Unbuilt(UnbuiltId::MeshViewWire)),
                unbuilt_big(Icon::Move, Entry::Unbuilt(UnbuiltId::MeshViewNormals)),
                unbuilt_big(
                    Icon::FourSquares,
                    Entry::Unbuilt(UnbuiltId::MeshViewUvCheck)
                ),
            ]
        ),
        group!(
            None,
            [
                row(RowKind::Check, Entry::Unbuilt(UnbuiltId::MeshShowBounds)),
                row(RowKind::Check, Entry::Unbuilt(UnbuiltId::MeshShowPivot)),
                row(RowKind::Check, Entry::Unbuilt(UnbuiltId::MeshShowCollision)),
            ]
        ),
        group!(
            None,
            [
                row(RowKind::List, Entry::Unbuilt(UnbuiltId::MeshLightPreset)),
                row(RowKind::Plain, Entry::Unbuilt(UnbuiltId::MeshLightRotate)),
                row(
                    RowKind::List,
                    Entry::Unbuilt(UnbuiltId::MeshLightBackground)
                ),
            ]
        ),
        group!(
            None,
            [
                row(RowKind::Check, Entry::Unbuilt(UnbuiltId::MeshLodAuto)),
                row(RowKind::List, Entry::Unbuilt(UnbuiltId::MeshLodPick)),
                row(RowKind::Check, Entry::Unbuilt(UnbuiltId::MeshLodDensity)),
            ]
        ),
        group!(
            Some("Tools"),
            [
                unbuilt_big(Icon::Import, Entry::Unbuilt(UnbuiltId::MeshToolsReimport)),
                unbuilt_big(Icon::Move, Entry::Unbuilt(UnbuiltId::MeshToolsPivot)),
                unbuilt_big(Icon::Camera, Entry::Unbuilt(UnbuiltId::MeshToolsShot)),
            ]
        ),
        group!(
            Some("Use"),
            [
                big(Icon::Model, "Place", C::PlacePicked),
                row(
                    RowKind::Plain,
                    Entry::Unbuilt(UnbuiltId::MeshUseShowInLevel)
                ),
            ]
        ),
    ]
}
