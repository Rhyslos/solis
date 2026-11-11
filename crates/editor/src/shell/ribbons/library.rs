//! Library's ribbon, as far as it goes before the Library's own screen
//! is rebuilt: bringing a model in, and the design's controls the engine
//! has nothing behind yet. Making files, the view and the kind filter
//! stay on the Library panel's own bar until then.

use super::group;
use crate::icons::Icon;
use crate::shell::ShellCommand as C;
use crate::shell::ribbon::{Group, Shape, big_list, unbuilt_at};
use crate::unbuilt::{Entry, RowKind, UnbuiltId};

pub(super) fn groups() -> Vec<Group> {
    vec![
        group!(
            Some("Add"),
            [big_list(
                Icon::Import,
                "Import",
                C::ImportModel,
                vec![
                    Entry::Built(("Import model…", C::ImportModel)),
                    Entry::Unbuilt(UnbuiltId::LibImportAnyKind),
                ],
            )]
        ),
        group!(
            Some("Filter"),
            [
                unbuilt_at(
                    Shape::Row(RowKind::List),
                    Entry::Unbuilt(UnbuiltId::LibFilterTags),
                ),
                unbuilt_at(
                    Shape::Row(RowKind::Check),
                    Entry::Unbuilt(UnbuiltId::LibFilterMissing),
                ),
            ]
        ),
        group!(
            Some("Source control"),
            [
                unbuilt_at(
                    Shape::Big(Icon::Check),
                    Entry::Unbuilt(UnbuiltId::LibSourceCheckIn),
                ),
                unbuilt_at(
                    Shape::Big(Icon::Clock),
                    Entry::Unbuilt(UnbuiltId::LibSourceHistory),
                ),
            ]
        ),
        group!(
            Some("Use"),
            [unbuilt_at(
                Shape::Row(RowKind::Plain),
                Entry::Unbuilt(UnbuiltId::LibUseShowInLevel),
            )]
        ),
    ]
}
