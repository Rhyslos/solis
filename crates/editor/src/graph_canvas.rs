//! The shared graph editor (decision 0050; controls from decision 0057):
//! boxes joined by arrows on a canvas that pans and zooms. The animator is
//! its first user (Phase 9, D3b); materials, effects and later graphs
//! draw on the same piece, so every graph in Solis works the same way.
//!
//! - Left-click picks a box or an arrow (Ctrl or Shift adds to what's
//!   picked); left-drag on a box moves the picked boxes; left-drag on
//!   empty space draws a picking box.
//! - Drag from a box's edge to another box joins them with an arrow.
//! - Right-click opens the caller's menu for empty space, a box or an
//!   arrow.
//! - Middle-drag pans, the wheel zooms around the pointer, F frames
//!   everything, Delete removes what's picked, double-click edits a box.
//!
//! The canvas only draws and reports what was done ([`GraphEvent`]); the
//! caller changes its own document, so every change can be undone.

use crate::theme;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2, pos2, vec2};

/// The nearest and farthest the canvas zooms.
pub(crate) const ZOOM_RANGE: (f32, f32) = (0.25, 2.5);
/// How close to a box's edge a drag joins instead of moving, in points.
const EDGE: f32 = 9.0;
/// How far apart two arrows between the same boxes (one each way) run.
const SIDE_BY_SIDE: f32 = 7.0;
/// The glow of what's live (the game's state).
pub(crate) const LIVE: Color32 = Color32::from_rgb(0x2f, 0x8f, 0xd0);
/// A box's corner radius.
const ROUND: f32 = 8.0;

/// Where the canvas looks: world (graph) places to the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GraphView {
    /// Where the graph's (0, 0) is, from the canvas's top left, in points.
    pub(crate) offset: Vec2,
    /// Points per graph unit.
    pub(crate) zoom: f32,
}

impl Default for GraphView {
    fn default() -> Self {
        Self {
            offset: vec2(40.0, 40.0),
            zoom: 1.0,
        }
    }
}

impl GraphView {
    fn to_screen(self, origin: Pos2, p: Pos2) -> Pos2 {
        origin + self.offset + p.to_vec2() * self.zoom
    }

    fn to_world(self, origin: Pos2, p: Pos2) -> Pos2 {
        ((p - origin - self.offset) / self.zoom).to_pos2()
    }

    /// Zooms by `factor` keeping `at` (on screen) where it is.
    fn zoom_at(&mut self, origin: Pos2, at: Pos2, factor: f32) {
        let before = self.to_world(origin, at);
        self.zoom = (self.zoom * factor).clamp(ZOOM_RANGE.0, ZOOM_RANGE.1);
        let after = self.to_screen(origin, before);
        self.offset += at - after;
    }

    /// Fits `bounds` (graph units) into a canvas of `size`.
    pub(crate) fn frame(&mut self, bounds: Rect, size: Vec2) {
        if !bounds.is_finite() || bounds.width() <= 0.0 {
            *self = Self::default();
            return;
        }
        let margin = 40.0;
        let fit = ((size - vec2(margin, margin) * 2.0) / bounds.size()).min_elem();
        self.zoom = fit.clamp(ZOOM_RANGE.0, 1.0);
        self.offset = size / 2.0 - bounds.center().to_vec2() * self.zoom;
    }
}

/// A box to draw.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GraphBox {
    /// Its ID, as the caller knows it.
    pub(crate) id: u64,
    /// Its top left, in graph units.
    pub(crate) place: Pos2,
    /// Its size, in graph units.
    pub(crate) size: Vec2,
    /// Its name.
    pub(crate) title: String,
    /// A quieter line under the name.
    pub(crate) line: String,
    /// A small coloured label at the bottom ("4 directions").
    pub(crate) chip: Option<String>,
    /// Whether it's picked.
    pub(crate) picked: bool,
    /// Whether it's live (glows blue).
    pub(crate) live: bool,
    /// Whether it's where the graph starts (marked "Start").
    pub(crate) start: bool,
    /// Whether it can be moved, joined from and removed (Any state can be
    /// moved and joined from, but not removed; that's the caller's).
    pub(crate) movable: bool,
    /// Whether arrows can end at it.
    pub(crate) takes_arrows: bool,
}

/// An arrow to draw, from one box to another.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GraphWire {
    /// Its ID, as the caller knows it.
    pub(crate) id: u64,
    /// The box it leaves.
    pub(crate) from: u64,
    /// The box it goes to.
    pub(crate) to: u64,
    /// Its label (its conditions).
    pub(crate) label: String,
    /// Whether it's picked.
    pub(crate) picked: bool,
}

/// What was picked or right-clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GraphTarget {
    /// A box.
    Box(u64),
    /// An arrow.
    Wire(u64),
}

/// What happened on the canvas this frame.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GraphEvent {
    /// Picked this (with `add`: added to, or taken from, what's picked).
    Pick(GraphTarget, bool),
    /// Picked these boxes with a picking box (with `add`: added).
    PickBoxes(Vec<u64>, bool),
    /// Clicked empty space: nothing picked.
    Clear,
    /// These boxes were dragged this far (graph units), on letting go.
    Moved(Vec<u64>, Vec2),
    /// An arrow was drawn from one box to another.
    Join(u64, u64),
    /// Delete was pressed over the canvas.
    Delete,
    /// A box was double-clicked.
    Edit(u64),
}

/// What's being dragged.
#[derive(Debug, Clone, Default, PartialEq)]
enum Drag {
    #[default]
    None,
    /// Picked boxes, this far so far (screen points).
    Boxes(Vec<u64>, Vec2),
    /// An arrow from a box to the pointer.
    Joining(u64),
    /// A picking box from here (screen).
    Picking(Pos2, bool),
    /// An arrow from a box, made from its menu: the next click on a box
    /// ends it there (anywhere else, or Escape, gives up).
    Aiming(u64),
}

/// The canvas's own state between frames: where it looks, and what's
/// being dragged.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct GraphCanvas {
    /// Where it looks.
    pub(crate) view: GraphView,
    drag: Drag,
    /// The last pointer place over the canvas, in graph units (where a
    /// right-clicked "Add" puts a new box).
    pub(crate) pointer: Pos2,
    /// Framed once, the first time it's shown.
    framed: bool,
}

impl GraphCanvas {
    /// Starts an arrow from box `from` that the next click ends (its menu's
    /// "Make transition").
    pub(crate) fn aim_from(&mut self, from: u64) {
        self.drag = Drag::Aiming(from);
    }
}

/// The menu shown on a right-click, given what was clicked (`None`: empty
/// space, at [`GraphCanvas::pointer`]).
pub(crate) type GraphMenu<'a> = dyn FnMut(&mut Ui, Option<GraphTarget>) + 'a;

/// Where an arrow between two boxes starts and ends: from edge to edge
/// along the line between their middles, moved aside when there's one
/// going the other way.
fn wire_ends(from: Rect, to: Rect, aside: bool) -> (Pos2, Pos2) {
    let (a, b) = (from.center(), to.center());
    let dir = (b - a).normalized();
    let shift = if aside {
        vec2(-dir.y, dir.x) * SIDE_BY_SIDE
    } else {
        Vec2::ZERO
    };
    let edge = |r: Rect, toward: Vec2| -> Pos2 {
        // Where the ray from the middle leaves the box.
        let half = r.size() / 2.0;
        let t = (half.x / toward.x.abs().max(1e-6)).min(half.y / toward.y.abs().max(1e-6));
        r.center() + toward * t
    };
    if !dir.is_finite() || dir == Vec2::ZERO {
        return (a, b);
    }
    (edge(from, dir) + shift, edge(to, -dir) + shift)
}

/// How far `p` is from the segment `a`–`b`.
fn distance_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// The arrow's head at `tip`, pointing along `dir`.
fn arrow_head(painter: &egui::Painter, tip: Pos2, dir: Vec2, color: Color32) {
    let back = -dir * 9.0;
    let side = vec2(-dir.y, dir.x) * 5.0;
    painter.add(egui::Shape::convex_polygon(
        vec![tip, tip + back + side, tip + back - side],
        color,
        Stroke::NONE,
    ));
}

/// The bounds of every box, in graph units.
pub(crate) fn bounds(boxes: &[GraphBox]) -> Rect {
    boxes.iter().fold(Rect::NOTHING, |r, b| {
        r.union(Rect::from_min_size(b.place, b.size))
    })
}

/// Draws the canvas into the space left in `ui`, and gives back what was
/// done to it. `accent` colours what's picked.
pub(crate) fn graph_canvas(
    ui: &mut Ui,
    canvas: &mut GraphCanvas,
    boxes: &[GraphBox],
    wires: &[GraphWire],
    accent: Color32,
    menu: &mut GraphMenu<'_>,
) -> Vec<GraphEvent> {
    let mut events = Vec::new();
    let full = ui.available_rect_before_wrap();
    let origin = full.min;
    let background = ui.interact(full, ui.id().with("graph canvas"), Sense::click_and_drag());
    background
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Graph canvas"));
    if !canvas.framed && full.width() > 1.0 {
        canvas.framed = true;
        if !boxes.is_empty() {
            canvas.view.frame(bounds(boxes), full.size());
        }
    }
    let painter = ui.painter_at(full);
    painter.rect_filled(full, 0.0, theme::BACKGROUND);
    // A quiet grid, so panning shows.
    let step = 40.0 * canvas.view.zoom;
    if step > 8.0 {
        let start = origin
            + vec2(
                canvas.view.offset.x.rem_euclid(step),
                canvas.view.offset.y.rem_euclid(step),
            );
        let grid = Color32::from_rgb(0x22, 0x22, 0x22);
        let mut x = start.x;
        while x < full.right() {
            painter.vline(x, full.y_range(), Stroke::new(1.0, grid));
            x += step;
        }
        let mut y = start.y;
        while y < full.bottom() {
            painter.hline(full.x_range(), y, Stroke::new(1.0, grid));
            y += step;
        }
    }
    let hovered = ui.rect_contains_pointer(full);
    let pointer = ui.ctx().pointer_latest_pos();
    if let Some(p) = pointer.filter(|_| hovered) {
        canvas.pointer = canvas.view.to_world(origin, p);
    }
    // Pan with the middle button; zoom with the wheel round the pointer.
    if hovered {
        let (middle, delta, scroll) = ui.input(|i| {
            (
                i.pointer.middle_down(),
                i.pointer.delta(),
                i.smooth_scroll_delta.y,
            )
        });
        if middle {
            canvas.view.offset += delta;
        }
        if scroll != 0.0
            && let Some(p) = pointer
        {
            canvas.view.zoom_at(origin, p, (scroll / 200.0).exp());
        }
        let (frame, delete) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::F) && i.modifiers.is_none(),
                i.key_pressed(egui::Key::Delete),
            )
        });
        if frame && !ui.ctx().egui_wants_keyboard_input() {
            canvas.view.frame(bounds(boxes), full.size());
        }
        if delete && !ui.ctx().egui_wants_keyboard_input() {
            events.push(GraphEvent::Delete);
        }
    }
    let view = canvas.view;
    // Boxes where they're drawn, with a drag's move added.
    let dragging: Option<(Vec<u64>, Vec2)> = match &canvas.drag {
        Drag::Boxes(ids, by) => Some((ids.clone(), *by)),
        _ => None,
    };
    let moved = |b: &GraphBox| -> Vec2 {
        match &dragging {
            Some((ids, by)) if ids.contains(&b.id) => *by,
            _ => Vec2::ZERO,
        }
    };
    let rect_of = |b: &GraphBox| -> Rect {
        Rect::from_min_size(view.to_screen(origin, b.place), b.size * view.zoom).translate(moved(b))
    };
    let rects: Vec<(u64, Rect)> = boxes.iter().map(|b| (b.id, rect_of(b))).collect();
    let find = |id: u64| rects.iter().find(|(i, _)| *i == id).map(|(_, r)| *r);
    let add = ui.input(|i| i.modifiers.ctrl || i.modifiers.shift || i.modifiers.command);
    // Arrows, under the boxes.
    let mut wire_hit: Option<u64> = None;
    let font = egui::FontId::proportional(11.0 * view.zoom.clamp(0.7, 1.4));
    for w in wires {
        let (Some(a), Some(b)) = (find(w.from), find(w.to)) else {
            continue;
        };
        let back = wires.iter().any(|o| o.from == w.to && o.to == w.from);
        let (start, end) = wire_ends(a, b, back);
        let color = if w.picked {
            accent
        } else {
            Color32::from_rgb(0x9a, 0x9a, 0x9a)
        };
        painter.line_segment(
            [start, end],
            Stroke::new(if w.picked { 2.5 } else { 1.5 }, color),
        );
        let dir = (end - start).normalized();
        if dir.is_finite() {
            arrow_head(&painter, end, dir, color);
        }
        let middle = start + (end - start) / 2.0;
        if !w.label.is_empty() {
            // On the arrow's own side, so two arrows each way keep their
            // labels apart: the label sits beyond the line, away from the
            // other arrow.
            let perp = if dir.is_finite() {
                vec2(-dir.y, dir.x)
            } else {
                vec2(0.0, -1.0)
            };
            let side = if back {
                perp
            } else if perp.y.abs() >= perp.x.abs() {
                // Lying down: above it.
                perp * -perp.y.signum()
            } else {
                // Standing up: to its right.
                perp * perp.x.signum()
            };
            let align = if side.y > 0.3 {
                egui::Align2::CENTER_TOP
            } else if side.y < -0.3 {
                egui::Align2::CENTER_BOTTOM
            } else if side.x > 0.0 {
                egui::Align2::LEFT_CENTER
            } else {
                egui::Align2::RIGHT_CENTER
            };
            painter.text(
                middle + side * 5.0,
                align,
                &w.label,
                font.clone(),
                if w.picked { accent } else { theme::TEXT_WEAK },
            );
        }
        if let Some(p) = pointer
            && hovered
            && distance_to_segment(p, start, end) < 6.0
        {
            wire_hit = Some(w.id);
        }
        // Each arrow can be found and clicked by its label (screen readers,
        // tests).
        let hit = Rect::from_center_size(middle, vec2(18.0, 18.0));
        let r = ui.interact(hit, ui.id().with(("wire", w.id)), Sense::click());
        let label = format!("Transition {}", w.label);
        r.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, w.picked, &label)
        });
        if r.clicked() {
            events.push(GraphEvent::Pick(GraphTarget::Wire(w.id), add));
        }
        if r.secondary_clicked() {
            wire_hit = Some(w.id);
        }
    }
    // Boxes.
    // Names in the heading font, where the editor's fonts are loaded.
    let semibold = egui::FontFamily::Name(theme::SEMIBOLD.into());
    let family = if ui.fonts(|f| f.families().contains(&semibold)) {
        semibold
    } else {
        egui::FontFamily::Proportional
    };
    let title_font = egui::FontId::new(13.0 * view.zoom, family);
    let small = egui::FontId::proportional(11.0 * view.zoom);
    let mut box_hit: Option<u64> = None;
    for b in boxes {
        let rect = rect_of(b);
        let response = ui.interact(rect, ui.id().with(("box", b.id)), Sense::click_and_drag());
        let label = format!("State {}", b.title);
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, b.picked, &label)
        });
        if response.hovered() || response.secondary_clicked() {
            box_hit = Some(b.id);
        }
        if b.live {
            for (grow, alpha) in [(6.0, 40), (3.0, 90)] {
                painter.rect_stroke(
                    rect.expand(grow),
                    ROUND + grow,
                    Stroke::new(3.0, LIVE.gamma_multiply(f32::from(alpha as u8) / 255.0)),
                    egui::StrokeKind::Outside,
                );
            }
        }
        painter.rect_filled(rect, ROUND, Color32::from_rgb(0x2c, 0x2c, 0x2c));
        let border = if b.live {
            Stroke::new(2.0, LIVE)
        } else if b.picked {
            Stroke::new(2.0, accent)
        } else {
            Stroke::new(1.0, Color32::from_rgb(0x4a, 0x4a, 0x4a))
        };
        painter.rect_stroke(rect, ROUND, border, egui::StrokeKind::Inside);
        let pad = 10.0 * view.zoom;
        let clip = painter.with_clip_rect(rect.shrink(2.0).intersect(full));
        clip.text(
            rect.min + vec2(pad, 7.0 * view.zoom),
            egui::Align2::LEFT_TOP,
            &b.title,
            title_font.clone(),
            theme::TEXT_STRONG,
        );
        clip.text(
            rect.min + vec2(pad, 28.0 * view.zoom),
            egui::Align2::LEFT_TOP,
            &b.line,
            small.clone(),
            theme::TEXT_WEAK,
        );
        if let Some(chip) = &b.chip {
            let galley = clip.layout_no_wrap(chip.clone(), small.clone(), theme::WARNING);
            let at = rect.min + vec2(pad, rect.height() - 22.0 * view.zoom);
            let chip_rect = Rect::from_min_size(at, galley.size() + vec2(12.0, 2.0) * view.zoom);
            clip.rect_filled(
                chip_rect,
                8.0 * view.zoom,
                Color32::from_rgb(0x3a, 0x30, 0x20),
            );
            clip.galley(at + vec2(6.0, 1.0) * view.zoom, galley, theme::WARNING);
        }
        if b.start {
            let at = pos2(rect.left() - 12.0, rect.center().y);
            painter.text(
                at - vec2(14.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                "Start",
                small.clone(),
                theme::COLLIDER,
            );
            arrow_head(
                &painter,
                at + vec2(8.0, 0.0),
                vec2(1.0, 0.0),
                theme::COLLIDER,
            );
            painter.line_segment(
                [at - vec2(10.0, 0.0), at],
                Stroke::new(1.5, theme::COLLIDER),
            );
        }
        // Clicks, drags and double-clicks.
        if response.double_clicked() {
            events.push(GraphEvent::Edit(b.id));
        } else if response.clicked() {
            events.push(GraphEvent::Pick(GraphTarget::Box(b.id), add));
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
            let near_edge = response.interact_pointer_pos().is_some_and(|p| {
                let inner = rect.shrink(EDGE);
                !inner.contains(p)
            });
            canvas.drag = if near_edge {
                Drag::Joining(b.id)
            } else if b.movable {
                // Dragging an unpicked box moves it alone (and picks it).
                let ids = if b.picked {
                    boxes
                        .iter()
                        .filter(|o| o.picked && o.movable)
                        .map(|o| o.id)
                        .collect()
                } else {
                    events.push(GraphEvent::Pick(GraphTarget::Box(b.id), false));
                    vec![b.id]
                };
                Drag::Boxes(ids, Vec2::ZERO)
            } else {
                Drag::None
            };
        }
        if response.dragged_by(egui::PointerButton::Primary)
            && let Drag::Boxes(_, by) = &mut canvas.drag
        {
            *by += response.drag_delta();
        }
        response.context_menu(|ui| menu(ui, Some(GraphTarget::Box(b.id))));
    }
    // An arrow made from a menu ends at the next click.
    if let Drag::Aiming(from) = canvas.drag {
        let (pressed, escape) = ui.input(|i| {
            (
                i.pointer.button_pressed(egui::PointerButton::Primary),
                i.key_pressed(egui::Key::Escape),
            )
        });
        if escape {
            canvas.drag = Drag::None;
        } else if pressed {
            canvas.drag = Drag::None;
            if let Some(p) = pointer
                && let Some(to) = boxes
                    .iter()
                    .find(|b| b.takes_arrows && b.id != from && rect_of(b).contains(p))
            {
                events.push(GraphEvent::Join(from, to.id));
            }
        }
    }
    // An arrow being drawn.
    if let (Drag::Joining(from) | Drag::Aiming(from), Some(p)) = (&canvas.drag, pointer)
        && let Some(a) = find(*from)
    {
        let start = a.center();
        painter.line_segment([start, p], Stroke::new(2.0, accent));
        let dir = (p - start).normalized();
        if dir.is_finite() {
            arrow_head(&painter, p, dir, accent);
        }
    }
    // A picking box.
    if let (Drag::Picking(from, _), Some(p)) = (&canvas.drag, pointer) {
        let r = Rect::from_two_pos(*from, p);
        painter.rect_filled(r, 0.0, accent.gamma_multiply(0.12));
        painter.rect_stroke(r, 0.0, Stroke::new(1.0, accent), egui::StrokeKind::Inside);
    }
    // Empty space: picking boxes, and clearing.
    if background.drag_started_by(egui::PointerButton::Primary)
        && let Some(p) = background.interact_pointer_pos()
    {
        canvas.drag = Drag::Picking(p, add);
    }
    if background.clicked() {
        match wire_hit {
            Some(w) => events.push(GraphEvent::Pick(GraphTarget::Wire(w), add)),
            None => events.push(GraphEvent::Clear),
        }
    }
    let right_on_wire = wire_hit.filter(|_| box_hit.is_none());
    background.context_menu(|ui| menu(ui, right_on_wire.map(GraphTarget::Wire)));
    // Letting go ends whatever drag it was.
    if ui.input(|i| i.pointer.primary_released()) {
        match std::mem::take(&mut canvas.drag) {
            Drag::Boxes(ids, by) if by != Vec2::ZERO => {
                events.push(GraphEvent::Moved(ids, by / view.zoom));
            }
            Drag::Joining(from) => {
                if let Some(p) = pointer
                    && let Some(to) = boxes
                        .iter()
                        .find(|b| b.takes_arrows && b.id != from && rect_of(b).contains(p))
                {
                    events.push(GraphEvent::Join(from, to.id));
                }
            }
            Drag::Picking(from, add) => {
                if let Some(p) = pointer {
                    let r = Rect::from_two_pos(from, p);
                    if r.width() > 3.0 || r.height() > 3.0 {
                        let ids = boxes
                            .iter()
                            .filter(|b| r.intersects(rect_of(b)))
                            .map(|b| b.id)
                            .collect();
                        events.push(GraphEvent::PickBoxes(ids, add));
                    }
                }
            }
            _ => {}
        }
    }
    ui.advance_cursor_after_rect(full);
    events
}

#[cfg(test)]
#[path = "graph_canvas_tests.rs"]
mod tests;
