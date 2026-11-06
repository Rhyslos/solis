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

/// An event a screen's button sent (decision 0088): it reaches the game
/// in the next tick, with the keys pressed, and recordings keep it.
#[derive(Debug, Clone, PartialEq)]
pub struct Sent {
    /// The entity it's for.
    pub to: reflect::EntityId,
    /// The event's name, as declared (`Resumed`).
    pub name: String,
    /// What it carries, in the declared order.
    pub values: Vec<reflect::Value>,
}

impl Sent {
    /// Whether a recording keeps it as it is (and reads it back the same):
    /// a name of letters, digits and `_` at most [`MAX_SENT_NAME`] long, at
    /// most [`MAX_SENT_VALUES`] values, finite numbers, texts of at most
    /// [`MAX_SENT_TEXT`] bytes, lists at most 4 deep, and no assets.
    pub fn recordable(&self) -> bool {
        fn value(v: &reflect::Value, depth: usize) -> bool {
            match v {
                reflect::Value::Float(f) => f.is_finite(),
                reflect::Value::Text(t) => t.len() <= MAX_SENT_TEXT,
                reflect::Value::Asset(_) => false,
                reflect::Value::List(items) => {
                    depth < 4
                        && items.len() <= MAX_SENT_VALUES
                        && items.iter().all(|i| value(i, depth + 1))
                }
                _ => true,
            }
        }
        !self.name.is_empty()
            && self.name.len() <= MAX_SENT_NAME
            && self
                .name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            && self.values.len() <= MAX_SENT_VALUES
            && self.values.iter().all(|v| value(v, 0))
    }
}

/// The longest text an event sent from a screen carries, in bytes.
pub const MAX_SENT_TEXT: usize = 4096;

/// The most events screens send in one tick.
pub const MAX_SENT: usize = 64;
/// The longest event name kept.
pub const MAX_SENT_NAME: usize = 64;
/// The most values one event carries.
pub const MAX_SENT_VALUES: usize = 32;

/// Every action's state in one tick, by name; actions doing nothing are
/// left out. And what screens sent.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Frame {
    /// The busy actions.
    pub actions: BTreeMap<String, ActionState>,
    /// What the screens' buttons sent since the last tick (decision 0088).
    pub sent: Vec<Sent>,
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
        // Only when there are any, so older recordings' sums stand.
        if !self.sent.is_empty() {
            sum.u64(self.sent.len() as u64);
            for sent in &self.sent {
                sum.entity(sent.to);
                sum.bytes(sent.name.as_bytes());
                let mut data = reflect::ComponentData::default();
                for (i, v) in sent.values.iter().enumerate() {
                    data.set(&i.to_string(), v.clone());
                }
                sum.data(&data);
            }
        }
    }
}
