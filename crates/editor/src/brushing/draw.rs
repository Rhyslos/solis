//! Drawing a brush out (the approved mock-up `02-brush-draw.png`): the
//! footprint dragged on the grid, then the height pulled up and a click
//! to keep it. The arithmetic only: the viewport hands each frame's mouse
//! to [`Draw::update`], [`super::ghost`] draws what it says, and the
//! workbench adds the brush [`Draw::update`] gives back.
//!
//! - **Where:** a drag in the 3D view (or the Top view) starts on top of
//!   whatever brush is under the pointer, else on the ground grid, and the
//!   footprint lies flat at that height. In the 2D view and the Front and
//!   Right views, the footprint is drawn in the view's own plane and the
//!   shape is one grid step deep: there is no height step.
//! - **Snapping:** the footprint's corners and the height land on the
//!   grid's step; Ctrl, held, draws freely.
//! - **The height** follows the pointer up a line standing on the
//!   footprint's middle; when the camera looks almost straight down that
//!   line is seen end on, so the height follows the pointer up the screen
//!   instead.
//! - **The Ramp and the Stairs** climb along the footprint's longer side,
//!   from where the drag started.

use crate::area_view::AreaView;
use crate::camera::{Camera, ViewKind};
use crate::screen::Ray;
use glam::{Quat, Vec2, Vec3};
use mesh::shapes::MIN_SIZE;
use mesh::{Shape, ShapeSettings};

/// Where a footprint is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Plane {
    /// Flat on the ground grid, or on top of what was under the pointer,
    /// at this height; then pulled up.
    Ground { base: f32 },
    /// In a flat view's own plane (2D, Front, Right), through the
    /// world's middle; the shape is one grid step deep.
    View(ViewKind),
}

impl Plane {
    /// The plane a drag starts on in a view of `kind`.
    fn for_view(kind: ViewKind, base: f32) -> Self {
        match kind {
            ViewKind::Perspective | ViewKind::Top => Self::Ground { base },
            ViewKind::TwoD | ViewKind::Front | ViewKind::Right => Self::View(kind),
        }
    }

    /// The world's axes across the plane, and the one it faces.
    fn axes(self) -> (Vec3, Vec3, Vec3) {
        match self {
            Self::Ground { .. } | Self::View(ViewKind::TwoD | ViewKind::Top) => {
                (Vec3::X, Vec3::Y, Vec3::Z)
            }
            Self::View(ViewKind::Front) => (Vec3::X, Vec3::Z, Vec3::Y),
            Self::View(ViewKind::Right) => (Vec3::Y, Vec3::Z, Vec3::X),
            Self::View(ViewKind::Perspective) => (Vec3::X, Vec3::Y, Vec3::Z),
        }
    }

    /// How far along its facing axis the plane is.
    fn depth(self) -> f32 {
        match self {
            Self::Ground { base } => base,
            Self::View(_) => 0.0,
        }
    }

    /// A point of the plane, from its place across it.
    pub(crate) fn world(self, at: Vec2) -> Vec3 {
        let (u, v, n) = self.axes();
        u * at.x + v * at.y + n * self.depth()
    }

    /// Where `ray` meets the plane, as a place across it.
    fn meet(self, ray: &Ray) -> Option<Vec2> {
        let (u, v, n) = self.axes();
        let facing = ray.direction.dot(n);
        if facing.abs() < 1e-6 {
            return None;
        }
        let distance = (self.depth() - ray.origin.dot(n)) / facing;
        if !distance.is_finite() || distance < 0.0 {
            return None;
        }
        let at = ray.at(distance);
        Some(Vec2::new(at.dot(u), at.dot(v)))
    }
}

/// Why nothing was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Nothing {
    /// A click without a drag: the footprint had no size.
    NoFootprint,
    /// A click before the mouse moved up: the brush had no height.
    NoHeight,
}

impl Nothing {
    /// Why, for the status bar.
    pub(crate) fn words(self) -> &'static str {
        match self {
            Self::NoFootprint => {
                "Nothing made: the footprint had no size. Drag on the ground to draw one"
            }
            Self::NoHeight => {
                "Nothing made: the brush had no height. Move the mouse up before clicking"
            }
        }
    }
}

/// How the height follows the pointer, chosen as the height step begins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Lift {
    /// Up the line standing on the footprint's middle: the height is how
    /// far up it the pointer's nearest point is, less where it began.
    Line { middle: Vec3, from: f32 },
    /// Up the screen, `per_point` metres a point, from `from` (a point's
    /// height on screen): for a camera looking almost straight down.
    Screen { from: f32, per_point: f32 },
}

/// The step a brush being drawn out is at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Step {
    /// Waiting for the drag; `nothing` says why the last one made nothing.
    Waiting { nothing: Option<Nothing> },
    /// The footprint being dragged, from corner `from` to corner `to`
    /// (places across `plane`, snapped unless `free`).
    Footprint {
        plane: Plane,
        from: Vec2,
        to: Vec2,
        free: bool,
    },
    /// The footprint kept; the height being pulled up.
    Height {
        plane: Plane,
        from: Vec2,
        to: Vec2,
        lift: Lift,
        height: f32,
        free: bool,
    },
}

/// A brush to make: its shape, where it stands and how it's built.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Planned {
    pub(crate) shape: Shape,
    /// Its entity's place: the middle of its base.
    pub(crate) position: Vec3,
    /// Its entity's turn (the Ramp and Stairs face their climb).
    pub(crate) rotation: Quat,
    /// Its size and counts.
    pub(crate) settings: ShapeSettings,
}

/// The mouse this frame, as drawing out needs it.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DrawInput {
    /// Where the pointer is, from the view's top left, in points.
    pub(crate) mouse: Option<Vec2>,
    pub(crate) pressed: bool,
    pub(crate) down: bool,
    pub(crate) released: bool,
    pub(crate) right_pressed: bool,
    pub(crate) ctrl: bool,
    /// The press was on the view itself (not its pills).
    pub(crate) on_view: bool,
}

/// What the view shows, as drawing out needs it.
pub(crate) struct DrawView<'a> {
    pub(crate) camera: &'a Camera,
    /// The view's size in points.
    pub(crate) size: Vec2,
    /// What is drawn in it (the brushes a drag can start on).
    pub(crate) area: &'a AreaView,
    /// The grid's step, in metres.
    pub(crate) grid: f32,
}

/// What came of a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Outcome {
    /// Still drawing.
    Going,
    /// A brush is drawn out: add it.
    Made(Planned),
    /// Given up (a right-click): nothing is left.
    GaveUp,
}

/// A brush being drawn out.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Draw {
    /// The shape taken from the list.
    pub(crate) shape: Shape,
    pub(crate) step: Step,
    /// Whether the view in use is a flat one that draws in its own plane:
    /// for the words, before a drag has started.
    pub(crate) flat_view: bool,
}

impl Draw {
    /// `shape`, taken from the list, waiting for its drag.
    pub(crate) fn new(shape: Shape) -> Self {
        Self {
            shape,
            step: Step::Waiting { nothing: None },
            flat_view: false,
        }
    }

    /// Whether a drag or the height is under way (the right button is
    /// the drawing's then, never the camera's).
    pub(crate) fn is_under_way(&self) -> bool {
        !matches!(self.step, Step::Waiting { .. })
    }

    /// Whether the brush is drawn in a flat view's own plane, one grid
    /// step deep, with no height step.
    pub(crate) fn in_plane(&self) -> bool {
        match self.step {
            Step::Footprint { plane, .. } | Step::Height { plane, .. } => {
                matches!(plane, Plane::View(_))
            }
            Step::Waiting { .. } => self.flat_view,
        }
    }

    /// Takes this frame's mouse.
    pub(crate) fn update(&mut self, input: DrawInput, view: &DrawView) -> Outcome {
        self.flat_view = matches!(Plane::for_view(view.camera.kind, 0.0), Plane::View(_));
        if input.right_pressed && self.is_under_way() {
            return Outcome::GaveUp;
        }
        let ray = input.mouse.map(|m| view.camera.ray_through(m, view.size));
        match self.step {
            Step::Waiting { .. } => {
                if input.pressed
                    && input.on_view
                    && let Some(ray) = ray
                {
                    self.start(&ray, view, input.ctrl);
                }
                Outcome::Going
            }
            Step::Footprint {
                plane, from, to, ..
            } => {
                let free = input.ctrl;
                let to = ray
                    .and_then(|r| plane.meet(&r))
                    .map_or(to, |at| snap2(at, view.grid, free));
                self.step = Step::Footprint {
                    plane,
                    from,
                    to,
                    free,
                };
                if input.released || !input.down {
                    return self.footprint_kept(input.mouse, view);
                }
                Outcome::Going
            }
            Step::Height {
                plane,
                from,
                to,
                lift,
                height,
                ..
            } => {
                let free = input.ctrl;
                let height = input
                    .mouse
                    .and_then(|m| lifted(lift, m, view))
                    .map_or(height, |h| snap(h, view.grid, free).max(0.0));
                self.step = Step::Height {
                    plane,
                    from,
                    to,
                    lift,
                    height,
                    free,
                };
                if input.pressed && input.on_view {
                    return self.finish(planned(self.shape, plane, (from, to), height));
                }
                Outcome::Going
            }
        }
    }

    /// A press: the footprint starts where `ray` meets the ground (or the
    /// top of what's under the pointer), or the view's plane.
    fn start(&mut self, ray: &Ray, view: &DrawView, ctrl: bool) {
        let base = super::surface::height_under(ray, view.area).unwrap_or(0.0);
        let plane = Plane::for_view(view.camera.kind, base);
        if let Some(at) = plane.meet(ray) {
            let at = snap2(at, view.grid, ctrl);
            self.step = Step::Footprint {
                plane,
                from: at,
                to: at,
                free: ctrl,
            };
        }
    }

    /// The footprint let go: nothing if it has no size; in a flat view's
    /// plane, the brush one grid step deep; else on to the height.
    fn footprint_kept(&mut self, mouse: Option<Vec2>, view: &DrawView) -> Outcome {
        let Step::Footprint {
            plane,
            from,
            to,
            free,
        } = self.step
        else {
            return Outcome::Going;
        };
        let size = (to - from).abs();
        if size.x < MIN_SIZE || size.y < MIN_SIZE {
            self.step = Step::Waiting {
                nothing: Some(Nothing::NoFootprint),
            };
            return Outcome::Going;
        }
        if let Plane::View(_) = plane {
            let deep = if view.grid > 0.0 { view.grid } else { 1.0 };
            return self.finish(planned(self.shape, plane, (from, to), deep));
        }
        let middle = plane.world((from + to) / 2.0);
        let lift = lift_for(middle, mouse.unwrap_or(view.size / 2.0), view);
        self.step = Step::Height {
            plane,
            from,
            to,
            lift,
            height: 0.0,
            free,
        };
        Outcome::Going
    }

    /// The brush made, or nothing (and why) if it has no size.
    fn finish(&mut self, planned: Option<Planned>) -> Outcome {
        match planned {
            Some(planned) => Outcome::Made(planned),
            None => {
                self.step = Step::Waiting {
                    nothing: Some(Nothing::NoHeight),
                };
                Outcome::Going
            }
        }
    }

    /// The brush as it would be made now, for the ghost: none before a
    /// footprint has a size.
    pub(crate) fn planned_now(&self, grid: f32) -> Option<Planned> {
        match self.step {
            Step::Waiting { .. } => None,
            Step::Footprint {
                plane, from, to, ..
            } => {
                let deep = match plane {
                    Plane::View(_) if grid > 0.0 => grid,
                    Plane::View(_) => 1.0,
                    Plane::Ground { .. } => MIN_SIZE,
                };
                planned(self.shape, plane, (from, to), deep)
            }
            Step::Height {
                plane,
                from,
                to,
                height,
                ..
            } => planned(self.shape, plane, (from, to), height.max(MIN_SIZE)),
        }
    }

    /// What the status bar says.
    pub(crate) fn status_words(&self, grid: f32) -> String {
        let name = self.shape.name();
        match self.step {
            Step::Waiting { nothing: Some(why) } => why.words().to_owned(),
            Step::Waiting { nothing: None } if self.flat_view => format!(
                "Drawing a {name}: drag for its footprint, one grid step deep · Ctrl for no \
                 snapping · Esc to give up"
            ),
            Step::Waiting { nothing: None } => format!(
                "Drawing a {name}: drag on the ground for its footprint · Ctrl for no snapping · \
                 Esc to give up"
            ),
            Step::Footprint {
                plane, from, to, ..
            } => {
                let next = match plane {
                    Plane::View(_) => format!(
                        "Let go to make it, {} m deep",
                        metres(if grid > 0.0 { grid } else { 1.0 })
                    ),
                    Plane::Ground { .. } => "Let go for the height".to_owned(),
                };
                format!(
                    "Drawing a {name}: footprint {} · {next} · Esc to give up",
                    footprint_words(from, to)
                )
            }
            Step::Height { height, .. } => format!(
                "Drawing a {name}: height {:.2} m · Click to keep · Esc to give up",
                height
            ),
        }
    }
}

/// `v` on the grid's `step`, unless `free` (or there's no step).
fn snap(v: f32, step: f32, free: bool) -> f32 {
    if free || step <= 0.0 || !step.is_finite() {
        v
    } else {
        (v / step).round() * step
    }
}

fn snap2(at: Vec2, step: f32, free: bool) -> Vec2 {
    Vec2::new(snap(at.x, step, free), snap(at.y, step, free))
}

/// A length in metres as the words say it: "4", "0.5", "1.25".
pub(crate) fn metres(v: f32) -> String {
    let text = format!("{v:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".to_owned()
    } else {
        text.to_owned()
    }
}

/// A footprint's two sides: "4 × 3 m".
pub(crate) fn footprint_words(from: Vec2, to: Vec2) -> String {
    let size = (to - from).abs();
    format!("{} \u{d7} {} m", metres(size.x), metres(size.y))
}

/// How the height will follow the pointer, the height step beginning
/// with the pointer at `mouse`: up the standing line, unless the camera
/// sees it nearly end on.
fn lift_for(middle: Vec3, mouse: Vec2, view: &DrawView) -> Lift {
    let ray = view.camera.ray_through(mouse, view.size);
    // Nearly straight down: the line is a dot on screen.
    if ray.direction.z.abs() > 0.97 {
        let project = view.camera.projector(view.size);
        let across = match (project(middle), project(middle + view.camera.right())) {
            (Some(a), Some(b)) => a.distance(b),
            _ => 0.0,
        };
        let per_point = if across > 1e-3 { 1.0 / across } else { 0.01 };
        return Lift::Screen {
            from: mouse.y,
            per_point,
        };
    }
    Lift::Line {
        middle,
        from: up_the_line(middle, &ray).unwrap_or(0.0),
    }
}

/// How far up the line standing on `middle` its nearest point to `ray`
/// is; none when they run side by side.
fn up_the_line(middle: Vec3, ray: &Ray) -> Option<f32> {
    let b = Vec3::Z.dot(ray.direction);
    let apart = 1.0 - b * b;
    if apart < 1e-6 {
        return None;
    }
    let w = middle - ray.origin;
    let (d, e) = (Vec3::Z.dot(w), ray.direction.dot(w));
    let up = (b * e - d) / apart;
    up.is_finite().then_some(up)
}

/// The height the pointer at `mouse` gives, before snapping.
fn lifted(lift: Lift, mouse: Vec2, view: &DrawView) -> Option<f32> {
    match lift {
        Lift::Line { middle, from } => {
            let ray = view.camera.ray_through(mouse, view.size);
            up_the_line(middle, &ray).map(|up| up - from)
        }
        Lift::Screen { from, per_point } => Some((from - mouse.y) * per_point),
    }
}

/// The brush a footprint from corner `from` to `to` on `plane` makes,
/// `height` high (or deep, in a view's plane); none if any side has no
/// size.
pub(crate) fn planned(
    shape: Shape,
    plane: Plane,
    (from, to): (Vec2, Vec2),
    height: f32,
) -> Option<Planned> {
    let size = (to - from).abs();
    if size.x < MIN_SIZE || size.y < MIN_SIZE || height.is_nan() || height < MIN_SIZE {
        return None;
    }
    let middle = (from + to) / 2.0;
    let (position, rotation, size) = match plane {
        Plane::Ground { .. } | Plane::View(ViewKind::TwoD | ViewKind::Top) => {
            let base = plane.world(middle);
            let (rotation, across, along) = climb(shape, from, to);
            (base, rotation, Vec3::new(across, along, height))
        }
        // The view's plane is upright: across it and up it, `height` deep.
        Plane::View(ViewKind::Front) => (
            Vec3::new(middle.x, 0.0, from.y.min(to.y)),
            Quat::IDENTITY,
            Vec3::new(size.x, height, size.y),
        ),
        Plane::View(ViewKind::Right) => (
            Vec3::new(0.0, middle.x, from.y.min(to.y)),
            Quat::IDENTITY,
            Vec3::new(height, size.x, size.y),
        ),
        Plane::View(ViewKind::Perspective) => return None,
    };
    Some(Planned {
        shape,
        position,
        rotation,
        settings: super::settings_for(shape, size),
    })
}

/// How a shape drawn from `from` to `to` on the ground is turned, and its
/// size across and along: the Ramp and Stairs climb along the longer side
/// (they climb along their own +Y), away from where the drag started;
/// every other shape stands square to the world.
fn climb(shape: Shape, from: Vec2, to: Vec2) -> (Quat, f32, f32) {
    let delta = to - from;
    let size = delta.abs();
    if !matches!(shape, Shape::Ramp | Shape::Stairs) {
        return (Quat::IDENTITY, size.x, size.y);
    }
    let half_turn = std::f32::consts::FRAC_PI_2;
    if size.x > size.y {
        // Along X: +Y turned a quarter to +X (or -X).
        let turn = if delta.x > 0.0 { -half_turn } else { half_turn };
        (Quat::from_rotation_z(turn), size.y, size.x)
    } else {
        let turn = if delta.y >= 0.0 {
            0.0
        } else {
            std::f32::consts::PI
        };
        (Quat::from_rotation_z(turn), size.x, size.y)
    }
}

#[cfg(test)]
#[path = "draw_tests.rs"]
mod tests;
