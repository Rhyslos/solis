//! What game code and the editor ask of a ground (decision 0116): its
//! height at a place, the kind at a place, and where a ray meets it.
//! Each answers [`Answer::Unknown`] when a chunk it needs isn't loaded,
//! never "air" (decision 0117).

use crate::ground::{Answer, Ground};
use crate::store::Store;
use glam::{IVec3, Vec3};

/// Where a ray met the ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    /// The place, in metres.
    pub place: Vec3,
    /// The ground's facing there (towards the air).
    pub normal: Vec3,
    /// How far along the ray, in metres.
    pub distance: f32,
}

/// How finely a ray steps before closing in on the surface, in metres.
const RAY_STEP: f32 = 0.25;
/// The longest ray asked about, in metres.
pub const MAX_REACH: f32 = 10_000.0;

impl<S: Store> Ground<S> {
    /// The distance from the surface at `p` (metres; negative inside),
    /// blended between the eight voxels round it.
    pub fn distance_at(&self, p: Vec3) -> Answer<f32> {
        let base = p.floor();
        let f = p - base;
        let b = base.as_ivec3();
        let mut d = [0f32; 8];
        for (i, slot) in d.iter_mut().enumerate() {
            let c = IVec3::new((i & 1) as i32, ((i >> 1) & 1) as i32, ((i >> 2) & 1) as i32);
            match self.voxel(b + c) {
                Answer::Known(v) => *slot = v.distance(),
                Answer::Unknown => return Answer::Unknown,
            }
        }
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let x0 = lerp(d[0], d[1], f.x);
        let x1 = lerp(d[2], d[3], f.x);
        let x2 = lerp(d[4], d[5], f.x);
        let x3 = lerp(d[6], d[7], f.x);
        Answer::Known(lerp(lerp(x0, x1, f.y), lerp(x2, x3, f.y), f.z))
    }

    /// The height of the highest surface above the column at `(x, y)`,
    /// in metres; `None` past the ground's edge or where the column is
    /// all air.
    pub fn height_at(&self, x: f32, y: f32) -> Answer<Option<f32>> {
        let size = self.size();
        if !(x >= 0.0 && y >= 0.0 && x <= size.x as f32 && y <= size.y as f32) {
            return Answer::Known(None);
        }
        let mut above = match self.distance_at(Vec3::new(x, y, size.z as f32)) {
            Answer::Known(d) => d,
            Answer::Unknown => return Answer::Unknown,
        };
        for z in (0..size.z).rev() {
            let d = match self.distance_at(Vec3::new(x, y, z as f32)) {
                Answer::Known(d) => d,
                Answer::Unknown => return Answer::Unknown,
            };
            if d < 0.0 && above >= 0.0 {
                let t = d / (d - above);
                return Answer::Known(Some(z as f32 + t));
            }
            above = d;
        }
        Answer::Known(None)
    }

    /// The ground kind of the voxel nearest `p`, or `None` if it's air
    /// or past the ground's edge.
    pub fn kind_at(&self, p: Vec3) -> Answer<Option<u8>> {
        let size = self.size().as_vec3();
        if !p.is_finite() || p.cmplt(Vec3::ZERO).any() || p.cmpgt(size).any() {
            return Answer::Known(None);
        }
        match self.voxel(p.round().as_ivec3()) {
            Answer::Known(v) => Answer::Known(v.is_solid().then_some(v.kind)),
            Answer::Unknown => Answer::Unknown,
        }
    }

    /// Where the ray from `from` along `way` first meets the ground
    /// within `reach` metres (at most [`MAX_REACH`]).
    pub fn ray(&self, from: Vec3, way: Vec3, reach: f32) -> Answer<Option<Hit>> {
        let way = way.normalize_or_zero();
        if way == Vec3::ZERO || !from.is_finite() || !reach.is_finite() {
            return Answer::Known(None);
        }
        let Some((enter, leave)) = self.span_of(from, way, reach.clamp(0.0, MAX_REACH)) else {
            return Answer::Known(None);
        };
        let at = |t: f32| self.distance_at(from + way * t);
        let mut t = enter;
        let start = match at(t) {
            Answer::Known(d) => d,
            Answer::Unknown => return Answer::Unknown,
        };
        if start < 0.0 {
            return Answer::Known(Some(self.hit(from, way, t)));
        }
        while t < leave {
            let next = (t + RAY_STEP).min(leave);
            let d = match at(next) {
                Answer::Known(d) => d,
                Answer::Unknown => return Answer::Unknown,
            };
            if d < 0.0 {
                let (mut lo, mut hi) = (t, next);
                for _ in 0..12 {
                    let mid = (lo + hi) * 0.5;
                    match at(mid) {
                        Answer::Known(m) if m < 0.0 => hi = mid,
                        Answer::Known(_) => lo = mid,
                        Answer::Unknown => return Answer::Unknown,
                    }
                }
                return Answer::Known(Some(self.hit(from, way, hi)));
            }
            t = next;
        }
        Answer::Known(None)
    }

    /// The part of the ray inside the ground's box, clipped to `reach`.
    fn span_of(&self, from: Vec3, way: Vec3, reach: f32) -> Option<(f32, f32)> {
        let size = self.size().as_vec3();
        let (mut enter, mut leave) = (0.0f32, reach);
        for a in 0..3 {
            if way[a].abs() < 1e-9 {
                if from[a] < 0.0 || from[a] > size[a] {
                    return None;
                }
                continue;
            }
            let t0 = (0.0 - from[a]) / way[a];
            let t1 = (size[a] - from[a]) / way[a];
            enter = enter.max(t0.min(t1));
            leave = leave.min(t0.max(t1));
        }
        (enter <= leave).then_some((enter, leave))
    }

    fn hit(&self, from: Vec3, way: Vec3, t: f32) -> Hit {
        let place = from + way * t;
        let e = 0.5;
        let d = |o: Vec3| self.distance_at(place + o).known().unwrap_or(0.0);
        let normal = Vec3::new(
            d(Vec3::X * e) - d(-Vec3::X * e),
            d(Vec3::Y * e) - d(-Vec3::Y * e),
            d(Vec3::Z * e) - d(-Vec3::Z * e),
        )
        .normalize_or(Vec3::Z);
        Hit {
            place,
            normal,
            distance: t,
        }
    }
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
