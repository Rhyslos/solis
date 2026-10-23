//! Lights: the sun, point lights and spotlights, in Blender's units
//! ([decision 0014](../../../../docs/decisions/0014-lights-placed-two-ways.md)).
//! A light shines along its entity's -Z (see `Transform::forward`).

use super::field;
use glam::{Quat, Vec3};
use reflect::{Component, ComponentData, ComponentSchema, Fields, Value};

/// What kind of light.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LightKind {
    /// Parallel light from far away, like daylight.
    Sun,
    /// Light in every direction from a point, like a bulb.
    #[default]
    Point,
    /// A cone of light, like a torch.
    Spot,
}

impl LightKind {
    const NAMES: [&'static str; 3] = ["Sun", "Point", "Spot"];

    fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }

    fn from_name(name: Option<&str>) -> Self {
        match name {
            Some("Sun") => Self::Sun,
            Some("Spot") => Self::Spot,
            _ => Self::Point,
        }
    }
}

/// A light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    /// Sun, point or spot.
    pub kind: LightKind,
    /// Its colour, sRGB 0 to 1.
    pub color: [f32; 3],
    /// Watts for lamps, watts per square metre for the sun (Blender's
    /// units).
    pub strength: f32,
    /// How far a lamp reaches in metres; 0 fades on forever.
    pub range: f32,
    /// A spotlight's cone, in degrees from its middle.
    pub cone: f32,
    /// How gradually a spotlight fades at its edge, in degrees.
    pub soft_edge: f32,
    /// Whether it casts shadows (the sun only, for now).
    pub shadows: bool,
}

impl Default for Light {
    fn default() -> Self {
        Self::point()
    }
}

impl Light {
    /// A sun as a new area has it.
    pub fn sun() -> Self {
        Self {
            kind: LightKind::Sun,
            color: [1.0, 250.0 / 255.0, 242.0 / 255.0],
            strength: 3.0,
            range: 0.0,
            cone: 30.0,
            soft_edge: 10.0,
            shadows: true,
        }
    }

    /// A point light as Entity > Add point light makes it.
    pub fn point() -> Self {
        Self {
            kind: LightKind::Point,
            color: [1.0, 1.0, 1.0],
            strength: 1000.0,
            shadows: false,
            ..Self::sun()
        }
    }

    /// A spotlight as Entity > Add spotlight makes it.
    pub fn spot() -> Self {
        Self {
            kind: LightKind::Spot,
            ..Self::point()
        }
    }
}

/// The rotation that points -Z at a sun `heading` degrees clockwise from
/// north (+Y) and `height` degrees above the horizon.
///
/// Built as a tilt from straight down around X, then a turn around Z, so
/// the inspector shows it as X = 90° − height and Z = 180° − heading.
pub fn sun_rotation(heading: f32, height: f32) -> Quat {
    Quat::from_euler(
        glam::EulerRot::ZXY,
        (180.0 - heading).to_radians(),
        (90.0 - height).to_radians(),
        0.0,
    )
}

/// The rotation that points -Z along `direction` (keeping it upright when
/// it can).
pub fn aim(direction: Vec3) -> Quat {
    let d = direction.try_normalize().unwrap_or(Vec3::NEG_Z);
    Quat::from_rotation_arc(Vec3::NEG_Z, d)
}

impl Component for Light {
    const NAME: &'static str = "Light";

    fn schema() -> ComponentSchema {
        let d = Self::point();
        ComponentSchema {
            name: Self::NAME.into(),
            help: "Light: the sun, a point light or a spotlight, in Blender's units. It shines \
                   along the entity's -Z."
                .into(),
            fields: vec![
                field::choice(
                    "kind",
                    "Kind",
                    &LightKind::NAMES,
                    d.kind.name(),
                    "Sun: parallel light from far away. Point: every way from a point. Spot: a cone.",
                ),
                field::color("color", "Colour", d.color, "The light's colour."),
                field::float(
                    "strength",
                    "Strength",
                    d.strength,
                    field::hint(
                        (0.0, 100_000.0),
                        5.0,
                        "",
                        "Watts for lamps (1000 lights a room), W/m² for the sun (3 is a clear day).",
                    ),
                ),
                field::float(
                    "range",
                    "Range",
                    d.range,
                    field::hint(
                        (0.0, 10_000.0),
                        0.1,
                        " m",
                        "How far a lamp reaches; 0 fades on forever.",
                    ),
                ),
                field::float(
                    "cone",
                    "Cone",
                    d.cone,
                    field::hint(
                        (1.0, 89.0),
                        0.5,
                        "°",
                        "A spotlight's width, from the middle of its cone.",
                    ),
                ),
                field::float(
                    "soft_edge",
                    "Soft edge",
                    d.soft_edge,
                    field::hint(
                        (0.0, 89.0),
                        0.5,
                        "°",
                        "How gradually a spotlight fades at its edge.",
                    ),
                ),
                field::boolean(
                    "shadows",
                    "Shadows",
                    d.shadows,
                    "Whether it casts shadows (the sun, for now).",
                ),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("kind", Value::Text(self.kind.name().into()));
        data.set("color", Value::floats(&self.color));
        data.set("strength", Value::Float(f64::from(self.strength)));
        data.set("range", Value::Float(f64::from(self.range)));
        data.set("cone", Value::Float(f64::from(self.cone)));
        data.set("soft_edge", Value::Float(f64::from(self.soft_edge)));
        data.set("shadows", Value::Bool(self.shadows));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        let f = Fields(data);
        let kind = LightKind::from_name(f.text("kind"));
        let d = match kind {
            LightKind::Sun => Self::sun(),
            LightKind::Point => Self::point(),
            LightKind::Spot => Self::spot(),
        };
        Self {
            kind,
            color: f.color("color", d.color),
            strength: f.float("strength", d.strength),
            range: f.float("range", d.range),
            cone: f.float("cone", d.cone),
            soft_edge: f.float("soft_edge", d.soft_edge),
            shadows: f.bool("shadows", d.shadows),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sun_rotation_points_down_from_the_heading() {
        let down = sun_rotation(150.0, 50.0) * Vec3::NEG_Z;
        assert!(down.z < -0.7, "{down}");
        assert!(down.x < 0.0 && down.y > 0.0, "from the south-east: {down}");
        let noon = sun_rotation(0.0, 90.0) * Vec3::NEG_Z;
        assert!(noon.abs_diff_eq(Vec3::NEG_Z, 1e-5));
        assert!((aim(Vec3::ZERO) * Vec3::NEG_Z).abs_diff_eq(Vec3::NEG_Z, 1e-6));
    }
}
