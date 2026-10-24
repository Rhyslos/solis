//! Components written in Rust, by a game (decision 0003): they run in the
//! same ticks as Lust's, in the same order (entities by ID, components by
//! name), and send and handle the same events, so the three languages
//! work together on one entity.

use crate::events::{Event, Queue};
use crate::host::Host;
use reflect::{ComponentData, ComponentSchema, EntityId, Value};

/// A component written in Rust. Its fields are a `reflect` schema, so
/// Properties shows them like any other component's.
pub trait RustComponent {
    /// Runs once a tick, `dt` seconds long.
    fn update(&mut self, _cx: &mut Context<'_>, _dt: f64) {}

    /// Handles an event sent to its entity. Returns whether it had a
    /// reaction to it.
    fn handle(&mut self, _cx: &mut Context<'_>, _event: &Event) -> bool {
        false
    }

    /// Its fields' values now, by name.
    fn values(&self) -> ComponentData;

    /// Takes in fields the editor changed while the game runs (only those
    /// in `data`); the rest stay as they are.
    fn set(&mut self, data: &ComponentData);
}

/// A kind of Rust component: its schema, and how to make one from the
/// area's values (a missing or wrong value takes the starting one).
#[derive(Clone)]
pub struct RustKind {
    /// Its name, fields and starting values.
    pub schema: ComponentSchema,
    /// Makes one from the area's values.
    pub make: fn(&ComponentData) -> Box<dyn RustComponent>,
}

impl std::fmt::Debug for RustKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RustKind")
            .field("name", &self.schema.name)
            .finish_non_exhaustive()
    }
}

/// What a Rust component can reach while it runs: the entity it's on, the
/// engine (the same [`Host`] Lust's native methods use), and sending
/// events.
pub struct Context<'a> {
    me: EntityId,
    host: &'a mut dyn Host,
    queue: &'a Queue,
}

impl<'a> Context<'a> {
    pub(crate) fn new(me: EntityId, host: &'a mut dyn Host, queue: &'a Queue) -> Self {
        Self { me, host, queue }
    }

    /// The entity this component is on.
    pub fn me(&self) -> EntityId {
        self.me
    }

    /// The engine: positions, names, the Console.
    pub fn host(&mut self) -> &mut dyn Host {
        self.host
    }

    /// Sends the event `name` carrying `values` to `to` (nowhere for
    /// `None`). It's delivered after this tick's updates, in order.
    /// Returns whether it was sent: not when too many events are waiting
    /// already ([`crate::events::MAX_WAITING`]).
    pub fn send(&mut self, to: Option<EntityId>, name: &str, values: Vec<Value>) -> bool {
        let Some(to) = to else {
            return false;
        };
        self.queue
            .borrow_mut()
            .push(Event {
                to,
                name: name.to_string(),
                values,
            })
            .is_ok()
    }
}
