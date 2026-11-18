//! Trigger zones in the editor's view (stage 1, triggers; the approved
//! mock-up `docs/design/triggers/`): every zone outlined in teal, solid
//! with handles when it's picked and dashed when it isn't, with its name
//! and size beside it; the handles' drags that size it; and a freeform
//! zone's points, drawn and edited in a 2D view.
//!
//! The view never changes the level: it reports [`ZoneEvent`]s, which the
//! workbench turns into undoable edits, one step a drag.

pub(crate) mod card;
pub(crate) mod draw;
pub(crate) mod points;
pub(crate) mod sizing;

use glam::{Mat4, Vec2, Vec3};
use level::builtin::{TriggerZone, ZoneForm};
use reflect::EntityId;

/// A trigger zone the view outlines, where it is.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewZone {
    /// Its entity.
    pub entity: EntityId,
    /// Its entity's name, for the label beside it.
    pub name: String,
    /// Its shape and settings.
    pub zone: TriggerZone,
    /// Where it is in the world (its scale grows it).
    pub transform: Mat4,
}

/// What the view's zone tools did, for the workbench to carry out.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ZoneEvent {
    /// A handle's drag sized a zone: its new settings, and where its
    /// entity is now (a side dragged moves the middle; the far side
    /// stays). Merged into one undo step a drag.
    Sized {
        /// The zone's entity.
        entity: EntityId,
        /// Its new settings.
        zone: TriggerZone,
        /// Its entity's new place in the world.
        place: Vec3,
    },
    /// A freeform zone's points changed (drawn, closed, moved, added or
    /// taken away): one undo step each, a drag's merged.
    Points {
        /// The zone's entity.
        entity: EntityId,
        /// Its points, in its own space.
        points: Vec<Vec2>,
        /// Whether this belongs to a drag still under way.
        dragging: bool,
    },
    /// Drawing a freeform shape was given up: the shape it had stays.
    GaveUp,
    /// A change was refused, and why, for the status line.
    Refused(String),
}

/// The zone tools' state, kept between frames by the viewport.
#[derive(Debug, Clone, Default)]
pub(crate) struct ZoneTool {
    /// The zones picked: outlined solid (the workbench sets it each
    /// frame).
    pub(crate) picked: Vec<EntityId>,
    /// The one zone picked alone: its handles are shown.
    pub(crate) alone: Option<EntityId>,
    /// A handle's drag under way.
    pub(crate) drag: Option<sizing::SizeDrag>,
    /// The handle under the pointer, by zone and number.
    pub(crate) hovered: Option<(EntityId, usize)>,
    /// A freeform zone's points being drawn or edited.
    pub(crate) points: Option<points::PointsEdit>,
    /// What happened this frame.
    pub(crate) events: Vec<ZoneEvent>,
    /// What the status bar says for a moment (why a change was refused),
    /// and since when.
    note: Option<(String, std::time::Instant)>,
}

/// How long a note stays in the status bar, in seconds.
const NOTE_FOR: f64 = 4.0;

impl ZoneTool {
    /// Says `words` in the status bar for a moment.
    pub(crate) fn tell(&mut self, words: impl Into<String>) {
        self.note = Some((words.into(), std::time::Instant::now()));
    }

    /// What was said, while it is still shown.
    pub(crate) fn note_now(&self) -> Option<&str> {
        self.note
            .as_ref()
            .filter(|(_, at)| at.elapsed().as_secs_f64() < NOTE_FOR)
            .map(|(words, _)| words.as_str())
    }
}

/// A zone's footprint on the ground, in its own space: a box's four
/// corners, a circle's ring, or a freeform outline's points (none while
/// it can't be used). Counter-clockwise for a box and a circle.
pub(crate) fn footprint(zone: &TriggerZone) -> Vec<Vec2> {
    match zone.form {
        ZoneForm::Box => {
            let half = zone.size.truncate() * 0.5;
            vec![
                Vec2::new(-half.x, -half.y),
                Vec2::new(half.x, -half.y),
                Vec2::new(half.x, half.y),
                Vec2::new(-half.x, half.y),
            ]
        }
        ZoneForm::Round => {
            const STEPS: usize = 48;
            (0..STEPS)
                .map(|i| {
                    let a = i as f32 / STEPS as f32 * std::f32::consts::TAU;
                    Vec2::from_angle(a) * zone.radius
                })
                .collect()
        }
        ZoneForm::Freeform => zone.outline().map(<[Vec2]>::to_vec).unwrap_or_default(),
    }
}

/// The lines that draw a zone in a 3D view, as pairs of points in its
/// own space: a box's twelve edges or a sphere's three rings. A freeform
/// zone is its outline on the ground.
pub(crate) fn lines_3d(zone: &TriggerZone) -> Vec<(Vec3, Vec3)> {
    let ring = |points: &[Vec2], lift: fn(Vec2) -> Vec3| -> Vec<(Vec3, Vec3)> {
        let n = points.len();
        (0..n)
            .map(|i| (lift(points[i]), lift(points[(i + 1) % n])))
            .collect()
    };
    match zone.form {
        ZoneForm::Box => {
            let half = zone.size * 0.5;
            let corner = |i: usize| {
                Vec3::new(
                    if i & 1 == 0 { -half.x } else { half.x },
                    if i & 2 == 0 { -half.y } else { half.y },
                    if i & 4 == 0 { -half.z } else { half.z },
                )
            };
            BOX_EDGES
                .iter()
                .map(|&(a, b)| (corner(a), corner(b)))
                .collect()
        }
        ZoneForm::Round => {
            let circle = footprint(zone);
            let mut lines = ring(&circle, |p| Vec3::new(p.x, p.y, 0.0));
            lines.extend(ring(&circle, |p| Vec3::new(p.x, 0.0, p.y)));
            lines.extend(ring(&circle, |p| Vec3::new(0.0, p.x, p.y)));
            lines
        }
        ZoneForm::Freeform => ring(&footprint(zone), |p| p.extend(0.0)),
    }
}

/// The zone whose outline passes within `reach` points of `at` (points
/// from the view's corner), nearest first: how a click picks a zone,
/// which has no surface for the renderer to find. Zones in `locked` can't
/// be picked.
pub(crate) fn zone_at(
    zones: &[ViewZone],
    locked: &std::collections::HashSet<EntityId>,
    (camera, size): (&crate::camera::Camera, Vec2),
    at: Vec2,
    reach: f32,
) -> Option<EntityId> {
    let flat = camera.kind == crate::camera::ViewKind::TwoD;
    let near = |a: Vec2, b: Vec2| {
        let along = (b - a).length_squared();
        let t = if along > 0.0 {
            ((at - a).dot(b - a) / along).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (a + (b - a) * t).distance(at)
    };
    zones
        .iter()
        .filter(|z| !locked.contains(&z.entity))
        .filter_map(|zone| {
            let lines: Vec<(Vec3, Vec3)> = if flat {
                let ring = footprint(&zone.zone);
                let n = ring.len();
                (0..n)
                    .map(|i| (ring[i].extend(0.0), ring[(i + 1) % n].extend(0.0)))
                    .collect()
            } else {
                lines_3d(&zone.zone)
            };
            let world = zone.transform;
            let nearest = lines
                .iter()
                .filter_map(|(a, b)| {
                    let a = camera.project(world.transform_point3(*a), size)?;
                    let b = camera.project(world.transform_point3(*b), size)?;
                    Some(near(a, b))
                })
                .fold(f32::MAX, f32::min);
            (nearest <= reach).then_some((nearest, zone.entity))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, id)| id)
}

/// The twelve edges of a box, by corner number (bit 0: x, 1: y, 2: z).
const BOX_EDGES: [(usize, usize); 12] = [
    (0, 1),
    (2, 3),
    (4, 5),
    (6, 7),
    (0, 2),
    (1, 3),
    (4, 6),
    (5, 7),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

/// The words beside a zone: its name and size ("Door zone · 2 × 1 m",
/// "Pit · circle 1.5 m", "Yard · 12 points"). `flat`: in a 2D view.
pub(crate) fn label(name: &str, zone: &TriggerZone, flat: bool) -> String {
    let n = |v: f32| {
        let rounded = (v * 100.0).round() / 100.0;
        format!("{rounded}")
    };
    let size = match zone.form {
        ZoneForm::Box if flat => format!("{} \u{d7} {} m", n(zone.size.x), n(zone.size.y)),
        ZoneForm::Box => format!(
            "{} \u{d7} {} \u{d7} {} m",
            n(zone.size.x),
            n(zone.size.y),
            n(zone.size.z)
        ),
        ZoneForm::Round => format!(
            "{} {} m",
            ZoneForm::Round.shown(flat).to_lowercase(),
            n(zone.radius)
        ),
        ZoneForm::Freeform => format!("{} points", zone.points.len()),
    };
    format!("{name} \u{b7} {size}")
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
