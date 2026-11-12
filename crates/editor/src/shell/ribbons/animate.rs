//! Animate's ribbon, by what is open (the approved additions, round 3,
//! `06-animate-ribbons.png`). A clips file's: Clip (New, Copy, Delete),
//! Frames (Take out) and Playback (how many times, the speed, the
//! direction). An animator's: State (Add state, Start here, Delete) and
//! Preview (Start again). Then, on every form, the design's sequencer
//! controls, none of which the engine has yet.

use super::group;
use crate::icons::Icon;
use crate::shell::ShellCommand as C;
use crate::shell::ribbon::{Group, Shape, big, fact, item, row_list, unbuilt_at};
use crate::unbuilt::{Entry, RowKind, UnbuiltId};
use crate::workbench::Workbench;
use animation::Direction;

/// A clip's directions, in the order its list gives them.
pub(crate) const DIRECTIONS: [Direction; 4] = [
    Direction::Forward,
    Direction::Reverse,
    Direction::PingPong,
    Direction::PingPongReverse,
];

pub(super) fn groups(w: &Workbench) -> Vec<Group> {
    let mut groups = match w.animate_form() {
        Some(AnimateForm::Clips) => clips_file(w),
        Some(AnimateForm::Animator) => animator(),
        None => Vec::new(),
    };
    groups.extend(sequencer());
    groups
}

/// What Animate's ribbon is for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnimateForm {
    /// A clips file's.
    Clips,
    /// An animator's.
    Animator,
}

impl Workbench {
    /// Which form Animate's ribbon takes: a clips file's while one is open
    /// and its shelf tabs are in front (or no animator is open), else an
    /// animator's while one is open.
    pub(crate) fn animate_form(&self) -> Option<AnimateForm> {
        let open = self.open_now();
        let clips_in_front = matches!(
            self.shelf_tab(),
            Some(crate::shell::ShelfTab::Frames | crate::shell::ShelfTab::Clip)
        );
        if open.clips && (clips_in_front || !open.animator) {
            Some(AnimateForm::Clips)
        } else if open.animator {
            Some(AnimateForm::Animator)
        } else {
            None
        }
    }
}

/// A clips file's groups.
fn clips_file(w: &Workbench) -> Vec<Group> {
    let clip = w.chosen_clip();
    let plays = clip.as_ref().map_or_else(
        || "—".to_owned(),
        |c| match c.repeats {
            0 => "over and over".to_owned(),
            1 => "once".to_owned(),
            n => format!("{n} times"),
        },
    );
    let speed = clip.as_ref().map_or(1.0, |c| c.speed);
    let direction = clip.as_ref().map_or("—", |c| c.direction.name());
    let directions = DIRECTIONS
        .into_iter()
        .enumerate()
        .map(|(i, d)| item(d.name(), C::ClipDirection(i as u8)))
        .collect();
    vec![
        group!(
            Some("Clip"),
            [
                big(Icon::Plus, "New", C::ClipNew),
                big(Icon::Copy, "Copy", C::ClipCopy),
                big(Icon::Trash, "Delete", C::ClipDelete),
            ]
        ),
        group!(
            Some("Frames"),
            [big(Icon::Close, "Take out", C::FramesTakeOut)]
        ),
        Group {
            label: None,
            slots: vec![
                fact(format!("Plays: {plays}")),
                fact(format!("Speed: {speed:.1}")),
                row_list(format!("Direction: {direction}"), directions),
            ],
        },
    ]
}

/// An animator's groups.
fn animator() -> Vec<Group> {
    vec![
        group!(
            Some("State"),
            [
                big(Icon::Plus, "Add state", C::StateAdd),
                big(Icon::Play, "Start here", C::StateStartHere),
                big(Icon::Trash, "Delete", C::StateDelete),
            ]
        ),
        group!(
            Some("Preview"),
            [big(Icon::Replay, "Start again", C::PreviewAgain)]
        ),
    ]
}

/// The design's sequencer controls, unfinished.
fn sequencer() -> Vec<Group> {
    let big = |icon, part| unbuilt_at(Shape::Big(icon), part);
    let row = |part| unbuilt_at(Shape::Row(RowKind::Check), part);
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
