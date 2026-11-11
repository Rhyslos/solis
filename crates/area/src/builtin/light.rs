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
    /// Whether it shines only at night (decision 0092): on at dusk, off
    /// at dawn, by the Environment's day and night.
    pub only_at_night: bool,
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
            only_at_night: false,
        }
    }

    /// A point light as Entity > Add point light makes it.
    pub fn point() -> Self {
        Self {
            kind: LightKind::Point,
            color: [1.0, 1.0, 1.0],
            strength: 1000.0,
            shadows: false,
            only_at_night: false,
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

    /// The narrowest and widest a spotlight's cone may be, in degrees
    /// from its middle.
    pub const CONE_RANGE: (f32, f32) = (1.0, 89.0);
    /// The most gradual a spotlight's edge may be, in degrees.
    pub const MAX_SOFT_EDGE: f32 = 89.0;

    /// The same light with anything unusable put right: a cone in
    /// [`Self::CONE_RANGE`], a soft edge from 0 to
    /// [`Self::MAX_SOFT_EDGE`], a strength and a range that aren't below
    /// zero, and a colour from 0 to 1. A number that isn't one at all
    /// takes its kind of light's standard. What reads a light's numbers
    /// can then work with them without checking again.
    #[must_use]
    pub fn sanitized(self) -> Self {
        let d = match self.kind {
            LightKind::Sun => Self::sun(),
            LightKind::Point => Self::point(),
            LightKind::Spot => Self::spot(),
        };
        let number = |v: f32, standard: f32| if v.is_finite() { v } else { standard };
        let (narrow, wide) = Self::CONE_RANGE;
        Self {
            kind: self.kind,
            color: [0, 1, 2].map(|i| number(self.color[i], d.color[i]).clamp(0.0, 1.0)),
            strength: number(self.strength, d.strength).max(0.0),
            range: number(self.range, d.range).max(0.0),
            cone: number(self.cone, d.cone).clamp(narrow, wide),
            soft_edge: number(self.soft_edge, d.soft_edge).clamp(0.0, Self::MAX_SOFT_EDGE),
            shadows: self.shadows,
            only_at_night: self.only_at_night,
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
                        (f64::from(Self::CONE_RANGE.0), f64::from(Self::CONE_RANGE.1)),
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
                        (0.0, f64::from(Self::MAX_SOFT_EDGE)),
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
                field::boolean(
                    "only_at_night",
                    "Only at night",
                    false,
                    "On at dusk, off at dawn, by the Environment's day and night: a street lamp.",
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
        data.set("only_at_night", Value::Bool(self.only_at_night));
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
            only_at_night: f.bool("only_at_night", false),
        }
        // Here a light's numbers come in (an area file, an assembly game
        // code spawns, a save, a change made while a game plays): put
        // right once, for everything that reads them. A cone below zero
        // used to crash the viewport.
        .sanitized()
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

    /// Whether every number of `light` is one a light can have.
    fn usable(light: &Light) -> bool {
        let (narrow, wide) = Light::CONE_RANGE;
        (narrow..=wide).contains(&light.cone)
            && (0.0..=Light::MAX_SOFT_EDGE).contains(&light.soft_edge)
            && light.strength.is_finite()
            && light.strength >= 0.0
            && light.range.is_finite()
            && light.range >= 0.0
            && light.color.iter().all(|c| (0.0..=1.0).contains(c))
    }

    /// Numbers a file, game code or a mistake might hold.
    const ODD: [f32; 12] = [
        -5.0,
        0.0,
        -0.0,
        0.2,
        30.0,
        89.0,
        500.0,
        -1.0e30,
        f32::MAX,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];

    #[test]
    fn a_light_read_from_values_is_always_usable() {
        // BUG-02: a cone below zero came through as written, and drawing
        // it crashed the editor.
        for cone in ODD {
            for soft_edge in ODD {
                for other in ODD {
                    let mut data = Light::spot().to_data();
                    data.set("cone", Value::Float(f64::from(cone)));
                    data.set("soft_edge", Value::Float(f64::from(soft_edge)));
                    data.set("strength", Value::Float(f64::from(other)));
                    data.set("range", Value::Float(f64::from(other)));
                    let light = Light::from_data(&data);
                    assert!(usable(&light), "{cone} {soft_edge} {other}: {light:?}");
                }
            }
        }
        // A number too big for a light's own kind of number, and a value
        // of the wrong kind.
        let mut data = Light::spot().to_data();
        data.set("cone", Value::Float(-1.0e300));
        data.set("soft_edge", Value::Text("wide".into()));
        assert!(usable(&Light::from_data(&data)));
    }

    #[test]
    fn a_light_made_in_code_is_put_right_and_a_good_one_is_left_alone() {
        for cone in ODD {
            for soft_edge in ODD {
                let wild = Light {
                    cone,
                    soft_edge,
                    strength: soft_edge,
                    range: cone,
                    color: [cone, soft_edge, 0.5],
                    ..Light::spot()
                }
                .sanitized();
                assert!(usable(&wild), "{cone} {soft_edge}: {wild:?}");
            }
        }
        let below = Light {
            cone: -5.0,
            soft_edge: -2.0,
            strength: -1.0,
            range: -4.0,
            ..Light::spot()
        }
        .sanitized();
        assert_eq!(
            (below.cone, below.soft_edge, below.strength, below.range),
            (Light::CONE_RANGE.0, 0.0, 0.0, 0.0)
        );
        let not_numbers = Light {
            cone: f32::NAN,
            soft_edge: f32::NAN,
            strength: f32::NAN,
            range: f32::INFINITY,
            ..Light::spot()
        }
        .sanitized();
        assert_eq!(not_numbers, Light::spot(), "each takes its standard");
        for good in [Light::sun(), Light::point(), Light::spot()] {
            assert_eq!(good.sanitized(), good);
            assert_eq!(Light::from_data(&good.to_data()), good);
        }
        // A soft edge wider than the cone is allowed: the whole cone fades.
        let soft = Light {
            cone: 10.0,
            soft_edge: 40.0,
            ..Light::spot()
        };
        assert_eq!(soft.sanitized(), soft);
    }
}
