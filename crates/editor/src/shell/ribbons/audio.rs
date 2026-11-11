//! Audio's ribbon: the design's mixer controls the engine has nothing
//! behind yet. Adding a bus and the rest of what works stay in the Mixer
//! panel until Audio's screen is rebuilt.

use super::group;
use crate::icons::Icon;
use crate::shell::ribbon::{Group, Shape, unbuilt_at};
use crate::unbuilt::{Entry, RowKind, UnbuiltId};

pub(super) fn groups() -> Vec<Group> {
    let big = |icon, part| unbuilt_at(Shape::Big(icon), part);
    let list = |part| unbuilt_at(Shape::Row(RowKind::List), part);
    vec![
        group!(
            Some("Mixer"),
            [
                big(Icon::ArrowUp, Entry::Unbuilt(UnbuiltId::AudioMixerSend)),
                big(Icon::Copy, Entry::Unbuilt(UnbuiltId::AudioMixerSnapshot)),
            ]
        ),
        group!(
            Some("Monitor"),
            [
                list(Entry::Unbuilt(UnbuiltId::AudioMonitorMeters)),
                list(Entry::Unbuilt(UnbuiltId::AudioListenChoose)),
            ]
        ),
        group!(
            Some("Effects"),
            [
                big(Icon::Sliders, Entry::Unbuilt(UnbuiltId::AudioFxEq)),
                big(Icon::Curve, Entry::Unbuilt(UnbuiltId::AudioFxCompress)),
                big(Icon::ReverbZone, Entry::Unbuilt(UnbuiltId::AudioFxReverb)),
                big(Icon::Lines, Entry::Unbuilt(UnbuiltId::AudioFxLimiter)),
            ]
        ),
    ]
}
