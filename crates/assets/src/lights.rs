//! Lights carried in glTF files (the `KHR_lights_punctual` extension, which
//! Blender exports): suns, point lights and spotlights, placed and turned
//! into Solis's units.
//!
//! **Units.** Solis measures a lamp's strength in watts and the sun's in
//! watts per square metre, as Blender does, so numbers typed in either
//! match. glTF stores candela for lamps and lux for suns; both are turned
//! back using the 683 lumens per watt Blender's exporter uses.

use crate::model::ImportError;
use glam::{Mat3, Mat4, Vec3};

/// Lumens per watt, as Blender's glTF exporter converts them.
pub const LUMENS_PER_WATT: f32 = 683.0;

/// What kind of light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LampKind {
    /// Parallel light from far away, everywhere at once.
    Sun,
    /// Light shining out in every direction from a point.
    Point,
    /// A cone of light. Angles in radians from the middle: full strength
    /// inside `inner`, fading to nothing at `outer`.
    Spot {
        /// Where the fade starts.
        inner: f32,
        /// Where the light ends.
        outer: f32,
    },
}

/// A light from a model file.
#[derive(Debug, Clone, PartialEq)]
pub struct Lamp {
    /// Its name in the file, or "Light 1" and so on.
    pub name: String,
    /// What kind.
    pub kind: LampKind,
    /// Where it is, in metres (unused for a sun).
    pub position: Vec3,
    /// The way it shines (length 1; unused for a point light).
    pub direction: Vec3,
    /// Its colour, linear RGB 0 to 1.
    pub color: [f32; 3],
    /// Its strength: watts for point lights and spotlights, watts per
    /// square metre for a sun.
    pub strength: f32,
    /// How far it reaches, in metres, if the file says; otherwise it fades
    /// with distance forever.
    pub range: Option<f32>,
}

/// The most lights read from one file.
pub const MAX_LAMPS: usize = 256;

/// The lights placed in the file's scene.
pub(crate) fn read_lamps(
    nodes: &[(gltf::Node<'_>, Mat4)],
    warnings: &mut Vec<String>,
) -> Result<Vec<Lamp>, ImportError> {
    let mut lamps = Vec::new();
    for (node, world) in nodes {
        let Some(light) = node.light() else {
            continue;
        };
        if lamps.len() == MAX_LAMPS {
            warnings.push(format!("only the first {MAX_LAMPS} lights were read"));
            break;
        }
        let tidy = |v: f32, fallback: f32| if v.is_finite() { v } else { fallback };
        let intensity = tidy(light.intensity(), 1.0).max(0.0);
        let (kind, strength) = match light.kind() {
            gltf::khr_lights_punctual::Kind::Directional => {
                (LampKind::Sun, intensity / LUMENS_PER_WATT)
            }
            gltf::khr_lights_punctual::Kind::Point => (
                LampKind::Point,
                intensity * 4.0 * std::f32::consts::PI / LUMENS_PER_WATT,
            ),
            gltf::khr_lights_punctual::Kind::Spot {
                inner_cone_angle,
                outer_cone_angle,
            } => {
                let outer = tidy(outer_cone_angle, 0.785).clamp(0.01, 1.57);
                let inner = tidy(inner_cone_angle, 0.0).clamp(0.0, outer);
                (
                    LampKind::Spot { inner, outer },
                    intensity * 4.0 * std::f32::consts::PI / LUMENS_PER_WATT,
                )
            }
        };
        // glTF lights shine along their node's -Z.
        let direction = (Mat3::from_mat4(*world) * -Vec3::Z)
            .try_normalize()
            .unwrap_or(-Vec3::Z);
        let position = world.transform_point3(Vec3::ZERO);
        lamps.push(Lamp {
            name: light
                .name()
                .map_or_else(|| format!("Light {}", lamps.len() + 1), str::to_string),
            kind,
            position: if position.is_finite() {
                position
            } else {
                Vec3::ZERO
            },
            direction,
            color: light.color().map(|c| tidy(c, 1.0).clamp(0.0, 1.0)),
            strength: tidy(strength, 0.0).min(1.0e6),
            range: light.range().filter(|r| r.is_finite() && *r > 0.0),
        });
    }
    Ok(lamps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blender_watts_survive_the_trip_through_candela() {
        // Blender exports a 1000 W point light as 1000 * 683 / 4π candela.
        let candela = 1000.0 * LUMENS_PER_WATT / (4.0 * std::f32::consts::PI);
        let back = candela * 4.0 * std::f32::consts::PI / LUMENS_PER_WATT;
        assert!((back - 1000.0).abs() < 0.01);
    }
}
