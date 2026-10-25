//! A component's values, and typed access to them for components written
//! in Rust.

use crate::id::EntityId;
use crate::schema::ComponentSchema;
use crate::value::{AssetRef, Value};
use glam::{Quat, Vec3};
use std::collections::BTreeMap;

/// One component's field values, by field name. Fields its schema doesn't
/// know (written by a newer version, or by a script that's gone) are kept
/// too, never dropped.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ComponentData {
    /// The values, by field name.
    pub fields: BTreeMap<String, Value>,
}

impl ComponentData {
    /// A component with every field at its starting value.
    pub fn defaults(schema: &ComponentSchema) -> Self {
        Self {
            fields: schema
                .fields
                .iter()
                .map(|f| (f.name.clone(), f.default.clone()))
                .collect(),
        }
    }

    /// The value of `field`.
    pub fn get(&self, field: &str) -> Option<&Value> {
        self.fields.get(field)
    }

    /// Sets `field` to `value`.
    pub fn set(&mut self, field: &str, value: Value) {
        self.fields.insert(field.to_string(), value);
    }
}

/// A component written in Rust, readable and writable as values. Lust
/// generates the same code for its components (decision 0016); Lua
/// components have a schema only.
pub trait Component: Sized {
    /// Its name, as in its schema.
    const NAME: &'static str;

    /// Its schema.
    fn schema() -> ComponentSchema;

    /// Its values.
    fn to_data(&self) -> ComponentData;

    /// The component from `data`. Missing or unusable values take their
    /// defaults, so a damaged or older area still loads.
    fn from_data(data: &ComponentData) -> Self;
}

/// Reads typed values out of [`ComponentData`], falling back to a default
/// for anything missing or of the wrong kind.
pub struct Fields<'a>(pub &'a ComponentData);

impl Fields<'_> {
    /// A decimal number.
    pub fn float(&self, name: &str, default: f32) -> f32 {
        self.0.get(name).and_then(Value::as_f32).unwrap_or(default)
    }

    /// A whole number.
    pub fn int(&self, name: &str, default: i64) -> i64 {
        match self.0.get(name) {
            Some(Value::Int(v)) => *v,
            _ => default,
        }
    }

    /// Yes or no.
    pub fn bool(&self, name: &str, default: bool) -> bool {
        match self.0.get(name) {
            Some(Value::Bool(v)) => *v,
            _ => default,
        }
    }

    /// Text (or a choice's option name).
    pub fn text(&self, name: &str) -> Option<&str> {
        match self.0.get(name) {
            Some(Value::Text(v)) => Some(v),
            _ => None,
        }
    }

    /// Three numbers.
    pub fn vec3(&self, name: &str, default: Vec3) -> Vec3 {
        self.0
            .get(name)
            .and_then(Value::as_floats::<3>)
            .map_or(default, Vec3::from)
    }

    /// A rotation, normalised; the default if it can't be.
    pub fn rotation(&self, name: &str, default: Quat) -> Quat {
        self.0
            .get(name)
            .and_then(Value::as_floats::<4>)
            .map(Quat::from_array)
            .filter(|q| q.length_squared() > 1e-12)
            .map_or(default, keep_unit)
    }

    /// Three numbers kept between 0 and 1: a colour.
    pub fn color(&self, name: &str, default: [f32; 3]) -> [f32; 3] {
        self.0
            .get(name)
            .and_then(Value::as_floats::<3>)
            .map_or(default, |c| c.map(|v| v.clamp(0.0, 1.0)))
    }

    /// An asset reference.
    pub fn asset(&self, name: &str) -> Option<AssetRef> {
        match self.0.get(name) {
            Some(Value::Asset(v)) => *v,
            _ => None,
        }
    }

    /// An entity reference.
    pub fn entity(&self, name: &str) -> Option<EntityId> {
        match self.0.get(name) {
            Some(Value::Entity(v)) => *v,
            _ => None,
        }
    }
}

/// `q` as a rotation: made unit length only when it's noticeably off (a
/// hand-written one), so a rotation that's already one (physics's, a saved
/// game's) reads back bit for bit. Normalising every time changed the last
/// digits, and a loaded game then drifted from the saved one (found by the
/// hardening milestone's invariant tests, decision 0042).
fn keep_unit(q: Quat) -> Quat {
    if (q.length_squared() - 1.0).abs() < 1.0e-5 {
        q
    } else {
        q.normalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{FieldInfo, FieldKind};

    #[test]
    fn a_rotation_reads_back_bit_for_bit_and_a_hand_written_one_is_made_unit() {
        // The hardening milestone's invariant tests: a spinning crate's
        // rotation (never exactly unit length) came back a hair different
        // from a save, and the loaded game drifted from the saved one.
        let spun = Quat::from_xyzw(0.074_591_175, -0.389_269_2, 0.001_934_788_7, 0.918_096_84);
        let mut data = ComponentData::default();
        data.set("r", Value::quat(spun));
        assert_eq!(
            Fields(&data).rotation("r", Quat::IDENTITY).to_array(),
            spun.to_array()
        );
        data.set("r", Value::floats(&[0.0, 0.0, 2.0, 0.0]));
        let made = Fields(&data).rotation("r", Quat::IDENTITY);
        assert!((made.length() - 1.0).abs() < 1e-6, "{made}");
        data.set("r", Value::floats(&[0.0, 0.0, 0.0, 0.0]));
        assert_eq!(Fields(&data).rotation("r", Quat::IDENTITY), Quat::IDENTITY);
    }

    #[test]
    fn missing_or_wrong_values_fall_back_to_defaults() {
        let mut data = ComponentData::default();
        data.set("speed", Value::Text("fast".into()));
        data.set("tint", Value::floats(&[2.0, -1.0, 0.5]));
        data.set("turn", Value::floats(&[0.0, 0.0, 0.0, 0.0]));
        let f = Fields(&data);
        assert_eq!(f.float("speed", 1.5), 1.5);
        assert_eq!(f.int("missing", 7), 7);
        assert!(f.bool("missing", true));
        assert_eq!(
            f.color("tint", [0.0; 3]),
            [1.0, 0.0, 0.5],
            "colours stay 0 to 1"
        );
        assert_eq!(f.rotation("turn", Quat::IDENTITY), Quat::IDENTITY);
        assert_eq!(f.text("speed"), Some("fast"));
        assert_eq!(f.asset("speed"), None);
    }

    #[test]
    fn defaults_follow_the_schema() {
        let schema = ComponentSchema {
            name: "Spin".into(),
            help: String::new(),
            fields: vec![FieldInfo::new(
                "rate",
                "Rate",
                FieldKind::Float,
                Value::Float(2.0),
            )],
        };
        let data = ComponentData::defaults(&schema);
        assert_eq!(data.get("rate"), Some(&Value::Float(2.0)));
    }
}
