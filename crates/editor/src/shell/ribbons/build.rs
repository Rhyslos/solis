//! Build's ribbon (round 3, `10-build-tab`): Playtest (Export for
//! playtesting, which is the tab's page, and Open folder for the last
//! export), then the full export the engine can't make yet: Targets
//! (Windows, Platforms) and Full export (Build, Installer, Your Rust
//! code).

use super::group;
use crate::icons::Icon;
use crate::shell::ShellCommand as C;
use crate::shell::ribbon::{Group, Shape, big, unbuilt_at};
use crate::unbuilt::{Entry, RowKind, UnbuiltId};
use crate::workbench::Workbench;

pub(super) fn groups(_w: &Workbench) -> Vec<Group> {
    let unbuilt_big = |icon, part| unbuilt_at(Shape::Big(icon), part);
    vec![
        group!(
            Some("Playtest"),
            [
                big(Icon::Build, "Export", C::ExportPlaytest),
                big(Icon::FolderOpen, "Open folder", C::OpenExportFolder),
            ]
        ),
        group!(
            Some("Targets"),
            [
                unbuilt_big(
                    Icon::Viewport,
                    Entry::Unbuilt(UnbuiltId::BuildTargetWindows)
                ),
                unbuilt_big(
                    Icon::FourSquares,
                    Entry::Unbuilt(UnbuiltId::BuildTargetPlatforms)
                ),
            ]
        ),
        group!(
            Some("Full export"),
            [
                unbuilt_big(Icon::Build, Entry::Unbuilt(UnbuiltId::BuildFull)),
                unbuilt_big(Icon::Import, Entry::Unbuilt(UnbuiltId::BuildInstaller)),
                unbuilt_at(
                    Shape::Row(RowKind::Check),
                    Entry::Unbuilt(UnbuiltId::BuildRustCode)
                ),
            ]
        ),
    ]
}
