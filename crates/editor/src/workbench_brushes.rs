//! The workbench's side of making shape brushes (Tools 1a, B2): the keys
//! of the brush list (B, Space, Esc), the list drawn beside the pointer,
//! a brush drawn out added as one undo step, named after its shape and
//! picked, and Made as making a brush's mesh again at its size.

use crate::brushing::draw::{Draw, Planned};
use crate::brushing::list::{Answer, List};
use crate::panel::Panel;
use crate::workbench::Workbench;
use area::builtin::{Brush, Transform};
use area::{Edit, EntityData};
use egui::{Key, Modifiers};
use mesh::Shape;
use reflect::{Component, EntityId, Value};
use std::sync::Arc;

impl Workbench {
    /// B, with the pointer over the 3D or 2D view, opens the brush list
    /// (unless a tile map is picked, when B is its Brush, decision 0108);
    /// while the list is open, B moves down it, Space takes the shape
    /// highlighted (before the search's Space) and Esc closes it.
    pub(crate) fn brush_keys(&mut self, ui: &egui::Ui) {
        if !self.keys.takes_shortcuts() || self.viewport.is_flying() {
            return;
        }
        if let Some(list) = &mut self.viewport.brushing.list {
            let (next, take, close) = ui.input_mut(|i| {
                (
                    i.consume_key(Modifiers::NONE, Key::B),
                    i.consume_key(Modifiers::NONE, Key::Space),
                    i.consume_key(Modifiers::NONE, Key::Escape),
                )
            });
            if next {
                list.next();
            }
            if take {
                let shape = list.chosen();
                self.take_brush(shape);
            } else if close {
                self.viewport.brushing.list = None;
            }
            return;
        }
        let over_a_view = self.pointer_over == Some(Panel::Viewport);
        let drawing = self
            .viewport
            .brushing
            .draw
            .as_ref()
            .is_some_and(Draw::is_under_way);
        if !self.keys.is_area()
            || !over_a_view
            || drawing
            || self.viewport.tile_overlay.paint.allowed
        {
            return;
        }
        if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::B)) {
            let at = ui.input(|i| i.pointer.latest_pos());
            self.open_brush_list(at);
        }
    }

    /// Opens the brush list beside `pointer` (or the viewport's middle),
    /// its first shape highlighted. Already open, it stays as it is.
    pub(crate) fn open_brush_list(&mut self, pointer: Option<egui::Pos2>) {
        if self.viewport.brushing.list.is_some() {
            return;
        }
        let at = pointer.unwrap_or_else(|| self.viewport.rect.center());
        self.viewport.brushing.list = Some(List::beside(at));
    }

    /// Takes `shape` from the list: the list closes and the shape waits to
    /// be drawn out.
    pub(crate) fn take_brush(&mut self, shape: Shape) {
        self.viewport.brushing.list = None;
        self.viewport.brushing.draw = Some(Draw::new(shape));
    }

    /// Draws the brush list while it is open, and does what was done with
    /// it: a row clicked takes its shape, a click elsewhere closes it. A
    /// page standing in place of the editor (File), or a tab without the
    /// 3D or 2D view, closes it and gives up a brush being drawn.
    pub(crate) fn brush_list_frame(&mut self, ctx: &egui::Context) {
        if self.shell.covers_center() || !self.in_front.contains(&Panel::Viewport) {
            self.viewport.brushing = crate::brushing::Brushing::default();
        }
        let Some(list) = self.viewport.brushing.list.as_mut() else {
            return;
        };
        match crate::brushing::list::show(ctx, list) {
            Some(Answer::Take(shape)) => self.take_brush(shape),
            Some(Answer::Close) => self.viewport.brushing.list = None,
            None => {}
        }
    }

    /// Adds the brush drawn out as `planned`: an entity named after its
    /// shape (Box_01, Ramp_01…), placed as the Create buttons place what
    /// they make, picked, as one undo step.
    pub(crate) fn add_drawn_brush(&mut self, planned: Planned) -> EntityId {
        let name = self.next_brush_name(planned.shape);
        let brush = Brush {
            shape: planned.shape,
            solid: true,
            mesh: Arc::new(planned.shape.build(&planned.settings)),
            damaged: false,
        };
        let id = self.spawn_selected(
            EntityData::new(&name)
                .with(&Transform {
                    position: planned.position,
                    rotation: planned.rotation,
                    ..Transform::default()
                })
                .with(&brush),
        );
        self.open.area_panel.reveal = Some(id);
        id
    }

    /// The next name for a brush made as `shape`: its name and a number
    /// one past the highest any entity has with that name ("Box_03" after
    /// "Box_02"), from "_01".
    fn next_brush_name(&self, shape: Shape) -> String {
        let start = format!("{}_", shape.name());
        let highest = self
            .open
            .document
            .area()
            .iter()
            .filter_map(|(_, e)| e.name.strip_prefix(&start)?.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        format!("{start}{:02}", highest.saturating_add(1))
    }

    /// The edits that set `entity`'s brush's Made as to `value` (a
    /// shape's name): the shape, and its mesh made again as that shape at
    /// the brush's present size, filling the same box. None when `value`
    /// names no shape or the entity has no brush: then the field is set
    /// alone, as any other.
    pub(crate) fn made_as_edits(&self, entity: EntityId, value: &Value) -> Option<Vec<Edit>> {
        let Value::Text(name) = value else {
            return None;
        };
        let shape = Shape::from_name(name)?;
        let brush = self.open.document.area().get(entity)?.get::<Brush>()?;
        let was = brush.mesh.bounds();
        let size = was.map_or(shape.standard().size, |b| b.size());
        let mut mesh = shape.build(&crate::brushing::settings_for(shape, size));
        if let (Some(was), Some(now)) = (was, mesh.bounds()) {
            let points: Vec<_> = mesh.vertices().map(|(v, _)| v).collect();
            mesh::edit::move_points(&mut mesh, &points, was.min - now.min);
        }
        let data = Brush {
            shape,
            mesh: Arc::new(mesh),
            damaged: false,
            ..brush
        }
        .to_data();
        let edits = ["shape", "points", "faces", "materials"]
            .into_iter()
            .map(|field| Edit::SetField {
                entity,
                component: Brush::NAME.into(),
                field: field.into(),
                value: data.get(field).cloned(),
            })
            .collect();
        Some(edits)
    }
}

#[cfg(test)]
#[path = "workbench_brushes_tests.rs"]
mod tests;
