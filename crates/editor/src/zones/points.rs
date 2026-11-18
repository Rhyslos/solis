//! A freeform zone's points, in a 2D view (stage 1, triggers; the project
//! lead's choice, 2026-11-18). Choosing Freeform starts **drawing**: each
//! click places a point, and clicking the first point or `Enter` closes
//! the shape; `Esc` gives up and keeps the shape the zone had. `Tab`
//! starts **editing** a freeform zone's points, as shape brushes do: drag
//! a point to move it, `Ctrl + click` on an edge adds a point there,
//! `Delete` takes the picked point away (never below three), and `Tab` or
//! `Esc` finishes.
//!
//! An outline may bend inward but never cross itself: a click, a drag or
//! a deletion that would make it cross is refused, saying why, and the
//! last good shape is kept. At most 256 points.

use super::ZoneEvent;
use crate::camera::Camera;
use glam::{Mat4, Vec2};
use level::builtin::TriggerZone;
use mesh::outline::{MAX_OUTLINE_POINTS, MIN_OUTLINE_POINTS};
use reflect::EntityId;

/// How close (points) the pointer must be to a point or an edge to take
/// it.
pub(crate) const POINT_REACH: f32 = 8.0;

/// Drawing a new outline, or editing the one the zone has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stage {
    /// Placing points one by one.
    Drawing,
    /// Moving, adding and taking away points.
    Editing,
}

/// A freeform zone's points being drawn or edited.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PointsEdit {
    /// The zone's entity.
    pub(crate) entity: EntityId,
    /// Where the zone is in the world (its points are in its own space).
    pub(crate) world: Mat4,
    /// Drawing or editing.
    pub(crate) stage: Stage,
    /// The points so far, in the zone's own space.
    pub(crate) points: Vec<Vec2>,
    /// The point picked while editing (`Delete` takes it away).
    pub(crate) picked: Option<usize>,
    /// The point being dragged, while editing.
    pub(crate) dragging: Option<usize>,
    /// Where the pointer is, in the zone's own space (for the faint line
    /// to it while drawing).
    pub(crate) pointer: Option<Vec2>,
}

/// One frame's pointer, in points from the view's corner.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PointsInput {
    pub(crate) mouse: Option<Vec2>,
    pub(crate) pressed: bool,
    pub(crate) down: bool,
    pub(crate) ctrl: bool,
}

impl PointsEdit {
    /// Starts drawing a new outline on the zone at `world`.
    pub(crate) fn drawing(entity: EntityId, world: Mat4) -> Self {
        Self {
            entity,
            world,
            stage: Stage::Drawing,
            points: Vec::new(),
            picked: None,
            dragging: None,
            pointer: None,
        }
    }

    /// Starts editing the points of `zone` at `world`.
    pub(crate) fn editing(entity: EntityId, world: Mat4, zone: &TriggerZone) -> Self {
        Self {
            stage: Stage::Editing,
            points: zone.points.clone(),
            ..Self::drawing(entity, world)
        }
    }

    /// Where a point of the zone's own space is on the screen.
    fn on_screen(&self, p: Vec2, camera: &Camera, size: Vec2) -> Option<Vec2> {
        camera.project(self.world.transform_point3(p.extend(0.0)), size)
    }

    /// Where the pointer at `mouse` is in the zone's own space, on its
    /// ground.
    fn in_zone(&self, mouse: Vec2, camera: &Camera, size: Vec2) -> Option<Vec2> {
        let ray = camera.ray_through(mouse, size);
        let z = self.world.w_axis.z;
        let ground = ray
            .hit_ground(z)
            .or_else(|| (ray.direction.z.abs() < 1e-6).then_some(ray.origin))?;
        let local = self.world.inverse().transform_point3(ground).truncate();
        local.is_finite().then_some(local)
    }

    /// The point within reach of `mouse`, nearest first.
    fn point_at(&self, mouse: Vec2, camera: &Camera, size: Vec2) -> Option<usize> {
        self.points
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                let d = self.on_screen(*p, camera, size)?.distance(mouse);
                (d <= POINT_REACH).then_some((d, i))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, i)| i)
    }

    /// The edge within reach of `mouse` (by the point it starts at), and
    /// where along it the mouse is (0 to 1).
    fn edge_at(&self, mouse: Vec2, camera: &Camera, size: Vec2) -> Option<(usize, f32)> {
        let n = self.points.len();
        (0..n)
            .filter_map(|i| {
                let a = self.on_screen(self.points[i], camera, size)?;
                let b = self.on_screen(self.points[(i + 1) % n], camera, size)?;
                let along = (b - a).length_squared();
                let t = if along > 0.0 {
                    ((mouse - a).dot(b - a) / along).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let d = (a + (b - a) * t).distance(mouse);
                (d <= POINT_REACH && t > 0.0 && t < 1.0).then_some((d, i, t))
            })
            .min_by(|x, y| x.0.total_cmp(&y.0))
            .map(|(_, i, t)| (i, t))
    }

    /// Takes this frame's pointer: places, closes, picks, drags or adds
    /// points. What happened goes into `events`. Returns whether the edit
    /// is over (a drawing closed).
    pub(crate) fn interact(
        &mut self,
        input: PointsInput,
        camera: &Camera,
        size: Vec2,
        events: &mut Vec<ZoneEvent>,
    ) -> bool {
        self.pointer = input.mouse.and_then(|m| self.in_zone(m, camera, size));
        match self.stage {
            Stage::Drawing => self.draw(input, camera, size, events),
            Stage::Editing => {
                self.edit(input, camera, size, events);
                false
            }
        }
    }

    fn draw(
        &mut self,
        input: PointsInput,
        camera: &Camera,
        size: Vec2,
        events: &mut Vec<ZoneEvent>,
    ) -> bool {
        let (Some(mouse), Some(at)) = (input.mouse, self.pointer) else {
            return false;
        };
        if !input.pressed {
            return false;
        }
        let on_first = self.points.len() >= MIN_OUTLINE_POINTS
            && self
                .on_screen(self.points[0], camera, size)
                .is_some_and(|p| p.distance(mouse) <= POINT_REACH);
        if on_first {
            return self.close(events);
        }
        self.place(at, events);
        false
    }

    /// Places a point at `at` while drawing, unless its edge would cross
    /// the outline so far or there are as many points as there can be.
    pub(crate) fn place(&mut self, at: Vec2, events: &mut Vec<ZoneEvent>) {
        if self.points.len() >= MAX_OUTLINE_POINTS {
            events.push(ZoneEvent::Refused(format!(
                "A shape has at most {MAX_OUTLINE_POINTS} points: click the first point or press \
                 Enter to close it."
            )));
            return;
        }
        if self.points.last() == Some(&at) {
            return;
        }
        if let Some(&last) = self.points.last() {
            let n = self.points.len();
            let crosses = (0..n.saturating_sub(2)).any(|i| {
                mesh::outline::segments_meet(self.points[i], self.points[i + 1], last, at)
            });
            if crosses {
                events.push(ZoneEvent::Refused(
                    "That point would make the outline cross itself.".into(),
                ));
                return;
            }
        }
        self.points.push(at);
    }

    /// Closes the outline drawn (the first point clicked, or `Enter`), if
    /// it's usable: the zone takes it. Returns whether it closed.
    pub(crate) fn close(&mut self, events: &mut Vec<ZoneEvent>) -> bool {
        match mesh::outline::check(&self.points) {
            Ok(()) => {
                events.push(ZoneEvent::Points {
                    entity: self.entity,
                    points: self.points.clone(),
                    dragging: false,
                });
                true
            }
            Err(why) => {
                events.push(ZoneEvent::Refused(format!("The shape can't close: {why}.")));
                false
            }
        }
    }

    fn edit(
        &mut self,
        input: PointsInput,
        camera: &Camera,
        size: Vec2,
        events: &mut Vec<ZoneEvent>,
    ) {
        if let Some(i) = self.dragging {
            match (input.down, self.pointer) {
                (true, Some(at)) => self.move_point(i, at, events),
                _ => {
                    // Let go: the drag's last change, in its undo step.
                    self.dragging = None;
                    events.push(ZoneEvent::Points {
                        entity: self.entity,
                        points: self.points.clone(),
                        dragging: true,
                    });
                }
            }
            return;
        }
        let Some(mouse) = input.mouse.filter(|_| input.pressed) else {
            return;
        };
        if input.ctrl {
            if let Some((edge, t)) = self.edge_at(mouse, camera, size) {
                self.add_on_edge(edge, t, events);
            }
            return;
        }
        self.picked = self.point_at(mouse, camera, size);
        self.dragging = self.picked;
    }

    /// Moves point `i` to `at`, unless the outline would then cross
    /// itself (it stays where it was last good).
    fn move_point(&mut self, i: usize, at: Vec2, events: &mut Vec<ZoneEvent>) {
        let mut moved = self.points.clone();
        if moved.get(i) == Some(&at) {
            return;
        }
        moved[i] = at;
        match mesh::outline::check(&moved) {
            Ok(()) => {
                self.points = moved;
                events.push(ZoneEvent::Points {
                    entity: self.entity,
                    points: self.points.clone(),
                    dragging: true,
                });
            }
            Err(why) => events.push(ZoneEvent::Refused(format!(
                "The point stays where it was: {why}."
            ))),
        }
    }

    /// Adds a point `t` of the way along the edge starting at point
    /// `edge`, and picks it.
    pub(crate) fn add_on_edge(&mut self, edge: usize, t: f32, events: &mut Vec<ZoneEvent>) {
        let n = self.points.len();
        if n >= MAX_OUTLINE_POINTS {
            events.push(ZoneEvent::Refused(format!(
                "A shape has at most {MAX_OUTLINE_POINTS} points."
            )));
            return;
        }
        let (a, b) = (self.points[edge], self.points[(edge + 1) % n]);
        self.points.insert(edge + 1, a.lerp(b, t));
        self.picked = Some(edge + 1);
        events.push(ZoneEvent::Points {
            entity: self.entity,
            points: self.points.clone(),
            dragging: false,
        });
    }

    /// Takes the picked point away (`Delete`), keeping three at least and
    /// an outline that doesn't cross itself.
    pub(crate) fn delete_picked(&mut self, events: &mut Vec<ZoneEvent>) {
        let Some(i) = self.picked.filter(|&i| i < self.points.len()) else {
            return;
        };
        if self.points.len() <= MIN_OUTLINE_POINTS {
            events.push(ZoneEvent::Refused(format!(
                "A shape keeps at least {MIN_OUTLINE_POINTS} points."
            )));
            return;
        }
        let mut fewer = self.points.clone();
        fewer.remove(i);
        match mesh::outline::check(&fewer) {
            Ok(()) => {
                self.points = fewer;
                self.picked = None;
                events.push(ZoneEvent::Points {
                    entity: self.entity,
                    points: self.points.clone(),
                    dragging: false,
                });
            }
            Err(why) => events.push(ZoneEvent::Refused(format!(
                "The point stays: without it {why}."
            ))),
        }
    }
}
