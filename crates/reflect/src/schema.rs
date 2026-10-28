//! Component schemas: a component's name and its fields, with their kinds,
//! starting values and hints for the inspector.

use crate::value::Value;

/// What kind of value a field holds.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    /// Yes or no.
    Bool,
    /// A whole number.
    Int,
    /// A decimal number, stored as `f32`.
    Float,
    /// Text.
    Text,
    /// One of these options, stored as its name.
    Choice(Vec<String>),
    /// Three numbers: a position, a size.
    Vec3,
    /// A rotation, stored as a quaternion (x, y, z, w) and shown in degrees.
    Rotation,
    /// A colour, three numbers from 0 to 1 in sRGB, as colour pickers show.
    Color,
    /// A reference to an asset.
    Asset,
    /// A reference to another entity.
    Entity,
}

impl FieldKind {
    /// Whether `value` is the right kind for this field.
    pub fn accepts(&self, value: &Value) -> bool {
        match (self, value) {
            (Self::Bool, Value::Bool(_))
            | (Self::Int, Value::Int(_))
            | (Self::Float, Value::Float(_) | Value::Int(_))
            | (Self::Text, Value::Text(_))
            | (Self::Asset, Value::Asset(_))
            | (Self::Entity, Value::Entity(_)) => true,
            (Self::Choice(options), Value::Text(t)) => options.contains(t),
            (Self::Vec3 | Self::Color, v) => v.as_floats::<3>().is_some(),
            (Self::Rotation, v) => v.as_floats::<4>().is_some(),
            _ => false,
        }
    }
}

/// How the inspector shows a field. Hints never limit what's stored:
/// `range` is where dragging stops, nothing more.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Hint {
    /// The smallest and largest value dragging reaches.
    pub range: Option<(f64, f64)>,
    /// How much one point of dragging changes the value.
    pub speed: Option<f64>,
    /// The unit shown after the value, such as " m" or " W".
    pub unit: &'static str,
    /// What the field does, in plain words, shown when pointed at.
    pub help: &'static str,
    /// What the inspector offers to pick from, beyond typing.
    pub offers: Offer,
}

/// What the inspector offers to pick a field's value from (Phase 9, D2).
/// The inspector knows no component by name: a schema says what to offer
/// here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Offer {
    /// Nothing beyond the field's own widget.
    #[default]
    Nothing,
    /// The project's assets of this kind ("sprite sheet", "model"), for
    /// an asset field.
    Assets(&'static str),
    /// The slice names of the sprite sheet in the named asset field of the
    /// same component, for a text field.
    SlicesOf(&'static str),
}

/// One field of a component.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldInfo {
    /// Its name, as written in area files: `strength`.
    pub name: String,
    /// The name the inspector shows: `Strength`.
    pub label: String,
    /// What it holds.
    pub kind: FieldKind,
    /// Its value in a new component, and when an area leaves it out.
    pub default: Value,
    /// How the inspector shows it.
    pub hint: Hint,
}

impl FieldInfo {
    /// A field named `name`, shown as `label`.
    pub fn new(name: &str, label: &str, kind: FieldKind, default: Value) -> Self {
        Self {
            name: name.to_string(),
            label: label.to_string(),
            kind,
            default,
            hint: Hint::default(),
        }
    }

    /// The same field, with `hint`.
    #[must_use]
    pub fn hint(mut self, hint: Hint) -> Self {
        self.hint = hint;
        self
    }
}

/// A component's name and fields, in the order they're shown and written.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentSchema {
    /// Its name, unique among components, as written in area files:
    /// `Light`.
    pub name: String,
    /// What it's for, in plain words.
    pub help: String,
    /// Its fields, in order.
    pub fields: Vec<FieldInfo>,
}

impl ComponentSchema {
    /// The field named `name`.
    pub fn field(&self, name: &str) -> Option<&FieldInfo> {
        self.fields.iter().find(|f| f.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_accept_only_their_values() {
        assert!(FieldKind::Float.accepts(&Value::Int(2)));
        assert!(!FieldKind::Int.accepts(&Value::Float(2.0)));
        let choice = FieldKind::Choice(vec!["sun".into(), "point".into()]);
        assert!(choice.accepts(&Value::Text("sun".into())));
        assert!(!choice.accepts(&Value::Text("moon".into())));
        assert!(FieldKind::Color.accepts(&Value::floats(&[1.0, 0.5, 0.0])));
        assert!(!FieldKind::Rotation.accepts(&Value::floats(&[1.0, 0.5, 0.0])));
        assert!(!FieldKind::Vec3.accepts(&Value::List(vec![Value::Float(f64::NAN); 3])));
        assert!(!FieldKind::Bool.accepts(&Value::Int(1)));
    }
}
