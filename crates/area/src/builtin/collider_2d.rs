//! The 2D collider (decision 0071): a shape on the ground, in pixels, for
//! 2D physics. Solid and still (a house, a shipping bin), or a character
//! game code walks (`moveCharacter`), which slides along walls, solid
//! tiles and other colliders. Plain values here; the game's world turns
//! them into 2D physics.

use super::field;
use reflect::{Component, ComponentData, ComponentSchema, Fields, Value};

/// A 2D collider's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shape2D {
    /// A circle: a character's footprint, which slides round corners.
    #[default]
    Circle,
    /// A box.
    Box,
}

impl Shape2D {
    const NAMES: [&'static str; 2] = ["Circle", "Box"];

    fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }

    fn from_name(name: Option<&str>) -> Self {
        match name {
            Some("Box") => Self::Box,
            _ => Self::Circle,
        }
    }
}

/// A shape on the ground for 2D physics, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Collider2D {
    /// Circle or box.
    pub shape: Shape2D,
    /// A circle's radius, pixels.
    pub radius: f32,
    /// A box's width, pixels.
    pub width: f32,
    /// A box's height, pixels.
    pub height: f32,
    /// How far right of the entity's place its middle is, pixels.
    pub across: f32,
    /// How far up from the entity's place its middle is, pixels (a
    /// sprite's feet are often a little above its place).
    pub up: f32,
    /// Walked by game code (`moveCharacter`), sliding along what's in its
    /// way; otherwise solid and still.
    pub character: bool,
}

impl Default for Collider2D {
    fn default() -> Self {
        Self {
            shape: Shape2D::Circle,
            radius: 5.0,
            width: 16.0,
            height: 16.0,
            across: 0.0,
            up: 0.0,
            character: false,
        }
    }
}

impl Component for Collider2D {
    const NAME: &'static str = "Collider2D";

    fn schema() -> ComponentSchema {
        let d = Self::default();
        let pixels = |name: &str, label: &str, default: f32, help: &'static str| {
            field::float(name, label, default, field::hint((0.1, 4096.0), 0.5, " px", help))
        };
        let offset = |name: &str, label: &str, default: f32, help: &'static str| {
            field::float(name, label, default, field::hint((-4096.0, 4096.0), 0.5, " px", help))
        };
        ComponentSchema {
            name: Self::NAME.into(),
            help: "A shape on the ground for 2D physics, in pixels. On its own it's solid and \
                   still (a house, a bin); tick Character and game code walks it, sliding along \
                   walls, solid tiles and other colliders."
                .into(),
            fields: vec![
                field::choice(
                    "shape",
                    "Shape",
                    &Shape2D::NAMES,
                    d.shape.name(),
                    "A circle (best for characters: it slides round corners) or a box.",
                ),
                pixels("radius", "Radius", d.radius, "A circle's radius."),
                pixels("width", "Width", d.width, "A box's width."),
                pixels("height", "Height", d.height, "A box's height."),
                offset(
                    "across",
                    "Across",
                    d.across,
                    "How far right of the entity's place the shape's middle is.",
                ),
                offset(
                    "up",
                    "Up",
                    d.up,
                    "How far up from the entity's place the shape's middle is: put a character's \
                     circle at its feet.",
                ),
                field::boolean(
                    "character",
                    "Character",
                    d.character,
                    "Walked by game code (moveCharacter, or the kit's TopDownController), sliding \
                     along what's in its way. Off: solid and still.",
                ),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("shape", Value::Text(self.shape.name().into()));
        data.set("radius", Value::Float(f64::from(self.radius)));
        data.set("width", Value::Float(f64::from(self.width)));
        data.set("height", Value::Float(f64::from(self.height)));
        data.set("across", Value::Float(f64::from(self.across)));
        data.set("up", Value::Float(f64::from(self.up)));
        data.set("character", Value::Bool(self.character));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        let d = Self::default();
        let f = Fields(data);
        Self {
            shape: Shape2D::from_name(f.text("shape")),
            radius: f.float("radius", d.radius),
            width: f.float("width", d.width),
            height: f.float("height", d.height),
            across: f.float("across", d.across),
            up: f.float("up", d.up),
            character: f.bool("character", d.character),
        }
    }
}
