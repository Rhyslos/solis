//! Shaping a brush while its insides are edited (Tools 1a, B4): Extrude
//! (E) and Inset with the mouse, Split and Delete at once, each kept as
//! one undo step only when the brush is still whole
//! ([`mesh::check_whole`]), or refused with why in the status bar.
//!
//! What each does with the pick is `brushing::shaping`; an extrude or
//! inset following the mouse is `brushing::pull`.

use crate::brushes::BrushModel;
use crate::brushing::edit::Part;
use crate::brushing::pull::{PullKind, PullOutcome};
use crate::brushing::reshape;
use crate::brushing::shaping::{self, Shaped};
use crate::shell::ShellCommand;
use crate::workbench::Workbench;
use area::Edit;
use area::builtin::Brush;
use mesh::{FaceId, Mesh, VertexId};
use reflect::Component;
use std::collections::HashMap;
use std::sync::Arc;

/// Why a shaping command can't be done while no brush is edited.
const NOT_EDITING: &str = "Pick one brush and press Tab first";

impl Workbench {
    /// Why Extrude, Inset, Split or Delete (`command`) can't be done now,
    /// or `None` when it can: the ribbon greys its button with this.
    pub(crate) fn why_not_shape(&self, command: ShellCommand) -> Option<String> {
        let Some(edit) = &self.viewport.brushing.edit else {
            return Some(NOT_EDITING.to_owned());
        };
        let why = match command {
            ShellCommand::BrushExtrude => shaping::why_not_extrude(edit),
            ShellCommand::BrushInset => shaping::why_not_inset(edit),
            ShellCommand::BrushSplit => shaping::why_not_split(edit),
            _ => shaping::why_not_delete(edit),
        };
        why.map(str::to_owned)
    }

    /// Does Extrude, Inset, Split or Delete (`command`) with what's
    /// picked; when it can't, the status bar says why.
    pub(crate) fn shape_brush(&mut self, command: ShellCommand) {
        let Some(edit) = &self.viewport.brushing.edit else {
            return;
        };
        // Its own button while it's under way: nothing more to do.
        if edit.pull.is_some() {
            return;
        }
        let world = self.open.document.world_matrix(edit.entity);
        let result = match command {
            ShellCommand::BrushExtrude => shaping::extrude(edit, world).map(Ok),
            ShellCommand::BrushInset => shaping::inset(edit, world).map(Ok),
            ShellCommand::BrushSplit => shaping::split(edit).map(Err),
            _ => shaping::delete(edit).map(Err),
        };
        match result {
            Ok(Ok(pull)) => {
                if let Some(edit) = self.viewport.brushing.edit.as_mut() {
                    edit.hovered = None;
                    edit.pull = Some(pull);
                }
            }
            Ok(Err(shaped)) => {
                self.keep_shaped(shaped, None);
            }
            Err(why) => self.viewport.brushing.tell(why),
        }
    }

    /// What came of an extrude's or inset's frame in the view: the brush
    /// drawn anew as it would be (only its own mesh), kept, or given up.
    pub(crate) fn pulled(&mut self, outcome: PullOutcome) {
        match outcome {
            PullOutcome::Going(false) => {}
            PullOutcome::Going(true) => {
                let Some(edit) = self.viewport.brushing.edit.as_mut() else {
                    return;
                };
                let Some(pull) = &edit.pull else {
                    return;
                };
                let preview = Arc::new(BrushModel::passing(&pull.now_shared()));
                edit.preview = Some(Arc::clone(&preview));
                let entity = edit.entity;
                self.show_edited_model(entity, Some(preview));
            }
            PullOutcome::Keep => self.keep_pull(),
            PullOutcome::GiveUp => self.give_up_pull(),
        }
    }

    /// Whether an extrude or inset is under way.
    pub(crate) fn is_pulling(&self) -> bool {
        self.viewport
            .brushing
            .edit
            .as_ref()
            .is_some_and(|e| e.pull.is_some())
    }

    /// The extrude or inset given up (Esc, a right-click): the brush as
    /// it was, nothing kept.
    pub(crate) fn give_up_pull(&mut self) {
        if let Some(edit) = self.viewport.brushing.edit.as_mut() {
            edit.pull = None;
            edit.preview = None;
            let entity = edit.entity;
            self.show_edited_model(entity, None);
        }
    }

    /// The extrude or inset clicked: kept as one undo step if the brush
    /// is whole, else put back with why. Nothing pulled out keeps
    /// nothing.
    fn keep_pull(&mut self) {
        let Some(edit) = self.viewport.brushing.edit.as_mut() else {
            return;
        };
        let Some(pull) = edit.pull.take() else {
            return;
        };
        let preview = edit.preview.take();
        if pull.metres == 0.0 {
            let entity = edit.entity;
            self.show_edited_model(entity, None);
            let nothing = match pull.kind {
                PullKind::Inset => "Nothing inset: move the mouse before clicking",
                _ => "Nothing extruded: move the mouse before clicking",
            };
            self.viewport.brushing.tell(nothing);
            return;
        }
        let picked = edit.picked.iter().copied().collect();
        let shaped = Shaped {
            mesh: pull.now().clone(),
            around: pull.moving_points(),
            label: pull.label(),
            picked,
        };
        self.keep_shaped(shaped, preview);
    }

    /// Keeps `shaped` as the edited brush's mesh in one undo step, if it
    /// is whole: its three lists written, the model taken from the mesh
    /// in hand (`made`, a pull's last frame, when there is one), the pick
    /// following. Otherwise the brush stays as it was and the status bar
    /// says why. Whether it was kept.
    pub(crate) fn keep_shaped(&mut self, shaped: Shaped, made: Option<Arc<BrushModel>>) -> bool {
        let Some(entity) = self.viewport.brushing.edit.as_ref().map(|e| e.entity) else {
            return false;
        };
        if let Err(broken) = mesh::check_whole(&shaped.mesh, &shaped.around) {
            self.viewport.brushing.tell(reshape::refused_words(broken));
            self.show_edited_model(entity, None);
            return false;
        }
        let written = Arc::new(shaped.mesh);
        let data = Brush {
            mesh: Arc::clone(&written),
            ..Brush::default()
        }
        .to_data();
        let edits: Vec<Edit> = ["points", "faces", "materials"]
            .into_iter()
            .map(|field| Edit::SetField {
                entity,
                component: Brush::NAME.into(),
                field: field.into(),
                value: data.get(field).cloned(),
            })
            .collect();
        // The ids any later read of the lists gives: the same when no
        // point or face left a gap.
        let read = if has_gaps(&written) {
            let read = Brush::from_data(&data);
            if read.damaged {
                return false;
            }
            read.mesh
        } else {
            Arc::clone(&written)
        };
        let renumbered = Renumbered::of(&written, &read);
        if !self.command(&shaped.label, edits, None, None) {
            self.show_edited_model(entity, None);
            return false;
        }
        self.open.document.close_step();
        let model = match made.filter(|_| renumbered.is_none()) {
            Some(model) => Arc::new(model.kept()),
            None => Arc::new(BrushModel::passing(&read).kept()),
        };
        self.open.brushes.adopt(entity, Arc::clone(&model));
        self.show_edited_model(entity, Some(model));
        if let Some(edit) = self.viewport.brushing.edit.as_mut() {
            edit.picked = shaped
                .picked
                .into_iter()
                .filter_map(|p| renumbered.as_ref().map_or(Some(p), |r| r.part(p)))
                .filter(|p| p.is_in(&read))
                .collect();
            edit.hovered = None;
        }
        true
    }
}

/// Whether `mesh` has gaps left by deleted points or faces, which
/// writing its lists closes up.
fn has_gaps(mesh: &Mesh) -> bool {
    mesh.point_slot_count() != mesh.vertex_count()
        || mesh.face_layout().0.len().saturating_sub(1) != mesh.face_count()
}

/// How ids changed when a mesh with gaps was written and read back.
struct Renumbered {
    points: HashMap<VertexId, VertexId>,
    faces: HashMap<FaceId, FaceId>,
}

impl Renumbered {
    /// The renumbering from `written` to `read`, or `None` if they are
    /// the same mesh.
    fn of(written: &Arc<Mesh>, read: &Arc<Mesh>) -> Option<Self> {
        if Arc::ptr_eq(written, read) {
            return None;
        }
        Some(Self {
            points: written
                .vertices()
                .zip(read.vertices())
                .map(|((a, _), (b, _))| (a, b))
                .collect(),
            faces: written
                .faces()
                .zip(read.faces())
                .map(|((a, _), (b, _))| (a, b))
                .collect(),
        })
    }

    fn part(&self, part: Part) -> Option<Part> {
        Some(match part {
            Part::Point(v) => Part::Point(*self.points.get(&v)?),
            Part::Edge(e) => {
                let (a, b) = e.ends();
                Part::Edge(mesh::EdgeKey::new(
                    *self.points.get(&a)?,
                    *self.points.get(&b)?,
                ))
            }
            Part::Face(f) => Part::Face(*self.faces.get(&f)?),
        })
    }
}

#[cfg(test)]
#[path = "workbench_brush_shaping_tests.rs"]
mod tests;
