//! What the keyboard, mouse and gamepad are doing, sampled into a
//! [`Frame`] once per tick.

use crate::{Action, ActionKind, ActionState, Bindings, DEAD_ZONE, Frame};
use glam::Vec2;
use std::collections::{BTreeMap, BTreeSet};

/// A gamepad's state: its buttons down (by [`crate::PAD_BUTTONS`] name)
/// and its sticks (x right, y up, each -1 to 1).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pad {
    /// The buttons down.
    pub buttons: BTreeSet<String>,
    /// The left stick.
    pub left: Vec2,
    /// The right stick.
    pub right: Vec2,
}

/// The input devices as they are, fed as things happen, and what changed
/// since the last tick.
#[derive(Debug, Clone, Default)]
pub struct Devices {
    /// Inputs down now (keys, `Mouse …`, `Pad …`).
    down: BTreeSet<String>,
    /// Inputs that went down since the last tick (even if up again).
    went_down: BTreeSet<String>,
    /// Mouse movement since the last tick, in pixels (y down).
    motion: Vec2,
    /// Wheel notches turned since the last tick (up is positive).
    wheel: f32,
    /// The gamepad, if one is connected.
    pad: Pad,
    /// Which actions were held at the last tick, for presses and
    /// releases.
    was_held: BTreeSet<String>,
}

impl Devices {
    /// Nothing down.
    pub fn new() -> Self {
        Self::default()
    }

    /// A key or mouse button (by its input name) went down or up.
    pub fn set(&mut self, input: &str, down: bool) {
        if down {
            if self.down.insert(input.to_string()) {
                self.went_down.insert(input.to_string());
            }
        } else {
            self.down.remove(input);
        }
    }

    /// The mouse moved by `delta` pixels (y down, as screens count).
    pub fn mouse_moved(&mut self, delta: Vec2) {
        if delta.is_finite() {
            self.motion += delta;
        }
    }

    /// The mouse wheel turned `notches` (up is positive; a trackpad gives
    /// parts of one).
    pub fn wheel_turned(&mut self, notches: f32) {
        if notches.is_finite() {
            self.wheel = (self.wheel + notches).clamp(-1000.0, 1000.0);
        }
    }

    /// The gamepad is now `pad` (none: disconnected).
    pub fn set_pad(&mut self, pad: Option<Pad>) {
        let pad = pad.unwrap_or_default();
        for button in &pad.buttons {
            if !self.pad.buttons.contains(button) {
                self.went_down.insert(format!("Pad {button}"));
            }
        }
        self.pad = pad;
    }

    /// Lets everything go (the game gave the input back): every input up,
    /// no movement.
    pub fn release_all(&mut self) {
        self.down.clear();
        self.motion = Vec2::ZERO;
        self.wheel = 0.0;
        self.pad = Pad::default();
    }

    /// Whether `input` is down now: a combination when its key and all
    /// its modifiers are, a key on its own when it's down and not
    /// `shadowed`, a wheel direction when the wheel turned that way.
    fn is_down(&self, input: &str, shadowed: &BTreeSet<&str>) -> bool {
        if let Some(button) = input.strip_prefix("Pad ") {
            // Triggers count as buttons when pulled; the app's reading of
            // the pad says so.
            return self.pad.buttons.contains(button);
        }
        match input {
            "Mouse WheelUp" => return self.wheel > 0.0,
            "Mouse WheelDown" => return self.wheel < 0.0,
            _ => {}
        }
        match crate::split_combination(input) {
            Some((mods, key)) if !mods.is_empty() => {
                self.down.contains(key) && mods.iter().all(|m| self.down.contains(*m))
            }
            _ => self.down.contains(input) && !shadowed.contains(input),
        }
    }

    /// Whether `input` went down since the last tick (a combination: its
    /// key, with its modifiers held now).
    fn went_down(&self, input: &str, shadowed: &BTreeSet<&str>) -> bool {
        match input {
            "Mouse WheelUp" => return self.wheel > 0.0,
            "Mouse WheelDown" => return self.wheel < 0.0,
            _ => {}
        }
        match crate::split_combination(input) {
            Some((mods, key)) if !mods.is_empty() => {
                self.went_down.contains(key) && mods.iter().all(|m| self.down.contains(*m))
            }
            _ => self.went_down.contains(input) && !shadowed.contains(input),
        }
    }

    /// The keys that don't count on their own now: those some binding
    /// uses with modifiers that are all held.
    fn shadowed<'a>(&self, bindings: &'a Bindings) -> BTreeSet<&'a str> {
        bindings
            .actions
            .iter()
            .flat_map(Action::all_inputs)
            .filter_map(|i| crate::split_combination(i))
            .filter(|(mods, _)| !mods.is_empty() && mods.iter().all(|m| self.down.contains(*m)))
            .map(|(_, key)| key)
            .collect()
    }

    /// A stick's position past the dead zone, rescaled so it starts at 0
    /// there.
    fn stick(v: Vec2) -> Vec2 {
        let length = v.length();
        if !length.is_finite() || length < DEAD_ZONE {
            return Vec2::ZERO;
        }
        v / length * ((length.min(1.0) - DEAD_ZONE) / (1.0 - DEAD_ZONE))
    }

    /// One action's state now, given whether it was held at the last
    /// tick, for a tick `dt` seconds long.
    fn state_of(
        &self,
        action: &Action,
        was_held: bool,
        dt: f32,
        shadowed: &BTreeSet<&str>,
    ) -> ActionState {
        let down = |list: &[String]| list.iter().any(|i| self.is_down(i, shadowed));
        let went = action.all_inputs().any(|i| self.went_down(i, shadowed));
        let (held, axis) = match action.kind {
            ActionKind::Button => (down(&action.inputs), Vec2::ZERO),
            ActionKind::Axis => {
                let keys = Vec2::new(
                    f32::from(u8::from(down(&action.right)))
                        - f32::from(u8::from(down(&action.left))),
                    f32::from(u8::from(down(&action.up))) - f32::from(u8::from(down(&action.down))),
                );
                let mut stick = Vec2::ZERO;
                for input in &action.inputs {
                    match input.as_str() {
                        "Pad LeftStick" => stick += Self::stick(self.pad.left),
                        "Pad RightStick" => stick += Self::stick(self.pad.right),
                        _ => {}
                    }
                }
                let lean = (keys + stick).clamp_length_max(1.0);
                let axis = if action.is_motion() {
                    // How far to go this tick: the mouse's pixels and the
                    // wheel's notches, and sticks and keys at their speed
                    // for the tick.
                    let mut mouse = Vec2::ZERO;
                    for input in &action.inputs {
                        match input.as_str() {
                            crate::MOUSE_MOTION => {
                                mouse += Vec2::new(self.motion.x, -self.motion.y)
                            }
                            crate::MOUSE_WHEEL => mouse.y += self.wheel,
                            _ => {}
                        }
                    }
                    mouse * action.mouse_speed + lean * action.pad_speed * dt
                } else {
                    lean
                };
                let axis = if axis.is_finite() { axis } else { Vec2::ZERO };
                (axis != Vec2::ZERO, axis)
            }
        };
        ActionState {
            held,
            pressed: (went || !was_held) && held || (went && !held),
            released: (was_held || went) && !held,
            axis,
        }
    }

    /// Every action's state for one tick `dt` seconds long, by
    /// `bindings`; then what changed since is forgotten, ready for the
    /// next tick.
    pub fn sample(&mut self, bindings: &Bindings, dt: f32) -> Frame {
        let mut actions = BTreeMap::new();
        let mut held = BTreeSet::new();
        let shadowed = self.shadowed(bindings);
        for action in &bindings.actions {
            let state = self.state_of(action, self.was_held.contains(&action.name), dt, &shadowed);
            if state.held {
                held.insert(action.name.clone());
            }
            if !state.is_idle() {
                actions.insert(action.name.clone(), state);
            }
        }
        self.was_held = held;
        self.went_down.clear();
        self.motion = Vec2::ZERO;
        self.wheel = 0.0;
        Frame { actions }
    }
}
