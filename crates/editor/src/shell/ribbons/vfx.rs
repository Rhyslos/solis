//! VFX's ribbon: a whole screen that isn't built, so every control is
//! an unfinished part.

use super::group;
use crate::icons::Icon;
use crate::shell::ribbon::{Group, Shape, unbuilt_at};
use crate::unbuilt::{Entry, RowKind, UnbuiltId};

pub(super) fn groups() -> Vec<Group> {
    let big = |icon, part| unbuilt_at(Shape::Big(icon), part);
    let row = |kind, part| unbuilt_at(Shape::Row(kind), part);
    vec![
        group!(
            Some("Emitter"),
            [
                big(Icon::Plus, Entry::Unbuilt(UnbuiltId::VfxEmitterAdd)),
                big(Icon::Copy, Entry::Unbuilt(UnbuiltId::VfxEmitterDuplicate)),
            ]
        ),
        group!(
            Some("Add module"),
            [
                big(Icon::Sun, Entry::Unbuilt(UnbuiltId::VfxModuleSpawn)),
                big(Icon::Move, Entry::Unbuilt(UnbuiltId::VfxModuleVelocity)),
                big(Icon::Material, Entry::Unbuilt(UnbuiltId::VfxModuleColor)),
                big(Icon::Scale, Entry::Unbuilt(UnbuiltId::VfxModuleSize)),
                big(Icon::Volume, Entry::Unbuilt(UnbuiltId::VfxModuleCollide)),
            ]
        ),
        group!(
            None,
            [
                row(RowKind::Check, Entry::Unbuilt(UnbuiltId::VfxSimLoop)),
                row(RowKind::List, Entry::Unbuilt(UnbuiltId::VfxSimSpeed)),
                row(RowKind::Check, Entry::Unbuilt(UnbuiltId::VfxSimBounds)),
            ]
        ),
        group!(
            None,
            [
                row(
                    RowKind::List,
                    Entry::Unbuilt(UnbuiltId::VfxPreviewBackground)
                ),
                row(RowKind::List, Entry::Unbuilt(UnbuiltId::VfxPreviewTime)),
                row(RowKind::Check, Entry::Unbuilt(UnbuiltId::VfxPreviewFollow)),
            ]
        ),
    ]
}
