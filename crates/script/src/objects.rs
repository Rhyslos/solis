//! Game-wide objects ([decision 0080](../../../docs/decisions/0080-objects-change-only-in-their-own-update.md)):
//! a Lust `object` its file passes (`pass Clock;`) is one thing for the
//! whole game: a clock, the money, the weather. Solis runs its
//! `update(float dt)` once a tick, objects in order of their names, before
//! any entity's components, and keeps its fields in save games.
//!
//! **An object changes only in its own update.** Components read it
//! (`Clock.hour`, `Clock:isNight()`) but writing one of its fields from
//! anywhere else stops with a message pointing at the line. That's what
//! lets the game run on many cores: each core has its own copy of the
//! game's code, objects run on the first, and their fields are copied to
//! the others before the components run, so every component everywhere
//! sees the same clock.
//!
//! How the rule is kept: once the Lust starts, each object's fields move
//! into a table of their own (its store), and the object is given a
//! metatable whose `__index` reads the store and whose `__newindex`
//! writes it only while the objects' updates run. The object keeps its
//! class's metamethods and names (`like` still knows it).
//!
//! Only fields of the kinds a component's are (`bool`, `int`, `float`,
//! `string`, a state, `vec3`, `rotation`, `entity`) are copied and saved.
//! Others (lists, maps, things) stay as each core's copy has them, so the
//! editor says so when an object has one.

use crate::convert::{from_lua, kind_of, label, to_lua};
use crate::lua_components::Field;
use crate::luau::Luau;
use mlua::{Function, Table, Value as LuaValue};
use reflect::{ComponentData, ComponentSchema, FieldInfo};
use std::cell::RefCell;
use std::collections::BTreeSet;

/// Wraps an object so its fields live in a store of their own, written
/// only while `guard.open`: `wrap(object, class, name, names, guard)`
/// gives back the store. `names` maps each field's key in Lua to its name
/// in Lust, for the message. [`Luau::new`] makes it, before any game code
/// runs, so the functions it keeps here are the libraries' real ones.
pub(crate) const WRAP: &str = r#"
local next, rawset, setmetatable, type, error, tostring = next, rawset, setmetatable, type, error, tostring
return function(o, class, name, names, guard)
  local store = {}
  for k, v in next, o do store[k] = v end
  for k in next, store do rawset(o, k, nil) end
  local mt = {}
  for k, v in next, class do
    if type(k) == "string" and k:sub(1, 2) == "__" then mt[k] = v end
  end
  mt.__index = function(_, k)
    local v = store[k]
    if v ~= nil then return v end
    return class[k]
  end
  mt.__newindex = function(_, k, v)
    if guard.open then
      store[k] = v
      return
    end
    local field = names[k] or tostring(k)
    error(name .. "." .. field .. " can't be changed here: an object changes only in its own "
      .. "update, so every part of the game sees it the same. Components can read it; change "
      .. field .. " in " .. name .. "'s update.", 2)
  end
  -- What it's like: its class (Lust's like walks __parent).
  mt.__parent = class
  setmetatable(mt, { __index = class })
  setmetatable(o, mt)
  return store
end
"#;

/// One of the game's objects.
pub(crate) struct GameObject {
    /// Its name in Lust (`Clock`).
    pub(crate) name: String,
    /// The object itself, as game code holds it.
    pub(crate) table: Table,
    /// Where its fields live.
    pub(crate) store: Table,
    /// Its `update`, if it has one.
    pub(crate) update: Option<Function>,
    /// Its fields that are copied and saved.
    pub(crate) fields: Vec<Field>,
    /// Its fields that aren't (lists, maps, things), by name.
    pub(crate) unkept: Vec<String>,
}

/// The game's objects, in name order: the order they tick in.
#[derive(Default)]
pub(crate) struct Objects {
    pub(crate) list: Vec<GameObject>,
    /// `{ open = … }`: whether objects may be changed now.
    guard: Option<Table>,
    /// Objects whose update failed: they stop until the code changes.
    pub(crate) stopped: RefCell<BTreeSet<String>>,
}

/// An object's values: by object name, its fields by name.
pub type ObjectValues = Vec<(String, ComponentData)>;

impl Objects {
    /// The objects the Lust's table `api` passes (`objects`,
    /// `objectNames`), each wrapped so it changes only in an update.
    pub(crate) fn read(luau: &Luau, api: &Table) -> mlua::Result<Self> {
        let Ok(LuaValue::Table(names)) = api.raw_get::<LuaValue>("objectNames") else {
            return Ok(Self::default());
        };
        let objects: Table = api.raw_get("objects")?;
        let wrap = &luau.wrap_object;
        let guard = luau.lua.create_table()?;
        guard.raw_set("open", false)?;
        let mut list = Vec::new();
        for name in names.sequence_values::<String>() {
            let name = name?;
            let table: Table = objects.raw_get(name.as_str())?;
            let Some(class) = table.metatable() else {
                continue;
            };
            let schema: Option<Table> = class.raw_get("__schema")?;
            let mut fields = Vec::new();
            let mut unkept = Vec::new();
            let keys = luau.lua.create_table()?;
            for entry in schema.iter().flat_map(|s| s.sequence_values::<Table>()) {
                let entry = entry?;
                let key: String = entry.get(1)?;
                let field_name: String = entry.get(2)?;
                let kind_name: String = entry.get(3)?;
                keys.raw_set(key.as_str(), field_name.as_str())?;
                let options: Vec<String> = match entry.get::<LuaValue>(4)? {
                    LuaValue::Table(list) => {
                        list.sequence_values::<String>().collect::<Result<_, _>>()?
                    }
                    _ => Vec::new(),
                };
                match kind_of(&kind_name, options) {
                    Some(kind) => fields.push(Field {
                        key,
                        name: field_name,
                        kind,
                    }),
                    None => unkept.push(field_name),
                }
            }
            let update = match class.get::<LuaValue>("update")? {
                LuaValue::Function(f) => Some(f),
                _ => None,
            };
            // Nothing to tick or keep: a helper, or one of the engine's
            // own objects (World), whose methods are the engine's.
            if update.is_none() && fields.is_empty() && unkept.is_empty() {
                continue;
            }
            let store: Table =
                wrap.call((table.clone(), class, name.as_str(), keys, guard.clone()))?;
            list.push(GameObject {
                name,
                table,
                store,
                update,
                fields,
                unkept,
            });
        }
        Ok(Self {
            list,
            guard: Some(guard),
            stopped: RefCell::default(),
        })
    }

    /// Runs `run` with the objects open to change.
    pub(crate) fn open<T>(&self, run: impl FnOnce() -> T) -> T {
        let set = |open: bool| {
            if let Some(guard) = &self.guard {
                let _ = guard.raw_set("open", open);
            }
        };
        set(true);
        let out = run();
        set(false);
        out
    }

    /// Every object's kept fields, as they are now.
    pub(crate) fn values(&self) -> ObjectValues {
        self.list
            .iter()
            .map(|o| {
                let mut data = ComponentData::default();
                for field in &o.fields {
                    if let Ok(raw) = o.store.raw_get::<LuaValue>(field.key.as_str())
                        && let Some(value) = from_lua(&field.kind, &raw)
                    {
                        data.set(&field.name, value);
                    }
                }
                (o.name.clone(), data)
            })
            .collect()
    }

    /// Gives the objects `values`: each field still there, of the same
    /// kind, takes its value; the rest stay as they are. Gives back notes
    /// on values left out, by object and field.
    pub(crate) fn set_values(&self, luau: &Luau, values: &ObjectValues) -> Vec<(String, String)> {
        let mut left_out = Vec::new();
        for (name, data) in values {
            let Some(object) = self.list.iter().find(|o| o.name == *name) else {
                continue;
            };
            for (field_name, value) in &data.fields {
                let Some(field) = object.fields.iter().find(|f| f.name == *field_name) else {
                    continue;
                };
                let given = to_lua(&luau.lua, &field.kind, value)
                    .is_some_and(|lua| object.store.raw_set(field.key.as_str(), lua).is_ok());
                if !given {
                    left_out.push((name.clone(), field_name.clone()));
                }
            }
        }
        left_out
    }

    /// The objects as the editor describes them: their kept fields, with
    /// their values now as the defaults.
    pub(crate) fn schemas(&self) -> Vec<ComponentSchema> {
        self.values()
            .into_iter()
            .zip(&self.list)
            .map(|((name, data), object)| ComponentSchema {
                help: format!("{name}, an object of the game's Lust."),
                fields: object
                    .fields
                    .iter()
                    .map(|f| {
                        let now = data
                            .fields
                            .get(&f.name)
                            .cloned()
                            .unwrap_or_else(|| crate::convert::empty(&f.kind));
                        FieldInfo::new(&f.name, &label(&f.name), f.kind.clone(), now)
                    })
                    .collect(),
                name,
            })
            .collect()
    }

    /// A checksum of every object's kept fields (and whether it stopped).
    pub(crate) fn checksum(&self) -> u64 {
        let stopped = self.stopped.borrow();
        let mut sum = reflect::Checksum::new();
        for (name, data) in self.values() {
            sum.bytes(name.as_bytes());
            sum.u64(u64::from(stopped.contains(&name)));
            sum.data(&data);
        }
        sum.finish()
    }
}
