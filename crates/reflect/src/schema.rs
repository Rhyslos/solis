//! Component schemas: a component's name and its fields, with their kinds,
//! starting values and hints for the inspector.

use crate::value::Value;
use std::borrow::Cow;

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
    /// Two numbers: a place or a direction in 2D (game code's `vec2`).
    Vec2,
    /// A rotation, stored as a quaternion (x, y, z, w) and shown in degrees.
    Rotation,
    /// A colour, three numbers from 0 to 1 in sRGB, as colour pickers show,
    /// and a fourth for its see-through when game code's colour has one
    /// (`Value::colour`).
    Color,
    /// A reference to an asset.
    Asset,
    /// A reference to another entity.
    Entity,
    /// A list of numbers of any length (a brush's points and faces): kept
    /// and saved as it is, made by tools rather than typed, so Properties
    /// only says how many there are.
    Numbers,
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
            (Self::Vec3, v) => v.as_floats::<3>().is_some(),
            (Self::Vec2, v) => v.as_floats::<2>().is_some(),
            (Self::Color, v) => v.as_colour().is_some(),
            (Self::Rotation, v) => v.as_floats::<4>().is_some(),
            (Self::Numbers, Value::List(items)) => items.iter().all(|v| match v {
                Value::Int(_) => true,
                Value::Float(f) => f.is_finite(),
                _ => false,
            }),
            _ => false,
        }
    }
}

/// How the inspector shows a field. Hints never limit what's stored:
/// `range` is where dragging and typing stop, nothing more, and game code
/// may set anything. They belong to the code, so they're never saved in a
/// level: new code brings new hints.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Hint {
    /// The smallest and largest value dragging and typing reach.
    pub range: Option<(f64, f64)>,
    /// How much one point of dragging changes the value.
    pub speed: Option<f64>,
    /// What a dragged or typed value is rounded to a multiple of
    /// (game code's `step 0.5`).
    pub step: Option<f64>,
    /// The unit shown after the value, such as " m" or " W".
    pub unit: Cow<'static, str>,
    /// What the field does, in plain words, shown when pointed at.
    pub help: Cow<'static, str>,
    /// A sentence shown under the field (game code's `tip "…"`).
    pub tip: Cow<'static, str>,
    /// The heading the field is shown under, with the fields after it
    /// (game code's `group "Movement"`); empty for none.
    pub group: Cow<'static, str>,
    /// A slider between the range's ends rather than a number box alone.
    pub slider: bool,
    /// Text over several lines.
    pub multiline: bool,
    /// Kept and saved, but not shown in Properties.
    pub hidden: bool,
    /// A yes or no shown as a switch rather than a checkbox: for a
    /// heavier setting that changes how something works (decision 0137;
    /// game code's `toggle`).
    pub toggle: bool,
    /// What the inspector offers to pick from, beyond typing.
    pub offers: Offer,
}

impl Hint {
    /// A hint with `help` (shown when pointed at) and nothing else.
    pub fn help(help: &'static str) -> Self {
        Self {
            help: Cow::Borrowed(help),
            ..Self::default()
        }
    }
}

/// A kind of the project's names (`solis/project.lst`), which a state
/// field of that kind picks from: a sound, a module, a picture… Kept as
/// the option's name, so a renamed file follows (decision 0127).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NameKind {
    /// A sound event (`Sound.Sfx.Footstep`).
    Sound,
    /// A sprite's clip (`Clip.Sprites.Farmer.Walk`).
    Clip,
    /// A picture, or a part of one cut with Atlas Crop
    /// (`Picture.Sprites.Hero`).
    Picture,
    /// A module (`Module.Ships.Fighter`).
    Module,
    /// A game screen (`Screen.PauseMenu`).
    Screen,
    /// A level (`Level.Forest`).
    Level,
    /// A tag (`Tag.Enemy`).
    Tag,
    /// A font (`Font.Lora`, stage 1's text layout).
    Font,
}

impl NameKind {
    /// Every kind, in the order Solis lists them.
    pub const ALL: [Self; 8] = [
        Self::Sound,
        Self::Clip,
        Self::Picture,
        Self::Module,
        Self::Screen,
        Self::Level,
        Self::Tag,
        Self::Font,
    ];

    /// The state's name in `solis/project.lst`: `Sound`.
    pub fn state(self) -> &'static str {
        match self {
            Self::Sound => "Sound",
            Self::Clip => "Clip",
            Self::Picture => "Picture",
            Self::Module => "Module",
            Self::Screen => "Screen",
            Self::Level => "Level",
            Self::Tag => "Tag",
            Self::Font => "Font",
        }
    }

    /// The kind whose state is called `state`.
    pub fn of_state(state: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.state() == state)
    }
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
    /// The clip names of the sprite sheet or clips file in the named asset
    /// field of the same component, for a text field (Phase 9, D3).
    ClipsOf(&'static str),
    /// The look names of the clips file in the named asset field of the
    /// same component, for a text field (Fixes 3).
    LooksOf(&'static str),
    /// The facing pad, for a choice of the eight facings (decision 0137).
    Facing,
    /// The state names of the animator in the named asset field of the
    /// same component, for a text field (Phase 9, D3b).
    StatesOf(&'static str),
    /// The names of the game's screens (`Hud`), for a text field
    /// (decision 0087).
    Screens,
    /// A day's colours at their hours, for a text field holding them
    /// (`6:00 #e99a6a, 9:00 #fff6e0`; decision 0092).
    DayColours,
    /// The project's names of this kind, for a state field holding one
    /// (a sound, a module: decision 0127): a list with search and a
    /// preview, and the Library's files dropped on it.
    Names(NameKind),
}

/// One field of a component.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldInfo {
    /// Its name, as written in level files: `strength`.
    pub name: String,
    /// The name the inspector shows: `Strength`.
    pub label: String,
    /// What it holds.
    pub kind: FieldKind,
    /// Its value in a new component, and when a level leaves it out.
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
    /// Its name, unique among components, as written in level files:
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

    #[test]
    fn numbers_are_a_list_of_any_length_of_finite_numbers() {
        assert!(FieldKind::Numbers.accepts(&Value::List(Vec::new())));
        let mixed = Value::List(vec![Value::Int(3), Value::Float(0.5), Value::Int(-2)]);
        assert!(FieldKind::Numbers.accepts(&mixed));
        assert!(!FieldKind::Numbers.accepts(&Value::List(vec![Value::Float(f64::NAN)])));
        assert!(!FieldKind::Numbers.accepts(&Value::List(vec![Value::Text("1".into())])));
        assert!(!FieldKind::Numbers.accepts(&Value::Int(1)));
    }

    #[test]
    fn a_vec2_is_two_numbers_and_a_colour_three_or_four() {
        assert!(FieldKind::Vec2.accepts(&Value::floats(&[1.0, -2.5])));
        assert!(!FieldKind::Vec2.accepts(&Value::floats(&[1.0, 2.0, 3.0])));
        assert!(!FieldKind::Vec2.accepts(&Value::List(vec![Value::Float(f64::INFINITY); 2])));
        assert!(!FieldKind::Vec3.accepts(&Value::floats(&[1.0, 2.0])));
        assert!(FieldKind::Color.accepts(&Value::floats(&[1.0, 0.5, 0.0, 0.25])));
        assert!(!FieldKind::Color.accepts(&Value::floats(&[1.0, 0.5])));
        assert!(!FieldKind::Color.accepts(&Value::floats(&[1.0, 0.5, 0.0, 0.25, 1.0])));
        assert!(!FieldKind::Vec3.accepts(&Value::floats(&[1.0, 0.5, 0.0, 0.25])));
    }
}
