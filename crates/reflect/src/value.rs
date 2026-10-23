//! Plain values: what a component's fields hold, whatever language the
//! component is written in.

use crate::id::{AssetId, EntityId};
use glam::{Quat, Vec3};

/// A reference to an asset, or to one part of it (a node of a model).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssetRef {
    /// The asset.
    pub asset: AssetId,
    /// The part inside it, by its stable part number; `None` for the whole.
    pub part: Option<u32>,
}

/// One field's value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Yes or no.
    Bool(bool),
    /// A whole number.
    Int(i64),
    /// A decimal number (kept at full precision; the schema says how it's
    /// stored).
    Float(f64),
    /// Text, or one of a choice's options.
    Text(String),
    /// A list: a vector, a rotation or a colour are lists of numbers.
    List(Vec<Value>),
    /// A reference to an asset, or none.
    Asset(Option<AssetRef>),
    /// A reference to another entity, or none.
    Entity(Option<EntityId>),
}

impl Value {
    /// The value as a decimal, if it's a number.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(v) => Some(*v),
            Self::Int(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// The value as a finite `f32`, if it's a number that fits.
    pub fn as_f32(&self) -> Option<f32> {
        self.as_f64().map(|v| v as f32).filter(|v| v.is_finite())
    }

    /// A list of `n` finite numbers.
    pub fn as_floats<const N: usize>(&self) -> Option<[f32; N]> {
        let Self::List(items) = self else {
            return None;
        };
        if items.len() != N {
            return None;
        }
        let mut out = [0.0; N];
        for (slot, item) in out.iter_mut().zip(items) {
            *slot = item.as_f32()?;
        }
        Some(out)
    }

    /// A list of numbers.
    pub fn floats(values: &[f32]) -> Self {
        Self::List(values.iter().map(|&v| Self::Float(f64::from(v))).collect())
    }

    /// A vector.
    pub fn vec3(v: Vec3) -> Self {
        Self::floats(&v.to_array())
    }

    /// A rotation, as x, y, z, w.
    pub fn quat(q: Quat) -> Self {
        Self::floats(&q.to_array())
    }

    /// A short description of the value's kind, for error messages.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Bool(_) => "yes or no",
            Self::Int(_) => "a whole number",
            Self::Float(_) => "a number",
            Self::Text(_) => "text",
            Self::List(_) => "a list",
            Self::Asset(_) => "an asset",
            Self::Entity(_) => "an entity",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_lists_read_back() {
        assert_eq!(Value::Int(3).as_f32(), Some(3.0));
        assert_eq!(Value::Float(f64::NAN).as_f32(), None);
        assert_eq!(Value::Float(1e300).as_f32(), None, "too big for f32");
        assert_eq!(Value::Text("1".into()).as_f64(), None);
        let v = Value::vec3(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(v.as_floats::<3>(), Some([1.0, 2.0, 3.0]));
        assert_eq!(v.as_floats::<4>(), None, "wrong length");
        assert_eq!(
            Value::quat(Quat::IDENTITY).as_floats::<4>(),
            Some([0.0, 0.0, 0.0, 1.0])
        );
        assert_eq!(Value::Bool(true).as_floats::<1>(), None);
        assert_eq!(Value::Entity(None).kind_name(), "an entity");
    }
}
