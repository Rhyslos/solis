//! Build's ribbon: exporting the game for someone to play, which is
//! what the engine can build today.

use super::group;
use crate::icons::Icon;
use crate::shell::ShellCommand as C;
use crate::shell::ribbon::{Group, big};

pub(super) fn groups() -> Vec<Group> {
    vec![group!(
        Some("Playtest"),
        [big(Icon::Build, "Export", C::ExportPlaytest)]
    )]
}
