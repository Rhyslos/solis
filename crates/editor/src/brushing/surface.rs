//! What a brush's footprint is drawn on: the top of the brush (or the
//! box of the model) under the pointer as a drag starts, found once by a
//! narrow test, never frame after frame.
//!
//! Each brush's box (its triangles' bounds, placed) is tried first, a few
//! numbers each; only the brushes whose box the ray goes through have
//! their triangles tried, nearest box first, and the search stops once a
//! hit is nearer than the next box. Ten thousand brushes cost a walk over
//! their boxes, once, as the button goes down.

use crate::area_view::AreaView;
use crate::screen::Ray;
use glam::{Mat4, Vec3};

/// The height of the first solid thing `ray` meets among `area`'s brushes
/// and models: the point it meets, or none if it meets nothing.
pub(crate) fn height_under(ray: &Ray, area: &AreaView) -> Option<f32> {
    first_hit(ray, area).map(|(_, at)| at.z)
}

/// How far along `ray` it first meets a brush's triangle or a model's
/// box, and where.
pub(crate) fn first_hit(ray: &Ray, area: &AreaView) -> Option<(f32, Vec3)> {
    // Every brush whose box the ray goes through, by how far.
    let mut boxes: Vec<(f32, usize)> = area
        .brushes
        .iter()
        .enumerate()
        .filter_map(|(n, brush)| {
            let bounds = &brush.model.model.bounds;
            let (min, max) = placed_box(brush.transform, bounds.min, bounds.max);
            enters(ray, min, max).map(|d| (d, n))
        })
        .collect();
    boxes.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut best: Option<f32> = None;
    for (entry, n) in boxes {
        if best.is_some_and(|b| b <= entry) {
            break;
        }
        let brush = &area.brushes[n];
        if let Some(d) = hit_triangles(ray, brush.transform, &brush.model.model)
            && best.is_none_or(|b| d < b)
        {
            best = Some(d);
        }
    }
    // A model's surface isn't known here, only its box.
    for model in &area.models {
        if let Some((min, max)) = model.bounds {
            let (min, max) = placed_box(model.transform, min, max);
            if let Some(d) = enters(ray, min, max)
                && best.is_none_or(|b| d < b)
            {
                best = Some(d);
            }
        }
    }
    best.map(|d| (d, ray.at(d)))
}

/// The world's box around the box `min`..`max` placed by `transform`.
fn placed_box(transform: Mat4, min: Vec3, max: Vec3) -> (Vec3, Vec3) {
    let mut lo = Vec3::splat(f32::INFINITY);
    let mut hi = Vec3::splat(f32::NEG_INFINITY);
    for i in 0..8 {
        let corner = Vec3::new(
            if i & 1 == 0 { min.x } else { max.x },
            if i & 2 == 0 { min.y } else { max.y },
            if i & 4 == 0 { min.z } else { max.z },
        );
        let at = transform.transform_point3(corner);
        lo = lo.min(at);
        hi = hi.max(at);
    }
    (lo, hi)
}

/// How far along `ray` it enters the box (0 if it starts inside), or none
/// if it misses it.
fn enters(ray: &Ray, min: Vec3, max: Vec3) -> Option<f32> {
    if !(min.is_finite() && max.is_finite()) {
        return None;
    }
    let mut near = 0.0_f32;
    let mut far = f32::INFINITY;
    for axis in 0..3 {
        let (o, d) = (ray.origin[axis], ray.direction[axis]);
        if d.abs() < 1e-9 {
            if o < min[axis] || o > max[axis] {
                return None;
            }
            continue;
        }
        let (a, b) = ((min[axis] - o) / d, (max[axis] - o) / d);
        near = near.max(a.min(b));
        far = far.min(a.max(b));
        if near > far {
            return None;
        }
    }
    Some(near)
}

/// How far along `ray` it first meets one of `model`'s triangles, placed
/// by `transform`. The ray is taken into the model's own space, its
/// direction not made unit length, so a distance there is one in the
/// world.
fn hit_triangles(ray: &Ray, transform: Mat4, model: &assets::Model) -> Option<f32> {
    let inverse = transform.inverse();
    if !inverse.is_finite() {
        return None;
    }
    let origin = inverse.transform_point3(ray.origin);
    let direction = inverse.transform_vector3(ray.direction);
    let mut best: Option<f32> = None;
    for mesh in &model.meshes {
        let point = |i: u32| {
            mesh.vertices
                .get(i as usize)
                .map(|v| Vec3::from_array(v.position))
        };
        for corners in mesh.indices.chunks_exact(3) {
            let (Some(a), Some(b), Some(c)) =
                (point(corners[0]), point(corners[1]), point(corners[2]))
            else {
                continue;
            };
            if let Some(d) = through_triangle(origin, direction, [a, b, c])
                && best.is_none_or(|b| d < b)
            {
                best = Some(d);
            }
        }
    }
    best
}

/// How far along the ray from `origin` going `direction` it goes through
/// the triangle, from either side (Möller and Trumbore's test).
fn through_triangle(origin: Vec3, direction: Vec3, [a, b, c]: [Vec3; 3]) -> Option<f32> {
    let (ab, ac) = (b - a, c - a);
    let p = direction.cross(ac);
    let det = ab.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let t = origin - a;
    let u = t.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = t.cross(ab);
    let v = direction.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let d = ac.dot(q) * inv;
    (d >= 0.0 && d.is_finite()).then_some(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn down_at(x: f32, y: f32) -> Ray {
        Ray {
            origin: Vec3::new(x, y, 50.0),
            direction: Vec3::NEG_Z,
        }
    }

    #[test]
    fn a_ray_through_a_triangle_finds_it_from_either_side() {
        let tri = [Vec3::ZERO, Vec3::X * 2.0, Vec3::Y * 2.0];
        let ray = down_at(0.5, 0.5);
        assert_eq!(through_triangle(ray.origin, ray.direction, tri), Some(50.0));
        let up = Ray {
            origin: Vec3::new(0.5, 0.5, -3.0),
            direction: Vec3::Z,
        };
        assert_eq!(through_triangle(up.origin, up.direction, tri), Some(3.0));
        assert_eq!(
            through_triangle(down_at(3.0, 3.0).origin, Vec3::NEG_Z, tri),
            None
        );
        // Edge on: never a hit, never a crash.
        assert_eq!(
            through_triangle(Vec3::new(-1.0, 0.5, 0.0), Vec3::X, tri),
            None
        );
    }

    #[test]
    fn a_box_is_entered_where_the_ray_reaches_it() {
        let (min, max) = (Vec3::new(-1.0, -1.0, 0.0), Vec3::new(1.0, 1.0, 2.0));
        assert_eq!(enters(&down_at(0.0, 0.0), min, max), Some(48.0));
        assert_eq!(enters(&down_at(5.0, 0.0), min, max), None);
        // Inside it: entered at once.
        let inside = Ray {
            origin: Vec3::new(0.0, 0.0, 1.0),
            direction: Vec3::X,
        };
        assert_eq!(enters(&inside, min, max), Some(0.0));
        // A box of numbers that aren't is missed, never a crash.
        assert_eq!(enters(&down_at(0.0, 0.0), Vec3::NAN, max), None);
    }

    #[test]
    fn a_placed_box_holds_every_turned_corner() {
        let turned = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_4);
        let (lo, hi) = placed_box(turned, Vec3::splat(-1.0), Vec3::splat(1.0));
        let r = 2.0_f32.sqrt();
        assert!((hi.x - r).abs() < 1e-5 && (lo.y + r).abs() < 1e-5);
    }
}
