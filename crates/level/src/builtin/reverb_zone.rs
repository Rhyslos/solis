//! A place where sounds echo (Phase 7, S6): a box or a ball around its
//! entity with a soft edge and a preset. While the game's listener (its
//! camera) is inside, every sound placed in the world gets the zone's
//! reverb; across the soft edge it fades in and out, and where zones
//! overlap the smaller one wins. It grows with the entity's scale.

use glam::Vec3;
use reflect::{
    Component, ComponentData, ComponentSchema, FieldInfo, FieldKind, Fields, Hint, Value,
};

use super::field;

/// A zone's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZoneShape {
    /// A box (a room, a hall, a tunnel).
    #[default]
    Box,
    /// A ball (a cave, a dome).
    Ball,
}

impl ZoneShape {
    /// The shapes' names, as saved and shown.
    pub const NAMES: [&'static str; 2] = ["Box", "Ball"];

    /// As saved and shown.
    pub fn name(self) -> &'static str {
        match self {
            Self::Box => "Box",
            Self::Ball => "Ball",
        }
    }

    fn named(name: &str) -> Self {
        if name == "Ball" {
            Self::Ball
        } else {
            Self::Box
        }
    }
}

/// A place where sounds echo.
#[derive(Debug, Clone, PartialEq)]
pub struct ReverbZone {
    /// Box or ball.
    pub shape: ZoneShape,
    /// A box's size along X, Y and Z, in metres.
    pub size: Vec3,
    /// A ball's radius, in metres.
    pub radius: f32,
    /// How far in from the edge the echo is full, in metres.
    pub soft_edge: f32,
    /// How it sounds, by name: one of [`Self::PRESETS`].
    pub preset: String,
    /// How strong the echo is, 0 to 1.
    pub strength: f32,
}

impl Default for ReverbZone {
    fn default() -> Self {
        Self {
            shape: ZoneShape::Box,
            size: Vec3::new(10.0, 10.0, 4.0),
            radius: 8.0,
            soft_edge: 2.0,
            preset: "Hall".to_owned(),
            strength: 1.0,
        }
    }
}

impl ReverbZone {
    /// The presets, as saved and shown (the sound crate's reverbs).
    pub const PRESETS: [&'static str; 5] = ["Small room", "Hall", "Cave", "Sewer", "Outdoors"];
    /// The largest a zone may be, in metres.
    pub const MAX_SIZE_M: f32 = 100_000.0;

    /// The same with anything unusable put right: sizes and the soft edge
    /// in range, a known preset, and a strength from 0 to 1.
    #[must_use]
    pub fn sanitized(self) -> Self {
        let d = Self::default();
        let metres = |v: f32, fallback: f32| {
            if v.is_finite() {
                v.clamp(0.0, Self::MAX_SIZE_M)
            } else {
                fallback
            }
        };
        Self {
            shape: self.shape,
            size: Vec3::new(
                metres(self.size.x, d.size.x),
                metres(self.size.y, d.size.y),
                metres(self.size.z, d.size.z),
            ),
            radius: metres(self.radius, d.radius),
            soft_edge: metres(self.soft_edge, d.soft_edge),
            preset: if Self::PRESETS.contains(&self.preset.as_str()) {
                self.preset
            } else {
                d.preset
            },
            strength: if self.strength.is_finite() {
                self.strength.clamp(0.0, 1.0)
            } else {
                d.strength
            },
        }
    }
}

impl Component for ReverbZone {
    const NAME: &'static str = "ReverbZone";

    fn schema() -> ComponentSchema {
        let d = Self::default();
        let metres = |name: &str, label: &str, default: f32, help: &'static str| {
            field::float(
                name,
                label,
                default,
                field::hint((0.0, f64::from(Self::MAX_SIZE_M)), 0.05, " m", help),
            )
        };
        ComponentSchema {
            name: Self::NAME.into(),
            help: "A place where sounds echo: while the game's camera is inside, every sound \
                   placed in the world gets its reverb. It grows with the entity's scale."
                .into(),
            fields: vec![
                field::choice(
                    "shape",
                    "Shape",
                    &ZoneShape::NAMES,
                    d.shape.name(),
                    "A box (a room, a hall, a tunnel) or a ball (a cave, a dome).",
                ),
                FieldInfo::new("size", "Size", FieldKind::Vec3, Value::vec3(d.size)).hint(Hint {
                    speed: Some(0.05),
                    unit: " m",
                    help: "A box's size along X, Y and Z, in metres.",
                    ..Hint::default()
                }),
                metres("radius", "Radius", d.radius, "A ball's radius."),
                metres(
                    "soft_edge",
                    "Soft edge",
                    d.soft_edge,
                    "How far in from the edge the echo is full: walking in, it fades in over \
                     this distance. 0 switches it on at the edge.",
                ),
                field::choice(
                    "preset",
                    "Sounds like",
                    &Self::PRESETS,
                    &d.preset,
                    "Small room: short and close. Hall: long and smooth. Cave: long and dark. \
                     Sewer: hard and ringing. Outdoors: a little air.",
                ),
                field::float(
                    "strength",
                    "Strength",
                    d.strength,
                    field::hint(
                        (0.0, 1.0),
                        0.01,
                        "",
                        "How strong the echo is: 1 full, 0.5 half, 0 none.",
                    ),
                ),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("shape", Value::Text(self.shape.name().to_owned()));
        data.set("size", Value::vec3(self.size));
        data.set("radius", Value::Float(f64::from(self.radius)));
        data.set("soft_edge", Value::Float(f64::from(self.soft_edge)));
        data.set("preset", Value::Text(self.preset.clone()));
        data.set("strength", Value::Float(f64::from(self.strength)));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        let d = Self::default();
        let f = Fields(data);
        Self {
            shape: ZoneShape::named(f.text("shape").unwrap_or_default()),
            size: f.vec3("size", d.size),
            radius: f.float("radius", d.radius),
            soft_edge: f.float("soft_edge", d.soft_edge),
            preset: f.text("preset").unwrap_or(&d.preset).to_owned(),
            strength: f.float("strength", d.strength),
        }
        .sanitized()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odd_values_are_put_right() {
        let odd = ReverbZone {
            size: Vec3::new(f32::NAN, -3.0, 1e9),
            radius: f32::INFINITY,
            soft_edge: -1.0,
            preset: "Bathroom".into(),
            strength: 7.0,
            ..ReverbZone::default()
        }
        .sanitized();
        assert_eq!(odd.size, Vec3::new(10.0, 0.0, ReverbZone::MAX_SIZE_M));
        assert_eq!(odd.radius, 8.0);
        assert_eq!(odd.soft_edge, 0.0);
        assert_eq!(odd.preset, "Hall");
        assert_eq!(odd.strength, 1.0);
    }

    #[test]
    fn a_ball_round_trips() {
        let ball = ReverbZone {
            shape: ZoneShape::Ball,
            radius: 12.0,
            preset: "Cave".into(),
            strength: 0.5,
            ..ReverbZone::default()
        };
        assert_eq!(ReverbZone::from_data(&ball.to_data()), ball);
    }
}
