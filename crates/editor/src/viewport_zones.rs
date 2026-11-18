//! Trigger zones in one view, each frame (stage 1, triggers): the left
//! button is the zone tools' while a freeform zone's points are drawn or
//! edited, or while a picked zone's handle is pointed at or dragged;
//! otherwise it's the usual tools'. What they did goes to the workbench as
//! `ZoneEvent`s.

use super::Viewport;
use super::view::ViewFrame;
use crate::zones::points::PointsInput;
use crate::zones::sizing::{SizeDrag, handle_at};
use crate::zones::{ViewZone, ZoneEvent};
use egui::Ui;
use glam::Vec2;

impl Viewport {
    /// Gives the zone tools this frame's pointer, if they take it.
    /// Returns whether they did (the usual tools then don't see it).
    pub(super) fn zone_pointer(
        &mut self,
        ui: &Ui,
        response: &egui::Response,
        frame: &ViewFrame,
    ) -> bool {
        let on_view = response.hovered() && !self.on_corner(ui);
        let rect = frame.rect;
        let (mouse, pressed, down, ctrl) = ui.input(|i| {
            (
                i.pointer
                    .latest_pos()
                    .map(|p| Vec2::new(p.x - rect.min.x, p.y - rect.min.y)),
                i.pointer.primary_pressed(),
                i.pointer.primary_down(),
                i.modifiers.command,
            )
        });
        if self.zones.points.is_some() {
            self.zone_points(
                PointsInput {
                    mouse: mouse.filter(|_| on_view),
                    pressed: pressed && on_view,
                    down,
                    ctrl,
                },
                frame,
            );
            return true;
        }
        if let Some(drag) = &self.zones.drag {
            if down {
                if let Some(mouse) = mouse {
                    let ray = frame.seen.ray_through(mouse, frame.size);
                    let step = self.size_step(frame.two_d);
                    if let Some((zone, place)) = drag.follow(&ray, step) {
                        self.zones.events.push(ZoneEvent::Sized {
                            entity: drag.entity,
                            zone,
                            place,
                        });
                    }
                }
            } else {
                self.zones.drag = None;
            }
            return true;
        }
        self.zones.hovered = None;
        let Some(mouse) = mouse.filter(|_| on_view) else {
            return false;
        };
        let Some(zone) = self.alone_zone() else {
            return false;
        };
        let Some(number) = handle_at(zone, frame.two_d, &frame.seen, frame.size, mouse) else {
            return false;
        };
        let drag = pressed
            .then(|| SizeDrag::new(zone, number, frame.two_d))
            .flatten();
        self.zones.hovered = Some((zone.entity, number));
        if drag.is_some() {
            self.zones.drag = drag;
        }
        true
    }

    /// What a zone's size snaps to: in a 2D view whole pixels or tiles, as
    /// moves do; in 3D the grid's step while it snaps.
    fn size_step(&self, flat: bool) -> Option<f32> {
        use crate::transforming::PixelSnap;
        if flat {
            match self.snapping.pixel_snap {
                PixelSnap::Off => None,
                PixelSnap::Pixels => Some(self.pixels.pixel_size()),
                PixelSnap::Tiles => Some(self.pixels.tile_size()[0]),
            }
        } else {
            self.snapping.grid.then_some(self.snapping.grid_step)
        }
    }

    /// The zone picked alone, as this view shows it.
    fn alone_zone(&self) -> Option<&ViewZone> {
        let alone = self.zones.alone?;
        self.level.zones.iter().find(|z| z.entity == alone)
    }

    /// Drawing or editing a freeform zone's points takes the pointer; a
    /// drawing closed ends it.
    fn zone_points(&mut self, input: PointsInput, frame: &ViewFrame) {
        // The zone may have moved since the points began (an undo).
        let world = self
            .zones
            .points
            .as_ref()
            .and_then(|edit| self.level.zones.iter().find(|z| z.entity == edit.entity))
            .map(|z| z.transform);
        let Some(edit) = self.zones.points.as_mut() else {
            return;
        };
        if let Some(world) = world {
            edit.world = world;
        }
        let closed = edit.interact(input, &frame.seen, frame.size, &mut self.zones.events);
        if closed {
            self.zones.points = None;
        }
    }
}
