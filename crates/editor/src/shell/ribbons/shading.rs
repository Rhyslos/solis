//! Shading's ribbon (the approved additions, round 1,
//! `01-placeholder-shading-A.png`): a whole screen that isn't built, so
//! every control is an unfinished part.

use super::group;
use crate::icons::Icon;
use crate::shell::ribbon::{Group, Shape, unbuilt_at};
use crate::unbuilt::{Entry, RowKind, UnbuiltId};

pub(super) fn groups() -> Vec<Group> {
    let big = |icon, part| unbuilt_at(Shape::Big(icon), part);
    let row = |kind, part| unbuilt_at(Shape::Row(kind), part);
    vec![
        group!(
            Some("Material"),
            [
                big(Icon::Check, Entry::Unbuilt(UnbuiltId::ShadingMaterialApply)),
                big(Icon::Save, Entry::Unbuilt(UnbuiltId::ShadingMaterialSave)),
            ]
        ),
        group!(
            Some("Add node"),
            [
                big(Icon::Picture, Entry::Unbuilt(UnbuiltId::ShadingAddTexture)),
                big(Icon::Math, Entry::Unbuilt(UnbuiltId::ShadingAddMath)),
                big(Icon::Sliders, Entry::Unbuilt(UnbuiltId::ShadingAddParam)),
            ]
        ),
        group!(
            None,
            [
                row(
                    RowKind::Plain,
                    Entry::Unbuilt(UnbuiltId::ShadingGraphAutoLayout)
                ),
                row(
                    RowKind::Plain,
                    Entry::Unbuilt(UnbuiltId::ShadingGraphComment)
                ),
                row(RowKind::List, Entry::Unbuilt(UnbuiltId::ShadingGraphWires)),
            ]
        ),
        group!(
            None,
            [
                row(
                    RowKind::List,
                    Entry::Unbuilt(UnbuiltId::ShadingPreviewShape)
                ),
                row(RowKind::Check, Entry::Unbuilt(UnbuiltId::ShadingPreviewLit)),
                row(
                    RowKind::Check,
                    Entry::Unbuilt(UnbuiltId::ShadingPreviewLive)
                ),
            ]
        ),
    ]
}
