//! Library's ribbon (spec 293-301; round 2, `08-library-ribbon.png`):
//! Add (Import, Create, Folder), View, Filter, Source control and Use.
//! The ribbon has no Play group on this tab only in the design; Solis
//! keeps Play on every tab (M3's notes).

use super::group;
use crate::icons::Icon;
use crate::library::Filter;
use crate::shell::ShellCommand as C;
use crate::shell::commands::NewFile;
use crate::shell::ribbon::{Control, Group, Shape, big, big_list, item, switch, unbuilt_at};
use crate::unbuilt::{Entry, RowKind, UnbuiltId};
use crate::workbench::Workbench;

pub(super) fn groups(w: &Workbench) -> Vec<Group> {
    let create = Control::List {
        label: "Create".into(),
        command: None,
        items: NewFile::ALL
            .into_iter()
            .map(|new| item(new.word(), C::NewInLibrary(new)))
            .collect(),
    };
    let types = Control::List {
        label: type_words(w.open.library_state.filter).into(),
        command: None,
        items: Filter::ALL
            .into_iter()
            .map(|filter| item(filter.name(), C::LibraryFilter(filter)))
            .collect(),
    };
    vec![
        group!(
            Some("Add"),
            [
                big_list(
                    Icon::Import,
                    "Import",
                    C::ImportModel,
                    vec![
                        item("Import model…", C::ImportModel),
                        Entry::Unbuilt(UnbuiltId::LibImportAnyKind),
                    ],
                ),
                (Shape::Big(Icon::Plus), vec![Entry::Built(create)]),
                big(Icon::Library, "Folder", C::NewInLibrary(NewFile::Folder)),
            ]
        ),
        group!(
            Some("View"),
            [
                switch("List", C::LibraryList),
                switch("Show folders", C::LibraryFolders),
            ]
        ),
        group!(
            Some("Filter"),
            [
                (Shape::Row(RowKind::List), vec![Entry::Built(types)]),
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
            [
                big(Icon::Select, "Place", C::PlacePicked),
                unbuilt_at(
                    Shape::Row(RowKind::Plain),
                    Entry::Unbuilt(UnbuiltId::LibUseShowInLevel),
                ),
            ]
        ),
    ]
}

/// "Type: All", the Filter group's list, saying the chip in force.
fn type_words(filter: Filter) -> &'static str {
    match filter {
        Filter::All => "Type: All",
        Filter::Models => "Type: Models",
        Filter::Sprites => "Type: Sprites",
        Filter::Maps => "Type: Maps",
        Filter::Sounds => "Type: Sounds",
        Filter::Code => "Type: Code",
    }
}
