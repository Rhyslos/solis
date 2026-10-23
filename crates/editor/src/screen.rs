//! Between the screen and the world: the ray through a point of the
//! viewport, where a world point appears on it, and how big a pixel is at
//! some depth. Picking, the gizmos and dropping things in the viewport all
//! use these. Points are in egui points from the viewport's top-left
//! corner; `size` is the viewport's size in points.

use crate::camera::Camera;
use glam::{Vec2, Vec3};

/// A line from a point, one way.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    /// Where it starts.
    pub origin: Vec3,
    /// Which way it points; length 1.
    pub direction: Vec3,
}

impl Ray {
    /// The point `distance` along the ray.
    pub fn at(&self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }

    /// Where the ray meets the ground (height `z`), if in front of it.
    pub fn hit_ground(&self, z: f32) -> Option<Vec3> {
        if self.direction.z.abs() < 1e-6 {
            return None;
        }
        let distance = (z - self.origin.z) / self.direction.z;
        (distance > 0.0 && distance.is_finite()).then(|| self.at(distance))
    }
}

fn aspect_of(size: Vec2) -> f32 {
    let size = size.max(Vec2::ONE);
    size.x / size.y
}

impl Camera {
    /// Whether the view is flat (orthographic). Perspective only, until
    /// flat views arrive with several viewports.
    pub fn is_flat(&self) -> bool {
        false
    }

    /// The ray from the camera through `point` of a viewport of `size`.
    pub fn ray_through(&self, point: Vec2, size: Vec2) -> Ray {
        let size = size.max(Vec2::ONE);
        let ndc_x = 2.0 * point.x / size.x - 1.0;
        let ndc_y = 1.0 - 2.0 * point.y / size.y;
        let half_height = (self.fov_y * 0.5).tan();
        let half_width = half_height * aspect_of(size);
        let direction = (self.forward()
            + self.right() * (ndc_x * half_width)
            + self.up() * (ndc_y * half_height))
            .normalize_or(self.forward());
        Ray {
            origin: self.position,
            direction,
        }
    }

    /// Where `point` in the world appears in a viewport of `size`, or
    /// `None` when it's behind the camera.
    pub fn project(&self, point: Vec3, size: Vec2) -> Option<Vec2> {
        let size = size.max(Vec2::ONE);
        let clip = self.view_projection(aspect_of(size)) * point.extend(1.0);
        if clip.w <= 1e-4 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        let screen = Vec2::new((ndc.x + 1.0) * 0.5 * size.x, (1.0 - ndc.y) * 0.5 * size.y);
        screen.is_finite().then_some(screen)
    }

    /// How many metres one point of the screen covers at `at`.
    pub fn world_per_pixel(&self, at: Vec3, size: Vec2) -> f32 {
        let height = size.y.max(1.0);
        let depth = (at - self.position).dot(self.forward()).max(1e-3);
        let per_pixel = 2.0 * depth * (self.fov_y * 0.5).tan() / height;
        if per_pixel.is_finite() {
            per_pixel.max(1e-6)
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: Vec2 = Vec2::new(800.0, 600.0);

    fn camera() -> Camera {
        let mut camera = Camera {
            position: Vec3::new(3.0, -10.0, 4.0),
            ..Camera::default()
        };
        camera.look_at(Vec3::ZERO);
        camera
    }

    #[test]
    fn a_point_projects_back_onto_its_ray() {
        let camera = camera();
        for p in [
            Vec3::ZERO,
            Vec3::new(2.0, 1.0, 0.5),
            Vec3::new(-3.0, 4.0, 2.0),
        ] {
            let screen = camera.project(p, SIZE).unwrap();
            let ray = camera.ray_through(screen, SIZE);
            let along = (p - ray.origin).dot(ray.direction);
            assert!(ray.at(along).distance(p) < 1e-3, "{p}");
        }
    }

    #[test]
    fn what_the_camera_looks_at_is_in_the_middle() {
        let screen = camera().project(Vec3::ZERO, SIZE).unwrap();
        assert!(screen.distance(SIZE / 2.0) < 1e-2, "{screen}");
    }

    #[test]
    fn behind_the_camera_has_no_place_on_screen() {
        let camera = camera();
        let behind = camera.position - camera.forward() * 5.0;
        assert_eq!(camera.project(behind, SIZE), None);
    }

    #[test]
    fn the_ground_is_hit_in_front_only() {
        let camera = camera();
        let down = camera.ray_through(SIZE / 2.0, SIZE);
        assert!(down.hit_ground(0.0).unwrap().length() < 1e-3);
        let up = Ray {
            origin: Vec3::new(0.0, 0.0, 2.0),
            direction: Vec3::Z,
        };
        assert_eq!(up.hit_ground(0.0), None);
    }

    #[test]
    fn a_pixel_covers_more_further_away() {
        let camera = camera();
        let near = camera.world_per_pixel(Vec3::ZERO, SIZE);
        let far = camera.world_per_pixel(camera.forward() * 100.0, SIZE);
        assert!(far > near && near > 0.0);
    }
}
