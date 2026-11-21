//! Reading another entity's component (Fixes 2, M5;
//! [decision 0135](../../../docs/decisions/0135-fixes-2-game-code-calls.md)):
//! `other:knight()` in Lust, `solis.entity.knight(other)` in Lua,
//! `Host::reading` in Rust, each a copy of the component as the last tick
//! left it.
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
//!
//! Spawning with values (M7) is here too: each `KnightValues` method notes
//! a field's value on the values object ([`VALUES_KEY`]), and
//! `World:spawnWith` reads them back as `reflect` values ([`given`]).

use crate::convert::{entity_of, from_lua, to_lua};
use crate::lua_components::Field;
use mlua::{Function, Lua, Table, Value as LuaValue};
use reflect::{ComponentData, EntityId, IdMap, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::Arc;

/// Where a values object keeps what its methods were given: a table of
/// field names and values.
pub(crate) const VALUES_KEY: &str = "__given";

/// Where a values object keeps which component its values are for.
pub(crate) const VALUES_OF: &str = "__of";

/// The components game code can read and spawn with, by name: each one's
/// fields as game code's tables hold them (key, name, kind), in one order
/// every core shares (they all run the same code).
pub(crate) type Layouts = BTreeMap<String, Vec<Field>>;

/// The layouts of a Luau's components, kept as its app data for the
/// natives that need them.
pub(crate) struct Kept(pub(crate) Rc<Layouts>);

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

    /// `entity`'s `kind` as Rust reads it.
    pub fn data(&self, kind: &str, entity: EntityId) -> Option<ComponentData> {
        let (names, row) = self.row(kind, entity)?;
        let mut data = ComponentData::default();
        for (name, value) in names.iter().zip(row) {
            if let Some(value) = value {
                data.set(name, value.clone());
            }
        }
        Some(data)
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
    if found == 0 {
        format!(
            "World:the(Component.{kind}) wants the one entity with a {kind}, and there's none \
             in the level"
        )
    } else {
        format!(
            "World:the(Component.{kind}) wants the one entity with a {kind}, and there are \
             {found}: give it to one only, or find them with tags"
        )
    }
}

/// The component kinds the game's code reads or asks `World:the` for, as
/// far as its text shows: in the Lust, each call of a reading method
/// (`_E_knight(`) and each `Component.Quest` written; in Lua, each
/// `.knight(` and each `the("Quest")`. Also every kind a Rust component
/// says it reads. Only these are copied each tick, so a game that reads
/// none pays nothing.
pub(crate) fn kinds_read(
    lust: Option<&str>,
    lust_files: &[(String, String)],
    lua: &[(String, String)],
    readable: &[(String, String)],
    rust: &[String],
) -> Vec<String> {
    let mut kinds: BTreeSet<String> = rust.iter().cloned().collect();
    for (kind, reader) in readable {
        let called = lust.is_some_and(|code| code.contains(&format!("_E_{reader}(")));
        let named = lust_files
            .iter()
            .filter(|(path, _)| !crate::describe::in_engine_folder(path))
            .any(|(_, text)| {
                text.contains(&format!("{}.{kind}", crate::component_code::STATE))
            });
        if called || named {
            kinds.insert(kind.clone());
        }
    }
    kinds.extend(lua_kinds(lua));
    kinds.into_iter().collect()
}

/// The kinds Lua's text reads (`solis.entity.knight(e)` reads `Knight`,
/// matched without regard to its first letter's case) or asks `the` for.
fn lua_kinds(lua: &[(String, String)]) -> BTreeSet<String> {
    let mut kinds = BTreeSet::new();
    for (_, text) in lua {
        for (at, _) in text.match_indices("entity.") {
            let name: String = text[at + "entity.".len()..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            let rest = &text[at + "entity.".len() + name.len()..];
            if rest.starts_with('(') && !name.is_empty() && kinds.len() < 4096 {
                kinds.insert(upper_first(&name));
            }
        }
        for opening in ["the(\"", "the('"] {
            for (at, _) in text.match_indices(opening) {
                let name: String = text[at + opening.len()..]
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() && kinds.len() < 4096 {
                    kinds.insert(name);
                }
            }
        }
    }
    kinds
}

/// `name` with its first letter a capital (`knight` gives `Knight`).
fn upper_first(name: &str) -> String {
    let mut chars = name.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_ascii_uppercase().to_string() + chars.as_str()
    })
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
    /// The values objects' methods (`KnightValues_keys`), made once for
    /// the layouts.
    setters: Option<Table>,
}

impl Lent {
    /// Takes the components' layouts of newly loaded code.
    pub(crate) fn set_layouts(&mut self, layouts: Rc<Layouts>) {
        self.layouts = layouts;
        self.from = None;
        self.natives = None;
        self.setters = None;
    }

    /// Puts the reading natives into `natives` for `readings` (none while
    /// no game runs: every reading is then nothing), and the values
    /// objects' methods.
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
        if self.setters.is_none() {
            let made = lua.create_table()?;
            for (kind, fields) in self.layouts.iter() {
                for field in fields {
                    made.raw_set(
                        format!("{kind}Values_{}", field.name),
                        setter(lua, kind, &field.name)?,
                    )?;
                }
            }
            self.setters = Some(made);
        }
        for made in [&self.natives, &self.setters].into_iter().flatten() {
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
                "{reader}() can't be answered: the game's code wasn't seen reading {kind}. Write \
                 the call out (solis.entity.{reader}(e) in Lua), or list {kind} in a Rust \
                 component's reads"
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

/// `<Kind>Values_<field>`: notes the value on the values object.
fn setter(lua: &Lua, kind: &str, field: &str) -> mlua::Result<Function> {
    let kind = kind.to_string();
    let field = field.to_string();
    lua.create_function(move |lua, (this, value): (Table, LuaValue)| {
        let given = match this.raw_get::<LuaValue>(VALUES_KEY)? {
            LuaValue::Table(given) => given,
            _ => {
                let given = lua.create_table()?;
                this.raw_set(VALUES_KEY, &given)?;
                given
            }
        };
        this.raw_set(VALUES_OF, kind.as_str())?;
        given.raw_set(field.as_str(), value)?;
        Ok(())
    })
}

/// The values `World:spawnWith` was given, as `reflect` values by
/// component: from Lust, a list of values objects; from Lua, a table of
/// tables by component name (`{ Pickup = { item = "KeyRed" } }`). A
/// component or field the game hasn't, or a value of the wrong kind, says
/// why.
pub(crate) fn given(
    lua: &Lua,
    values: &LuaValue,
) -> Result<crate::game::SpawnValues, String> {
    let layouts = lua
        .app_data_ref::<Kept>()
        .map(|kept| Rc::clone(&kept.0))
        .unwrap_or_default();
    let LuaValue::Table(values) = values else {
        return Ok(Vec::new());
    };
    let mut out: crate::game::SpawnValues = Vec::new();
    let mut add = |kind: &str, fields: &Table| -> Result<(), String> {
        let Some(layout) = layouts.get(kind) else {
            return Err(format!("the game has no component called {kind}"));
        };
        let mut data = ComponentData::default();
        for pair in fields.pairs::<String, LuaValue>().take(1024) {
            let (name, value) = pair.map_err(|_| format!("{kind}'s values are named by text"))?;
            let Some(field) = layout.iter().find(|f| f.name == name) else {
                return Err(format!("{kind} has no field called {name}"));
            };
            let Some(value) = from_lua(&field.kind, &value) else {
                return Err(format!("{kind}'s {name} can't hold that value"));
            };
            data.set(&name, value);
        }
        match out.iter_mut().find(|(k, _)| k == kind) {
            Some((_, kept)) => kept.fields.extend(data.fields),
            None => out.push((kind.to_string(), data)),
        }
        Ok(())
    };
    let listed = values.raw_len() > 0;
    if listed {
        for item in values.sequence_values::<LuaValue>().take(256) {
            let Ok(LuaValue::Table(item)) = item else {
                continue;
            };
            let (Ok(LuaValue::String(kind)), Ok(LuaValue::Table(fields))) = (
                item.raw_get::<LuaValue>(VALUES_OF),
                item.raw_get::<LuaValue>(VALUES_KEY),
            ) else {
                continue;
            };
            let kind = kind.to_str().map_err(|e| e.to_string())?.to_string();
            add(&kind, &fields)?;
        }
    } else {
        for pair in values.pairs::<String, Table>().take(256) {
            let (kind, fields) = pair.map_err(|_| {
                "spawnWith wants its values as a table of tables by component name, such as \
                 { Pickup = { item = \"KeyRed\" } }"
                    .to_string()
            })?;
            add(&kind, &fields)?;
        }
    }
    let _ = lua;
    Ok(out)
}

#[cfg(test)]
#[path = "readings_tests.rs"]
mod tests;
