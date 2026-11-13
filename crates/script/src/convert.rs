//! Values between the editor's kinds (`reflect`) and how Lust keeps them
//! in Luau: numbers, text, Luau vectors for `vec3` (a table with `x`, `y`,
//! `z` is taken too), `{x, y, z, w}` rotations (quaternions), `{x, y}`
//! tables for `vec2`, `{r, g, b, a}` tables for `colour` (Solis's colour,
//! with its see-through kept when it isn't 1, `Value::colour`), and
//! entities as their 13-letter text. Tables are read and written raw, so
//! game code's metatables never run here.

use glam::{Quat, Vec2, Vec3};
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
        "vec2" => FieldKind::Vec2,
        "colour" => FieldKind::Color,
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
        FieldKind::Vec2 => Value::floats(&[0.0, 0.0]),
        FieldKind::Rotation => Value::floats(&[0.0, 0.0, 0.0, 1.0]),
        FieldKind::Entity => Value::Entity(None),
        FieldKind::Asset => Value::Asset(None),
        FieldKind::Text => Value::Text(String::new()),
        FieldKind::Numbers => Value::List(Vec::new()),
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

/// A vec2 as game code gives it: a table with `x` and `y`, or a Luau
/// vector (its z left out), as Lust takes one from a native.
pub(crate) fn vec2_of(value: &LuaValue) -> Option<Vec2> {
    let v = match value {
        LuaValue::Vector(v) => Vec2::new(v.x(), v.y()),
        LuaValue::Table(table) => {
            let n = |k: &str| number(&table.raw_get::<LuaValue>(k).ok()?).map(|v| v as f32);
            Vec2::new(n("x")?, n("y")?)
        }
        _ => return None,
    };
    v.is_finite().then_some(v)
}

/// A colour as game code gives it: a table with `r`, `g` and `b`, and
/// `a` (1 when it's left out), each kept between 0 and 1 as Lust keeps
/// them.
pub(crate) fn colour_of(value: &LuaValue) -> Option<[f32; 4]> {
    let LuaValue::Table(table) = value else {
        return None;
    };
    let n = |k: &str| number(&table.raw_get::<LuaValue>(k).ok()?).map(|v| v as f32);
    let a = match table.raw_get::<LuaValue>("a").ok()? {
        LuaValue::Nil => 1.0,
        _ => n("a")?,
    };
    Some([n("r")?, n("g")?, n("b")?, a].map(|v| v.clamp(0.0, 1.0)))
}

/// A table with these keys and numbers, as Lust keeps a vec2 or colour.
fn numbers_table(lua: &Lua, keys: &[&str], values: &[f32]) -> mlua::Result<Table> {
    let t = lua.create_table_with_capacity(0, keys.len())?;
    for (k, v) in keys.iter().zip(values) {
        t.raw_set(*k, *v)?;
    }
    Ok(t)
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
pub(crate) struct QuatKeys([mlua::LuaString; 4]);

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
    let n = |k: &mlua::LuaString| number(&table.raw_get::<LuaValue>(k).ok()?).map(|v| v as f32);
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
        (FieldKind::Vec2, v) => {
            LuaValue::Table(numbers_table(lua, &["x", "y"], &v.as_floats::<2>()?).ok()?)
        }
        (FieldKind::Color, v) => {
            let colour = v.as_colour()?.map(|c| c.clamp(0.0, 1.0));
            LuaValue::Table(numbers_table(lua, &["r", "g", "b", "a"], &colour).ok()?)
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
        (FieldKind::Vec2, v @ (LuaValue::Vector(_) | LuaValue::Table(_))) => {
            Value::vec2(vec2_of(v)?)
        }
        (FieldKind::Color, v) => Value::colour(colour_of(v)?),
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
/// (and `w`) a vector (or rotation), one with only `x` and `y` a vec2, one
/// with `r` a colour, nil an empty entity.
pub(crate) fn guessed(value: &LuaValue) -> Option<Value> {
    Some(match value {
        LuaValue::Nil => Value::Entity(None),
        LuaValue::Boolean(b) => Value::Bool(*b),
        LuaValue::Integer(n) => Value::Float(*n as f64),
        LuaValue::Number(n) if n.is_finite() => Value::Float(*n),
        LuaValue::String(s) => Value::Text(s.to_str().ok()?.to_string()),
        LuaValue::Vector(_) => from_lua(&FieldKind::Vec3, value)?,
        LuaValue::Table(t) => {
            let has = |k: &str| {
                t.raw_get::<LuaValue>(k)
                    .is_ok_and(|v| !matches!(v, LuaValue::Nil))
            };
            let kind = if has("r") {
                FieldKind::Color
            } else if has("w") {
                FieldKind::Rotation
            } else if has("z") {
                FieldKind::Vec3
            } else {
                FieldKind::Vec2
            };
            from_lua(&kind, value)?
        }
        _ => return None,
    })
}

/// An editor value as Lua takes it, by its own kind (for an event no Lust
/// declares): four numbers are a rotation, three a vector and two a vec2,
/// so a colour arrives as its numbers.
pub(crate) fn guessed_to_lua(lua: &Lua, value: &Value) -> Option<LuaValue> {
    let kind = match value {
        Value::Bool(_) => FieldKind::Bool,
        Value::Int(_) => FieldKind::Int,
        Value::Float(_) => FieldKind::Float,
        Value::Text(_) => FieldKind::Text,
        Value::Entity(_) => FieldKind::Entity,
        Value::List(items) if items.len() == 4 => FieldKind::Rotation,
        Value::List(items) if items.len() == 2 => FieldKind::Vec2,
        Value::List(_) => FieldKind::Vec3,
        Value::Asset(_) => return None,
    };
    to_lua(lua, &kind, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// A field's kind and a value of it, as game code can hold one.
    fn kind_and_value() -> impl Strategy<Value = (FieldKind, Value)> {
        let f = || -1.0e6f32..1.0e6;
        prop_oneof![
            any::<bool>().prop_map(|b| (FieldKind::Bool, Value::Bool(b))),
            (-9_000_000_000_000_000i64..9_000_000_000_000_000)
                .prop_map(|n| (FieldKind::Int, Value::Int(n))),
            (-1.0e300f64..1.0e300).prop_map(|v| (FieldKind::Float, Value::Float(v))),
            ".{0,40}".prop_map(|t| (FieldKind::Text, Value::Text(t))),
            (0usize..3).prop_map(|i| {
                let options = vec!["Red".to_string(), "Blue".into(), "Green".into()];
                let t = options[i].clone();
                (FieldKind::Choice(options), Value::Text(t))
            }),
            (f(), f(), f()).prop_map(|(x, y, z)| (FieldKind::Vec3, Value::floats(&[x, y, z]))),
            (f(), f(), f(), f())
                .prop_map(|(x, y, z, w)| (FieldKind::Rotation, Value::floats(&[x, y, z, w]))),
            (f(), f()).prop_map(|(x, y)| (FieldKind::Vec2, Value::floats(&[x, y]))),
            // Lust keeps a colour's parts between 0 and 1; one fully solid
            // a time in four, as most are.
            (
                0.0f32..=1.0,
                0.0f32..=1.0,
                0.0f32..=1.0,
                prop_oneof![Just(1.0f32), 0.0f32..=1.0]
            )
                .prop_map(|(r, g, b, a)| (FieldKind::Color, Value::colour([r, g, b, a]))),
            any::<u64>().prop_map(|n| (
                FieldKind::Entity,
                Value::Entity(reflect::Id::from_raw(n).map(EntityId))
            )),
        ]
    }

    proptest! {
        /// A field's value handed to game code and back is the same value
        /// (a save keeps game code's values this way, so a difference
        /// would make a loaded game differ; decision 0042).
        #[test]
        fn a_value_crosses_to_game_code_and_back_unchanged((kind, value) in kind_and_value()) {
            let lua = Lua::new();
            let there = to_lua(&lua, &kind, &value).expect("its own kind");
            prop_assert_eq!(from_lua(&kind, &there), Some(value));
        }

        /// A value sent in an event no Lust declares (from Lua) arrives
        /// as its own kind: numbers as floats, nothing as no entity.
        #[test]
        fn an_undeclared_event_s_value_arrives_as_its_kind((_, value) in kind_and_value()) {
            let lua = Lua::new();
            let Some(there) = guessed_to_lua(&lua, &value) else {
                return Ok(());
            };
            let back = guessed(&there);
            let expected = match &value {
                Value::Int(n) => Some(Value::Float(*n as f64)),
                // An entity is text in Lua: it arrives as text.
                Value::Entity(Some(id)) => Some(Value::Text(id.to_string())),
                // Text that looks like nothing else stays text.
                v => Some(v.clone()),
            };
            prop_assert_eq!(back, expected);
        }
    }

    #[test]
    fn every_kind_s_empty_value_is_of_its_kind() {
        let kinds = [
            FieldKind::Bool,
            FieldKind::Int,
            FieldKind::Float,
            FieldKind::Text,
            FieldKind::Choice(vec!["A".into()]),
            FieldKind::Vec3,
            FieldKind::Vec2,
            FieldKind::Color,
            FieldKind::Rotation,
            FieldKind::Entity,
            FieldKind::Asset,
        ];
        for kind in kinds {
            assert!(kind.accepts(&empty(&kind)), "{kind:?}");
        }
        assert_eq!(
            empty(&FieldKind::Rotation),
            Value::floats(&[0.0, 0.0, 0.0, 1.0])
        );
        assert_eq!(
            empty(&FieldKind::Choice(vec!["A".into()])),
            Value::Text("A".into())
        );
    }

    #[test]
    fn a_vec2_comes_as_a_table_or_a_vector_and_only_finite() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.raw_set("x", 3).unwrap();
        table.raw_set("y", 4.5).unwrap();
        assert_eq!(
            vec2_of(&LuaValue::Table(table.clone())),
            Some(Vec2::new(3.0, 4.5))
        );
        let v = LuaValue::Vector(mlua::Vector::new(1.0, 2.0, 9.0));
        assert_eq!(vec2_of(&v), Some(Vec2::new(1.0, 2.0)), "its z left out");
        table.raw_set("y", f64::INFINITY).unwrap();
        assert_eq!(vec2_of(&LuaValue::Table(table.clone())), None);
        table.raw_set("y", "four").unwrap();
        assert_eq!(vec2_of(&LuaValue::Table(table)), None);
        assert_eq!(vec2_of(&LuaValue::Integer(2)), None);
        let there = to_lua(&lua, &FieldKind::Vec2, &Value::floats(&[3.0, 4.5])).unwrap();
        let LuaValue::Table(there) = there else {
            panic!("a vec2 is a table in Lust")
        };
        assert_eq!(there.raw_get::<f32>("x").unwrap(), 3.0);
        assert_eq!(there.raw_get::<LuaValue>("z").unwrap(), LuaValue::Nil);
    }

    #[test]
    fn a_colour_is_kept_between_0_and_1_and_solid_when_its_see_through_is_left_out() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.raw_set("r", 2.0).unwrap();
        table.raw_set("g", 0.5).unwrap();
        table.raw_set("b", -1).unwrap();
        assert_eq!(
            from_lua(&FieldKind::Color, &LuaValue::Table(table.clone())),
            Some(Value::floats(&[1.0, 0.5, 0.0])),
            "no a: fully solid, kept as three numbers"
        );
        table.raw_set("a", 0.25).unwrap();
        assert_eq!(
            from_lua(&FieldKind::Color, &LuaValue::Table(table.clone())),
            Some(Value::floats(&[1.0, 0.5, 0.0, 0.25]))
        );
        table.raw_set("a", "half").unwrap();
        assert_eq!(from_lua(&FieldKind::Color, &LuaValue::Table(table)), None);
        // A colour the area holds out of range (written by hand) reaches
        // game code within 0 and 1, solid when it has three numbers.
        let there = to_lua(&lua, &FieldKind::Color, &Value::floats(&[1.5, 0.5, -2.0])).unwrap();
        assert_eq!(colour_of(&there), Some([1.0, 0.5, 0.0, 1.0]));
        assert_eq!(
            colour_of(&LuaValue::Vector(mlua::Vector::new(1.0, 1.0, 1.0))),
            None
        );
    }

    #[test]
    fn a_vec3_comes_as_a_vector_or_a_table_and_only_finite() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.raw_set("x", 1.0).unwrap();
        table.raw_set("y", 2).unwrap();
        table.raw_set("z", -3.5).unwrap();
        assert_eq!(
            vec3_of(&LuaValue::Table(table.clone())),
            Some(Vec3::new(1.0, 2.0, -3.5))
        );
        table.raw_set("z", f64::NAN).unwrap();
        assert_eq!(vec3_of(&LuaValue::Table(table.clone())), None);
        table.raw_set("z", "three").unwrap();
        assert_eq!(vec3_of(&LuaValue::Table(table)), None);
        let v = LuaValue::Vector(mlua::Vector::new(f32::INFINITY, 0.0, 0.0));
        assert_eq!(vec3_of(&v), None);
        assert_eq!(vec3_of(&LuaValue::Boolean(true)), None);
    }
}
