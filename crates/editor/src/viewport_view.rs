//! One view of the viewport, each frame (`Viewport::show` draws each):
//! the pointer taken by the tools or the tile painter, a drop from the
//! Library, the renderer's picture, what's drawn over it, and its pills.

use super::{VIEWPORT_IMAGE, Viewport, ViewportRenderer, cursor_readout};
use crate::camera::{Camera, ViewKind};
use crate::theme::{self, Accent};
use crate::tokens::size;
use crate::transforming::Snapping;
use crate::viewport_tools::{self, ToolInput};
use egui::{Color32, Rect, Sense, Ui, vec2};
use glam::Vec3;

/// What one view's frame works from, worked out as it begins.
struct ViewFrame {
    /// The view's space.
    rect: Rect,
    /// Its size in physical pixels.
    size_px: [u32; 2],
    /// Its size in points.
    size: glam::Vec2,
    /// Pixels per point.
    ppp: f32,
    /// What its camera sees at that size.
    seen: Camera,
    /// Whether it's a 2D view.
    two_d: bool,
    /// Snapping as it applies here: a 2D view moves things across the
    /// ground only.
    snapping: Snapping,
    /// The pointer, from the view's top left.
    mouse: Option<glam::Vec2>,
}

impl Viewport {
    /// Draws the view in use into `ui`, filling it. `in_use` shows its
    /// tool bar, snapping and frame rate; `several` adds a button to
    /// show it alone.
    pub(super) fn show_view(
        &mut self,
        ui: &mut Ui,
        renderer: &mut dyn ViewportRenderer,
        accent: Accent,
        dt: f32,
        in_use: bool,
        several: bool,
    ) {
        let rect = ui.max_rect();
        self.rect = rect;
        self.tile_ms = (ui.input(|i| i.time).max(0.0) * 1000.0) as u64;
        if !self.area.tile_maps.is_empty() && self.sheets.tiles_animate() {
            // Animated tiles move on by themselves.
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(40));
        }
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        // While painting a tile map (D6), the left and right buttons
        // paint and empty: the middle button and the wheel still move.
        let painting = in_use && self.tile_overlay.paint.painting();
        // While a brush is drawn out (B2), the left button draws it, and
        // once under way the right button gives it up, never flies.
        let drawing = in_use && self.brushing.draw.is_some();
        let under_way = drawing
            && self
                .brushing
                .draw
                .as_ref()
                .is_some_and(crate::brushing::draw::Draw::is_under_way);
        self.navigate(ui, &response, dt, painting || under_way);
        let frame = self.view_frame(ui, rect);
        self.tools
            .receive(renderer.picked(VIEWPORT_IMAGE + self.active));
        if painting {
            self.tile_overlay.input = self.paint_input(ui, &response);
        } else if self.brushing.list.is_some() {
            // The list has the pointer: a click on the view only closes it.
        } else if drawing && !self.is_flying() {
            self.draw_out(ui, &response, &frame);
        } else if !self.is_flying() && self.brushing.draw.is_none() {
            self.use_tools(ui, &response, &frame);
        }
        if let Some(dropped) = response.dnd_release_payload::<crate::library::LibraryDrag>()
            && let (Some(asset), Some(at)) = (dropped.0.id, frame.mouse)
            && dropped.0.kind.placeable()
        {
            // Ctrl held while dropping places without snapping (answer 25).
            let free = ui.input(|i| i.modifiers.command);
            self.tools.drop_asset(
                asset,
                (at, free),
                &frame.seen,
                &self.area,
                (frame.size, frame.ppp),
            );
        }
        self.draw_picture(ui, renderer, &response, &frame);
        self.draw_over_view(ui, &response, &frame, accent);
        let edited = self.brushing.edit.as_ref().and_then(|edit| {
            let world = self.area.brushes.iter().find(|b| b.entity == edit.entity)?;
            Some((edit, world.transform))
        });
        if let Some(edited) = edited {
            crate::brushing::edit_draw::show(ui, rect, &frame.seen, edited);
        }
        // The pills over the view (spec 182): the view's own at the top
        // left; the frame rate, the locks, space and camera speed and the
        // tool bar on the view in use only.
        let (covered, strip) = self.view_pills(ui, rect, several, in_use, frame.two_d);
        self.covered = covered;
        if !in_use {
            return;
        }
        if let Some(edit) = &self.brushing.edit {
            let words = crate::brushing::edit_draw::words(ui, rect, edit, strip);
            self.covered.extend(words);
        }
        if let Some(draw) = &self.brushing.draw {
            let grid = self.snapping.grid_step;
            let covered = crate::brushing::ghost::show(ui, rect, &frame.seen, (draw, grid), strip);
            self.covered.extend(covered);
        }
    }

    /// Drawing a brush out takes this frame's pointer and buttons (B2): a
    /// brush drawn out is reported to the workbench; one given up, or
    /// drawn with no size, leaves nothing.
    fn draw_out(&mut self, ui: &Ui, response: &egui::Response, frame: &ViewFrame) {
        use crate::brushing::draw::{DrawInput, DrawView, Outcome};
        let on_corner = self.on_corner(ui);
        let rect = frame.rect;
        let input = ui.input(|i| DrawInput {
            mouse: i
                .pointer
                .latest_pos()
                .map(|p| glam::Vec2::new(p.x - rect.min.x, p.y - rect.min.y)),
            pressed: i.pointer.primary_pressed(),
            down: i.pointer.primary_down(),
            released: i.pointer.primary_released(),
            right_pressed: i.pointer.secondary_pressed(),
            ctrl: i.modifiers.command,
            on_view: response.hovered() && !on_corner,
        });
        let view = DrawView {
            camera: &frame.seen,
            size: frame.size,
            area: &self.area,
            grid: self.snapping.grid_step,
        };
        let Some(draw) = self.brushing.draw.as_mut() else {
            return;
        };
        match draw.update(input, &view) {
            Outcome::Going => {}
            Outcome::GaveUp => self.brushing.draw = None,
            Outcome::Made(planned) => {
                self.brushing.draw = None;
                self.tools
                    .events
                    .push(viewport_tools::ViewportEvent::BrushDrawn(planned));
            }
        }
    }

    /// What one view's frame works from: its size, what it sees, its
    /// snapping and the pointer over it.
    fn view_frame(&mut self, ui: &Ui, rect: Rect) -> ViewFrame {
        let ppp = ui.ctx().pixels_per_point();
        let size_px = [
            (rect.width() * ppp).round().max(1.0) as u32,
            (rect.height() * ppp).round().max(1.0) as u32,
        ];
        // The 2D view shows as much as its zoom says at this size: kept, so
        // the next frame's drags and zooms start from it.
        let seen = self.seen(size_px);
        self.camera.flat_height = seen.flat_height;
        let two_d = self.camera.kind == ViewKind::TwoD;
        // The 2D view moves things across the ground only.
        let mut snapping = self.snapping;
        if two_d {
            snapping.locked[2] = true;
        }
        ViewFrame {
            rect,
            size_px,
            size: glam::Vec2::new(rect.width(), rect.height()),
            ppp,
            seen,
            two_d,
            snapping,
            mouse: ui
                .input(|i| i.pointer.latest_pos())
                .map(|p| glam::Vec2::new(p.x - rect.min.x, p.y - rect.min.y)),
        }
    }

    /// Whether the pointer is over one of the view's corner controls or a
    /// menu open over the view: egui gives the viewport the hover even
    /// under a corner button (it's the only thing there that can be
    /// dragged), so a press there is left out by hand.
    fn on_corner(&self, ui: &Ui) -> bool {
        ui.input(|i| i.pointer.latest_pos()).is_some_and(|p| {
            self.covered.iter().any(|r| r.contains(p))
                // A menu open over the view (its own layer) takes
                // the press too.
                || ui.ctx().layer_id_at(p).is_some_and(|l| l != ui.layer_id())
        })
    }

    /// The buttons as a tile map's painter takes them (D6).
    fn paint_input(&self, ui: &Ui, response: &egui::Response) -> crate::tile_paint::PaintInput {
        let on_corner = self.on_corner(ui);
        ui.input(|i| {
            use crate::tile_paint::Button;
            crate::tile_paint::PaintInput {
                on_view: response.hovered() && !on_corner,
                left: Button {
                    pressed: i.pointer.primary_pressed(),
                    down: i.pointer.primary_down(),
                },
                right: Button {
                    pressed: i.pointer.secondary_pressed(),
                    down: i.pointer.secondary_down(),
                },
                ctrl: i.modifiers.command,
                alt: i.modifiers.alt,
            }
        })
    }

    /// The tools take this frame's pointer and buttons.
    fn use_tools(&mut self, ui: &Ui, response: &egui::Response, frame: &ViewFrame) {
        let on_corner = self.on_corner(ui);
        let rect = frame.rect;
        let input = ui.input(|i| ToolInput {
            mouse: i
                .pointer
                .latest_pos()
                .map(|p| glam::Vec2::new(p.x - rect.min.x, p.y - rect.min.y)),
            pressed: i.pointer.primary_pressed(),
            down: i.pointer.primary_down(),
            released: i.pointer.primary_released(),
            right_pressed: i.pointer.secondary_pressed(),
            ctrl: i.modifiers.command,
            shift: i.modifiers.shift,
            on_viewport: response.hovered() && !on_corner,
        });
        self.tools.interact(
            input,
            &frame.seen,
            &self.area,
            &frame.snapping,
            frame.size,
            frame.ppp,
        );
        if self.brushing.edit.is_some() {
            self.find_parts(input, frame);
        }
    }

    /// While a brush's insides are edited (Tools 1a, B3): the clicks and
    /// boxes of this frame answered with its parts this view sees, and
    /// the part under the pointer found (only the brush being edited is
    /// tried, its box first, then its faces).
    fn find_parts(&mut self, input: ToolInput, frame: &ViewFrame) {
        let asks = std::mem::take(&mut self.tools.part_asks);
        let Some(edit) = self.brushing.edit.as_mut() else {
            return;
        };
        let Some(world) = self
            .area
            .brushes
            .iter()
            .find(|b| b.entity == edit.entity)
            .map(|b| b.transform)
        else {
            return;
        };
        let Some(parts) = edit.parts.clone() else {
            return;
        };
        let project = frame.seen.projector(frame.size);
        let mode = edit.mode;
        let under = |at: glam::Vec2| {
            let ray = frame.seen.ray_through(at, frame.size);
            parts.under(mode, (world, ray), &project, at)
        };
        for ask in asks {
            let (hits, how) = match ask {
                viewport_tools::PartAsk::At(at, how) => (under(at).into_iter().collect(), how),
                viewport_tools::PartAsk::In(min, max, how) => {
                    (parts.in_box(mode, world, &project, (min, max)), how)
                }
            };
            self.tools
                .events
                .push(viewport_tools::ViewportEvent::Parts { hits, how });
        }
        let free = self.tools.pressing == viewport_tools::Pressing::Nothing
            && self.tools.hovered.is_none()
            && edit.drag.is_none();
        if input.on_viewport
            && free
            && let Some(mouse) = input.mouse
        {
            edit.hovered = under(mouse);
        }
    }

    /// The renderer's picture of the view, or why there's none.
    fn draw_picture(
        &mut self,
        ui: &Ui,
        renderer: &mut dyn ViewportRenderer,
        response: &egui::Response,
        frame: &ViewFrame,
    ) {
        let rect = frame.rect;
        let mut view = self.view(frame.size_px, frame.ppp);
        view.pick = self.tools.send_with(&self.area.pickable);
        let painter = ui.painter_at(rect);
        match renderer.render(&view) {
            Ok(texture) => {
                painter.image(
                    texture,
                    rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            Err(reason) => {
                let message = format!("No 3D view: {reason}");
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &message)
                });
                painter.rect_filled(rect, 0.0, theme::backdrop::horizon());
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    message,
                    egui::FontId::proportional(13.0),
                    theme::text_weak(),
                );
            }
        }
    }

    /// What's drawn over the picture: game screens in 2D, cameras, the
    /// selected tile map's overlay, the tools, where the pointer is in
    /// 2D, what a drag from the Library would place, and a gizmo's
    /// change as it's dragged.
    fn draw_over_view(
        &mut self,
        ui: &mut Ui,
        response: &egui::Response,
        frame: &ViewFrame,
        accent: Accent,
    ) {
        let (rect, seen, mouse) = (frame.rect, &frame.seen, frame.mouse);
        let painter = ui.painter_at(rect);
        if frame.two_d {
            draw_screens(ui, rect, &self.area.screens, seen, self.pixels);
        }
        viewport_tools::draw_cameras(ui, rect, &self.area.cameras, seen);
        crate::tile_view::draw_overlay(
            ui,
            rect,
            seen,
            &mut self.tile_overlay,
            self.pixels.tile_size(),
            mouse.filter(|_| response.hovered()),
            accent,
        );
        viewport_tools::draw(ui, rect, &self.tools, seen, frame.snapping.locked, accent);
        // In 2D, where the pointer is, in the game's pixels, and the map's
        // cell under it (decision 0085), beside the pointer.
        if frame.two_d
            && response.hovered()
            && let Some(at) = mouse
            && let Some(ground) = seen
                .ray_through(at, glam::Vec2::new(rect.width(), rect.height()))
                .hit_ground(0.0)
            && let Some(pointer) = ui.input(|i| i.pointer.latest_pos())
        {
            let words = cursor_readout(ground, self.pixels, self.tile_overlay.hovered);
            let galley =
                painter.layout_no_wrap(words, egui::FontId::monospace(11.5), theme::text_strong());
            let corner = pointer + egui::vec2(16.0, 14.0);
            let back =
                egui::Rect::from_min_size(corner, galley.size()).expand2(egui::vec2(6.0, 3.0));
            painter.rect_filled(back, 4.0, theme::header().gamma_multiply(0.9));
            painter.galley(corner, galley, theme::text_strong());
        }
        if let Some(dragged) = response.dnd_hover_payload::<crate::library::LibraryDrag>()
            && dragged.0.kind.placeable()
            && let (Some(pointer), Some(at)) = (ui.input(|i| i.pointer.latest_pos()), mouse)
        {
            self.placing_overlay(ui, rect, (pointer, at), seen, &dragged.0.name);
        }
        if let viewport_tools::Pressing::Gizmo {
            last: Some(change), ..
        } = &self.tools.pressing
            && let Some(mouse) = ui.input(|i| i.pointer.latest_pos())
        {
            painter.text(
                mouse + egui::vec2(18.0, 14.0),
                egui::Align2::LEFT_TOP,
                viewport_tools::readout(*change),
                egui::FontId::monospace(12.0),
                theme::text_strong(),
            );
        }
    }

    /// The pills over the view (spec 182): the view's own at the top left;
    /// the frame rate, the locks, space and camera speed and the tool bar
    /// (decision 0114) on the view in use only, and the tile painter's
    /// keys over its foot. Returns where they cover the view, and where
    /// the foot of a strip of keys over the view's foot goes: over the
    /// tool bar, or else over the bottom pills.
    fn view_pills(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        several: bool,
        in_use: bool,
        two_d: bool,
    ) -> (Vec<Rect>, f32) {
        let mut covered = vec![self.top_left_pills(ui, rect, several)];
        if self.overlays.stats && in_use {
            covered.push(self.stats_pill(ui, rect));
        }
        let mut corner = None;
        if self.overlays.snapping && in_use {
            let pills = if two_d {
                self.pixel_pills(ui, rect)
            } else {
                self.space_pills(ui, rect)
            };
            covered.push(pills);
            corner = Some(pills);
        }
        let mut bar = None;
        if self.overlays.tools && in_use && !self.placing {
            let at = self.tool_bar(ui, rect, corner);
            covered.push(at);
            bar = Some(at);
        }
        let strip = bar.map_or(
            rect.bottom() - size::PILL_H - 2.0 * size::PILL_MARGIN,
            |bar| bar.top() - size::TOOL_BAR_STRIP_GAP,
        );
        if in_use && let Some(keys) = self.tile_overlay.paint.hint_keys() {
            // The painter's keys, over the view's foot (round 3), or over
            // the tool bar.
            let foot = bar.map_or(rect.bottom(), |_| strip + size::PILL_MARGIN);
            covered.push(crate::widgets::pill::key_hints(
                ui,
                rect.with_max_y(foot),
                keys,
            ));
        }
        (covered, strip)
    }
}

/// The 2D view's frames: what each 2D camera shows of the world at the
/// project's base resolution, dashed, the game's own labelled "Game
/// screen".
fn draw_screens(
    ui: &Ui,
    rect: Rect,
    screens: &[(Vec3, bool)],
    camera: &Camera,
    pixels: area::pixels::PixelSettings,
) {
    let painter = ui.painter_at(rect);
    let size = glam::Vec2::new(rect.width(), rect.height());
    let half =
        glam::Vec2::new(pixels.base[0] as f32, pixels.base[1] as f32) * pixels.pixel_size() * 0.5;
    for &(at, game) in screens {
        let corner = |dx: f32, dy: f32| {
            camera
                .project(Vec3::new(at.x + dx, at.y + dy, 0.0), size)
                .map(|p| rect.min + vec2(p.x, p.y))
        };
        let (Some(top_left), Some(bottom_right)) =
            (corner(-half.x, half.y), corner(half.x, -half.y))
        else {
            continue;
        };
        let frame = Rect::from_two_pos(top_left, bottom_right);
        let colour = if game {
            theme::text_strong()
        } else {
            theme::text_weak()
        };
        let stroke = egui::Stroke::new(1.5, colour);
        for (a, b) in [
            (frame.left_top(), frame.right_top()),
            (frame.right_top(), frame.right_bottom()),
            (frame.right_bottom(), frame.left_bottom()),
            (frame.left_bottom(), frame.left_top()),
        ] {
            painter.add(egui::Shape::dashed_line(&[a, b], stroke, 8.0, 5.0));
        }
        let [w, h] = pixels.base;
        let label = if game {
            format!("Game screen, {w} × {h}")
        } else {
            format!("2D camera, {w} × {h}")
        };
        painter.text(
            frame.left_top() - vec2(0.0, 4.0),
            egui::Align2::LEFT_BOTTOM,
            label,
            egui::FontId::proportional(12.0),
            colour,
        );
    }
}
