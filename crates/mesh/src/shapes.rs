//! The shapes a brush starts as: Box, Ramp, Cylinder, Stairs, Arch,
//! Sphere and Wedge, each built from a few settings ([`ShapeSettings`]).
//!
//! Every shape fills the box its `size` gives (metres: across X, deep
//! along Y, up Z), standing on the ground with its pivot at the middle of
//! its base, so a brush drawn out on the ground is that box. Every one is
//! closed and faces outward, so it is solid to physics and drawn from
//! outside. Settings are brought within limits first ([`ShapeSettings::clamped`]),
//! so no setting can ask for a shape too small to see or too big to build.
//!
//! Five shapes are a profile drawn across X and Z and pushed out along Y
//! (a prism): the Box's is a rectangle, the Ramp's a slope, the Wedge's a
//! roof, the Stairs' a staircase and the Arch's a doorway. The Cylinder
//! and Sphere are the [primitives](crate::primitives) of the same name,
//! stretched to fit.
//!
//! Building the same settings always gives the same mesh, ids included.

use crate::primitives::{CylinderParams, Primitive, SphereParams};
use crate::{Mesh, VertexId};
use glam::{Vec2, Vec3};
use std::f32::consts::PI;

/// The smallest size along any side, in metres (1 cm).
pub const MIN_SIZE: f32 = 0.01;
/// The largest size along any side, in metres (10 km).
pub const MAX_SIZE: f32 = 10_000.0;
/// The fewest sides a round shape has.
pub const MIN_SIDES: u32 = 3;
/// The most sides a round shape has: enough to look round at any size a
/// brush is used at, and a sphere of them is 2,048 faces.
pub const MAX_SIDES: u32 = 64;
/// The most steps a flight of stairs has.
pub const MAX_STEPS: u32 = 64;

/// What a brush was made as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Shape {
    /// A box.
    Box,
    /// A slope, low at the front (-Y) and full height at the back.
    Ramp,
    /// An upright cylinder with flat ends.
    Cylinder,
    /// A flight of stairs climbing from the front (-Y) to the back.
    Stairs,
    /// A block with a doorway through it, front to back.
    Arch,
    /// A ball.
    Sphere,
    /// A roof: two slopes meeting in a ridge along Y.
    Wedge,
}

impl Shape {
    /// Every shape, in the order the brush list shows them.
    pub const ALL: [Self; 7] = [
        Self::Box,
        Self::Ramp,
        Self::Cylinder,
        Self::Stairs,
        Self::Arch,
        Self::Sphere,
        Self::Wedge,
    ];

    /// Its name, as shown and as written in area files.
    pub fn name(self) -> &'static str {
        match self {
            Self::Box => "Box",
            Self::Ramp => "Ramp",
            Self::Cylinder => "Cylinder",
            Self::Stairs => "Stairs",
            Self::Arch => "Arch",
            Self::Sphere => "Sphere",
            Self::Wedge => "Wedge",
        }
    }

    /// The shape with this name, if there is one.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.name() == name)
    }

    /// Its settings when nothing else is said.
    pub fn standard(self) -> ShapeSettings {
        let size = match self {
            Self::Box | Self::Cylinder | Self::Sphere => Vec3::splat(2.0),
            Self::Ramp => Vec3::new(2.0, 4.0, 1.0),
            Self::Stairs => Vec3::new(2.0, 3.0, 1.5),
            Self::Arch => Vec3::new(3.0, 1.0, 3.0),
            Self::Wedge => Vec3::new(2.0, 2.0, 1.0),
        };
        ShapeSettings {
            size,
            ..ShapeSettings::default()
        }
    }

    /// The shape built from `settings` (brought within limits first).
    pub fn build(self, settings: &ShapeSettings) -> Mesh {
        let s = settings.clamped();
        let (w, d, h) = (s.size.x, s.size.y, s.size.z);
        match self {
            Self::Box => prism(&rectangle(w, h), d),
            Self::Ramp => prism(
                &[
                    Vec2::new(-d / 2.0, 0.0),
                    Vec2::new(d / 2.0, 0.0),
                    Vec2::new(d / 2.0, h),
                ],
                w,
            )
            .turned_to_climb_along_y(),
            Self::Wedge => prism(
                &[
                    Vec2::new(-w / 2.0, 0.0),
                    Vec2::new(w / 2.0, 0.0),
                    Vec2::new(0.0, h),
                ],
                d,
            ),
            Self::Stairs => prism(&staircase(d, h, s.steps), w).turned_to_climb_along_y(),
            Self::Arch => prism(&doorway(w, h, s.sides, s.roundness), d),
            Self::Cylinder => stretched(
                Primitive::Cylinder(CylinderParams {
                    radius: 50.0,
                    height: 100.0,
                    sides: s.sides,
                    height_segments: 1,
                    caps: true,
                    cap_segments: 1,
                    cap_centre: false,
                }),
                s.size,
            ),
            Self::Sphere => stretched(
                Primitive::Sphere(SphereParams {
                    radius: 50.0,
                    sides: s.sides,
                    rings: (s.sides / 2).max(2),
                }),
                s.size,
            ),
        }
    }
}

/// How a shape is built: its size, and the settings some shapes use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapeSettings {
    /// Across X, deep along Y and up Z, in metres.
    pub size: Vec3,
    /// Sides around a Cylinder or Sphere, or pieces of an Arch's curve.
    pub sides: u32,
    /// Steps of the Stairs.
    pub steps: u32,
    /// How round an Arch's top is, 0 (a flat lintel) to 1 (a half
    /// circle).
    pub roundness: f32,
}

impl Default for ShapeSettings {
    fn default() -> Self {
        Self {
            size: Vec3::splat(2.0),
            sides: 16,
            steps: 6,
            roundness: 1.0,
        }
    }
}

impl ShapeSettings {
    /// The same settings within limits: each side of the size from
    /// [`MIN_SIZE`] to [`MAX_SIZE`], [`MIN_SIDES`] to [`MAX_SIDES`] sides,
    /// 1 to [`MAX_STEPS`] steps, a roundness of 0 to 1. A number that isn't
    /// one takes the standard.
    #[must_use]
    pub fn clamped(self) -> Self {
        let standard = Self::default();
        let side = |v: f32, s: f32| {
            if v.is_finite() {
                v.clamp(MIN_SIZE, MAX_SIZE)
            } else {
                s
            }
        };
        Self {
            size: Vec3::new(
                side(self.size.x, standard.size.x),
                side(self.size.y, standard.size.y),
                side(self.size.z, standard.size.z),
            ),
            sides: self.sides.clamp(MIN_SIDES, MAX_SIDES),
            steps: self.steps.clamp(1, MAX_STEPS),
            roundness: if self.roundness.is_finite() {
                self.roundness.clamp(0.0, 1.0)
            } else {
                standard.roundness
            },
        }
    }
}

/// A rectangle `w` across and `h` high, standing on the ground, counter-
/// clockwise.
fn rectangle(w: f32, h: f32) -> Vec<Vec2> {
    vec![
        Vec2::new(-w / 2.0, 0.0),
        Vec2::new(w / 2.0, 0.0),
        Vec2::new(w / 2.0, h),
        Vec2::new(-w / 2.0, h),
    ]
}

/// A staircase's side, `d` deep and `h` high, climbing towards +X in
/// `steps` steps, counter-clockwise: along the ground, up the back, then
/// down each step's tread and riser to the front.
fn staircase(d: f32, h: f32, steps: u32) -> Vec<Vec2> {
    let (run, rise) = (d / steps as f32, h / steps as f32);
    let mut out = vec![Vec2::new(-d / 2.0, 0.0), Vec2::new(d / 2.0, 0.0)];
    for step in (1..=steps).rev() {
        let top = rise * step as f32;
        let back = -d / 2.0 + run * step as f32;
        out.push(Vec2::new(back, top));
        out.push(Vec2::new(back - run, top));
    }
    out
}

/// A doorway's outline, `w` across and `h` high, counter-clockwise: a
/// block with an opening from the ground, half its width across, whose top
/// is a curve of `pieces` (round by `roundness`) on straight sides.
fn doorway(w: f32, h: f32, pieces: u32, roundness: f32) -> Vec<Vec2> {
    let half = w / 4.0;
    // The opening reaches three quarters of the height; its curve takes
    // up to its half width of that.
    let top = h * 0.75;
    let rise = (half * roundness).min(top * 0.5);
    let spring = top - rise;
    let mut out = vec![
        Vec2::new(-w / 2.0, 0.0),
        Vec2::new(-half, 0.0),
        Vec2::new(-half, spring),
    ];
    if rise > 0.0 {
        // From the left side over to the right: round, clockwise in the
        // outline (it is the inside of the block).
        for i in 1..pieces {
            let a = PI * (1.0 - i as f32 / pieces as f32);
            out.push(Vec2::new(half * a.cos(), spring + rise * a.sin()));
        }
    }
    out.extend([
        Vec2::new(half, spring),
        Vec2::new(half, 0.0),
        Vec2::new(w / 2.0, 0.0),
        Vec2::new(w / 2.0, h),
        Vec2::new(-w / 2.0, h),
    ]);
    out
}

/// `outline` (across X, up Z, counter-clockwise seen from the front, -Y)
/// pushed out `depth` along Y, centred on Y = 0: a face at each end and a
/// four-sided face along each side of the outline.
fn prism(outline: &[Vec2], depth: f32) -> Mesh {
    let mut mesh = Mesh::new();
    let n = outline.len();
    let point = |mesh: &mut Mesh, p: Vec2, y: f32| mesh.push_vertex(Vec3::new(p.x, y, p.y));
    let front: Vec<VertexId> = outline
        .iter()
        .map(|p| point(&mut mesh, *p, -depth / 2.0))
        .collect();
    let back: Vec<VertexId> = outline
        .iter()
        .map(|p| point(&mut mesh, *p, depth / 2.0))
        .collect();
    mesh.push_face(&front);
    let reversed: Vec<VertexId> = back.iter().rev().copied().collect();
    mesh.push_face(&reversed);
    for i in 0..n {
        let j = (i + 1) % n;
        mesh.push_face(&[front[i], back[i], back[j], front[j]]);
    }
    mesh
}

/// A primitive (built 100 units across) stretched to fill `size`, without
/// the hard edges it marks for smooth shading: brushes are drawn flat.
fn stretched(primitive: Primitive, size: Vec3) -> Mesh {
    let mut mesh = primitive.mesh();
    mesh.remove_attribute(crate::attributes::HARD);
    let scale = size / 100.0;
    let points: Vec<(VertexId, Vec3)> = mesh.vertices().collect();
    for (id, at) in points {
        mesh.set_position(id, at * scale);
    }
    mesh
}

impl Mesh {
    /// The mesh turned a quarter round Z, so a profile drawn across X
    /// (climbing towards +X) climbs along +Y, away from the front: X
    /// becomes Y and Y becomes -X. A turn keeps every face facing out.
    fn turned_to_climb_along_y(mut self) -> Self {
        let points: Vec<(VertexId, Vec3)> = self.vertices().collect();
        for (id, at) in points {
            self.set_position(id, Vec3::new(-at.y, at.x, at.z));
        }
        self
    }
}

#[cfg(test)]
#[path = "shapes_tests.rs"]
mod tests;
