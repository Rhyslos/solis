//! The workbench's side of the viewport's tools: where the gizmo goes and
//! what the selection's outline is, each frame; carrying out clicks,
//! boxes and gizmo drags; and the keys that go with them (Esc, Ctrl+A, X,
//! Y, Z and L, chosen with the project lead, 2026-10-23).

use crate::commands::{component_edits, merge_key};
use crate::gizmo::{Change, Gizmo, GizmoTool};
use crate::picking::{after_box, after_click};
use crate::transforming::{Targets, moved, snapped, targets};
use crate::viewport::Tool;
use crate::viewport_tools::{Outline, ViewportEvent};
use crate::workbench::Workbench;
use area::builtin::{Model, Transform};
use reflect::Component;

impl Workbench {
    /// The gizmo for the chosen tool, at the selection, if it has
    /// something to move.
    fn gizmo(&self) -> Option<Gizmo> {
        let tool = match self.viewport.tool {
            Tool::Select => return None,
            Tool::Move => GizmoTool::Move,
            Tool::Rotate => GizmoTool::Rotate,
            Tool::Scale => GizmoTool::Scale,
            Tool::Transform => GizmoTool::All,
        };
        let t = targets(
            self.document.area(),
            &self.document.selection,
            self.viewport.snapping.object_axes,
        )?;
        Some(Gizmo {
            origin: t.pivot,
            axes: t.axes,
            tool,
        })
    }

    /// What the selection's outline goes around: each selected model's
    /// box, or a point for anything else.
    fn outlines(&self) -> Vec<Outline> {
        let area = self.document.area();
        let hidden = self.area_panel.in_effect(area).0;
        self.document
            .selection
            .iter()
            .filter(|id| !hidden.contains(id))
            .filter_map(|id| {
                let entity = area.get(id)?;
                let world = area.world_matrix(id);
                let whole_model = entity
                    .get::<Model>()
                    .and_then(|m| m.asset)
                    .filter(|r| r.part.is_none())
                    .and_then(|r| self.models.get(&r.asset));
                Some(match whole_model {
                    Some(info) => Outline::Box {
                        world,
                        min: info.min,
                        max: info.max,
                    },
                    None => Outline::Point(world.w_axis.truncate()),
                })
            })
            .collect()
    }

    /// Hands the viewport this frame's gizmo and outline.
    pub(crate) fn prepare_tools(&mut self) {
        self.viewport.tools.gizmo = self.gizmo();
        self.viewport.tools.outlines = self.outlines();
    }

    /// Carries out what the viewport's tools did this frame.
    pub(crate) fn tool_events(&mut self) {
        for event in std::mem::take(&mut self.viewport.tools.events) {
            match event {
                ViewportEvent::GizmoStart => {
                    self.moving = targets(
                        self.document.area(),
                        &self.document.selection,
                        self.viewport.snapping.object_axes,
                    )
                    .map(|t| (t, self.gesture));
                }
                ViewportEvent::GizmoChange { change, flip } => {
                    if let Some((t, gesture)) = self.moving.take() {
                        let change = snapped(change, &t, &self.viewport.snapping, flip);
                        self.set_transforms(&t, Some(change), gesture);
                        self.moving = Some((t, gesture));
                    }
                }
                ViewportEvent::GizmoEnd => self.moving = None,
                ViewportEvent::GizmoCancel => {
                    if let Some((t, gesture)) = self.moving.take() {
                        self.set_transforms(&t, None, gesture);
                    }
                }
                ViewportEvent::Click { hit, mode } => {
                    self.document.selection =
                        after_click(self.document.area(), &self.document.selection, hit, mode);
                    self.area_panel.reveal = self.document.selection.primary();
                }
                ViewportEvent::Box { hits, mode } => {
                    self.document.selection =
                        after_box(self.document.area(), &self.document.selection, &hits, mode);
                    self.area_panel.reveal = self.document.selection.primary();
                }
            }
        }
    }

    /// Sets every target's transform for `change` (or back to where it
    /// started, for `None`), merged into the drag's one undo step. A
    /// cancelled drag ends where it started, so it leaves no step.
    fn set_transforms(&mut self, targets: &Targets, change: Option<Change>, gesture: u64) {
        let area = self.document.area();
        let new: Vec<_> = match change {
            Some(change) => moved(targets, change),
            None => targets.entities.iter().map(|t| (t.id, t.start)).collect(),
        };
        let edits: Vec<_> = new
            .iter()
            .filter_map(|(id, transform)| {
                let entity = area.get(*id)?;
                Some(component_edits(
                    *id,
                    entity.components.get(Transform::NAME),
                    transform,
                ))
            })
            .flatten()
            .collect();
        let verb = match change {
            Some(Change::Rotate { .. }) => "Rotate",
            Some(Change::Scale(_)) => "Scale",
            _ => "Move",
        };
        let label = match targets.entities.len() {
            1 => verb.to_string(),
            n => format!("{verb} {n} entities"),
        };
        self.command(&label, edits, Some(merge_key("gizmo", gesture)), None);
    }

    /// Esc cancels a drag or selects nothing; Ctrl+A selects everything;
    /// X, Y and Z lock an axis and L switches world and object axes. Not
    /// while typing, nor while flying.
    pub(crate) fn tool_extra_keys(&mut self, ui: &egui::Ui) {
        if ui.ctx().egui_wants_keyboard_input() || self.viewport.is_flying() {
            return;
        }
        let (escape, all, x, y, z, l) = ui.input_mut(|i| {
            let plain = |i: &egui::InputState, key| i.key_pressed(key) && i.modifiers.is_none();
            (
                plain(i, egui::Key::Escape),
                i.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::COMMAND,
                    egui::Key::A,
                )),
                plain(i, egui::Key::X),
                plain(i, egui::Key::Y),
                plain(i, egui::Key::Z),
                plain(i, egui::Key::L),
            )
        });
        if escape && !self.viewport.tools.cancel() {
            self.document.selection.clear();
        }
        if all {
            self.select_all();
        }
        let snapping = &mut self.viewport.snapping;
        for (pressed, axis) in [(x, 0), (y, 1), (z, 2)] {
            if pressed {
                snapping.locked[axis] = !snapping.locked[axis];
            }
        }
        if l {
            snapping.object_axes = !snapping.object_axes;
        }
    }
}

#[cfg(test)]
#[path = "workbench_tools_tests.rs"]
mod tests;
