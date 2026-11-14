//! Making shape brushes (Tools 1a, B2; decision 0111; the approved
//! mock-ups `docs/design/tools-1/01-brush-list.png` and
//! `02-brush-draw.png`): the brush list B opens, and drawing the shape
//! taken out on the ground.
//!
//! - **The list** ([`list`]): **B** with the pointer over the 3D or 2D
//!   view (or Home › Create › Shape) opens it beside the pointer; each
//!   further B moves the highlight down, wrapping at the end; **Space**
//!   or a click takes the shape; **Esc** or a click elsewhere closes it.
//! - **Drawing out** ([`draw`]): drag for the footprint on the grid, let
//!   go, move the mouse up for the height and click; Ctrl draws freely,
//!   Esc or a right-click gives up. In a flat side view or the 2D view
//!   the footprint is drawn in the view's plane and the shape is one grid
//!   step deep. What is drawn over the view while it happens is
//!   [`ghost`]; where a drag starts on top of a brush is [`surface`].
//!
//! - **Editing the insides** ([`edit`], Tools 1a B3): **Tab** with one
//!   brush picked, then **1**, **2** and **3** for points, edges and
//!   faces; picking them ([`parts`]), moving, turning and stretching them
//!   with the gizmo ([`reshape`]), what's drawn over the view meanwhile
//!   ([`edit_draw`]) and the Brush card ([`card`]). B4: extruding and
//!   insetting with the mouse ([`pull`]), and what Extrude, Inset, Split
//!   and Delete do with what's picked ([`shaping`]).
//!
//! The viewport keeps this state ([`Brushing`]) and reports a brush drawn
//! as a [`crate::viewport_tools::ViewportEvent::BrushDrawn`]; the
//! workbench adds it as one undo step (`workbench_brushes.rs`). Nothing
//! here walks the area: opening and stepping the list cost the same in
//! any area, and drawing out tests what's under the pointer once, as a
//! drag starts; editing tries only the brush being edited.

pub(crate) mod card;
pub(crate) mod draw;
pub(crate) mod edit;
pub(crate) mod edit_draw;
pub(crate) mod ghost;
pub(crate) mod list;
pub(crate) mod parts;
pub(crate) mod pull;
pub(crate) mod reshape;
pub(crate) mod shaping;
pub(crate) mod surface;

use crate::icons::Icon;
use glam::Vec3;
use mesh::{Shape, ShapeSettings};

/// The brush list and a brush being drawn out, kept by the viewport.
#[derive(Debug, Clone, Default)]
pub(crate) struct Brushing {
    /// The brush list, while it is open.
    pub(crate) list: Option<list::List>,
    /// The shape taken from the list, while it is drawn out.
    pub(crate) draw: Option<draw::Draw>,
    /// The brush whose insides are being edited (B3).
    pub(crate) edit: Option<edit::Editing>,
    /// Something the status bar says for a moment (why a change was put
    /// back, that Made as replaced the edits), and when it was said.
    pub(crate) note: Option<(String, std::time::Instant)>,
}

/// How long a note stays in the status bar, in seconds: as long as what
/// a click on an unfinished part says.
const NOTE_FOR: f64 = crate::unbuilt::SAID_FOR;

impl Brushing {
    /// Whether the list is open or a brush is being drawn out: the Shape
    /// button is lit, and the status bar and Hints say what is going on.
    pub(crate) fn is_active(&self) -> bool {
        self.list.is_some() || self.draw.is_some()
    }

    /// Esc: closes the list, or gives up the brush being drawn, leaving
    /// nothing. Whether there was anything to close.
    pub(crate) fn escape(&mut self) -> bool {
        self.list.take().is_some() || self.draw.take().is_some()
    }

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

    /// What the status bar says while the list is open or a brush is
    /// being drawn out.
    pub(crate) fn status_words(&self, grid: f32) -> Option<String> {
        if let Some(list) = &self.list {
            return Some(format!("Choosing a shape brush: {}", list.chosen().name()));
        }
        self.draw.as_ref().map(|draw| draw.status_words(grid))
    }

    /// What the Hints card says while the list is open or a brush is
    /// being drawn out. Keys are in backticks, which the card draws as
    /// keycaps.
    pub(crate) fn hint(&self, grid: f32) -> Option<String> {
        if let Some(edit) = &self.edit {
            return Some(edit_hint(edit, grid));
        }
        if self.list.is_some() {
            return Some(
                "Shape brushes are blocks you build a level from: floors, walls, ramps, stairs. \
                 Press `B` over the 3D view to open the list, `B` again to move down it and \
                 `Space` to take one; then draw it out on the ground."
                    .to_owned(),
            );
        }
        let draw = self.draw.as_ref()?;
        let grid = draw::metres(grid);
        Some(if draw.in_plane() {
            format!(
                "Drawing out a brush in a flat view: drag for its footprint and let go to make \
                 it, one grid step deep. Sizes snap to the grid (Grid · {grid} in the ribbon); \
                 hold `Ctrl` to draw freely."
            )
        } else {
            format!(
                "Drawing out a brush: drag on the ground for its footprint; let go, then move the \
                 mouse up for its height and click. Sizes snap to the grid (Grid · {grid} in the \
                 ribbon); hold `Ctrl` to draw freely."
            )
        })
    }
}

/// What the Hints card says while a brush's insides are edited: what
/// an extrude or inset under way does, or what editing offers.
fn edit_hint(edit: &edit::Editing, grid: f32) -> String {
    let grid = draw::metres(grid);
    match edit.pull.as_ref().map(|p| p.kind) {
        Some(pull::PullKind::Extrude | pull::PullKind::ExtrudeEdges) => format!(
            "Extruding: move the mouse to pull the new part out, and click to keep it. The \
             distance snaps to the grid (Grid · {grid}); hold `Ctrl` for no snapping. `Esc` or \
             a right-click gives up and leaves nothing."
        ),
        Some(pull::PullKind::Inset) => format!(
            "Insetting: move the mouse to set how wide the border round each face is, and \
             click to keep it. It snaps to the grid (Grid · {grid}); hold `Ctrl` for no \
             snapping. `Esc` or a right-click gives up."
        ),
        None => "Editing a brush's insides: `1` picks points, `2` edges, `3` faces. Click one, \
                 `Ctrl` + click to add or take away, or drag a box. Move, turn or stretch them \
                 with the gizmo (`W`, `R`, `S`); `Ctrl` turns snapping off for one drag. `E` \
                 extrudes the picked faces (or `Shift` + drag the move gizmo); Inset, Split and \
                 Delete are on the ribbon, and `Delete` deletes what's picked, closing the gap. \
                 `Tab` when done."
            .to_owned(),
    }
}

/// The icon of a shape, in the brush list and the steps over the view.
pub(crate) fn icon(shape: Shape) -> Icon {
    match shape {
        Shape::Box => Icon::ShapeBox,
        Shape::Ramp => Icon::ShapeRamp,
        Shape::Cylinder => Icon::ShapeCylinder,
        Shape::Stairs => Icon::ShapeStairs,
        Shape::Arch => Icon::ShapeArch,
        Shape::Sphere => Icon::ShapeSphere,
        Shape::Wedge => Icon::ShapeWedge,
    }
}

/// How high one step of drawn-out Stairs is, in metres.
pub(crate) const STEP_RISE: f32 = 0.25;
/// Sides round a drawn-out Cylinder or Sphere.
pub(crate) const ROUND_SIDES: u32 = 16;
/// Pieces of a drawn-out Arch's curve.
pub(crate) const ARCH_PIECES: u32 = 12;

/// A shape's settings when it is drawn out (or made again at a size, as
/// Made as does) at `size`: the size, and the counts the brush list
/// starts with (the project lead's morning list, 2026-11-14): 16 sides
/// round a Cylinder or Sphere, 12 pieces in an Arch's curve, a step of
/// Stairs for each 25 cm of height. Within the shapes' limits.
pub(crate) fn settings_for(shape: Shape, size: Vec3) -> ShapeSettings {
    let mut settings = ShapeSettings {
        size,
        ..ShapeSettings::default()
    };
    match shape {
        Shape::Cylinder | Shape::Sphere => settings.sides = ROUND_SIDES,
        Shape::Arch => settings.sides = ARCH_PIECES,
        Shape::Stairs => {
            let steps = (size.z / STEP_RISE).round();
            settings.steps = if steps.is_finite() && steps >= 1.0 {
                (steps as u32).min(mesh::shapes::MAX_STEPS)
            } else {
                1
            };
        }
        Shape::Box | Shape::Ramp | Shape::Wedge => {}
    }
    settings.clamped()
}

#[cfg(test)]
#[path = "brushing/picture_tests.rs"]
mod picture_tests;
#[cfg(test)]
#[path = "brushing/tests.rs"]
mod tests;
