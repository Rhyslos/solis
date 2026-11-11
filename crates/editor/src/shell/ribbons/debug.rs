//! Debug's ribbon (the approved additions, round 1,
//! `07-debug-ribbon.png`): Capture, Target, Show, Tools, and Session,
//! where what the Play menu had beyond the buttons lives now.

use super::group;
use crate::icons::Icon;
use crate::profiler::KEPT;
use crate::shell::ShellCommand as C;
use crate::shell::ribbon::{Group, Shape, big, fact, row, switch, unbuilt_at};
use crate::unbuilt::{Entry, RowKind, UnbuiltId};

pub(super) fn groups() -> Vec<Group> {
    let unbuilt_big = |icon, part| unbuilt_at(Shape::Big(icon), part);
    vec![
        group!(
            Some("Capture"),
            [
                unbuilt_big(Icon::Record, Entry::Unbuilt(UnbuiltId::DebugCaptureRecord)),
                unbuilt_big(Icon::Pause, Entry::Unbuilt(UnbuiltId::DebugCapturePause)),
                unbuilt_big(Icon::Save, Entry::Unbuilt(UnbuiltId::DebugCaptureSave)),
            ]
        ),
        group!(
            None,
            [
                unbuilt_at(
                    Shape::Row(RowKind::List),
                    Entry::Unbuilt(UnbuiltId::DebugTargetSource),
                ),
                fact(format!("Frames: last {KEPT}")),
                unbuilt_at(
                    Shape::Row(RowKind::Check),
                    Entry::Unbuilt(UnbuiltId::DebugTargetDeep),
                ),
            ]
        ),
        group!(
            None,
            [
                switch("CPU", C::ProfilerCpu),
                switch("GPU", C::ProfilerGpu),
                unbuilt_at(
                    Shape::Row(RowKind::Check),
                    Entry::Unbuilt(UnbuiltId::DebugShowMemory),
                ),
            ]
        ),
        group!(
            Some("Tools"),
            [
                unbuilt_big(
                    Icon::Debugger,
                    Entry::Unbuilt(UnbuiltId::DebugToolsDebugger)
                ),
                unbuilt_big(Icon::Memory, Entry::Unbuilt(UnbuiltId::DebugToolsMemory)),
            ]
        ),
        group!(
            Some("Session"),
            [
                big(Icon::Load, "Load save", C::LoadSave),
                row("Play and record inputs", C::PlayAndRecord),
                row("Replay a recording…", C::ReplayRecording),
            ]
        ),
    ]
}
