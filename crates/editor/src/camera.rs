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
    /// 2D (Phase 9, D1): flat, looking straight down like Top, measured in
    /// the project's pixels: it zooms in whole steps and draws the
    /// world's pixels on whole screen pixels.
    TwoD,
}

impl ViewKind {
    /// Every kind, in the menu's order.
    pub const ALL: [Self; 5] = [
        Self::Perspective,
        Self::Top,
        Self::Front,
        Self::Right,
        Self::TwoD,
    ];

    /// Its name in the viewport's menu.
    pub fn name(self) -> &'static str {
        match self {
            Self::Perspective => "Perspective",
            Self::Top => "Top",
            Self::Front => "Front",
            Self::Right => "Right",
            Self::TwoD => "2D",
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
            Self::Top | Self::TwoD => Some((Vec3::NEG_Z, Vec3::X, Vec3::Y)),
            Self::Front => Some((Vec3::Y, Vec3::X, Vec3::Z)),
            Self::Right => Some((Vec3::NEG_X, Vec3::Y, Vec3::Z)),
        }
    }

    /// The point of `p` on the plane a flat view looks at (through the
    /// origin, across its forward); `p` itself in perspective.
    pub fn onto_plane(self, p: Vec3) -> Vec3 {
        match self {
            Self::Perspective => p,
            Self::Top | Self::TwoD => Vec3::new(p.x, p.y, 0.0),
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
/// The 2D view's zoom steps: screen pixels per pixel of the game. Whole
/// steps keep pixels sharp; the ones below 1 show more of a big map.
pub const ZOOM_STEPS: [f32; 13] = [
    0.25, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0, 12.0, 16.0, 24.0,
];
/// The 2D view's zoom when it's first chosen.
pub const FIRST_ZOOM: f32 = 2.0;

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
    /// The 2D view's zoom: screen pixels per pixel of the game, one of
    /// [`ZOOM_STEPS`].
    pub zoom: f32,
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
            zoom: FIRST_ZOOM,
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
            zoom: if ZOOM_STEPS.contains(&self.zoom) {
                self.zoom
            } else {
                default.zoom
            },
        }
    }

    /// For the 2D view (otherwise itself): as it's seen in a view of
    /// `view_px` screen pixels, with `pixels_per_unit` pixels to a metre.
    /// It shows as much as its zoom says, and its middle sits so the
    /// world's pixels fall on whole screen pixels, so nothing shimmers or
    /// blurs. The camera itself keeps its exact place, so a slow drag
    /// still moves it.
    #[must_use]
    pub fn seen(self, view_px: [u32; 2], pixels_per_unit: u32) -> Self {
        if self.kind != ViewKind::TwoD {
            return self;
        }
        let ppu = pixels_per_unit.max(1) as f32;
        let zoom = if ZOOM_STEPS.contains(&self.zoom) {
            self.zoom
        } else {
            FIRST_ZOOM
        };
        let [w, h] = view_px.map(|v| v.max(1) as f32);
        // One screen pixel, in metres.
        let px = 1.0 / (ppu * zoom);
        let mut camera = self;
        camera.flat_height = (h * px).clamp(*FLAT_HEIGHT_RANGE.start(), *FLAT_HEIGHT_RANGE.end());
        // The view's left and bottom edges on whole screen pixels (and so,
        // zoomed in, the world's pixels on whole screen pixels too).
        let edge = |middle: f32, across: f32| {
            let first = middle - across * 0.5 * px;
            (first / px).round() * px + across * 0.5 * px
        };
        if self.position.is_finite() {
            camera.position.x = edge(self.position.x, w);
            camera.position.y = edge(self.position.y, h);
        }
        camera
    }

    /// The next zoom step in (`steps` above 0) or out from `zoom`.
    pub fn zoom_step(zoom: f32, steps: i32) -> f32 {
        let at = ZOOM_STEPS
            .iter()
            .position(|z| *z >= zoom)
            .unwrap_or(ZOOM_STEPS.len() - 1) as i32;
        ZOOM_STEPS[(at + steps).clamp(0, ZOOM_STEPS.len() as i32 - 1) as usize]
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
            let new_height = if self.kind == ViewKind::TwoD {
                // Whole zoom steps, one a notch: up zooms in.
                let notches = input.dolly_notches.clamp(-20.0, 20.0).round() as i32;
                let zoom = Self::zoom_step(self.zoom, notches);
                let height = self.flat_height * self.zoom / zoom;
                self.zoom = zoom;
                height
            } else {
                let factor = FLAT_ZOOM_PER_NOTCH.powf(input.dolly_notches.clamp(-20.0, 20.0));
                self.flat_height * factor
            }
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
#[path = "camera_tests.rs"]
mod tests;
