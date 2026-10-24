//! Values between the editor's kinds (`reflect`) and how Lust keeps them
//! in Luau: numbers, text, Luau vectors for `vec3` (a table with `x`, `y`,
//! `z` is taken too), `{x, y, z, w}` rotations (quaternions), and entities
//! as their 13-letter text. Tables are read
//! and written raw, so game code's metatables never run here.

use glam::{Quat, Vec3};
use mlua::{Lua, Table, Value as LuaValue};
use reflect::{EntityId, FieldKind, Value};

/// A Lust field kind as the editor's.
pub(crate) fn kind_of(name: &str, options: Vec<String>) -> Option<FieldKind> {
    Some(match name {
        "bool" => FieldKind::Bool,
        "int" => FieldKind::Int,
        "float" => FieldKind::Float,
        "string" => FieldKind::Text,
        "state" => FieldKind::Choice(options),
        "vec3" => FieldKind::Vec3,
        "rotation" => FieldKind::Rotation,
        "entity" => FieldKind::Entity,
        _ => return None,
    })
}

/// A value of `kind` when a new component somehow has none.
pub(crate) fn empty(kind: &FieldKind) -> Value {
    match kind {
        FieldKind::Bool => Value::Bool(false),
        FieldKind::Int => Value::Int(0),
        FieldKind::Float => Value::Float(0.0),
        FieldKind::Choice(options) => Value::Text(options.first().cloned().unwrap_or_default()),
        FieldKind::Vec3 | FieldKind::Color => Value::floats(&[0.0, 0.0, 0.0]),
        FieldKind::Rotation => Value::floats(&[0.0, 0.0, 0.0, 1.0]),
        FieldKind::Entity => Value::Entity(None),
        FieldKind::Asset => Value::Asset(None),
        FieldKind::Text => Value::Text(String::new()),
    }
}

/// `lastShot` as the inspector shows it: "Last shot".
pub(crate) fn label(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c == '_' {
            out.push(' ');
        } else if i == 0 {
            out.extend(c.to_uppercase());
        } else if c.is_uppercase() {
            out.push(' ');
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// A Lua number as an `f64`.
pub(crate) fn number(value: &LuaValue) -> Option<f64> {
    match value {
        LuaValue::Integer(n) => Some(*n as f64),
        LuaValue::Number(n) if n.is_finite() => Some(*n),
        _ => None,
    }
}

/// An entity as Lust hands it over (its 13-letter text, or nil).
pub(crate) fn entity_of(value: &LuaValue) -> Option<EntityId> {
    match value {
        LuaValue::String(text) => text.to_str().ok()?.parse().ok(),
        _ => None,
    }
}

/// A vec3 as game code gives it: a Luau vector, or a table with `x`, `y`
/// and `z`.
pub(crate) fn vec3_of(value: &LuaValue) -> Option<Vec3> {
    let v = match value {
        LuaValue::Vector(v) => Vec3::new(v.x(), v.y(), v.z()),
        LuaValue::Table(table) => {
            let n = |k: &str| number(&table.raw_get::<LuaValue>(k).ok()?).map(|v| v as f32);
            Vec3::new(n("x")?, n("y")?, n("z")?)
        }
        _ => return None,
    };
    v.is_finite().then_some(v)
}

/// A vec3 as Lust keeps it in Luau: a Luau vector.
pub(crate) fn vec3_value(v: Vec3) -> LuaValue {
    LuaValue::Vector(mlua::Vector::new(v.x, v.y, v.z))
}

pub(crate) fn quat_table(lua: &Lua, q: Quat) -> mlua::Result<Table> {
    quat_table_with(lua, q, &QuatKeys::new(lua)?)
}

/// A rotation's four keys as Lua strings, made once for many rotations:
/// handing mlua a Rust `&str` key makes a Lua string, in a protected
/// call, every time, which cost a fifth of a tick turning ships.
pub(crate) struct QuatKeys([mlua::String; 4]);

impl QuatKeys {
    pub(crate) fn new(lua: &Lua) -> mlua::Result<Self> {
        Ok(Self([
            lua.create_string("x")?,
            lua.create_string("y")?,
            lua.create_string("z")?,
            lua.create_string("w")?,
        ]))
    }
}

/// [`quat_of`], with the keys made already.
pub(crate) fn quat_of_with(table: &Table, keys: &QuatKeys) -> Option<Quat> {
    let n = |k: &mlua::String| number(&table.raw_get::<LuaValue>(k).ok()?).map(|v| v as f32);
    let [x, y, z, w] = &keys.0;
    let q = Quat::from_xyzw(n(x)?, n(y)?, n(z)?, n(w)?);
    (q.is_finite() && q.length_squared() > 1e-12).then(|| q.normalize())
}

/// [`quat_table`], with the keys made already.
pub(crate) fn quat_table_with(lua: &Lua, q: Quat, keys: &QuatKeys) -> mlua::Result<Table> {
    let t = lua.create_table_with_capacity(0, 4)?;
    let [x, y, z, w] = &keys.0;
    t.raw_set(x, q.x)?;
    t.raw_set(y, q.y)?;
    t.raw_set(z, q.z)?;
    t.raw_set(w, q.w)?;
    Ok(t)
}

/// An editor value as Lust keeps it, if it's the right kind.
pub(crate) fn to_lua(lua: &Lua, kind: &FieldKind, value: &Value) -> Option<LuaValue> {
    if !kind.accepts(value) {
        return None;
    }
    Some(match (kind, value) {
        (FieldKind::Bool, Value::Bool(b)) => LuaValue::Boolean(*b),
        (FieldKind::Int, Value::Int(n)) => LuaValue::Number(*n as f64),
        (FieldKind::Float, v) => LuaValue::Number(v.as_f64()?),
        (FieldKind::Text | FieldKind::Choice(_), Value::Text(t)) => {
            LuaValue::String(lua.create_string(t).ok()?)
        }
        (FieldKind::Vec3, v) => {
            let [x, y, z] = v.as_floats::<3>()?;
            vec3_value(Vec3::new(x, y, z))
        }
        (FieldKind::Rotation, v) => {
            let [x, y, z, w] = v.as_floats::<4>()?;
            LuaValue::Table(quat_table(lua, Quat::from_xyzw(x, y, z, w)).ok()?)
        }
        (FieldKind::Entity, Value::Entity(None)) => LuaValue::Nil,
        (FieldKind::Entity, Value::Entity(Some(id))) => {
            LuaValue::String(lua.create_string(id.to_string()).ok()?)
        }
        _ => return None,
    })
}

/// A Lust field's value as the editor's, if it's the right kind.
pub(crate) fn from_lua(kind: &FieldKind, value: &LuaValue) -> Option<Value> {
    Some(match (kind, value) {
        (FieldKind::Bool, LuaValue::Boolean(b)) => Value::Bool(*b),
        (FieldKind::Int, v) => {
            let n = number(v)?;
            (n.fract() == 0.0 && n.abs() < 9.0e15).then_some(Value::Int(n as i64))?
        }
        (FieldKind::Float, v) => Value::Float(number(v)?),
        (FieldKind::Text, LuaValue::String(s)) => Value::Text(s.to_str().ok()?.to_string()),
        (FieldKind::Choice(options), LuaValue::String(s)) => {
            let text = s.to_str().ok()?.to_string();
            options.contains(&text).then_some(Value::Text(text))?
        }
        (FieldKind::Vec3, v @ (LuaValue::Vector(_) | LuaValue::Table(_))) => {
            let v = vec3_of(v)?;
            Value::floats(&[v.x, v.y, v.z])
        }
        (FieldKind::Rotation, LuaValue::Table(t)) => {
            let n = |k: &str| number(&t.raw_get::<LuaValue>(k).ok()?).map(|v| v as f32);
            Value::floats(&[n("x")?, n("y")?, n("z")?, n("w")?])
        }
        (FieldKind::Entity, LuaValue::Nil) => Value::Entity(None),
        (FieldKind::Entity, v) => Value::Entity(Some(entity_of(v)?)),
        _ => return None,
    })
}

/// A Lua value as the editor's, guessing its kind (for an event no Lust
/// declares, sent from Lua): a number is a float (Lua has one kind of
/// number, so `2` and `2.5` arrive alike), a table with `x`, `y`, `z`
/// (and `w`) a vector (or rotation), nil an empty entity.
pub(crate) fn guessed(value: &LuaValue) -> Option<Value> {
    Some(match value {
        LuaValue::Nil => Value::Entity(None),
        LuaValue::Boolean(b) => Value::Bool(*b),
        LuaValue::Integer(n) => Value::Float(*n as f64),
        LuaValue::Number(n) if n.is_finite() => Value::Float(*n),
        LuaValue::String(s) => Value::Text(s.to_str().ok()?.to_string()),
        LuaValue::Vector(_) => from_lua(&FieldKind::Vec3, value)?,
        LuaValue::Table(t) => {
            let has_w = !matches!(t.raw_get::<LuaValue>("w").ok()?, LuaValue::Nil);
            if has_w {
                from_lua(&FieldKind::Rotation, value)?
            } else {
                from_lua(&FieldKind::Vec3, value)?
            }
        }
        _ => return None,
    })
}

/// An editor value as Lua takes it, by its own kind (for an event no Lust
/// declares).
pub(crate) fn guessed_to_lua(lua: &Lua, value: &Value) -> Option<LuaValue> {
    let kind = match value {
        Value::Bool(_) => FieldKind::Bool,
        Value::Int(_) => FieldKind::Int,
        Value::Float(_) => FieldKind::Float,
        Value::Text(_) => FieldKind::Text,
        Value::Entity(_) => FieldKind::Entity,
        Value::List(items) if items.len() == 4 => FieldKind::Rotation,
        Value::List(_) => FieldKind::Vec3,
        Value::Asset(_) => return None,
    };
    to_lua(lua, &kind, value)
}
