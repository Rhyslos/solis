//! What game code sees of the input in one tick.

use glam::Vec2;
use std::collections::BTreeMap;

/// One action's state in a tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ActionState {
    /// Down now (an axis: not at rest).
    pub held: bool,
    /// Went down since the last tick.
    pub pressed: bool,
    /// Came up since the last tick.
    pub released: bool,
    /// An axis's two numbers: across (x) and up (y).
    pub axis: Vec2,
}

impl ActionState {
    /// Whether nothing is happening (nothing to record).
    pub fn is_idle(&self) -> bool {
        !self.held && !self.pressed && !self.released && self.axis == Vec2::ZERO
    }
}

/// Every action's state in one tick, by name; actions doing nothing are
/// left out.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Frame {
    /// The busy actions.
    pub actions: BTreeMap<String, ActionState>,
}

impl Frame {
    /// `name`'s state (at rest if it isn't busy, or isn't an action).
    pub fn get(&self, name: &str) -> ActionState {
        self.actions.get(name).copied().unwrap_or_default()
    }

    /// Feeds a checksum (for recordings and determinism checks).
    pub fn checksum(&self, sum: &mut reflect::Checksum) {
        sum.u64(self.actions.len() as u64);
        for (name, s) in &self.actions {
            sum.bytes(name.as_bytes());
            sum.u64(u64::from(s.held) | u64::from(s.pressed) << 1 | u64::from(s.released) << 2);
            sum.f32(s.axis.x);
            sum.f32(s.axis.y);
        }
    }
}
