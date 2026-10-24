//! The viewport camera, and how it flies
//! ([decision 0009](../../../docs/decisions/0009-fly-camera-controls.md)).
//!
//! The camera keeps a position and which way it faces (a heading and a
//! tilt). Each frame the viewport gathers what the mouse and keys did into a
//! [`FlyInput`] and hands it to [`Camera::fly`]; that split keeps all the
//! movement maths here, where tests can drive it without a window.

use glam::{Mat4, Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};

/// Camera speeds in metres per second, one per step shown in the viewport's
/// Camera box (step 1 is the first).
pub const SPEEDS: [f32; 8] = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0];
/// The speed step a new camera starts at.
pub const DEFAULT_SPEED: u8 = 4;
/// How many times faster the camera moves while Shift is held.
pub const BOOST: f32 = 3.0;
/// How far the camera turns per point the mouse moves, in radians.
pub const LOOK_PER_POINT: f32 = 0.004;
/// How far one wheel notch moves the camera, as seconds of travel at the
/// current speed.
pub const DOLLY_SECONDS_PER_NOTCH: f32 = 0.25;
/// The steepest the camera can look up or down, in radians (just short of
/// straight, where heading would stop meaning anything).
pub const MAX_TILT: f32 = 89.0 * std::f32::consts::PI / 180.0;
/// The longest frame movement is worked out for, in seconds, so a hitch
/// never throws the camera across the world.
pub const MAX_STEP: f32 = 0.1;
/// Furthest ahead the grid's centre is placed, in metres.
const MAX_FOCUS_DISTANCE: f32 = 1_000.0;

/// What a view shows: the world in perspective, or flat (orthographic)
/// from one side, where sizes don't shrink with distance, for lining
/// things up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ViewKind {
    /// In perspective, flown with the right mouse button.
    #[default]
    Perspective,
    /// Flat, looking straight down: X right, Y up the screen.
    Top,
    /// Flat, looking north (along +Y): X right, Z up.
    Front,
    /// Flat, looking west (along -X) from the right: Y right, Z up.
    Right,
}

impl ViewKind {
    /// Every kind, in the menu's order.
    pub const ALL: [Self; 4] = [Self::Perspective, Self::Top, Self::Front, Self::Right];

    /// Its name in the viewport's menu.
    pub fn name(self) -> &'static str {
        match self {
            Self::Perspective => "Perspective",
            Self::Top => "Top",
            Self::Front => "Front",
            Self::Right => "Right",
        }
    }

    /// Whether it's a flat view.
    pub fn is_flat(self) -> bool {
        self != Self::Perspective
    }

    /// A flat view's forward, right and up.
    fn axes(self) -> Option<(Vec3, Vec3, Vec3)> {
        match self {
            Self::Perspective => None,
            Self::Top => Some((Vec3::NEG_Z, Vec3::X, Vec3::Y)),
            Self::Front => Some((Vec3::Y, Vec3::X, Vec3::Z)),
            Self::Right => Some((Vec3::NEG_X, Vec3::Y, Vec3::Z)),
        }
    }

    /// The point of `p` on the plane a flat view looks at (through the
    /// origin, across its forward); `p` itself in perspective.
    pub fn onto_plane(self, p: Vec3) -> Vec3 {
        match self {
            Self::Perspective => p,
            Self::Top => Vec3::new(p.x, p.y, 0.0),
            Self::Front => Vec3::new(p.x, 0.0, p.z),
            Self::Right => Vec3::new(0.0, p.y, p.z),
        }
    }
}

/// How far in front of and behind its centre a flat view sees, in metres.
pub const FLAT_DEPTH: f32 = 2_500.0;
/// The least and most a flat view can show top to bottom, in metres.
pub const FLAT_HEIGHT_RANGE: std::ops::RangeInclusive<f32> = 0.05..=100_000.0;
/// How much one wheel notch zooms a flat view (less than 1: in).
pub const FLAT_ZOOM_PER_NOTCH: f32 = 0.85;

/// The viewport's camera. Z is up.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Camera {
    /// Where the camera is, in metres; for a flat view, the middle of
    /// what it shows.
    pub position: Vec3,
    /// Which way it faces across the ground, in radians: 0 looks along +Y,
    /// and the angle grows turning right (towards +X), like a compass.
    pub heading: f32,
    /// How far it looks up (positive) or down (negative), in radians.
    pub tilt: f32,
    /// Vertical field of view, in radians.
    pub fov_y: f32,
    /// The speed step, 1 to 8 (see [`SPEEDS`]).
    pub speed: u8,
    /// Perspective, or a flat view.
    pub kind: ViewKind,
    /// How much a flat view shows, top to bottom, in metres.
    pub flat_height: f32,
}

impl Default for Camera {
    fn default() -> Self {
        // Up and to the side of the origin, looking down at it.
        let mut camera = Self {
            position: Vec3::new(-16.0, -24.0, 10.0),
            heading: 0.0,
            tilt: 0.0,
            fov_y: 50f32.to_radians(),
            speed: DEFAULT_SPEED,
            kind: ViewKind::Perspective,
            flat_height: 20.0,
        };
        camera.look_at(Vec3::new(0.0, 0.0, 1.0));
        camera
    }
}

/// What the mouse and keys did to the camera in one frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FlyInput {
    /// Mouse movement while looking around, in points (x right, y down).
    pub look: Vec2,
    /// Keys held while looking around: x right, y forward, z up, each -1,
    /// 0 or 1.
    pub movement: Vec3,
    /// Shift held: move faster.
    pub boost: bool,
    /// Wheel notches while looking around: up is positive, and changes the
    /// speed step.
    pub speed_notches: i32,
    /// Wheel notches otherwise: up is positive, and moves forward.
    pub dolly_notches: f32,
    /// Middle-mouse drag (or, in a flat view, right or middle), in points
    /// (x right, y down).
    pub pan: Vec2,
    /// Where the mouse is, in points from the view's top-left corner (a
    /// flat view zooms towards it).
    pub mouse: Option<Vec2>,
    /// The viewport's width in points.
    pub view_width: f32,
    /// Seconds since the last frame.
    pub dt: f32,
    /// The viewport's height in points, so a drag moves the world with the
    /// mouse.
    pub view_height: f32,
}

/// The narrowest and widest field of view a saved camera may have, in
/// radians.
const FOV_RANGE: std::ops::RangeInclusive<f32> = 0.17..=2.1;

impl Camera {
    /// The camera with anything unusable replaced: a damaged or hand-edited
    /// save can hold numbers that would break the picture (not a number,
    /// infinite, out of range). Each such part falls back to the default.
    #[must_use]
    pub fn sanitized(self) -> Self {
        let default = Self::default();
        let position = if self.position.is_finite() && self.position.abs().max_element() < 1.0e7 {
            self.position
        } else {
            default.position
        };
        let heading = if self.heading.abs() <= std::f32::consts::PI {
            self.heading
        } else if self.heading.is_finite() {
            wrap_angle(self.heading)
        } else {
            default.heading
        };
        let tilt = if self.tilt.is_finite() {
            self.tilt.clamp(-MAX_TILT, MAX_TILT)
        } else {
            default.tilt
        };
        let fov_y = if FOV_RANGE.contains(&self.fov_y) {
            self.fov_y
        } else {
            default.fov_y
        };
        let flat_height = if FLAT_HEIGHT_RANGE.contains(&self.flat_height) {
            self.flat_height
        } else {
            default.flat_height
        };
        Self {
            position,
            heading,
            tilt,
            fov_y,
            speed: self.speed.clamp(1, SPEEDS.len() as u8),
            kind: self.kind,
            flat_height,
        }
    }

    /// The same camera made a flat view of `kind` (or perspective again),
    /// looking at the same place. A flat view starts centred where the
    /// camera looked, showing about as much as the camera did there.
    #[must_use]
    pub fn as_kind(self, kind: ViewKind) -> Self {
        if kind == self.kind {
            return self;
        }
        let mut camera = self;
        if kind.is_flat() && !self.kind.is_flat() {
            let focus = self.focus();
            let distance = (focus - self.position).length().max(1.0);
            camera.flat_height = (2.0 * distance * (self.fov_y * 0.5).tan())
                .clamp(*FLAT_HEIGHT_RANGE.start(), *FLAT_HEIGHT_RANGE.end());
            camera.position = focus;
        }
        if !kind.is_flat() && self.kind.is_flat() {
            // Back to perspective: from above and in front of the middle.
            let centre = self.position;
            camera.position = centre + Vec3::new(0.0, -1.0, 0.6) * self.flat_height;
            camera.kind = kind;
            camera.look_at(centre);
            return camera;
        }
        camera.kind = kind;
        camera
    }

    /// Where someone looking at the view would stand: the camera itself in
    /// perspective, and for a flat view a point in front of its middle, as
    /// far away as the view is tall. Lights are chosen and shaded from
    /// here, and things added "at the camera" go here.
    pub fn viewpoint(&self) -> Vec3 {
        if self.kind.is_flat() {
            self.position - self.forward() * self.flat_height.max(1.0)
        } else {
            self.position
        }
    }

    /// Where the view is seen from: the camera itself in perspective, and
    /// for a flat view a point far behind its middle, so everything near
    /// it is in front.
    pub fn eye(&self) -> Vec3 {
        if self.kind.is_flat() {
            self.position - self.forward() * FLAT_DEPTH
        } else {
            self.position
        }
    }

    /// Nearest distance drawn, in metres.
    pub const NEAR: f32 = 0.05;
    /// Furthest distance drawn, in metres.
    pub const FAR: f32 = 5_000.0;

    /// The direction the camera looks.
    pub fn forward(&self) -> Vec3 {
        if let Some((forward, _, _)) = self.kind.axes() {
            return forward;
        }
        let (sin_h, cos_h) = self.heading.sin_cos();
        let (sin_t, cos_t) = self.tilt.sin_cos();
        Vec3::new(sin_h * cos_t, cos_h * cos_t, sin_t)
    }

    /// The camera's right, level with the ground.
    pub fn right(&self) -> Vec3 {
        if let Some((_, right, _)) = self.kind.axes() {
            return right;
        }
        let (sin_h, cos_h) = self.heading.sin_cos();
        Vec3::new(cos_h, -sin_h, 0.0)
    }

    /// The camera's up, square to forward and right.
    pub fn up(&self) -> Vec3 {
        if let Some((_, _, up)) = self.kind.axes() {
            return up;
        }
        self.right().cross(self.forward())
    }

    /// Turns the camera to face `point` (ignored if it is where the camera
    /// is).
    pub fn look_at(&mut self, point: Vec3) {
        if self.kind.is_flat() {
            // A flat view always faces the same way: it centres instead.
            if point.is_finite() {
                self.position = point;
            }
            return;
        }
        let to = point - self.position;
        if to.length_squared() < 1e-12 || !to.is_finite() {
            return;
        }
        self.heading = to.x.atan2(to.y);
        self.tilt = (to.z / to.length()).asin().clamp(-MAX_TILT, MAX_TILT);
    }

    /// Moves the camera back from the box `min` to `max` until all of it
    /// is in view, keeping the direction it looks (or, if it looks up or
    /// level, looking down at a gentle angle), and picks a speed that
    /// suits the box's size: crossing it takes a few seconds.
    pub fn frame(&mut self, min: Vec3, max: Vec3) {
        if !min.is_finite() || !max.is_finite() {
            return;
        }
        let center = (min + max) * 0.5;
        let radius = ((max - min).length() * 0.5).max(0.05);
        // A box so big its size overflows (a damaged model) would make the
        // camera's position meaningless.
        if !center.is_finite() || !radius.is_finite() {
            return;
        }
        if self.kind.is_flat() {
            self.position = center;
            self.flat_height =
                (radius * 2.2).clamp(*FLAT_HEIGHT_RANGE.start(), *FLAT_HEIGHT_RANGE.end());
            return;
        }
        if self.tilt > -0.15 {
            self.tilt = -0.35;
        }
        let distance = radius / (self.fov_y * 0.5).sin().max(0.05) * 1.1;
        self.position = center - self.forward() * distance;
        // The speed step whose metres per second is closest to the box's
        // size.
        let size = radius * 2.0;
        self.speed = SPEEDS
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| (*a - size).abs().total_cmp(&(*b - size).abs()))
            .map_or(DEFAULT_SPEED, |(i, _)| i as u8 + 1);
    }

    /// The current speed in metres per second.
    pub fn speed_mps(&self) -> f32 {
        let step = usize::from(self.speed.clamp(1, SPEEDS.len() as u8)) - 1;
        SPEEDS[step]
    }

    /// Moves and turns the camera for one frame of input.
    pub fn fly(&mut self, input: &FlyInput) {
        if self.kind.is_flat() {
            self.slide_and_zoom(input);
            return;
        }
        let dt = if input.dt.is_finite() {
            input.dt.clamp(0.0, MAX_STEP)
        } else {
            0.0
        };

        // Looking around: mouse right turns right, mouse down looks down.
        if input.look.is_finite() {
            self.heading = wrap_angle(self.heading + input.look.x * LOOK_PER_POINT);
            self.tilt = (self.tilt - input.look.y * LOOK_PER_POINT).clamp(-MAX_TILT, MAX_TILT);
        }

        if input.speed_notches != 0 {
            let step = i32::from(self.speed) + input.speed_notches;
            self.speed = step.clamp(1, SPEEDS.len() as i32) as u8;
        }

        let boost = if input.boost { BOOST } else { 1.0 };
        let speed = self.speed_mps() * boost;

        // Flying: forward follows the view (so looking down and pressing W
        // goes down), sideways stays level, Q and E go straight down and up.
        let movement = input.movement.clamp(Vec3::splat(-1.0), Vec3::splat(1.0));
        if movement.is_finite() && movement != Vec3::ZERO {
            let direction =
                self.right() * movement.x + self.forward() * movement.y + Vec3::Z * movement.z;
            self.position += direction.normalize_or_zero() * speed * dt;
        }

        // The wheel moves forward and back in steps.
        if input.dolly_notches.is_finite() && input.dolly_notches != 0.0 {
            self.position += self.forward() * input.dolly_notches * speed * DOLLY_SECONDS_PER_NOTCH;
        }

        // Middle drag slides the camera so the ground under the mouse
        // follows it.
        if input.pan.is_finite() && input.pan != Vec2::ZERO && input.view_height > 0.0 {
            let distance = (self.focus() - self.position).length().max(1.0);
            let metres_per_point = distance * 2.0 * (self.fov_y * 0.5).tan() / input.view_height;
            self.position +=
                (self.up() * input.pan.y - self.right() * input.pan.x) * metres_per_point;
        }
    }

    /// A flat view's movement: a drag (right or middle button) slides the
    /// view with the mouse, and the wheel zooms towards the mouse, as in
    /// Unreal's flat views.
    fn slide_and_zoom(&mut self, input: &FlyInput) {
        let height = input.view_height;
        if !(height.is_finite() && height > 0.0) {
            return;
        }
        let metres_per_point = self.flat_height / height;
        if input.pan.is_finite() && input.pan != Vec2::ZERO {
            self.position -=
                (self.right() * input.pan.x - self.up() * input.pan.y) * metres_per_point;
        }
        if input.dolly_notches.is_finite() && input.dolly_notches != 0.0 {
            let factor = FLAT_ZOOM_PER_NOTCH.powf(input.dolly_notches.clamp(-20.0, 20.0));
            let new_height = (self.flat_height * factor)
                .clamp(*FLAT_HEIGHT_RANGE.start(), *FLAT_HEIGHT_RANGE.end());
            // The point under the mouse stays under it.
            if let Some(mouse) = input.mouse.filter(|m| m.is_finite()) {
                let from_middle =
                    Vec2::new(mouse.x - input.view_width * 0.5, height * 0.5 - mouse.y);
                let under = self.right() * from_middle.x + self.up() * from_middle.y;
                let shrink = 1.0 - new_height / self.flat_height;
                self.position += under * metres_per_point * shrink;
            }
            self.flat_height = new_height;
        }
    }

    /// World-to-screen transform for an image `aspect` wide per unit high,
    /// with depth reversed: 1 at near, 0 at far. Floating-point depth has
    /// most of its precision near 0, so reversing it spreads precision
    /// evenly over distance instead of wasting it close to the camera
    /// ("reversed-Z"): surfaces a kilometre away still don't flicker
    /// against each other.
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        let aspect = if aspect.is_finite() && aspect > 0.0 {
            aspect
        } else {
            1.0
        };
        if self.kind.is_flat() {
            // Flat (orthographic), depth reversed the same way.
            let half_height = self.flat_height * 0.5;
            let near = 0.0;
            let far = FLAT_DEPTH * 2.0;
            let range = far - near;
            let projection = Mat4::from_cols(
                Vec4::new(1.0 / (half_height * aspect), 0.0, 0.0, 0.0),
                Vec4::new(0.0, 1.0 / half_height, 0.0, 0.0),
                Vec4::new(0.0, 0.0, 1.0 / range, 0.0),
                Vec4::new(0.0, 0.0, far / range, 1.0),
            );
            return projection * self.view_matrix();
        }
        // Right-handed perspective with depth 1 at near and 0 at far.
        let y_scale = 1.0 / (self.fov_y * 0.5).tan();
        let x_scale = y_scale / aspect;
        let range = Self::FAR - Self::NEAR;
        let projection = Mat4::from_cols(
            Vec4::new(x_scale, 0.0, 0.0, 0.0),
            Vec4::new(0.0, y_scale, 0.0, 0.0),
            Vec4::new(0.0, 0.0, Self::NEAR / range, -1.0),
            Vec4::new(0.0, 0.0, Self::FAR * Self::NEAR / range, 0.0),
        );
        projection * self.view_matrix()
    }

    /// World-to-camera transform: the camera looks down its own -Z, with Y
    /// up on screen.
    pub fn view_matrix(&self) -> Mat4 {
        let (forward, right, up) = (self.forward(), self.right(), self.up());
        let eye = self.eye();
        Mat4::from_cols(
            Vec4::new(right.x, up.x, -forward.x, 0.0),
            Vec4::new(right.y, up.y, -forward.y, 0.0),
            Vec4::new(right.z, up.z, -forward.z, 0.0),
            Vec4::new(-right.dot(eye), -up.dot(eye), forward.dot(eye), 1.0),
        )
    }

    /// The point on the ground the grid is centred on: where the view meets
    /// the ground, or below the camera when it looks level or up.
    pub fn focus(&self) -> Vec3 {
        if self.kind.is_flat() {
            return self.kind.onto_plane(self.position);
        }
        let forward = self.forward();
        let below = Vec3::new(self.position.x, self.position.y, 0.0);
        if self.position.z <= 0.0 || forward.z > -0.01 {
            return below;
        }
        let distance = (self.position.z / -forward.z).min(MAX_FOCUS_DISTANCE);
        let hit = self.position + forward * distance;
        Vec3::new(hit.x, hit.y, 0.0)
    }
}

/// An angle brought into -π to π.
fn wrap_angle(angle: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    (angle + PI).rem_euclid(TAU) - PI
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-3
    }

    fn frame(input: FlyInput) -> FlyInput {
        FlyInput {
            dt: 0.1,
            view_height: 600.0,
            ..input
        }
    }

    #[test]
    fn the_new_camera_looks_at_the_origin() {
        let camera = Camera::default();
        let to_target = (Vec3::new(0.0, 0.0, 1.0) - camera.position).normalize();
        assert!(close(camera.forward(), to_target));
        assert_eq!(camera.speed, DEFAULT_SPEED);
    }

    #[test]
    fn the_target_lands_in_the_middle_of_the_picture() {
        let camera = Camera::default();
        let clip = camera.view_projection(16.0 / 9.0) * Vec3::new(0.0, 0.0, 1.0).extend(1.0);
        let ndc = clip.truncate() / clip.w;
        assert!(ndc.x.abs() < 1e-4 && ndc.y.abs() < 1e-4, "{ndc}");
        assert!(ndc.z > 0.0 && ndc.z < 1.0);
    }

    #[test]
    fn depth_is_reversed_one_near_and_zero_far() {
        let camera = Camera {
            position: Vec3::ZERO,
            heading: 0.0,
            tilt: 0.0,
            ..Camera::default()
        };
        let vp = camera.view_projection(1.0);
        let depth = |distance: f32| {
            let c = vp * Vec3::new(0.0, distance, 0.0).extend(1.0);
            c.z / c.w
        };
        assert!((depth(Camera::NEAR) - 1.0).abs() < 1e-5);
        assert!(depth(Camera::FAR).abs() < 1e-5);
        assert!(depth(10.0) > depth(100.0), "nearer is deeper");
        // A kilometre away, a few centimetres still tell surfaces apart.
        let gap = depth(1000.0) - depth(1000.05);
        assert!(gap > 4.0 * f32::EPSILON * depth(1000.0), "{gap}");
    }

    #[test]
    fn a_box_too_big_to_measure_leaves_the_camera_as_it_was() {
        let mut camera = Camera::default();
        let before = camera;
        camera.frame(Vec3::splat(-3e38), Vec3::splat(3e38));
        assert_eq!(camera, before);
        assert!(camera.view_projection(1.0).is_finite());
    }

    #[test]
    fn up_in_the_world_is_up_on_screen() {
        let camera = Camera::default();
        let vp = camera.view_projection(1.0);
        let screen = |p: Vec3| {
            let c = vp * p.extend(1.0);
            c.truncate() / c.w
        };
        assert!(screen(Vec3::new(0.0, 0.0, 3.0)).y > screen(Vec3::new(0.0, 0.0, 1.0)).y);
    }

    #[test]
    fn right_forward_and_up_are_square_to_each_other() {
        let mut camera = Camera::default();
        for (heading, tilt) in [(0.0, 0.0), (1.0, 0.5), (-2.5, -1.2), (3.0, MAX_TILT)] {
            camera.heading = heading;
            camera.tilt = tilt;
            let (f, r, u) = (camera.forward(), camera.right(), camera.up());
            assert!(f.dot(r).abs() < 1e-5 && f.dot(u).abs() < 1e-5 && r.dot(u).abs() < 1e-5);
            assert!((f.length() - 1.0).abs() < 1e-5 && (u.length() - 1.0).abs() < 1e-5);
            assert!(u.z >= 0.0, "up never points into the ground");
        }
    }

    #[test]
    fn moving_the_mouse_right_turns_right_and_down_looks_down() {
        let mut camera = Camera::default();
        let (heading, tilt) = (camera.heading, camera.tilt);
        camera.fly(&frame(FlyInput {
            look: Vec2::new(50.0, 30.0),
            ..Default::default()
        }));
        assert!(camera.heading > heading);
        assert!(camera.tilt < tilt);
    }

    #[test]
    fn looking_never_flips_over_the_top() {
        let mut camera = Camera::default();
        camera.fly(&frame(FlyInput {
            look: Vec2::new(0.0, -1.0e6),
            ..Default::default()
        }));
        assert_eq!(camera.tilt, MAX_TILT);
        camera.fly(&frame(FlyInput {
            look: Vec2::new(0.0, 1.0e6),
            ..Default::default()
        }));
        assert_eq!(camera.tilt, -MAX_TILT);
    }

    #[test]
    fn heading_stays_in_range_after_many_turns() {
        let mut camera = Camera::default();
        for _ in 0..1000 {
            camera.fly(&frame(FlyInput {
                look: Vec2::new(400.0, 0.0),
                ..Default::default()
            }));
        }
        assert!(camera.heading.abs() <= std::f32::consts::PI + 1e-5);
    }

    #[test]
    fn w_moves_forward_at_the_chosen_speed() {
        let mut camera = Camera::default();
        let start = camera.position;
        camera.fly(&frame(FlyInput {
            movement: Vec3::Y,
            ..Default::default()
        }));
        let expected = start + camera.forward() * camera.speed_mps() * 0.1;
        assert!(close(camera.position, expected));
    }

    #[test]
    fn sideways_stays_level_and_e_goes_straight_up() {
        let mut camera = Camera::default();
        let start = camera.position;
        camera.fly(&frame(FlyInput {
            movement: Vec3::X,
            ..Default::default()
        }));
        assert!((camera.position.z - start.z).abs() < 1e-5);
        let side = camera.position;
        camera.fly(&frame(FlyInput {
            movement: Vec3::Z,
            ..Default::default()
        }));
        assert!(close(
            camera.position - side,
            Vec3::Z * camera.speed_mps() * 0.1
        ));
    }

    #[test]
    fn diagonal_movement_is_not_faster() {
        let mut straight = Camera::default();
        let mut diagonal = Camera::default();
        let start = straight.position;
        straight.fly(&frame(FlyInput {
            movement: Vec3::X,
            ..Default::default()
        }));
        diagonal.fly(&frame(FlyInput {
            movement: Vec3::new(1.0, 1.0, 1.0),
            ..Default::default()
        }));
        let a = (straight.position - start).length();
        let b = (diagonal.position - start).length();
        assert!((a - b).abs() < 1e-4);
    }

    #[test]
    fn shift_moves_faster() {
        let mut normal = Camera::default();
        let mut boosted = Camera::default();
        let start = normal.position;
        let input = FlyInput {
            movement: Vec3::Y,
            ..Default::default()
        };
        normal.fly(&frame(input));
        boosted.fly(&frame(FlyInput {
            boost: true,
            ..input
        }));
        let ratio = (boosted.position - start).length() / (normal.position - start).length();
        assert!((ratio - BOOST).abs() < 1e-3);
    }

    #[test]
    fn the_wheel_while_flying_changes_speed_within_its_steps() {
        let mut camera = Camera::default();
        camera.fly(&frame(FlyInput {
            speed_notches: 2,
            ..Default::default()
        }));
        assert_eq!(camera.speed, DEFAULT_SPEED + 2);
        camera.fly(&frame(FlyInput {
            speed_notches: 100,
            ..Default::default()
        }));
        assert_eq!(camera.speed, 8);
        camera.fly(&frame(FlyInput {
            speed_notches: -100,
            ..Default::default()
        }));
        assert_eq!(camera.speed, 1);
    }

    #[test]
    fn a_saved_speed_out_of_range_is_still_usable() {
        let camera = Camera {
            speed: 0,
            ..Camera::default()
        };
        assert_eq!(camera.speed_mps(), SPEEDS[0]);
        let camera = Camera {
            speed: 200,
            ..Camera::default()
        };
        assert_eq!(camera.speed_mps(), SPEEDS[7]);
    }

    #[test]
    fn the_wheel_moves_forward_and_back() {
        let mut camera = Camera::default();
        let start = camera.position;
        camera.fly(&frame(FlyInput {
            dolly_notches: 1.0,
            ..Default::default()
        }));
        assert!((camera.position - start).dot(camera.forward()) > 0.0);
        camera.fly(&frame(FlyInput {
            dolly_notches: -1.0,
            ..Default::default()
        }));
        assert!(close(camera.position, start));
    }

    #[test]
    fn dragging_right_slides_the_camera_left() {
        let mut camera = Camera::default();
        let start = camera.position;
        camera.fly(&frame(FlyInput {
            pan: Vec2::new(100.0, 0.0),
            ..Default::default()
        }));
        assert!((camera.position - start).dot(camera.right()) < 0.0);
        // The view direction doesn't change.
        assert!(close(camera.forward(), Camera::default().forward()));
    }

    #[test]
    fn a_long_hitch_does_not_throw_the_camera_away() {
        let mut camera = Camera::default();
        let start = camera.position;
        camera.fly(&FlyInput {
            movement: Vec3::Y,
            dt: 30.0,
            ..Default::default()
        });
        assert!((camera.position - start).length() <= camera.speed_mps() * MAX_STEP + 1e-4);
    }

    #[test]
    fn broken_input_leaves_the_camera_alone() {
        let mut camera = Camera::default();
        let before = camera;
        camera.fly(&FlyInput {
            look: Vec2::NAN,
            movement: Vec3::new(f32::NAN, 0.0, 0.0),
            dolly_notches: f32::INFINITY,
            pan: Vec2::new(f32::NAN, 1.0),
            dt: f32::NAN,
            view_height: 0.0,
            ..Default::default()
        });
        assert_eq!(camera, before);
    }

    #[test]
    fn the_grid_centres_where_the_view_meets_the_ground() {
        let mut camera = Camera {
            position: Vec3::new(0.0, 0.0, 10.0),
            ..Camera::default()
        };
        camera.look_at(Vec3::new(0.0, 10.0, 0.0));
        assert!(close(camera.focus(), Vec3::new(0.0, 10.0, 0.0)));
        // Looking up, the grid stays under the camera.
        camera.look_at(Vec3::new(0.0, 10.0, 20.0));
        assert!(close(camera.focus(), Vec3::new(0.0, 0.0, 0.0)));
        // Below the ground too.
        camera.position.z = -5.0;
        assert!(close(camera.focus(), Vec3::ZERO));
    }

    #[test]
    fn a_bad_aspect_does_not_break_the_picture() {
        let camera = Camera::default();
        for aspect in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(camera.view_projection(aspect).is_finite());
        }
    }

    #[test]
    fn the_camera_saves_and_loads() {
        let camera = Camera {
            speed: 6,
            position: Vec3::new(1.0, 2.0, 3.0),
            ..Camera::default()
        };
        let text = ron::to_string(&camera).unwrap();
        let back: Camera = ron::from_str(&text).unwrap();
        assert_eq!(back, camera);
        // Missing fields (an older save) fall back to the defaults.
        let partial: Camera = ron::from_str("(speed: 2)").unwrap();
        assert_eq!(partial.speed, 2);
        assert_eq!(partial.position, Camera::default().position);
    }

    fn usable(camera: &Camera) -> bool {
        camera.position.is_finite()
            && camera.heading.is_finite()
            && camera.tilt.abs() <= MAX_TILT
            && FOV_RANGE.contains(&camera.fov_y)
            && (1..=8).contains(&camera.speed)
            && camera.view_projection(1.5).is_finite()
    }

    #[test]
    fn a_damaged_save_is_made_usable() {
        let broken = Camera {
            position: Vec3::new(f32::NAN, 1.0, f32::INFINITY),
            heading: f32::NAN,
            tilt: 50.0,
            fov_y: 0.0,
            speed: 0,
            kind: ViewKind::Top,
            flat_height: f32::NAN,
        };
        let fixed = broken.sanitized();
        assert_eq!(fixed.flat_height, Camera::default().flat_height);
        assert_eq!(fixed.kind, ViewKind::Top);
        assert!(usable(&fixed));
        assert_eq!(fixed.position, Camera::default().position);
        assert_eq!(fixed.tilt, MAX_TILT);
        assert_eq!(fixed.speed, 1);
        // A good camera is left as it is.
        let good = Camera::default();
        assert_eq!(good.sanitized(), good);
    }

    /// Random damage to a saved camera, thousands of times: reading it
    /// never crashes, and whatever reads back is made usable.
    #[test]
    fn random_damage_to_a_saved_camera_never_breaks_it() {
        let saved = ron::to_string(&Camera::default()).unwrap();
        let bytes = saved.as_bytes();
        let replacements: &[&[u8]] = &[
            b"NaN", b"inf", b"-inf", b"1e39", b"-0", b"999", b"(", b")", b",", b"\"", b"",
        ];
        let mut seed: u64 = 7;
        let mut next = |n: usize| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) as usize % n.max(1)
        };
        for _ in 0..3_000 {
            let mut damaged = bytes.to_vec();
            for _ in 0..1 + next(3) {
                let at = next(damaged.len());
                let cut = next(4).min(damaged.len() - at);
                let with = replacements[next(replacements.len())];
                damaged.splice(at..at + cut, with.iter().copied());
            }
            let text = String::from_utf8_lossy(&damaged);
            if let Ok(camera) = ron::from_str::<Camera>(&text) {
                assert!(usable(&camera.sanitized()), "{text}");
            }
        }
    }

    #[test]
    fn framing_a_box_brings_all_of_it_into_view() {
        for (min, max) in [
            (Vec3::new(-1.0, -1.0, 0.0), Vec3::new(1.0, 1.0, 2.0)),
            (Vec3::new(100.0, 50.0, 0.0), Vec3::new(400.0, 90.0, 30.0)),
            (Vec3::splat(-0.01), Vec3::splat(0.01)),
        ] {
            let mut camera = Camera::default();
            camera.frame(min, max);
            let vp = camera.view_projection(1.0);
            for corner in 0..8 {
                let p = Vec3::new(
                    if corner & 1 == 0 { min.x } else { max.x },
                    if corner & 2 == 0 { min.y } else { max.y },
                    if corner & 4 == 0 { min.z } else { max.z },
                );
                let clip = vp * p.extend(1.0);
                let ndc = clip.truncate() / clip.w;
                assert!(
                    clip.w > 0.0 && ndc.x.abs() <= 1.0 && ndc.y.abs() <= 1.0,
                    "{p} -> {ndc}"
                );
            }
        }
    }

    #[test]
    fn framing_picks_a_speed_to_suit_the_size() {
        let mut small = Camera::default();
        small.frame(Vec3::splat(-0.5), Vec3::splat(0.5));
        let mut big = Camera::default();
        big.frame(Vec3::splat(-200.0), Vec3::splat(200.0));
        assert!(small.speed < big.speed);
        assert_eq!(big.speed, 8);
    }

    #[test]
    fn framing_looks_down_when_the_camera_looked_up() {
        let mut camera = Camera {
            tilt: 0.5,
            ..Camera::default()
        };
        camera.frame(Vec3::ZERO, Vec3::ONE);
        assert!(camera.tilt < 0.0);
        let before = camera;
        camera.frame(Vec3::NAN, Vec3::ONE);
        assert_eq!(camera, before, "a broken box changes nothing");
    }

    fn flat(kind: ViewKind) -> Camera {
        Camera {
            position: Vec3::new(2.0, 3.0, 1.0),
            kind,
            flat_height: 10.0,
            ..Camera::default()
        }
    }

    #[test]
    fn flat_views_look_along_an_axis_and_keep_sizes() {
        for (kind, forward) in [
            (ViewKind::Top, Vec3::NEG_Z),
            (ViewKind::Front, Vec3::Y),
            (ViewKind::Right, Vec3::NEG_X),
        ] {
            let camera = flat(kind);
            assert_eq!(camera.forward(), forward);
            assert!(
                close(camera.right().cross(camera.up()), -forward),
                "{kind:?}"
            );
            // The middle of the view is in the middle of the picture,
            // near or far.
            let size = Vec2::new(800.0, 600.0);
            for depth in [-100.0, 0.0, 400.0] {
                let p = camera.position + forward * depth;
                let at = camera.project(p, size).unwrap();
                assert!((at - size * 0.5).length() < 1e-2, "{kind:?} {depth}");
            }
            // One metre is the same size on screen at any depth.
            let near = camera
                .project(camera.position + camera.right(), size)
                .unwrap();
            let far = camera
                .project(camera.position + camera.right() + forward * 300.0, size)
                .unwrap();
            assert!((near - far).length() < 1e-2);
            assert!(
                (near.x - size.x * 0.5 - 60.0).abs() < 1e-2,
                "10 m in 600 points"
            );
            assert!((camera.world_per_pixel(Vec3::ZERO, size) - 10.0 / 600.0).abs() < 1e-6);
            // Every ray points the same way, starting behind what it sees.
            let ray = camera.ray_through(Vec2::new(100.0, 50.0), size);
            assert_eq!(ray.direction, forward);
            assert!((ray.origin - camera.position).dot(forward) < -1000.0);
            assert!(camera.view_projection(1.3).is_finite());
            assert_eq!(camera.focus(), kind.onto_plane(camera.position));
        }
    }

    #[test]
    fn a_flat_view_slides_with_the_mouse_and_zooms_towards_it() {
        let mut camera = flat(ViewKind::Top);
        let size = Vec2::new(800.0, 600.0);
        // Dragging right by 60 points (one metre here): the world follows.
        let under = camera.ray_through(Vec2::new(400.0, 300.0), size).origin;
        camera.fly(&FlyInput {
            pan: Vec2::new(60.0, 0.0),
            view_height: 600.0,
            view_width: 800.0,
            dt: 0.016,
            ..FlyInput::default()
        });
        let moved = camera
            .project(Vec3::new(under.x, under.y, 0.0), size)
            .unwrap();
        assert!((moved.x - 460.0).abs() < 1e-2, "{moved:?}");
        // Zooming in at a corner keeps the point under the mouse there.
        let mouse = Vec2::new(700.0, 100.0);
        let ray = camera.ray_through(mouse, size);
        let point = Vec3::new(ray.origin.x, ray.origin.y, 0.0);
        camera.fly(&FlyInput {
            dolly_notches: 3.0,
            mouse: Some(mouse),
            view_height: 600.0,
            view_width: 800.0,
            dt: 0.016,
            ..FlyInput::default()
        });
        assert!(camera.flat_height < 10.0);
        let still = camera.project(point, size).unwrap();
        assert!((still - mouse).length() < 1e-2, "{still:?}");
        // Zooming never goes past its limits, and bad input is ignored.
        for notches in [500.0, -500.0, f32::NAN] {
            camera.fly(&FlyInput {
                dolly_notches: notches,
                view_height: 600.0,
                ..FlyInput::default()
            });
            assert!(FLAT_HEIGHT_RANGE.contains(&camera.flat_height));
        }
        let before = camera;
        camera.fly(&FlyInput {
            pan: Vec2::new(5.0, 5.0),
            view_height: 0.0,
            ..FlyInput::default()
        });
        assert_eq!(camera, before);
    }

    #[test]
    fn switching_kinds_keeps_looking_at_the_same_place() {
        let camera = Camera::default();
        let focus = camera.focus();
        let top = camera.as_kind(ViewKind::Top);
        assert_eq!(top.position, focus);
        assert!(top.flat_height > 1.0);
        let back = top.as_kind(ViewKind::Perspective);
        assert!(!back.is_flat());
        assert!((back.focus() - focus).length() < 0.5, "{:?}", back.focus());
        assert_eq!(top.as_kind(ViewKind::Front).position, top.position);
        // Framing a box in a flat view centres it and fits it.
        let mut front = flat(ViewKind::Front);
        front.frame(Vec3::new(10.0, 10.0, 0.0), Vec3::new(12.0, 12.0, 2.0));
        assert_eq!(front.position, Vec3::new(11.0, 11.0, 1.0));
        assert!(front.flat_height > 2.0 && front.flat_height < 10.0);
        // look_at centres a flat view rather than turning it.
        front.look_at(Vec3::ONE);
        assert_eq!(front.position, Vec3::ONE);
        assert_eq!(front.forward(), Vec3::Y);
    }
}
