//! The Play group, at the ribbon's right end on every tab (the approved
//! additions, round 1, `04-home-ribbon-playing.png` and `-paused.png`):
//! Play, the one light button, becomes Stop while the game runs, with
//! Pause and Step beside it. Nothing else changes colour while the game
//! runs.

use super::ShellCommand as C;
use super::ribbon::{Control, Group, Shape, big, unbuilt_at};
use crate::icons::Icon;
use crate::play::PlayState;
use crate::unbuilt::{Entry, UnbuiltId};
use crate::workbench::Workbench;

/// The Play group as the game is now.
pub(super) fn group(w: &Workbench) -> Group {
    let state = w.play_state();
    let light = |icon, label, command| {
        let control = Control::Do {
            label,
            command,
            lit: false,
        };
        (Shape::Light(icon), vec![Entry::Built(control)])
    };
    let mut slots = Vec::new();
    if state == PlayState::Editing {
        slots.push(light(Icon::Play, "Play", C::Play));
    } else {
        slots.push(light(Icon::Stop, "Stop", C::Stop));
        // Lit while paused: a click then lets the game run on.
        let pause = Control::Do {
            label: "Pause",
            command: C::Pause,
            lit: state == PlayState::Paused,
        };
        slots.push((Shape::Big(Icon::Pause), vec![Entry::Built(pause)]));
        slots.push(big(Icon::Step, "Step", C::Step));
    }
    slots.push(unbuilt_at(
        Shape::Big(Icon::Simulate),
        Entry::Unbuilt(UnbuiltId::ShellPlaySimulate),
    ));
    Group {
        label: Some("Play"),
        slots,
    }
}
