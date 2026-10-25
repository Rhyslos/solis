//! Physics components ([decision 0036](../../../../docs/decisions/0036-physics-is-rapier-strict-on-every-pc.md)):
//! a `RigidBody` makes an entity move by physics (or be moved by game
//! code, pushing things), and a `Collider` gives it a shape to bump into.
//! An entity with a collider and no body never moves: floors and walls.
//! Plain values here; the game's world turns them into physics.

use super::field;
use glam::Vec3;
use reflect::{
    Component, ComponentData, ComponentSchema, FieldInfo, FieldKind, Fields, Hint, Value,
};

/// How a body moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BodyKind {
    /// Moved by physics: falls, is pushed, bounces.
    #[default]
    Dynamic,
    /// Moved by game code, pushing dynamic bodies out of its way but
    /// never pushed itself.
    Kinematic,
    /// Never moves.
    Fixed,
}

impl BodyKind {
    const NAMES: [&'static str; 3] = ["Dynamic", "Kinematic", "Fixed"];

    fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }

    fn from_name(name: Option<&str>) -> Self {
        match name {
            Some("Kinematic") => Self::Kinematic,
            Some("Fixed") => Self::Fixed,
            _ => Self::Dynamic,
        }
    }
}

/// An entity physics moves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RigidBody {
    /// Dynamic, kinematic or fixed.
    pub kind: BodyKind,
    /// Its mass in kilograms; 0 works it out from its collider's size.
    pub mass: f32,
    /// How quickly it slows down moving.
    pub linear_damping: f32,
    /// How quickly it slows down turning.
    pub angular_damping: f32,
    /// How much gravity pulls it: 1 as normal, 0 floats.
    pub gravity: f32,
    /// Never tips over or spins.
    pub upright: bool,
    /// Checked between ticks too, so it can't pass through thin walls
    /// when fast.
    pub fast: bool,
}

impl Default for RigidBody {
    fn default() -> Self {
        Self {
            kind: BodyKind::Dynamic,
            mass: 0.0,
            linear_damping: 0.0,
            angular_damping: 0.05,
            gravity: 1.0,
            upright: false,
            fast: false,
        }
    }
}

impl Component for RigidBody {
    const NAME: &'static str = "RigidBody";

    fn schema() -> ComponentSchema {
        let d = Self::default();
        ComponentSchema {
            name: Self::NAME.into(),
            help: "Makes the entity move by physics while the game plays: it falls, is pushed and \
                   bounces. Give it a Collider too, so it has a shape to bump into."
                .into(),
            fields: vec![
                field::choice(
                    "kind",
                    "Kind",
                    &BodyKind::NAMES,
                    d.kind.name(),
                    "Dynamic: moved by physics (falls, is pushed). Kinematic: moved only by game \
                     code, pushing dynamic things out of its way (platforms, doors). Fixed: never \
                     moves.",
                ),
                field::float(
                    "mass",
                    "Mass",
                    d.mass,
                    field::hint(
                        (0.0, 1.0e9),
                        0.1,
                        " kg",
                        "How heavy it is. 0 works it out from the collider's size: a kilogram \
                         for each cubic metre.",
                    ),
                ),
                field::float(
                    "linear_damping",
                    "Damping",
                    d.linear_damping,
                    field::hint(
                        (0.0, 100.0),
                        0.01,
                        "",
                        "How quickly it slows down by itself, like air resistance: 0 never.",
                    ),
                ),
                field::float(
                    "angular_damping",
                    "Turn damping",
                    d.angular_damping,
                    field::hint(
                        (0.0, 100.0),
                        0.01,
                        "",
                        "How quickly it stops spinning by itself.",
                    ),
                ),
                field::float(
                    "gravity",
                    "Gravity",
                    d.gravity,
                    field::hint(
                        (-100.0, 100.0),
                        0.05,
                        "×",
                        "How much gravity pulls it: 1 as normal, 0 floats, below 0 rises.",
                    ),
                ),
                field::boolean(
                    "upright",
                    "Stays upright",
                    d.upright,
                    "It never tips over or spins: for characters and things that should stand.",
                ),
                field::boolean(
                    "fast",
                    "Fast",
                    d.fast,
                    "Checked between ticks too, so it can't pass through thin walls when it moves \
                     fast (bullets, thrown things). Costs a little more.",
                ),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("kind", Value::Text(self.kind.name().into()));
        data.set("mass", Value::Float(f64::from(self.mass)));
        data.set(
            "linear_damping",
            Value::Float(f64::from(self.linear_damping)),
        );
        data.set(
            "angular_damping",
            Value::Float(f64::from(self.angular_damping)),
        );
        data.set("gravity", Value::Float(f64::from(self.gravity)));
        data.set("upright", Value::Bool(self.upright));
        data.set("fast", Value::Bool(self.fast));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        let d = Self::default();
        let f = Fields(data);
        Self {
            kind: BodyKind::from_name(f.text("kind")),
            mass: f.float("mass", d.mass),
            linear_damping: f.float("linear_damping", d.linear_damping),
            angular_damping: f.float("angular_damping", d.angular_damping),
            gravity: f.float("gravity", d.gravity),
            upright: f.bool("upright", d.upright),
            fast: f.bool("fast", d.fast),
        }
    }
}

/// A collider's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColliderShape {
    /// A box.
    #[default]
    Box,
    /// A ball.
    Sphere,
    /// A capsule standing along Z: a cylinder with rounded ends.
    Capsule,
}

impl ColliderShape {
    const NAMES: [&'static str; 3] = ["Box", "Sphere", "Capsule"];

    fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }

    fn from_name(name: Option<&str>) -> Self {
        match name {
            Some("Sphere") => Self::Sphere,
            Some("Capsule") => Self::Capsule,
            _ => Self::Box,
        }
    }
}

/// A shape physics bumps into, at the entity (scaled with it).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Collider {
    /// Box, sphere or capsule.
    pub shape: ColliderShape,
    /// A box's size, full widths in metres.
    pub size: Vec3,
    /// A sphere's or capsule's radius, metres.
    pub radius: f32,
    /// A capsule's height, end to end, metres.
    pub height: f32,
    /// Where its middle is, from the entity's origin.
    pub center: Vec3,
    /// How much it grips: 0 ice, 1 rubber.
    pub friction: f32,
    /// How much it bounces: 0 not at all, 1 fully.
    pub bounciness: f32,
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            shape: ColliderShape::Box,
            size: Vec3::ONE,
            radius: 0.5,
            height: 1.8,
            center: Vec3::ZERO,
            friction: 0.5,
            bounciness: 0.0,
        }
    }
}

impl Component for Collider {
    const NAME: &'static str = "Collider";

    fn schema() -> ComponentSchema {
        let d = Self::default();
        let metres = |name: &str, label: &str, default: f32, help: &'static str| {
            field::float(
                name,
                label,
                default,
                field::hint((0.001, 1.0e5), 0.01, " m", help),
            )
        };
        ComponentSchema {
            name: Self::NAME.into(),
            help: "A shape physics bumps into while the game plays. On its own the entity never \
                   moves (floors, walls); with a RigidBody, physics moves it. It grows with the \
                   entity's scale."
                .into(),
            fields: vec![
                field::choice(
                    "shape",
                    "Shape",
                    &ColliderShape::NAMES,
                    d.shape.name(),
                    "Box, sphere, or capsule (a cylinder with rounded ends, standing along Z: \
                     good for characters).",
                ),
                FieldInfo::new("size", "Size", FieldKind::Vec3, Value::vec3(d.size)).hint(Hint {
                    speed: Some(0.01),
                    unit: " m",
                    help: "A box's size along X, Y and Z, in metres.",
                    ..Hint::default()
                }),
                metres(
                    "radius",
                    "Radius",
                    d.radius,
                    "A sphere's or capsule's radius.",
                ),
                metres(
                    "height",
                    "Height",
                    d.height,
                    "A capsule's height, end to end.",
                ),
                FieldInfo::new("center", "Centre", FieldKind::Vec3, Value::vec3(d.center)).hint(
                    Hint {
                        speed: Some(0.01),
                        unit: " m",
                        help: "Where the shape's middle is, from the entity's origin: raise a \
                               character's capsule so its feet are at the origin.",
                        ..Hint::default()
                    },
                ),
                field::float(
                    "friction",
                    "Friction",
                    d.friction,
                    field::hint((0.0, 10.0), 0.01, "", "How much it grips: 0 ice, 1 rubber."),
                ),
                field::float(
                    "bounciness",
                    "Bounciness",
                    d.bounciness,
                    field::hint(
                        (0.0, 1.0),
                        0.01,
                        "",
                        "How much it bounces: 0 not at all, 1 keeps every bit of its speed.",
                    ),
                ),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("shape", Value::Text(self.shape.name().into()));
        data.set("size", Value::vec3(self.size));
        data.set("radius", Value::Float(f64::from(self.radius)));
        data.set("height", Value::Float(f64::from(self.height)));
        data.set("center", Value::vec3(self.center));
        data.set("friction", Value::Float(f64::from(self.friction)));
        data.set("bounciness", Value::Float(f64::from(self.bounciness)));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        let d = Self::default();
        let f = Fields(data);
        Self {
            shape: ColliderShape::from_name(f.text("shape")),
            size: f.vec3("size", d.size),
            radius: f.float("radius", d.radius),
            height: f.float("height", d.height),
            center: f.vec3("center", d.center),
            friction: f.float("friction", d.friction),
            bounciness: f.float("bounciness", d.bounciness),
        }
    }
}

/// A character moved by game code (`moveCharacter`): a standing capsule,
/// feet at the entity's origin, that climbs steps and slopes, slides
/// along walls, pushes crates and rides moving platforms. It takes the
/// place of a RigidBody and Collider on the same entity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharacterBody {
    /// How tall, feet to head, metres.
    pub height: f32,
    /// How wide: the capsule's radius, metres.
    pub radius: f32,
    /// The tallest step it climbs without jumping, metres.
    pub step: f32,
    /// The steepest slope it walks up, degrees.
    pub slope: f32,
    /// How far down it keeps to the ground walking down steps and
    /// slopes (instead of flying off them), metres.
    pub snap: f32,
    /// How heavy, kilograms: how hard it pushes what it walks into.
    pub mass: f32,
}

impl Default for CharacterBody {
    fn default() -> Self {
        Self {
            height: 1.8,
            radius: 0.35,
            step: 0.35,
            slope: 45.0,
            snap: 0.3,
            mass: 80.0,
        }
    }
}

impl CharacterBody {
    /// Its capsule as a collider: standing on the entity's origin.
    pub fn collider(&self) -> Collider {
        let height = sane(self.height, 0.001, 1000.0);
        let radius = sane(self.radius, 0.001, 1000.0).min(height * 0.5);
        Collider {
            shape: ColliderShape::Capsule,
            radius,
            height,
            center: Vec3::new(0.0, 0.0, height * 0.5),
            friction: 0.0,
            ..Collider::default()
        }
    }
}

/// `value` within `low` to `high`; `low` if it isn't a number.
fn sane(value: f32, low: f32, high: f32) -> f32 {
    if value.is_finite() {
        value.clamp(low, high)
    } else {
        low
    }
}

impl Component for CharacterBody {
    const NAME: &'static str = "CharacterBody";

    fn schema() -> ComponentSchema {
        let d = Self::default();
        let metres = |name: &str, label: &str, default: f32, help: &'static str| {
            field::float(
                name,
                label,
                default,
                field::hint((0.001, 1000.0), 0.01, " m", help),
            )
        };
        ComponentSchema {
            name: Self::NAME.into(),
            help: "A character that game code walks around (moveCharacter): a standing capsule \
                   with its feet at the entity's origin. It climbs steps and slopes, slides along \
                   walls, pushes crates and rides moving platforms. Use it instead of a RigidBody \
                   and Collider."
                .into(),
            fields: vec![
                metres("height", "Height", d.height, "How tall, feet to head."),
                metres(
                    "radius",
                    "Radius",
                    d.radius,
                    "How wide: the capsule's radius (half its width).",
                ),
                field::float(
                    "step",
                    "Step height",
                    d.step,
                    field::hint(
                        (0.0, 1000.0),
                        0.01,
                        " m",
                        "The tallest step it walks up without jumping.",
                    ),
                ),
                field::float(
                    "slope",
                    "Steepest slope",
                    d.slope,
                    field::hint(
                        (0.0, 89.0),
                        0.5,
                        "°",
                        "The steepest slope it walks up; steeper ones it slides down.",
                    ),
                ),
                field::float(
                    "snap",
                    "Keeps to the ground",
                    d.snap,
                    field::hint(
                        (0.0, 1000.0),
                        0.01,
                        " m",
                        "How far down it stays on the ground walking down steps and slopes, \
                         instead of flying off them. 0 never.",
                    ),
                ),
                field::float(
                    "mass",
                    "Mass",
                    d.mass,
                    field::hint(
                        (0.001, 1.0e6),
                        0.5,
                        " kg",
                        "How heavy it is: how hard it pushes crates it walks into.",
                    ),
                ),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("height", Value::Float(f64::from(self.height)));
        data.set("radius", Value::Float(f64::from(self.radius)));
        data.set("step", Value::Float(f64::from(self.step)));
        data.set("slope", Value::Float(f64::from(self.slope)));
        data.set("snap", Value::Float(f64::from(self.snap)));
        data.set("mass", Value::Float(f64::from(self.mass)));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        let d = Self::default();
        let f = Fields(data);
        Self {
            height: f.float("height", d.height),
            radius: f.float("radius", d.radius),
            step: f.float("step", d.step),
            slope: f.float("slope", d.slope),
            snap: f.float("snap", d.snap),
            mass: f.float("mass", d.mass),
        }
    }
}
