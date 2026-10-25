//! Making running components from the area's values, and giving running
//! ones what changed (see [`crate::Runtime::sync_many`]). The natives are
//! lent by the caller, once for a whole batch.

use crate::code::CodeProblem;
use crate::convert::to_lua;
use crate::lua_components::{Field, LuaClass, lua_problem, make};
use crate::luau::Luau;
use crate::lust_program::{LustProgram, located};
use crate::runtime::Failure;
use crate::rust_component::RustKind;
use crate::ticking::{Instance, Running, RunningMap, caught};
use mlua::Table;
use reflect::{ComponentData, EntityId};
use std::collections::{BTreeMap, BTreeSet};

/// The loaded code, as syncing needs it.
pub(crate) struct Syncing<'a> {
    pub(crate) luau: &'a Luau,
    pub(crate) lust: Option<&'a LustProgram>,
    pub(crate) lua_classes: &'a BTreeMap<String, LuaClass>,
    pub(crate) rust: &'a BTreeMap<String, RustKind>,
    pub(crate) left_out: &'a BTreeSet<String>,
}

impl Syncing<'_> {
    /// Makes `entity`'s component `name` run with the area's `data`, or
    /// gives it the fields that changed. Nothing for a component there's
    /// no code for, or one left out.
    pub(crate) fn sync(
        &self,
        running: &mut RunningMap,
        entity: EntityId,
        name: &str,
        data: &ComponentData,
    ) -> Result<(), Failure> {
        let fail = |at: CodeProblem| Failure {
            entity: Some(entity),
            component: name.to_string(),
            at,
            stopped: true,
        };
        let key = (entity, name.to_string());
        if let Some(kind) = self.rust.get(name) {
            match running.get_mut(&key) {
                // Nothing changed: nothing to do (no copy, no call).
                Some(already) if already.seen == *data => {}
                Some(already) => {
                    let mut changed = ComponentData::default();
                    for field in &kind.schema.fields {
                        if let Some(now) = data.get(&field.name)
                            && already.seen.get(&field.name) != Some(now)
                        {
                            changed.set(&field.name, now.clone());
                        }
                    }
                    already.seen = data.clone();
                    if let Instance::Rust(component) = &mut already.instance
                        && !already.stopped
                        && !changed.fields.is_empty()
                        && let Err(at) = caught(|| component.set(&changed))
                    {
                        already.stopped = true;
                        return Err(fail(at));
                    }
                }
                None => {
                    let made = caught(|| (kind.make)(data)).map_err(fail)?;
                    running.insert(key, Running::new(Instance::Rust(made), data));
                }
            }
            return Ok(());
        }
        if self.left_out.contains(name) {
            return Ok(());
        }
        let lua_class = self.lua_classes.get(name);
        let locate = |err: mlua::Error| match (lua_class, self.lust) {
            (None, Some(program)) => located(&program.chunk, &err),
            (class, _) => lua_problem(class.map_or("", |c| &c.path), &err),
        };
        let Some(fields) = lua_class
            .map(|c| &c.fields)
            .or_else(|| self.lust.and_then(|p| p.fields.get(name)))
        else {
            return Ok(());
        };
        if let Some(already) = running.get_mut(&key) {
            if already.seen == *data {
                return Ok(());
            }
            if let Instance::Lust(table) | Instance::Lua(table) = &already.instance {
                for field in fields {
                    let now = data.get(&field.name);
                    if now != already.seen.get(&field.name)
                        && let Some(value) =
                            now.and_then(|v| to_lua(&self.luau.lua, &field.kind, v))
                    {
                        // Raw: the field is the component's own, and its
                        // metatable is game code's.
                        table
                            .raw_set(field.key.as_str(), value)
                            .map_err(|e| fail(locate(e)))?;
                    }
                }
            }
            already.seen = data.clone();
            return Ok(());
        }
        let instance = match (lua_class, self.lust) {
            (Some(class), _) => Instance::Lua(
                make(&self.luau.lua, class, entity, data).map_err(|e| fail(locate(e)))?,
            ),
            (None, Some(program)) => Instance::Lust(
                self.make_lust(program, fields, entity, name, data)
                    .map_err(|e| fail(locate(e)))?,
            ),
            (None, None) => return Ok(()),
        };
        running.insert(key, Running::new(instance, data));
        Ok(())
    }

    /// A new Lust component (Lust's `new`, its start values), with the
    /// area's values where they fit. The natives are lent already.
    fn make_lust(
        &self,
        program: &LustProgram,
        fields: &[Field],
        entity: EntityId,
        name: &str,
        data: &ComponentData,
    ) -> mlua::Result<Table> {
        let new: mlua::Function = program.api.raw_get("new")?;
        let table: Table = self
            .luau
            .clock
            .limited(|| new.call((name, entity.to_string())))?;
        for field in fields {
            if let Some(value) = data
                .get(&field.name)
                .and_then(|v| to_lua(&self.luau.lua, &field.kind, v))
            {
                table.raw_set(field.key.as_str(), value)?;
            }
        }
        Ok(table)
    }
}
