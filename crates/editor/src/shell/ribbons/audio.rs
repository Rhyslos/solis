//! Audio's ribbon (Screen 10, spec 751-756; round 3, `08-audio-mixer.png`):
//! Mixer (a new bus; Send and Snapshot unfinished), Bus (a ducking rule,
//! Remove: the project lead's answer 30 keeps the group), Monitor (Clear
//! solo, Mute the game, how many sounds are heard at once; what the
//! meters measure and who listens are unfinished) and the unfinished
//! Effects.

use super::group;
use crate::icons::Icon;
use crate::shell::ShellCommand as C;
use crate::shell::ribbon::{Group, Shape, big, item, row, row_list, switch, unbuilt_at};
use crate::unbuilt::{Entry, RowKind, UnbuiltId};
use crate::workbench::Workbench;
use crate::workbench_audio_tools::HEARD_AT_ONCE;

pub(super) fn groups(w: &Workbench) -> Vec<Group> {
    let unbuilt_big = |icon, part| unbuilt_at(Shape::Big(icon), part);
    let list = |part| unbuilt_at(Shape::Row(RowKind::List), part);
    let voices = w.open.mixer.mixer.voices;
    let heard = HEARD_AT_ONCE
        .into_iter()
        .map(|n| item(n.to_string(), C::HeardAtOnce(n)))
        .collect();
    vec![
        group!(
            Some("Mixer"),
            [
                big(Icon::Plus, "Bus", C::AddBus),
                unbuilt_big(Icon::Sound, Entry::Unbuilt(UnbuiltId::AudioMixerSend)),
                unbuilt_big(Icon::Save, Entry::Unbuilt(UnbuiltId::AudioMixerSnapshot)),
            ]
        ),
        group!(
            Some("Bus"),
            [
                big(Icon::Plus, "Add rule", C::AddDuckingRule),
                big(Icon::Trash, "Remove", C::RemoveBus),
            ]
        ),
        group!(
            None,
            [
                row("Clear solo", C::ClearSolo),
                switch("Mute the game", C::MuteGame),
                row_list(format!("Heard at once \u{b7} {voices}"), heard),
            ]
        ),
        group!(
            None,
            [
                list(Entry::Unbuilt(UnbuiltId::AudioMonitorMeters)),
                list(Entry::Unbuilt(UnbuiltId::AudioListenChoose)),
            ]
        ),
        group!(
            Some("Effects"),
            [
                unbuilt_big(Icon::Sliders, Entry::Unbuilt(UnbuiltId::AudioFxEq)),
                unbuilt_big(Icon::Curve, Entry::Unbuilt(UnbuiltId::AudioFxCompress)),
                unbuilt_big(Icon::ReverbZone, Entry::Unbuilt(UnbuiltId::AudioFxReverb)),
                unbuilt_big(Icon::Lines, Entry::Unbuilt(UnbuiltId::AudioFxLimiter)),
            ]
        ),
    ]
}
