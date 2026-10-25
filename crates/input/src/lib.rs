//! The game's input as named actions ([decision 0037](../../../docs/decisions/0037-input-as-named-actions.md)):
//! game code asks about "Jump" or "Move", never about keys, so players
//! can rebind, gamepads work without extra code, and a game's input can
//! be recorded and played back.
//!
//! - [`Bindings`]: a project's actions and what sets each off (keys,
//!   mouse buttons and movement, gamepad buttons and sticks), kept in
//!   `input.solinput`, a TOML file with one line per binding.
//! - [`Devices`]: what the keyboard, mouse and gamepad are doing, fed by
//!   the editor as it happens, and sampled into a [`Frame`] once per
//!   tick, so every core sees the same input for a tick.
//! - [`Recording`]: each tick's frame, with checksums, to play a game
//!   back exactly.
//!
//! Inputs are named as text: keys by egui's names (`W`, `Space`,
//! `ArrowUp`) plus `Shift`, `Ctrl` and `Alt`; `Mouse Left`, `Mouse Right`,
//! `Mouse Middle`, `Mouse Motion`; and `Pad A` … (see [`PAD_BUTTONS`]),
//! `Pad LeftStick` and `Pad RightStick` (as axes: the stick; as buttons:
//! pressing it in).

mod bindings;
mod devices;
mod frame;
mod recording;

pub use bindings::{Action, ActionKind, Bindings, BindingsError, MAX_ACTIONS, MAX_INPUTS};
pub use devices::{Devices, Pad};
pub use frame::{ActionState, Frame};
pub use recording::{Recording, RecordingError, Tick};

/// The file in a project that holds its input actions, beside
/// `project.solis`.
pub const FILE: &str = "input.solinput";

/// The gamepad's buttons, by the names bindings use (after `Pad `): the
/// Xbox layout, which most PC gamepads follow.
pub const PAD_BUTTONS: [&str; 16] = [
    "A",
    "B",
    "X",
    "Y",
    "LB",
    "RB",
    "LT",
    "RT",
    "Back",
    "Start",
    "LeftStick",
    "RightStick",
    "Up",
    "Down",
    "Left",
    "Right",
];

/// The mouse's buttons, by the names bindings use (after `Mouse `).
pub const MOUSE_BUTTONS: [&str; 5] = ["Left", "Right", "Middle", "Back", "Forward"];

/// The movement of the mouse, as an axis's input.
pub const MOUSE_MOTION: &str = "Mouse Motion";

/// How far a stick must lean before it counts (sticks rarely rest at
/// exactly zero).
pub const DEAD_ZONE: f32 = 0.15;

/// Whether `name` is a sensible action name: letters, digits and `_`,
/// starting with a letter, at most 64 long (it's used in game code and in
/// recordings).
pub fn valid_action_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Whether `name` is something an input can be: a key's name (letters,
/// digits and a few signs, at most 32 long), or a mouse or pad input.
pub fn valid_input_name(name: &str) -> bool {
    if let Some(button) = name.strip_prefix("Pad ") {
        return PAD_BUTTONS.contains(&button);
    }
    if let Some(button) = name.strip_prefix("Mouse ") {
        return MOUSE_BUTTONS.contains(&button) || button == "Motion";
    }
    !name.is_empty()
        && name.len() <= 32
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+-=[]\\;',./`".contains(c))
}

#[cfg(test)]
mod tests;
