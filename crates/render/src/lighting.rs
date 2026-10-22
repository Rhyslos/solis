//! Light: the sun with its shadows, and lamps (point lights and
//! spotlights). Strengths are in Blender's units, so values match:
//! watts for lamps, watts per square metre for the sun
//! ([decision 0014](../../../docs/decisions/0014-lights-placed-two-ways.md)).
//!
//! The sun's shadows come from a shadow map: the models drawn once more,
//! from the sun's side, into a depth image covering the models, which the
//! shaders then look up to see whether each point can see the sun.

use crate::frame::srgb_to_linear;
use crate::renderer::LineVertex;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};

/// The most lamps drawn at once; further ones are left out (nearest the
/// camera are kept).
pub const MAX_LIGHTS: usize = 16;
/// The shadow map's width and height, in texels.
pub const SHADOW_MAP_SIZE: u32 = 2048;
/// The shadow map's format.
pub const SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// The sun: parallel light from far away.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sun {
    /// The way its light travels (from the sky towards the ground).
    pub direction: Vec3,
    /// Its colour, sRGB 0 to 1.
    pub color: [f32; 3],
    /// Its strength in watts per square metre (Blender's unit; 3 is a
    /// clear day in the editor's light).
    pub strength: f32,
    /// Whether it casts shadows.
    pub shadows: bool,
}

impl Default for Sun {
    fn default() -> Self {
        Self {
            direction: -Vec3::new(0.35, -0.55, 0.76).normalize(),
            color: [1.0, 0.98, 0.95],
            strength: 3.0,
            shadows: true,
        }
    }
}

/// What kind of lamp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LightKind {
    /// Light in every direction from a point.
    Point,
    /// A cone of light; angles in radians from its middle: full strength
    /// inside `inner`, fading to nothing at `outer`.
    Spot {
        /// Where the fade starts.
        inner: f32,
        /// Where the light ends.
        outer: f32,
    },
}

/// A lamp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    /// Point or spot.
    pub kind: LightKind,
    /// Where it is, in metres.
    pub position: Vec3,
    /// The way a spotlight shines.
    pub direction: Vec3,
    /// Its colour, sRGB 0 to 1.
    pub color: [f32; 3],
    /// Its strength in watts (Blender's unit).
    pub strength: f32,
    /// How far it reaches, in metres; `None` fades forever.
    pub range: Option<f32>,
}

/// One lamp exactly as the shader's `Light` struct expects it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct LightUniform {
    /// Position (xyz); w: range, 0 for none.
    position: [f32; 4],
    /// Linear colour times strength (rgb); w: 0 point, 1 spot.
    color: [f32; 4],
    /// Spot direction (xyz).
    direction: [f32; 4],
    /// x: cosine of the inner angle; y: cosine of the outer angle.
    cone: [f32; 4],
}

/// All lamps for one frame, as the shader's `Lights` struct expects them.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct LightsUniform {
    lights: [LightUniform; MAX_LIGHTS],
}

fn linear(c: [f32; 3]) -> [f32; 3] {
    c.map(srgb_to_linear)
}

fn finite_or(v: f32, fallback: f32) -> f32 {
    if v.is_finite() { v } else { fallback }
}

impl LightsUniform {
    /// The lamps nearest `camera`, up to [`MAX_LIGHTS`], and how many.
    pub(crate) fn new(lights: &[Light], camera: Vec3) -> (Self, u32) {
        let mut usable: Vec<&Light> = lights
            .iter()
            .filter(|l| l.position.is_finite() && l.strength.is_finite() && l.strength > 0.0)
            .collect();
        usable.sort_by(|a, b| {
            a.position
                .distance_squared(camera)
                .total_cmp(&b.position.distance_squared(camera))
        });
        let mut out = Self::zeroed();
        let count = usable.len().min(MAX_LIGHTS);
        for (slot, light) in out.lights.iter_mut().zip(&usable) {
            let [r, g, b] = linear(light.color);
            let strength = light.strength.clamp(0.0, 1.0e6);
            let (kind, inner, outer) = match light.kind {
                LightKind::Point => (0.0, 0.0, 0.0),
                LightKind::Spot { inner, outer } => {
                    let outer = finite_or(outer, 0.785).clamp(0.01, 1.57);
                    let inner = finite_or(inner, 0.0).clamp(0.0, outer);
                    (1.0, inner, outer)
                }
            };
            let direction = light.direction.try_normalize().unwrap_or(-Vec3::Z);
            *slot = LightUniform {
                position: light
                    .position
                    .extend(
                        light
                            .range
                            .filter(|r| r.is_finite() && *r > 0.0)
                            .unwrap_or(0.0),
                    )
                    .to_array(),
                color: [r * strength, g * strength, b * strength, kind],
                direction: direction.extend(0.0).to_array(),
                cone: [inner.cos(), outer.cos(), 0.0, 0.0],
            };
        }
        (out, count as u32)
    }
}

/// The sun's light as the shader's frame data holds it: the direction
/// towards the sun (xyz, w: 1 when shadows are on), and its linear colour
/// times strength.
pub(crate) fn sun_uniforms(sun: &Sun, shadows_drawn: bool) -> ([f32; 4], [f32; 4]) {
    let towards = (-sun.direction).try_normalize().unwrap_or(Vec3::Z);
    let strength = finite_or(sun.strength, 0.0).clamp(0.0, 1000.0);
    let [r, g, b] = linear(sun.color);
    (
        towards
            .extend(if shadows_drawn { 1.0 } else { 0.0 })
            .to_array(),
        [r * strength, g * strength, b * strength, 0.0],
    )
}

/// A right-handed look-at transform (camera looks down its own -Z).
fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Mat4 {
    let forward = (target - eye).normalize_or(-Vec3::Z);
    let up = if forward.cross(up).length_squared() < 1e-6 {
        Vec3::Y
    } else {
        up
    };
    let right = forward.cross(up).normalize_or(Vec3::X);
    let up = right.cross(forward);
    Mat4::from_cols(
        Vec4::new(right.x, up.x, -forward.x, 0.0),
        Vec4::new(right.y, up.y, -forward.y, 0.0),
        Vec4::new(right.z, up.z, -forward.z, 0.0),
        Vec4::new(-right.dot(eye), -up.dot(eye), forward.dot(eye), 1.0),
    )
}

/// The sun's view of the box `min`–`max`, for the shadow map: an
/// orthographic view along the sun's light that just covers the box, depth
/// 0 to 1. Also returns the width one shadow-map texel covers, in metres.
pub fn sun_view_projection(sun: &Sun, min: Vec3, max: Vec3) -> (Mat4, f32) {
    let center = (min + max) * 0.5;
    let radius = ((max - min).length() * 0.5).max(0.5);
    let direction = sun.direction.try_normalize().unwrap_or(-Vec3::Z);
    let eye = center - direction * radius * 2.0;
    let view = look_at(eye, center, Vec3::Z);
    let (near, far) = (0.0, radius * 4.0);
    let projection = Mat4::from_cols(
        Vec4::new(1.0 / radius, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 1.0 / radius, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0 / (near - far), 0.0),
        Vec4::new(0.0, 0.0, near / (near - far), 1.0),
    );
    (projection * view, 2.0 * radius / SHADOW_MAP_SIZE as f32)
}

/// The most line ends one lamp's marker uses (a cross, and a spotlight's
/// line).
pub(crate) const MARKER_VERTICES: usize = 8;

/// Small markers showing where lamps are: a cross in the lamp's colour,
/// and for a spotlight a line the way it shines.
pub fn light_markers(lights: &[Light]) -> Vec<LineVertex> {
    let mut lines = Vec::new();
    for light in lights.iter().filter(|l| l.position.is_finite()) {
        let color = {
            let [r, g, b] = linear(light.color);
            [r, g, b, 1.0]
        };
        let p = light.position;
        let s = 0.25;
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            for end in [p - axis * s, p + axis * s] {
                lines.push(LineVertex {
                    position: end.to_array(),
                    color,
                });
            }
        }
        if let LightKind::Spot { .. } = light.kind {
            let d = light.direction.try_normalize().unwrap_or(-Vec3::Z);
            for end in [p, p + d * 0.8] {
                lines.push(LineVertex {
                    position: end.to_array(),
                    color,
                });
            }
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lamp(position: Vec3) -> Light {
        Light {
            kind: LightKind::Point,
            position,
            direction: -Vec3::Z,
            color: [1.0, 1.0, 1.0],
            strength: 100.0,
            range: None,
        }
    }

    #[test]
    fn lights_match_the_shader_layout() {
        assert_eq!(std::mem::size_of::<LightUniform>(), 64);
        assert_eq!(std::mem::size_of::<LightsUniform>(), 64 * MAX_LIGHTS);
    }

    #[test]
    fn the_nearest_lamps_are_kept_when_there_are_too_many() {
        let lights: Vec<Light> = (0..40)
            .map(|i| lamp(Vec3::new(i as f32, 0.0, 0.0)))
            .collect();
        let (u, count) = LightsUniform::new(&lights, Vec3::new(39.0, 0.0, 0.0));
        assert_eq!(count as usize, MAX_LIGHTS);
        assert_eq!(u.lights[0].position[0], 39.0, "nearest first");
        assert!(u.lights.iter().all(|l| l.position[0] >= 24.0));
    }

    #[test]
    fn broken_lamps_are_left_out_and_odd_cones_tidied() {
        let mut broken = lamp(Vec3::NAN);
        broken.strength = 5.0;
        let spot = Light {
            kind: LightKind::Spot {
                inner: 2.0,
                outer: f32::NAN,
            },
            ..lamp(Vec3::ZERO)
        };
        let (u, count) = LightsUniform::new(&[broken, spot, lamp(Vec3::ONE)], Vec3::ZERO);
        assert_eq!(count, 2);
        let cone = u.lights[0].cone;
        assert!(cone[0] >= cone[1], "inner within outer");
        assert_eq!(u.lights[0].color[3], 1.0);
    }

    #[test]
    fn the_sun_points_towards_the_sky_in_the_shader() {
        let (towards, color) = sun_uniforms(&Sun::default(), true);
        assert!(towards[2] > 0.0, "the sun is above");
        assert_eq!(towards[3], 1.0);
        assert!(color[0] > 2.5 && color[0] < 3.1);
    }

    #[test]
    fn the_shadow_view_covers_the_whole_box() {
        let sun = Sun::default();
        let (min, max) = (Vec3::new(-3.0, -2.0, 0.0), Vec3::new(4.0, 5.0, 3.0));
        let (vp, texel) = sun_view_projection(&sun, min, max);
        assert!(texel > 0.0);
        for corner in 0..8 {
            let p = Vec3::new(
                if corner & 1 == 0 { min.x } else { max.x },
                if corner & 2 == 0 { min.y } else { max.y },
                if corner & 4 == 0 { min.z } else { max.z },
            );
            let clip = vp * p.extend(1.0);
            let ndc = clip.truncate() / clip.w;
            assert!(ndc.x.abs() <= 1.0 && ndc.y.abs() <= 1.0, "{p} -> {ndc}");
            assert!((0.0..=1.0).contains(&ndc.z), "{p} depth {}", ndc.z);
        }
        // Straight down works too (the sun's direction parallel to up).
        let noon = Sun {
            direction: -Vec3::Z,
            ..sun
        };
        assert!(sun_view_projection(&noon, min, max).0.is_finite());
    }

    #[test]
    fn markers_show_every_lamp() {
        let spot = Light {
            kind: LightKind::Spot {
                inner: 0.2,
                outer: 0.5,
            },
            ..lamp(Vec3::ZERO)
        };
        assert_eq!(light_markers(&[lamp(Vec3::ONE), spot]).len(), 6 + 8);
        assert!(light_markers(&[lamp(Vec3::NAN)]).is_empty());
    }
}
