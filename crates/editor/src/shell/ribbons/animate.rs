//! Animate's ribbon: the design's sequencer controls, none of which the
//! engine has yet. A clips file's and an animator's own controls stay on
//! their panels until Animate's screen is rebuilt.

use super::group;
use crate::icons::Icon;
use crate::shell::ribbon::{Group, Shape, unbuilt_at};
use crate::unbuilt::{Entry, RowKind, UnbuiltId};

pub(super) fn groups() -> Vec<Group> {
    let big = |icon, part| unbuilt_at(Shape::Big(icon), part);
    let row = |part| unbuilt_at(Shape::Row(RowKind::Plain), part);
    vec![
        group!(
            Some("Keys"),
            [
                big(Icon::Key, Entry::Unbuilt(UnbuiltId::AnimKeysAdd)),
                big(Icon::Record, Entry::Unbuilt(UnbuiltId::AnimKeysAuto)),
            ]
        ),
        group!(
            None,
            [
                row(Entry::Unbuilt(UnbuiltId::AnimInterpStep)),
                row(Entry::Unbuilt(UnbuiltId::AnimInterpLinear)),
                row(Entry::Unbuilt(UnbuiltId::AnimInterpSmooth)),
            ]
        ),
        group!(
            Some("Tracks"),
            [
                big(Icon::Track, Entry::Unbuilt(UnbuiltId::AnimTracksTrack)),
                big(Icon::Curve, Entry::Unbuilt(UnbuiltId::AnimTracksCurves)),
            ]
        ),
    ]
}
