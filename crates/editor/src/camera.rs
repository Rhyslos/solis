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

/// The viewport's camera. Z is up.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Camera {
    /// Where the camera is, in metres.
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
    /// Middle-mouse drag, in points (x right, y down).
    pub pan: Vec2,
    /// Seconds since the last frame.
    pub dt: f32,
    /// The viewport's height in points, so a drag moves the world with the
    /// mouse.
    pub view_height: f32,
}

impl Camera {
    /// Nearest distance drawn, in metres.
    pub const NEAR: f32 = 0.05;
    /// Furthest distance drawn, in metres.
    pub const FAR: f32 = 5_000.0;

    /// The direction the camera looks.
    pub fn forward(&self) -> Vec3 {
        let (sin_h, cos_h) = self.heading.sin_cos();
        let (sin_t, cos_t) = self.tilt.sin_cos();
        Vec3::new(sin_h * cos_t, cos_h * cos_t, sin_t)
    }

    /// The camera's right, level with the ground.
    pub fn right(&self) -> Vec3 {
        let (sin_h, cos_h) = self.heading.sin_cos();
        Vec3::new(cos_h, -sin_h, 0.0)
    }

    /// The camera's up, square to forward and right.
    pub fn up(&self) -> Vec3 {
        self.right().cross(self.forward())
    }

    /// Turns the camera to face `point` (ignored if it is where the camera
    /// is).
    pub fn look_at(&mut self, point: Vec3) {
        let to = point - self.position;
        if to.length_squared() < 1e-12 || !to.is_finite() {
            return;
        }
        self.heading = to.x.atan2(to.y);
        self.tilt = (to.z / to.length()).asin().clamp(-MAX_TILT, MAX_TILT);
    }

    /// The current speed in metres per second.
    pub fn speed_mps(&self) -> f32 {
        let step = usize::from(self.speed.clamp(1, SPEEDS.len() as u8)) - 1;
        SPEEDS[step]
    }

    /// Moves and turns the camera for one frame of input.
    pub fn fly(&mut self, input: &FlyInput) {
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

    /// World-to-screen transform for an image `aspect` wide per unit high
    /// (depth 0 at near, 1 at far).
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        let aspect = if aspect.is_finite() && aspect > 0.0 {
            aspect
        } else {
            1.0
        };
        // Right-handed perspective with depth 0 at near and 1 at far, as
        // wgpu expects.
        let y_scale = 1.0 / (self.fov_y * 0.5).tan();
        let x_scale = y_scale / aspect;
        let depth = Self::FAR / (Self::NEAR - Self::FAR);
        let projection = Mat4::from_cols(
            Vec4::new(x_scale, 0.0, 0.0, 0.0),
            Vec4::new(0.0, y_scale, 0.0, 0.0),
            Vec4::new(0.0, 0.0, depth, -1.0),
            Vec4::new(0.0, 0.0, depth * Self::NEAR, 0.0),
        );
        projection * self.view_matrix()
    }

    /// World-to-camera transform: the camera looks down its own -Z, with Y
    /// up on screen.
    pub fn view_matrix(&self) -> Mat4 {
        let (forward, right, up) = (self.forward(), self.right(), self.up());
        Mat4::from_cols(
            Vec4::new(right.x, up.x, -forward.x, 0.0),
            Vec4::new(right.y, up.y, -forward.y, 0.0),
            Vec4::new(right.z, up.z, -forward.z, 0.0),
            Vec4::new(
                -right.dot(self.position),
                -up.dot(self.position),
                forward.dot(self.position),
                1.0,
            ),
        )
    }

    /// The point on the ground the grid is centred on: where the view meets
    /// the ground, or below the camera when it looks level or up.
    pub fn focus(&self) -> Vec3 {
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
}
