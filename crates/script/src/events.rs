//! Events between components (LUST.md §21, "Events"): `send` queues an
//! event for an entity, and Solis delivers the queue after each tick's
//! updates, in order, to each component on the entity in order of their
//! names. Events sent while delivering join the end of the same tick's
//! queue, so a game runs the same way every time. Lust, Rust and (from M5)
//! Lua components send and handle the same events.

use crate::convert::{from_lua, guessed, guessed_to_lua, kind_of, to_lua};
use mlua::{Lua, Table, Value as LuaValue};
use reflect::{EntityId, FieldKind, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

/// The most events delivered in one tick. Past it, the rest of the tick's
/// queue is dropped with a message: components that keep sending to each
/// other would otherwise never let the tick end.
pub const MAX_DELIVERIES: usize = 10_000;

/// The most values one event carries.
const MAX_VALUES: usize = 64;

/// The most events waiting at once.
pub const MAX_WAITING: usize = 10_000;

/// The most text (names and text values) the waiting events may hold
/// together: game code's text is copied out of Luau, past its memory
/// limit, so this keeps a huge string sent over and over from filling
/// the editor's memory.
pub const MAX_WAITING_BYTES: usize = 16 << 20;

/// The longest name an event may have.
const MAX_NAME_BYTES: usize = 200;

/// An event on its way to an entity.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// The entity it's for.
    pub to: EntityId,
    /// Its name, as declared (`Hit`).
    pub name: String,
    /// What it carries, in the declared order.
    pub values: Vec<Value>,
}

/// Events waiting to be delivered, within [`MAX_WAITING`] and
/// [`MAX_WAITING_BYTES`].
#[derive(Debug, Default)]
pub(crate) struct Waiting {
    events: VecDeque<Event>,
    /// The text the events hold.
    bytes: usize,
}

impl Waiting {
    /// Puts `event` at the end of the queue, or says why it can't wait.
    pub(crate) fn push(&mut self, event: Event) -> Result<(), String> {
        if self.events.len() >= MAX_WAITING {
            return Err(format!(
                "more than {MAX_WAITING} events are waiting to be delivered; do components keep \
                 sending events to each other?"
            ));
        }
        let bytes = size(&event);
        if self.bytes + bytes > MAX_WAITING_BYTES {
            return Err(format!(
                "the events waiting hold more than {} MB of text together",
                MAX_WAITING_BYTES >> 20
            ));
        }
        self.bytes += bytes;
        self.events.push_back(event);
        Ok(())
    }

    /// The next event, out of the queue.
    pub(crate) fn pop(&mut self) -> Option<Event> {
        let event = self.events.pop_front()?;
        self.bytes -= size(&event);
        Some(event)
    }

    /// How many events are waiting.
    pub(crate) fn len(&self) -> usize {
        self.events.len()
    }

    /// Drops every waiting event.
    pub(crate) fn clear(&mut self) {
        self.events.clear();
        self.bytes = 0;
    }
}

/// The text an event holds.
fn size(event: &Event) -> usize {
    event.name.len()
        + event
            .values
            .iter()
            .map(|v| match v {
                Value::Text(t) => t.len(),
                _ => 0,
            })
            .sum::<usize>()
}

/// The tick's queue, shared by Lust's `send_event` and Rust components.
pub(crate) type Queue = Rc<RefCell<Waiting>>;

/// What each event the Lust declares carries: the kinds of its values,
/// in order, by event name.
pub(crate) type EventKinds = BTreeMap<String, Vec<FieldKind>>;

/// Reads the events the program declares (`api.events`).
pub(crate) fn read_events(api: &Table) -> mlua::Result<EventKinds> {
    let mut kinds = EventKinds::new();
    let events: Table = api.get("events")?;
    for pair in events.pairs::<String, Table>() {
        let (name, fields) = pair?;
        let mut list = Vec::new();
        for entry in fields.sequence_values::<Table>() {
            let entry = entry?;
            let kind_name: String = entry.get(3)?;
            let options: Vec<String> = match entry.get::<LuaValue>(4)? {
                LuaValue::Table(t) => t.sequence_values::<String>().collect::<Result<_, _>>()?,
                _ => Vec::new(),
            };
            list.push(kind_of(&kind_name, options).unwrap_or(FieldKind::Text));
        }
        kinds.insert(name, list);
    }
    Ok(kinds)
}

/// The Lua function Lust's `send` (and Lua's `solis.send`) calls:
/// `send_event(to, name, values)`, queuing the event with its values read
/// by the kinds the Lust declares for it (`kinds`, which the runtime
/// updates when code loads), or by their own kinds for an event no Lust
/// declares. An event to `nothing` goes nowhere.
pub(crate) fn sender(
    lua: &Lua,
    queue: &Queue,
    kinds: Rc<RefCell<Rc<EventKinds>>>,
) -> mlua::Result<mlua::Function> {
    let queue = Rc::clone(queue);
    lua.create_function(
        move |_, (to, name, values): (LuaValue, mlua::String, Table)| {
            let Some(to) = crate::convert::entity_of(&to) else {
                return Ok(());
            };
            if name.as_bytes().len() > MAX_NAME_BYTES {
                return Err(mlua::Error::runtime(format!(
                    "an event's name is at most {MAX_NAME_BYTES} letters long"
                )));
            }
            let name = name.to_str()?.to_string();
            // An event carries a handful of values; a damaged count can't ask
            // for a huge list.
            let count = values
                .raw_get::<Option<usize>>("n")?
                .unwrap_or(0)
                .min(MAX_VALUES);
            let kinds = Rc::clone(&kinds.borrow());
            let declared = kinds.get(&name);
            let mut read = Vec::with_capacity(count);
            for i in 1..=count {
                let raw: LuaValue = values.raw_get(i)?;
                let value = match declared.and_then(|k| k.get(i - 1)) {
                    Some(kind) => from_lua(kind, &raw),
                    None => guessed(&raw),
                };
                read.push(value.unwrap_or(Value::Entity(None)));
            }
            queue
                .borrow_mut()
                .push(Event {
                    to,
                    name,
                    values: read,
                })
                .map_err(mlua::Error::runtime)
        },
    )
}

/// An event's values as Lust's `handle` takes them: a list with its count
/// in `n`, each value as its declared kind. `None` if the values don't fit
/// what the event declares.
pub(crate) fn values_table(lua: &Lua, kinds: &[FieldKind], values: &[Value]) -> Option<Table> {
    if kinds.len() != values.len() {
        return None;
    }
    let list = lua.create_table().ok()?;
    for (i, (kind, value)) in kinds.iter().zip(values).enumerate() {
        let converted = to_lua(lua, kind, value)?;
        list.raw_set(i + 1, converted).ok()?;
    }
    list.raw_set("n", values.len()).ok()?;
    Some(list)
}

/// An event's values as arguments for a Lua component's `on_` function:
/// by the kinds the Lust declares for the event, or by their own kinds.
/// `None` if they don't fit what's declared.
pub(crate) fn arguments(
    lua: &Lua,
    kinds: Option<&[FieldKind]>,
    values: &[Value],
) -> Option<mlua::MultiValue> {
    if kinds.is_some_and(|k| k.len() != values.len()) {
        return None;
    }
    let mut out = Vec::with_capacity(values.len());
    for (i, value) in values.iter().enumerate() {
        out.push(match kinds {
            Some(kinds) => to_lua(lua, &kinds[i], value)?,
            None => guessed_to_lua(lua, value)?,
        });
    }
    Some(out.into_iter().collect())
}
