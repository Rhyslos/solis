//! The workbench's side of editing a brush's insides (Tools 1a, B3):
//! Tab starting and ending it, 1, 2 and 3 choosing what clicks pick,
//! E, Delete and Backspace shaping it (B4), editing kept in step with the area
//! each frame (and ended when the brush goes, something else is picked,
//! the game plays or an undo goes back past its start), the status bar's
//! words, the Brush card's facts and Solid.
//!
//! Picking parts, the gizmo's drags and typing a size are in
//! `workbench_brush_parts.rs`.

use crate::brushing::card::BrushFacts;
use crate::brushing::edit::{Editing, Mode};
use crate::brushing::parts::PartCache;
use crate::play::PlayState;
use crate::workbench::Workbench;
use area::builtin::Brush;
use egui::{Key, Modifiers};
use reflect::{Component, EntityId, Value};
use std::sync::Arc;

impl Workbench {
    /// Tab starts or ends editing the picked brush's insides; while
    /// editing, 1, 2 and 3 choose points, edges or faces, E extrudes the
    /// picked faces, and Delete and Backspace delete the picked parts
    /// (never the brush; B4). While an extrude or inset is under way Esc
    /// gives it up and the other keys wait. Only where the keys are the
    /// area's: the tile map's Tab (its Meaning view, with a map picked)
    /// and the UI canvas's 1, 2 and 3 are theirs.
    pub(crate) fn brush_edit_keys(&mut self, ui: &egui::Ui) {
        if !self.keys.is_area()
            || self.viewport.is_flying()
            || self.viewport.tile_overlay.paint.allowed
            || self.viewport.brushing.list.is_some()
            || self.viewport.brushing.draw.is_some()
        {
            return;
        }
        let editing = self.viewport.brushing.edit.is_some();
        if !editing && self.why_not_edit_brush().is_some() {
            return;
        }
        if self.is_pulling() {
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                self.give_up_pull();
            }
            return;
        }
        let (tab, mode, delete, extrude) = ui.input_mut(|i| {
            let tab = i.consume_key(Modifiers::NONE, Key::Tab);
            if !editing {
                return (tab, None, false, false);
            }
            let mode = Mode::ALL
                .into_iter()
                .find(|m| i.consume_key(Modifiers::NONE, m.key()));
            let delete = i.consume_key(Modifiers::NONE, Key::Delete)
                | i.consume_key(Modifiers::NONE, Key::Backspace);
            let extrude = i.consume_key(Modifiers::NONE, Key::E);
            (tab, mode, delete, extrude)
        });
        if tab {
            // Taken from egui too, which would move the keyboard's focus
            // to the first field this frame draws.
            ui.ctx()
                .memory_mut(|m| m.move_focus(egui::FocusDirection::None));
            self.toggle_brush_editing();
        }
        if let Some(mode) = mode {
            self.brush_mode(mode);
        }
        if delete {
            self.shape_brush(crate::shell::ShellCommand::BrushDelete);
        }
        if extrude {
            self.shape_brush(crate::shell::ShellCommand::BrushExtrude);
        }
    }

    /// Why the picked brush's insides can't be edited now, or `None` when
    /// they can (or are being edited, when Tab ends it).
    pub(crate) fn why_not_edit_brush(&self) -> Option<String> {
        if self.viewport.brushing.edit.is_some() {
            return None;
        }
        if self.play_state() != PlayState::Editing {
            return Some("Stop the game first".to_owned());
        }
        let selection = &self.open.document.selection;
        let one = selection
            .primary()
            .filter(|_| selection.iter().count() == 1);
        let brush = one.filter(|id| {
            self.open
                .document
                .area()
                .get(*id)
                .is_some_and(|e| e.has(Brush::NAME))
        });
        brush
            .is_none()
            .then(|| "Pick one shape brush first".to_owned())
    }

    /// Starts editing the picked brush's insides, or ends it (Tab, Done).
    pub(crate) fn toggle_brush_editing(&mut self) {
        if self.viewport.brushing.edit.is_some() {
            self.stop_brush_editing();
        } else if self.why_not_edit_brush().is_none()
            && let Some(id) = self.open.document.selection.primary()
        {
            self.start_brush_editing(id);
        }
    }

    /// Starts editing `brush`'s insides: faces mode, nothing picked, HOME
    /// TOOLS · Brush in front.
    fn start_brush_editing(&mut self, brush: EntityId) {
        // A drag of the entity's gizmo, or a box, ends first.
        self.viewport.tools.cancel();
        self.viewport.tools.events.clear();
        self.open.moving = None;
        let since = self.open.document.history().state();
        self.viewport.brushing.edit = Some(Editing::new(brush, since));
        self.open.document.close_step();
        self.show_brush_tools();
        self.sync_brush_edit();
    }

    /// Ends editing: a drag under way is given up, and Home's own ribbon
    /// comes back.
    pub(crate) fn stop_brush_editing(&mut self) {
        if let Some(edit) = self.viewport.brushing.edit.take() {
            self.viewport.tools.cancel();
            self.viewport.tools.events.clear();
            self.viewport.tools.parts = false;
            self.show_edited_model(edit.entity, None);
        }
    }

    /// Picks points, edges or faces from now on (1, 2, 3).
    pub(crate) fn brush_mode(&mut self, mode: Mode) {
        if let Some(edit) = self.viewport.brushing.edit.as_mut() {
            edit.switch_to(mode);
        }
    }

    /// Whether the brush being edited is solid, or `None` when none is.
    pub(crate) fn edited_brush_solid(&self) -> Option<bool> {
        let edit = self.viewport.brushing.edit.as_ref()?;
        let brush = self.open.document.area().get(edit.entity)?;
        match brush.components.get(Brush::NAME)?.get("solid") {
            Some(Value::Bool(solid)) => Some(*solid),
            _ => Some(true),
        }
    }

    /// Makes the brush being edited solid, or not, as one undo step.
    pub(crate) fn switch_edited_brush_solid(&mut self) {
        let (Some(edit), Some(solid)) = (&self.viewport.brushing.edit, self.edited_brush_solid())
        else {
            return;
        };
        let edits = vec![area::Edit::SetField {
            entity: edit.entity,
            component: Brush::NAME.into(),
            field: "solid".into(),
            value: Some(Value::Bool(!solid)),
        }];
        let label = if solid {
            "Make not solid"
        } else {
            "Make solid"
        };
        self.command(label, edits, None, None);
    }

    /// Keeps editing in step with the area, each frame before the tools
    /// are prepared: it ends when the brush goes or loses its Brush,
    /// something else is picked, the game plays, or an undo goes back
    /// past where editing began; otherwise the picking geometry follows
    /// the brush's model, and a drag's brush is drawn in its place.
    pub(crate) fn sync_brush_edit(&mut self) {
        let Some(edit) = &self.viewport.brushing.edit else {
            self.viewport.tools.parts = false;
            return;
        };
        let entity = edit.entity;
        let selection = &self.open.document.selection;
        let still_picked = selection.primary() == Some(entity) && selection.iter().count() == 1;
        let model = self.open.brushes.get(entity).cloned();
        let undone_past = self.open.document.history().state() < edit.since;
        let playing = self.play_state() != PlayState::Editing;
        let Some(model) = model.filter(|_| still_picked && !undone_past && !playing) else {
            self.stop_brush_editing();
            return;
        };
        self.viewport.tools.parts = true;
        let Some(edit) = self.viewport.brushing.edit.as_mut() else {
            return;
        };
        if edit.parts.as_ref().is_none_or(|p| p.key != model.key) {
            edit.parts = Some(Arc::new(PartCache::of(Arc::clone(&model.mesh), model.key)));
            edit.forget_missing();
            // An undo under way changed the brush: what was being pulled
            // out of it is given up.
            if edit.pull.take().is_some() {
                edit.preview = None;
            }
        }
        if let Some(name) = self.open.document.area().get(entity).map(|e| &e.name)
            && *name != edit.name
        {
            edit.name.clone_from(name);
        }
        let preview = edit.preview.clone();
        self.show_edited_model(entity, preview);
    }

    /// Draws `entity`'s brush with `preview` (a drag's model) in what
    /// the view shows, or with its own model again: only its own entry
    /// changes.
    pub(crate) fn show_edited_model(
        &mut self,
        entity: EntityId,
        preview: Option<Arc<crate::brushes::BrushModel>>,
    ) {
        let Some(model) = preview.or_else(|| self.open.brushes.get(entity).cloned()) else {
            return;
        };
        if let Some(shown) = self
            .viewport
            .area
            .brushes
            .iter_mut()
            .find(|b| b.entity == entity)
            && !Arc::ptr_eq(&model, &shown.model)
        {
            shown.model = model;
        }
    }

    /// What the status bar says about brushes: a note for a moment (why a
    /// change was put back), or while editing "Editing Box_03 · Faces ·
    /// 1 face picked", or the brush list's and drawing out's words.
    pub(crate) fn brush_status(&self) -> Option<String> {
        let brushing = &self.viewport.brushing;
        if let Some(note) = brushing.note_now() {
            return Some(note.to_owned());
        }
        if let Some(edit) = &brushing.edit {
            return Some(edit.status_words());
        }
        brushing.status_words(self.viewport.snapping.grid_step)
    }

    /// What the Brush card shows of the brush Properties shows, worked
    /// out from its model's mesh (already read): its counts, its size,
    /// and while it is edited, the mode and what's picked.
    pub(crate) fn brush_facts(&self) -> Option<BrushFacts> {
        let selection = &self.open.document.selection;
        let id = selection.primary()?;
        let model = self.open.brushes.get(id)?;
        let editing = self
            .viewport
            .brushing
            .edit
            .as_ref()
            .filter(|e| e.entity == id)
            .map(|e| (e.mode, e.picked_words()));
        // While an extrude or inset is under way the counts stay the
        // brush's own, as the mock-up draws them.
        let mesh = self
            .viewport
            .brushing
            .edit
            .as_ref()
            .filter(|e| e.entity == id && e.pull.is_none())
            .and_then(|e| e.mesh())
            .unwrap_or(&model.mesh);
        Some(crate::brushing::card::facts_of(mesh, editing))
    }
}

#[cfg(test)]
#[path = "brush_edit_scale_tests.rs"]
mod scale_tests;
#[cfg(test)]
#[path = "workbench_brush_edit_tests.rs"]
mod tests;
