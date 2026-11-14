//! Finding a brush's parts under the pointer or in a box, while its
//! insides are edited (Tools 1a, B3): its points, edges and faces in its
//! own space, worked out once each time the brush's mesh changes.
//!
//! Only the brush being edited is tried, never the area's other
//! triangles, and narrowly: the ray first against the brush's box, then
//! against each face's box, and only the faces it reaches triangle by
//! triangle. A point or an edge is found as the corner or side of the
//! face under the pointer nearest to it on screen, so what is picked is
//! always on the side facing the camera.

use super::edit::{Mode, Part};
use crate::screen::Ray;
use glam::{Mat4, Vec2, Vec3};
use mesh::{EdgeKey, FaceId, Mesh};
use std::sync::Arc;

/// One face's picking geometry, in the brush's own space.
#[derive(Debug, Clone, Copy)]
struct FacePlace {
    id: FaceId,
    lo: Vec3,
    hi: Vec3,
    /// Its middle (the average of its corners): faces mode shows a dot
    /// there, and a box picks the face when it holds it.
    middle: Vec3,
    /// Its triangles' place in [`PartCache::triangles`].
    first: u32,
    count: u32,
}

/// A brush's mesh, and what finding its parts needs, made once each time
/// the mesh changes.
#[derive(Debug)]
pub(crate) struct PartCache {
    /// The model it was made from: a new one means a new mesh.
    pub(crate) key: u64,
    mesh: Arc<Mesh>,
    lo: Vec3,
    hi: Vec3,
    faces: Vec<FacePlace>,
    triangles: Vec<[Vec3; 3]>,
}

impl PartCache {
    /// What finding `mesh`'s parts needs (its model's `key`).
    pub(crate) fn of(mesh: Arc<Mesh>, key: u64) -> Self {
        let mut faces = Vec::with_capacity(mesh.face_count());
        let mut triangles = Vec::with_capacity(mesh.face_count() * 2);
        for (id, corners) in mesh.faces() {
            let at = |i: usize| mesh.position(corners[i]).unwrap_or(Vec3::ZERO);
            let first = triangles.len() as u32;
            mesh::triangulate(corners.len(), at, |[a, b, c]| {
                triangles.push([at(a), at(b), at(c)]);
            });
            let points = (0..corners.len()).map(at);
            let (lo, hi) = points
                .clone()
                .fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |(lo, hi), p| {
                    (lo.min(p), hi.max(p))
                });
            let middle = points.sum::<Vec3>() / corners.len().max(1) as f32;
            faces.push(FacePlace {
                id,
                lo,
                hi,
                middle,
                first,
                count: triangles.len() as u32 - first,
            });
        }
        let (lo, hi) = faces
            .iter()
            .fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |(lo, hi), f| {
                (lo.min(f.lo), hi.max(f.hi))
            });
        Self {
            key,
            mesh,
            lo,
            hi,
            faces,
            triangles,
        }
    }

    /// The mesh.
    pub(crate) fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    /// The mesh, shared.
    pub(crate) fn shared(&self) -> Arc<Mesh> {
        Arc::clone(&self.mesh)
    }

    /// The face `ray` (in the brush's own space; its direction needn't be
    /// unit length) meets first, and how far along it.
    pub(crate) fn face_hit(&self, ray: Ray) -> Option<(FaceId, f32)> {
        slab(ray, self.lo, self.hi)?;
        let mut best: Option<(FaceId, f32)> = None;
        for face in &self.faces {
            let Some(near) = slab(ray, face.lo, face.hi) else {
                continue;
            };
            if best.is_some_and(|(_, t)| near > t) {
                continue;
            }
            let range = face.first as usize..(face.first + face.count) as usize;
            for triangle in &self.triangles[range] {
                if let Some(t) = triangle_hit(ray, triangle)
                    && best.is_none_or(|(_, b)| t < b)
                {
                    best = Some((face.id, t));
                }
            }
        }
        best
    }

    /// The part of `mode` under `mouse` (points from the view's corner):
    /// the face hit, or its corner or side nearest the mouse on screen.
    /// `world` places the brush; `project` puts a world point on screen;
    /// `ray` is the pointer's, in the world.
    pub(crate) fn under(
        &self,
        mode: Mode,
        (world, ray): (Mat4, Ray),
        project: &dyn Fn(Vec3) -> Option<Vec2>,
        mouse: Vec2,
    ) -> Option<Part> {
        let own = world.inverse();
        if !own.is_finite() {
            return None;
        }
        let local = Ray {
            origin: own.transform_point3(ray.origin),
            direction: own.transform_vector3(ray.direction),
        };
        let (face, _) = self.face_hit(local)?;
        let corners = self.mesh.face(face)?;
        let on_screen = |v: mesh::VertexId| {
            self.mesh
                .position(v)
                .and_then(|p| project(world.transform_point3(p)))
        };
        match mode {
            Mode::Faces => Some(Part::Face(face)),
            Mode::Points => corners
                .iter()
                .filter_map(|v| on_screen(*v).map(|s| (s.distance(mouse), *v)))
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, v)| Part::Point(v)),
            Mode::Edges => (0..corners.len())
                .filter_map(|i| {
                    let (a, b) = (corners[i], corners[(i + 1) % corners.len()]);
                    let d = to_segment(mouse, on_screen(a)?, on_screen(b)?);
                    Some((d, EdgeKey::new(a, b)))
                })
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, e)| Part::Edge(e)),
        }
    }

    /// Every part of `mode` whose place (a point, an edge's middle, a
    /// face's middle) is on screen between `min` and `max`, through the
    /// brush: a box picks the far side too.
    pub(crate) fn in_box(
        &self,
        mode: Mode,
        world: Mat4,
        project: &dyn Fn(Vec3) -> Option<Vec2>,
        (min, max): (Vec2, Vec2),
    ) -> Vec<Part> {
        let inside = |p: Vec3| {
            project(world.transform_point3(p))
                .is_some_and(|s| s.cmpge(min).all() && s.cmple(max).all())
        };
        match mode {
            Mode::Points => self
                .mesh
                .vertices()
                .filter(|(_, p)| inside(*p))
                .map(|(v, _)| Part::Point(v))
                .collect(),
            Mode::Edges => self
                .mesh
                .edges()
                .iter()
                .filter(|e| {
                    let (a, b) = e.ends();
                    match (self.mesh.position(a), self.mesh.position(b)) {
                        (Some(p), Some(q)) => inside((p + q) * 0.5),
                        _ => false,
                    }
                })
                .map(|e| Part::Edge(*e))
                .collect(),
            Mode::Faces => self
                .faces
                .iter()
                .filter(|f| inside(f.middle))
                .map(|f| Part::Face(f.id))
                .collect(),
        }
    }
}

/// Where `ray` enters the box from `lo` to `hi` (0 if it starts inside),
/// or `None` if it misses.
fn slab(ray: Ray, lo: Vec3, hi: Vec3) -> Option<f32> {
    let inverse = ray.direction.recip();
    let a = (lo - ray.origin) * inverse;
    let b = (hi - ray.origin) * inverse;
    // A ray along a face of the box gives "not a number" there: that axis
    // keeps nothing out.
    let pick = |x: f32, fallback: f32| if x.is_nan() { fallback } else { x };
    let near = a.min(b);
    let far = a.max(b);
    let enter = pick(near.x, f32::NEG_INFINITY)
        .max(pick(near.y, f32::NEG_INFINITY))
        .max(pick(near.z, f32::NEG_INFINITY));
    let leave = pick(far.x, f32::INFINITY)
        .min(pick(far.y, f32::INFINITY))
        .min(pick(far.z, f32::INFINITY));
    (leave >= enter.max(0.0)).then_some(enter.max(0.0))
}

/// How far along `ray` it meets `triangle` (from either side), if it
/// does (Möller and Trumbore's test).
fn triangle_hit(ray: Ray, [a, b, c]: &[Vec3; 3]) -> Option<f32> {
    let (ab, ac) = (*b - *a, *c - *a);
    let p = ray.direction.cross(ac);
    let det = ab.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inverse = 1.0 / det;
    let s = ray.origin - *a;
    let u = s.dot(p) * inverse;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(ab);
    let v = ray.direction.dot(q) * inverse;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = ac.dot(q) * inverse;
    (t >= 0.0 && t.is_finite()).then_some(t)
}

/// How far `p` is from the segment from `a` to `b`.
pub(crate) fn to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let t = if ab.length_squared() > 0.0 {
        ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}

#[cfg(test)]
#[path = "parts_tests.rs"]
mod tests;
