//! A trigger zone (stage 1, triggers): a place in a level that tells game
//! code when something walks in or out of it, such as a door, a shop
//! counter or a pit. Built once for 2D and 3D (decision 0097): a box or a
//! round shape (a circle in 2D, a sphere in 3D), or in 2D a freeform
//! outline drawn point by point. It turns and grows with its entity.
//!
//! Plain values here; the game's world asks physics each tick what
//! overlaps it, and keeps who is inside as game state.

use super::field;
use glam::{Vec2, Vec3};
use mesh::outline::{MAX_OUTLINE_POINTS, OutlineError};
use reflect::{
    Component, ComponentData, ComponentSchema, FieldInfo, FieldKind, Fields, Hint, Value,
};

/// A zone's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZoneForm {
    /// A box ([`TriggerZone::size`]).
    #[default]
    Box,
    /// A circle in 2D, a sphere in 3D ([`TriggerZone::radius`]).
    Round,
    /// An outline of points on the ground, in 2D only
    /// ([`TriggerZone::points`]).
    Freeform,
}

impl ZoneForm {
    /// The shapes' names, as saved.
    pub const NAMES: [&'static str; 3] = ["Box", "Round", "Freeform"];

    /// As saved.
    pub fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }

    /// The shape saved as `name` (a box for anything else).
    pub fn named(name: &str) -> Self {
        match name {
            "Round" => Self::Round,
            "Freeform" => Self::Freeform,
            _ => Self::Box,
        }
    }

    /// As shown to someone working on a 2D level (`flat`) or a 3D one:
    /// a round zone is a circle in 2D and a sphere in 3D.
    pub fn shown(self, flat: bool) -> &'static str {
        match (self, flat) {
            (Self::Box, _) => "Box",
            (Self::Round, true) => "Circle",
            (Self::Round, false) => "Sphere",
            (Self::Freeform, _) => "Freeform",
        }
    }
}

/// Whether a zone tells game code every time something enters, or only
/// the first time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fires {
    /// Every time something walks in or out.
    #[default]
    EveryTime,
    /// The first time something walks in, then never again (kept in saves).
    Once,
}

impl Fires {
    /// The choices' names, as saved and shown.
    pub const NAMES: [&'static str; 2] = ["Every time", "Once"];

    /// As saved and shown.
    pub fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }

    fn named(name: &str) -> Self {
        if name == "Once" {
            Self::Once
        } else {
            Self::EveryTime
        }
    }
}

/// What a zone notices. Only one choice until tags exist, when "only
/// these tags" joins it.
pub const NOTICES: [&str; 1] = ["Characters and bodies"];

/// A place that tells game code when something walks in or out.
#[derive(Debug, Clone, PartialEq)]
pub struct TriggerZone {
    /// Its shape.
    pub form: ZoneForm,
    /// A box's full size along X, Y and Z, in metres (2D uses X and Y).
    pub size: Vec3,
    /// A circle's or sphere's radius, in metres.
    pub radius: f32,
    /// A freeform outline's points, in metres from the entity's place,
    /// going round (2D only). Kept as read even when unusable, so nothing
    /// is lost; [`TriggerZone::outline`] says whether it can be used.
    pub points: Vec<Vec2>,
    /// Every time, or once.
    pub fires: Fires,
}

impl Default for TriggerZone {
    fn default() -> Self {
        Self {
            form: ZoneForm::Box,
            size: Vec3::splat(2.0),
            radius: 1.0,
            points: Vec::new(),
            fires: Fires::EveryTime,
        }
    }
}

impl TriggerZone {
    /// The smallest a zone may be along a side, in metres (a centimetre).
    pub const MIN_SIZE_M: f32 = 0.01;
    /// The largest a zone may be along a side, in metres.
    pub const MAX_SIZE_M: f32 = 100_000.0;

    /// The same with anything unusable put right: sizes in range. Points
    /// are left as they are (see [`TriggerZone::outline`]).
    #[must_use]
    pub fn sanitized(self) -> Self {
        let d = Self::default();
        let metres = |v: f32, fallback: f32| {
            if v.is_finite() {
                v.clamp(Self::MIN_SIZE_M, Self::MAX_SIZE_M)
            } else {
                fallback
            }
        };
        Self {
            size: Vec3::new(
                metres(self.size.x, d.size.x),
                metres(self.size.y, d.size.y),
                metres(self.size.z, d.size.z),
            ),
            radius: metres(self.radius, d.radius),
            ..self
        }
    }

    /// The freeform outline, if it can be used: at least three points, at
    /// most [`MAX_OUTLINE_POINTS`], never crossing itself; or why not.
    pub fn outline(&self) -> Result<&[Vec2], OutlineError> {
        mesh::outline::check(&self.points).map(|()| self.points.as_slice())
    }

    /// A freeform zone's standard outline, around a box of `size` (X and
    /// Y), for a zone just switched to Freeform before any point is drawn.
    pub fn square_outline(size: Vec3) -> Vec<Vec2> {
        let (x, y) = (size.x * 0.5, size.y * 0.5);
        vec![
            Vec2::new(-x, -y),
            Vec2::new(x, -y),
            Vec2::new(x, y),
            Vec2::new(-x, y),
        ]
    }
}

/// Reads a list of numbers as points (x, y pairs), checking the count
/// before anything is made: more than [`MAX_OUTLINE_POINTS`] points, an
/// odd count, or anything that isn't a number reads as none.
fn points_of(value: Option<&Value>) -> Vec<Vec2> {
    let Some(Value::List(items)) = value else {
        return Vec::new();
    };
    if items.len() > MAX_OUTLINE_POINTS * 2 || items.len() % 2 != 0 {
        return Vec::new();
    }
    let numbers: Option<Vec<f32>> = items
        .iter()
        .map(|v| match v {
            Value::Float(f) if f.is_finite() => Some(*f as f32),
            Value::Int(i) => Some(*i as f32),
            _ => None,
        })
        .collect();
    numbers.map_or_else(Vec::new, |n| {
        n.chunks_exact(2).map(|p| Vec2::new(p[0], p[1])).collect()
    })
}

impl Component for TriggerZone {
    const NAME: &'static str = "TriggerZone";

    fn schema() -> ComponentSchema {
        let d = Self::default();
        ComponentSchema {
            name: Self::NAME.into(),
            help: "A place that tells game code when something walks in or out: a door, a shop \
                   counter, a pit. The zone hears Entered and Exited; whoever walked in hears \
                   EnteredZone and LeftZone."
                .into(),
            fields: vec![
                field::choice(
                    "shape",
                    "Shape",
                    &ZoneForm::NAMES,
                    d.form.name(),
                    "A box; a circle in 2D or a sphere in 3D; or, in 2D, a freeform outline \
                     drawn point by point.",
                ),
                FieldInfo::new("size", "Size", FieldKind::Vec3, Value::vec3(d.size)).hint(Hint {
                    speed: Some(0.05),
                    unit: " m",
                    help: "A box's size along X, Y and Z, in metres (2D uses X and Y).",
                    ..Hint::default()
                }),
                field::float(
                    "radius",
                    "Radius",
                    d.radius,
                    field::hint(
                        (f64::from(Self::MIN_SIZE_M), f64::from(Self::MAX_SIZE_M)),
                        0.05,
                        " m",
                        "A circle's or sphere's radius.",
                    ),
                ),
                FieldInfo::new(
                    "points",
                    "Points",
                    FieldKind::Numbers,
                    Value::List(Vec::new()),
                )
                .hint(Hint {
                    help: "A freeform outline's points: two numbers (x, y) for each, in metres \
                           from the entity's place.",
                    ..Hint::default()
                }),
                field::choice(
                    "notices",
                    "Notices",
                    &NOTICES,
                    NOTICES[0],
                    "What the zone notices: characters, and bodies that move.",
                ),
                field::choice(
                    "fires",
                    "Fires",
                    &Fires::NAMES,
                    d.fires.name(),
                    "Every time: whenever something walks in or out. Once: the first time \
                     something walks in, then never again, even after loading a save.",
                ),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("shape", Value::Text(self.form.name().to_owned()));
        data.set("size", Value::vec3(self.size));
        data.set("radius", Value::Float(f64::from(self.radius)));
        data.set(
            "points",
            Value::List(
                self.points
                    .iter()
                    .flat_map(|p| [p.x, p.y])
                    .map(|v| Value::Float(f64::from(v)))
                    .collect(),
            ),
        );
        data.set("notices", Value::Text(NOTICES[0].to_owned()));
        data.set("fires", Value::Text(self.fires.name().to_owned()));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        let d = Self::default();
        let f = Fields(data);
        Self {
            form: ZoneForm::named(f.text("shape").unwrap_or_default()),
            size: f.vec3("size", d.size),
            radius: f.float("radius", d.radius),
            points: points_of(data.get("points")),
            fires: Fires::named(f.text("fires").unwrap_or_default()),
        }
        .sanitized()
    }
}

#[cfg(test)]
#[path = "trigger_zone_tests.rs"]
mod tests;
