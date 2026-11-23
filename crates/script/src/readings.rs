//! Reading another entity's component (Fixes 2, M5;
//! [the manual](../../../docs/manual/game-code.md#reading-other-entities)):
//! `other:knight()` in Lust, a copy of the component as the last tick
//! left it (new calls are Lust only, decision 0135).
//!
//! A component lives in the Luau of the core its entity falls to, and
//! every core runs at once, so no core can look into another's. Instead,
//! as each tick starts, every core copies out its components of the kinds
//! game code reads (found in the code: [`kinds_read`]) into one
//! [`Readings`], which every core then reads for the whole tick
//! ([decision 0033](../../../docs/decisions/0033-game-code-sees-the-start-of-the-tick.md)):
//! the same values on one core or sixteen, and never another core's
//! half-done work. A copy is made into a table once a tick on each core
//! and handed out read-only (`table.freeze`), so a Lua component writing
//! to one stops with Lua's error; Lust's own class for it has every field
//! `lock`ed, so there writing to one is a mistake before the game runs.
//!
//! The same copy answers `World:the(Component.Quest)`: the one entity with
//! that component.

use crate::convert::{entity_of, from_lua, to_lua};
use crate::lua_components::Field;
use mlua::{Function, Lua, Table, Value as LuaValue};
use reflect::{ComponentData, EntityId, IdMap, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::Arc;

/// The components game code can read, by name: each one's
/// fields as game code's tables hold them (key, name, kind), in one order
/// every core shares (they all run the same code).
pub(crate) type Layouts = BTreeMap<String, Vec<Field>>;

/// One kind's copies: its fields' names, and each entity's values in that
/// order (`None` where a value couldn't be read).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rows {
    names: Arc<[String]>,
    rows: IdMap<Vec<Option<Value>>>,
}

/// The components game code reads, as the tick started: made once a tick
/// from every core's and read by all of them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Readings {
    kinds: BTreeMap<String, Rows>,
}

/// One core's copies of one kind: its name, its fields' names, and each
/// entity's values.
pub(crate) type Copied = (String, Arc<[String]>, Vec<(EntityId, Vec<Option<Value>>)>);

impl Readings {
    /// The readings of `kinds` (each read, even with no copies), made from
    /// every core's copies.
    pub(crate) fn gathered(kinds: &[String], copies: impl IntoIterator<Item = Copied>) -> Self {
        let mut made = Self::default();
        for kind in kinds {
            made.kinds.insert(kind.clone(), Rows::default());
        }
        for (kind, names, rows) in copies {
            let Some(kept) = made.kinds.get_mut(&kind) else {
                continue;
            };
            if kept.names.is_empty() {
                kept.names = names;
            }
            kept.rows.extend(rows);
        }
        made
    }

    /// Whether `kind` is copied each tick (game code reads it).
    pub fn reads(&self, kind: &str) -> bool {
        self.kinds.contains_key(kind)
    }

    /// `entity`'s `kind`, as the tick started: its fields' names and
    /// values; none when it has none (or `kind` isn't copied).
    pub fn row(&self, kind: &str, entity: EntityId) -> Option<(&[String], &[Option<Value>])> {
        let rows = self.kinds.get(kind)?;
        rows.rows
            .get(&entity)
            .map(|row| (&*rows.names, row.as_slice()))
    }

    /// The one entity with `kind` as the tick started (`World:the`), or
    /// how many there were instead: 0, or more than 1.
    pub fn the(&self, kind: &str) -> Result<EntityId, usize> {
        let Some(rows) = self.kinds.get(kind) else {
            return Err(0);
        };
        match rows.rows.len() {
            1 => rows.rows.keys().next().copied().ok_or(0),
            n => Err(n),
        }
    }

    /// How many copies there are, of every kind.
    pub fn len(&self) -> usize {
        self.kinds.values().map(|r| r.rows.len()).sum()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// What `World:the` says when the level hasn't exactly one entity with
/// `kind`.
pub(crate) fn not_one(kind: &str, found: usize) -> String {
    // Named as Lua writes the call, as every native's mistakes are.
    if found == 0 {
        format!(
            "solis.World:the(which) wants the one entity with a {kind} (Component.{kind}), and \
             there's none in the level"
        )
    } else {
        format!(
            "solis.World:the(which) wants the one entity with a {kind} (Component.{kind}), and \
             there are {found}: give it to one only, or find them with tags"
        )
    }
}

/// The component kinds the game's Lust reads or asks `World:the` for, as
/// far as its text shows: each call of a reading method (`_E_knight(`)
/// and each `Component.Quest` written. Only these are copied each tick,
/// so a game that reads none pays nothing.
pub(crate) fn kinds_read(
    lust: Option<&str>,
    lust_files: &[(String, String)],
    readable: &[(String, String)],
) -> Vec<String> {
    let mut kinds = BTreeSet::new();
    for (kind, reader) in readable {
        let called = lust.is_some_and(|code| code.contains(&format!("_E_{reader}(")));
        let named = lust_files
            .iter()
            .filter(|(path, _)| !crate::describe::in_engine_folder(path))
            .any(|(_, text)| text.contains(&format!("{}.{kind}", crate::component_code::STATE)));
        if called || named {
            kinds.insert(kind.clone());
        }
    }
    kinds.into_iter().collect()
}

/// One core's copy of each of its components of `kinds`, its fields read
/// by `layouts`.
pub(crate) fn copy_row(table: &Table, fields: &[Field]) -> Vec<Option<Value>> {
    fields
        .iter()
        .map(|field| {
            table
                .raw_get::<LuaValue>(field.key.as_str())
                .ok()
                .and_then(|raw| from_lua(&field.kind, &raw))
        })
        .collect()
}

/// A Rust component's values in its layout's order.
pub(crate) fn rust_row(data: &ComponentData, fields: &[Field]) -> Vec<Option<Value>> {
    fields
        .iter()
        .map(|field| data.get(&field.name).cloned())
        .collect()
}

/// What a Luau lends for the readings it was last given: the reading
/// natives (`entity_knight`), made once for each tick's copies, each
/// keeping the tables it made so a copy is made once a tick on each core.
#[derive(Default)]
pub(crate) struct Lent {
    /// The components' layouts, from the code loaded now.
    layouts: Rc<Layouts>,
    /// The copies the natives were made for.
    from: Option<Arc<Readings>>,
    /// The natives, by key.
    natives: Option<Table>,
}

impl Lent {
    /// Takes the components' layouts of newly loaded code.
    pub(crate) fn set_layouts(&mut self, layouts: Rc<Layouts>) {
        self.layouts = layouts;
        self.from = None;
        self.natives = None;
    }

    /// Puts the reading natives into `natives` for `readings` (none while
    /// no game runs: every reading is then nothing).
    pub(crate) fn lend(
        &mut self,
        lua: &Lua,
        readings: Option<Arc<Readings>>,
        natives: &Table,
    ) -> mlua::Result<()> {
        let same = match (&self.from, &readings) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if !same || self.natives.is_none() {
            let made = lua.create_table()?;
            for (kind, fields) in self.layouts.iter() {
                let reader = crate::screens::reader_name(kind);
                let native = reader_native(lua, kind, &reader, fields, readings.clone())?;
                made.raw_set(format!("entity_{reader}"), native)?;
            }
            self.natives = Some(made);
            self.from = readings;
        }
        if let Some(made) = &self.natives {
            for pair in made.pairs::<LuaValue, LuaValue>() {
                let (key, native) = pair?;
                // The engine's own natives come first: a component named
                // like one of them (Position) isn't read this way.
                if natives.raw_get::<LuaValue>(key.clone())?.is_nil() {
                    natives.raw_set(key, native)?;
                }
            }
        }
        Ok(())
    }
}

/// `entity_<reader>`: the entity's `kind` as the tick started, as a
/// read-only table, or nil.
fn reader_native(
    lua: &Lua,
    kind: &str,
    reader: &str,
    fields: &[Field],
    readings: Option<Arc<Readings>>,
) -> mlua::Result<Function> {
    let kind = kind.to_string();
    let reader = reader.to_string();
    let fields: Vec<(String, reflect::FieldKind)> = fields
        .iter()
        .map(|f| (f.key.clone(), f.kind.clone()))
        .collect();
    let made = lua.create_table()?;
    lua.create_function(move |lua, which: LuaValue| {
        let Some(readings) = &readings else {
            return Ok(LuaValue::Nil);
        };
        if !readings.reads(&kind) {
            return Err(mlua::Error::runtime(format!(
                "{reader}() can't be answered: the game's Lust wasn't seen reading {kind}, so \
                 it isn't copied. Write the call out in Lust (e:{reader}())"
            )));
        }
        let Some(entity) = entity_of(&which) else {
            return Ok(LuaValue::Nil);
        };
        let key = entity.to_string();
        if let Ok(LuaValue::Table(kept)) = made.raw_get::<LuaValue>(key.as_str()) {
            return Ok(LuaValue::Table(kept));
        }
        let Some((_, row)) = readings.row(&kind, entity) else {
            return Ok(LuaValue::Nil);
        };
        let copy = lua.create_table_with_capacity(0, fields.len())?;
        for ((key, field_kind), value) in fields.iter().zip(row) {
            if let Some(value) = value.as_ref().and_then(|v| to_lua(lua, field_kind, v)) {
                copy.raw_set(key.as_str(), value)?;
            }
        }
        copy.set_readonly(true);
        made.raw_set(key, &copy)?;
        Ok(LuaValue::Table(copy))
    })
}

#[cfg(test)]
#[path = "readings_tests.rs"]
mod tests;
