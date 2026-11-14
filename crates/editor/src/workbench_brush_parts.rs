//! Changing a brush's insides (Tools 1a, B3): the gizmo on what's picked
//! (Shift + drag on a move handle extruding the picked faces, B4)
//! (its pivot the middle of the picked points; its axes the world's, or
//! the brush's own with Local), each drag drawn on the brush's own copy
//! and let go as one undo step or put back when it would break the
//! brush; clicks and boxes changing the pick; a size typed on the Brush
//! card stretching the brush about its middle; and Made as replacing the
//! edits.

use crate::brushes::BrushModel;
use crate::brushing::edit::{Part, PickHow, points_of};
use crate::brushing::reshape::{self, Drag};
use crate::commands::merge_key;
use crate::gizmo::{Change, Gizmo, GizmoTool};
use crate::transforming::{WORLD_AXES, own_axes, snapped, snapped_in_2d};
use crate::workbench::Workbench;
use area::Edit;
use area::builtin::Brush;
use glam::{Mat4, Vec3};
use reflect::{Component, EntityId, Value};
use std::sync::Arc;

impl Workbench {
    /// The gizmo on the parts picked while a brush is edited: `None`
    /// when nothing is picked or the Select tool is in use.
    pub(crate) fn part_gizmo(&self, tool: GizmoTool) -> Option<Gizmo> {
        let edit = self.viewport.brushing.edit.as_ref()?;
        let mesh = edit.mesh()?;
        let points = points_of(mesh, &edit.picked);
        let world = self.open.document.world_matrix(edit.entity);
        let origin = reshape::pivot_of(mesh, &points, world)?;
        Some(Gizmo {
            origin,
            axes: self.part_axes(world),
            tool,
        })
    }

    /// The gizmo's axes for parts: the world's, or with Local (the
    /// existing switch, L) the brush's own.
    fn part_axes(&self, world: Mat4) -> [Vec3; 3] {
        if self.viewport.snapping.object_axes {
            own_axes(world)
        } else {
            WORLD_AXES
        }
    }

    /// A gizmo drag began on the picked parts; `extrude` (Shift held on
    /// a move handle, B4) extrudes the picked faces as it moves them.
    pub(crate) fn part_drag_start(&mut self, extrude: bool) {
        let world = self
            .open
            .document
            .world_matrix(match &self.viewport.brushing.edit {
                Some(edit) => edit.entity,
                None => return,
            });
        let axes = self.part_axes(world);
        let Some(edit) = self.viewport.brushing.edit.as_mut() else {
            return;
        };
        let Some(parts) = &edit.parts else {
            return;
        };
        let points = points_of(parts.mesh(), &edit.picked);
        let Some(pivot) = reshape::pivot_of(parts.mesh(), &points, world) else {
            return;
        };
        edit.hovered = None;
        if !extrude || edit.mode == crate::brushing::edit::Mode::Points {
            edit.drag = Some(Drag::begin(parts.shared(), points, (pivot, axes), world));
            return;
        }
        if let Some(why) = crate::brushing::shaping::why_not_extrude(edit) {
            self.viewport.brushing.tell(why);
            return;
        }
        let faces = crate::brushing::shaping::picked_faces(edit);
        edit.drag = Drag::extruding(parts.mesh(), &faces, (pivot, axes), world);
    }

    /// The drag so far (`flip`: Ctrl turns snapping around for it): the
    /// brush's own copy changed and drawn in its place, nothing else.
    pub(crate) fn part_drag_change(&mut self, change: Change, flip: bool) {
        let snapping = self.viewport.snapping;
        let two_d = self.viewport.camera.kind == crate::camera::ViewKind::TwoD;
        let pixels = self.open.pixels;
        let Some(edit) = self.viewport.brushing.edit.as_mut() else {
            return;
        };
        let Some(drag) = edit.drag.as_mut() else {
            return;
        };
        let change = if two_d {
            snapped_in_2d(change, &drag.targets, &snapping, pixels, flip)
        } else {
            snapped(change, &drag.targets, &snapping, flip)
        };
        drag.set(change);
        let preview = Arc::new(BrushModel::passing(&drag.now_shared()));
        edit.preview = Some(Arc::clone(&preview));
        let entity = edit.entity;
        self.show_edited_model(entity, Some(preview));
    }

    /// The drag let go: checked, then one undo step, or put back with the
    /// reason in the status bar.
    pub(crate) fn part_drag_end(&mut self) {
        let Some(edit) = self.viewport.brushing.edit.as_mut() else {
            return;
        };
        let Some(drag) = edit.drag.take() else {
            return;
        };
        let last = edit.preview.take();
        let entity = edit.entity;
        let what = edit.picked_words();
        match drag.finish() {
            Ok(Some(mesh)) if drag.is_extruding() => {
                let shaped = crate::brushing::shaping::Shaped {
                    mesh: (*mesh).clone(),
                    around: drag.points().to_vec(),
                    label: drag.label(&what),
                    picked: edit.picked.iter().copied().collect(),
                };
                self.keep_shaped(shaped, last);
                return;
            }
            Ok(Some(mesh)) => {
                let label = drag.label(&what);
                let edits = vec![points_edit(entity, &mesh)];
                if self.command(&label, edits, None, None) {
                    self.open.document.close_step();
                    // The drag's last frame is the brush's model: made
                    // once, from the mesh in hand, never read back.
                    let kept = Arc::new(match &last {
                        Some(last) => last.kept(),
                        None => BrushModel::passing(&mesh).kept(),
                    });
                    self.open.brushes.adopt(entity, Arc::clone(&kept));
                    self.show_edited_model(entity, Some(kept));
                    return;
                }
            }
            Ok(None) => {}
            Err(broken) => self.viewport.brushing.tell(reshape::refused_words(broken)),
        }
        self.show_edited_model(entity, None);
    }

    /// The drag given up (Esc, a right-click): the brush as it was.
    pub(crate) fn part_drag_cancel(&mut self) {
        if let Some(edit) = self.viewport.brushing.edit.as_mut() {
            edit.drag = None;
            edit.preview = None;
            let entity = edit.entity;
            self.show_edited_model(entity, None);
        }
    }

    /// A click or a box found `hits`: the pick changes as `how` says.
    pub(crate) fn pick_parts(&mut self, hits: &[Part], how: PickHow) {
        if let Some(edit) = self.viewport.brushing.edit.as_mut() {
            edit.pick(hits, how);
        }
    }

    /// A size typed on the Brush card: `entity`'s brush stretched about
    /// its middle to `size`, as one undo step (a drag on the numbers
    /// merges into one), or put back with the reason in the status bar.
    pub(crate) fn resize_brush(&mut self, entity: EntityId, size: Vec3, gesture: u64) {
        let Some(model) = self.open.brushes.get(entity).cloned() else {
            return;
        };
        match reshape::stretched(&model.mesh, size) {
            Ok(Some((mesh, _))) => {
                let edits = vec![points_edit(entity, &mesh)];
                self.command(
                    "Resize brush",
                    edits,
                    Some(merge_key(("brush size", entity), gesture)),
                    None,
                );
            }
            Ok(None) => {}
            Err(broken) => self.viewport.brushing.tell(reshape::refused_words(broken)),
        }
    }

    /// As Made as gives `entity` a fresh shape (called before it does):
    /// if the brush's insides had been edited (its mesh isn't its shape's
    /// at its size), the status bar says the edits were replaced and how
    /// to get them back.
    pub(crate) fn tell_if_edits_replaced(&mut self, entity: EntityId) {
        let Some(model) = self.open.brushes.get(entity) else {
            return;
        };
        let Some(Brush { shape, .. }) = self
            .open
            .document
            .area()
            .get(entity)
            .and_then(|e| e.get::<Brush>())
        else {
            return;
        };
        let size = reshape::size_of(&model.mesh);
        let mut fresh = shape.build(&crate::brushing::settings_for(shape, size));
        if let (Some(was), Some(now)) = (model.mesh.bounds(), fresh.bounds()) {
            let points: Vec<_> = fresh.vertices().map(|(v, _)| v).collect();
            mesh::edit::move_points(&mut fresh, &points, was.min - now.min);
        }
        if !same_points(&fresh, &model.mesh) {
            self.viewport
                .brushing
                .tell("Made as replaced the edits with a fresh shape (Ctrl + Z brings them back)");
        }
    }
}

/// Whether two meshes have the same faces with their points in the same
/// places (to a hair).
fn same_points(a: &mesh::Mesh, b: &mesh::Mesh) -> bool {
    a.face_layout() == b.face_layout()
        && a.vertex_count() == b.vertex_count()
        && a.vertices()
            .zip(b.vertices())
            .all(|((va, pa), (vb, pb))| va == vb && pa.distance(pb) < 1e-4)
}

/// The edit that writes `mesh`'s points into `entity`'s brush: only the
/// points change when parts move, never the faces.
pub(crate) fn points_edit(entity: EntityId, mesh: &mesh::Mesh) -> Edit {
    Edit::SetField {
        entity,
        component: Brush::NAME.into(),
        field: "points".into(),
        value: Some(Value::List(area::builtin::points_of(mesh))),
    }
}
