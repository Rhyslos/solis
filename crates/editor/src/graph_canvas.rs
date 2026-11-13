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
use drawing::{box_fonts, draw_box, draw_drag, draw_grid, draw_wires};
use egui::{Color32, Pos2, Rect, Sense, Ui, Vec2, vec2};

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

/// How far `p` is from the segment `a`–`b`.
fn distance_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// The bounds of every box, in graph units.
pub(crate) fn bounds(boxes: &[GraphBox]) -> Rect {
    boxes.iter().fold(Rect::NOTHING, |r, b| {
        r.union(Rect::from_min_size(b.place, b.size))
    })
}

/// One frame of the canvas: where it is on screen, the pointer, and where
/// each box is drawn (with a drag's move, as it was when the frame began).
struct Frame {
    /// The canvas's space.
    full: Rect,
    /// Its top left: where the view's offset counts from.
    origin: Pos2,
    /// Where it looks, after this frame's pan and zoom.
    view: GraphView,
    /// The pointer, if it's anywhere.
    pointer: Option<Pos2>,
    /// Whether the pointer is over the canvas.
    hovered: bool,
    /// Whether a click adds to what's picked (Ctrl or Shift).
    add: bool,
    /// Boxes being dragged, and how far, as the frame began.
    dragging: Option<(Vec<u64>, Vec2)>,
    /// What's picked is drawn in this.
    accent: Color32,
}

impl Frame {
    /// Where box `b` is drawn, on screen.
    fn rect_of(&self, b: &GraphBox) -> Rect {
        let moved = match &self.dragging {
            Some((ids, by)) if ids.contains(&b.id) => *by,
            _ => Vec2::ZERO,
        };
        Rect::from_min_size(
            self.view.to_screen(self.origin, b.place),
            b.size * self.view.zoom,
        )
        .translate(moved)
    }

    /// The box under `p` that an arrow from `from` can end at.
    fn arrow_end_at(&self, boxes: &[GraphBox], from: u64, p: Pos2) -> Option<u64> {
        boxes
            .iter()
            .find(|b| b.takes_arrows && b.id != from && self.rect_of(b).contains(p))
            .map(|b| b.id)
    }
}

/// Draws the canvas into the space left in `ui`, and gives back what was
/// done to it. `accent` colours what's picked. F, Delete and Esc are acted
/// on while the keys are the graph's editor's (`keys`: the pointer rests
/// over its tab and nothing is being typed).
pub(crate) fn graph_canvas(
    ui: &mut Ui,
    canvas: &mut GraphCanvas,
    boxes: &[GraphBox],
    wires: &[GraphWire],
    accent: Color32,
    keys: bool,
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
    painter.rect_filled(full, 0.0, theme::background());
    draw_grid(&painter, full, canvas.view);
    let hovered = ui.rect_contains_pointer(full);
    let pointer = ui.ctx().pointer_latest_pos();
    pan_and_zoom(ui, canvas, origin, pointer, hovered);
    if keys {
        events.extend(canvas_keys(ui, canvas, boxes, full.size()));
    }
    // Boxes where they're drawn, with a drag's move added.
    let frame = Frame {
        full,
        origin,
        view: canvas.view,
        pointer,
        hovered,
        add: ui.input(|i| i.modifiers.ctrl || i.modifiers.shift || i.modifiers.command),
        dragging: match &canvas.drag {
            Drag::Boxes(ids, by) => Some((ids.clone(), *by)),
            _ => None,
        },
        accent,
    };
    // Arrows, under the boxes.
    let wire_hit = draw_wires(ui, &painter, &frame, boxes, wires, &mut events);
    let box_hit = draw_boxes(ui, &painter, &frame, canvas, boxes, &mut events, menu);
    // An arrow made from a menu ends at the next click.
    events.extend(end_aim(ui, &frame, canvas, boxes, keys));
    draw_drag(&painter, &frame, canvas, boxes);
    empty_space(
        &background,
        &frame,
        canvas,
        (wire_hit, box_hit),
        &mut events,
        menu,
    );
    // Letting go ends whatever drag it was.
    if ui.input(|i| i.pointer.primary_released()) {
        events.extend(end_drag(&frame, canvas, boxes));
    }
    ui.advance_cursor_after_rect(full);
    events
}

/// Notes where the pointer is over the canvas, in graph units; pan with
/// the middle button; zoom with the wheel round the pointer.
fn pan_and_zoom(
    ui: &Ui,
    canvas: &mut GraphCanvas,
    origin: Pos2,
    pointer: Option<Pos2>,
    hovered: bool,
) {
    if let Some(p) = pointer.filter(|_| hovered) {
        canvas.pointer = canvas.view.to_world(origin, p);
    }
    if !hovered {
        return;
    }
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
}

/// F frames everything; Delete removes what's picked.
fn canvas_keys(
    ui: &Ui,
    canvas: &mut GraphCanvas,
    boxes: &[GraphBox],
    size: Vec2,
) -> Option<GraphEvent> {
    let (frame, delete) = ui.input(|i| {
        (
            i.key_pressed(egui::Key::F) && i.modifiers.is_none(),
            i.key_pressed(egui::Key::Delete),
        )
    });
    if frame {
        canvas.view.frame(bounds(boxes), size);
    }
    delete.then_some(GraphEvent::Delete)
}

/// Draws the boxes and takes their clicks, drags and right-clicks; gives
/// back the box under the pointer (or right-clicked), if any.
fn draw_boxes(
    ui: &Ui,
    painter: &egui::Painter,
    frame: &Frame,
    canvas: &mut GraphCanvas,
    boxes: &[GraphBox],
    events: &mut Vec<GraphEvent>,
    menu: &mut GraphMenu<'_>,
) -> Option<u64> {
    let fonts = box_fonts(ui, frame.view.zoom);
    let mut box_hit: Option<u64> = None;
    for b in boxes {
        let rect = frame.rect_of(b);
        let response = ui.interact(rect, ui.id().with(("box", b.id)), Sense::click_and_drag());
        let label = format!("State {}", b.title);
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, b.picked, &label)
        });
        if response.hovered() || response.secondary_clicked() {
            box_hit = Some(b.id);
        }
        draw_box(painter, frame, b, rect, &fonts);
        // Clicks, drags and double-clicks.
        if response.double_clicked() {
            events.push(GraphEvent::Edit(b.id));
        } else if response.clicked() {
            events.push(GraphEvent::Pick(GraphTarget::Box(b.id), frame.add));
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
            let near_edge = response.interact_pointer_pos().is_some_and(|p| {
                let inner = rect.shrink(EDGE);
                !inner.contains(p)
            });
            canvas.drag = drag_from(b, near_edge, boxes, events);
        }
        if response.dragged_by(egui::PointerButton::Primary)
            && let Drag::Boxes(_, by) = &mut canvas.drag
        {
            *by += response.drag_delta();
        }
        response.context_menu(|ui| menu(ui, Some(GraphTarget::Box(b.id))));
    }
    box_hit
}

/// What a drag started on box `b` drags: an arrow from it when it's near
/// the edge, otherwise the picked boxes (an unpicked box moves alone, and
/// is picked).
fn drag_from(
    b: &GraphBox,
    near_edge: bool,
    boxes: &[GraphBox],
    events: &mut Vec<GraphEvent>,
) -> Drag {
    if near_edge {
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
    }
}

/// An arrow made from a box's menu ends at the next click: joined to
/// the box clicked, if it takes arrows; Escape, or a click anywhere
/// else, gives up.
fn end_aim(
    ui: &Ui,
    frame: &Frame,
    canvas: &mut GraphCanvas,
    boxes: &[GraphBox],
    keys: bool,
) -> Option<GraphEvent> {
    let Drag::Aiming(from) = canvas.drag else {
        return None;
    };
    let (pressed, escape) = ui.input(|i| {
        (
            i.pointer.button_pressed(egui::PointerButton::Primary),
            keys && i.key_pressed(egui::Key::Escape),
        )
    });
    if escape {
        canvas.drag = Drag::None;
        return None;
    }
    if !pressed {
        return None;
    }
    canvas.drag = Drag::None;
    let to = frame
        .pointer
        .and_then(|p| frame.arrow_end_at(boxes, from, p))?;
    Some(GraphEvent::Join(from, to))
}

/// Empty space: a drag there starts a picking box, a click clears what's
/// picked (or picks the arrow under the pointer), and a right-click opens
/// the menu for empty space or the arrow under it (`hits`: the arrow and
/// the box under the pointer).
fn empty_space(
    background: &egui::Response,
    frame: &Frame,
    canvas: &mut GraphCanvas,
    (wire_hit, box_hit): (Option<u64>, Option<u64>),
    events: &mut Vec<GraphEvent>,
    menu: &mut GraphMenu<'_>,
) {
    if background.drag_started_by(egui::PointerButton::Primary)
        && let Some(p) = background.interact_pointer_pos()
    {
        canvas.drag = Drag::Picking(p, frame.add);
    }
    if background.clicked() {
        match wire_hit {
            Some(w) => events.push(GraphEvent::Pick(GraphTarget::Wire(w), frame.add)),
            None => events.push(GraphEvent::Clear),
        }
    }
    let right_on_wire = wire_hit.filter(|_| box_hit.is_none());
    background.context_menu(|ui| menu(ui, right_on_wire.map(GraphTarget::Wire)));
}

/// Letting go ends whatever drag it was: moved boxes, an arrow joined,
/// or boxes picked by a picking box.
fn end_drag(frame: &Frame, canvas: &mut GraphCanvas, boxes: &[GraphBox]) -> Option<GraphEvent> {
    let pointer = frame.pointer;
    match std::mem::take(&mut canvas.drag) {
        Drag::Boxes(ids, by) if by != Vec2::ZERO => {
            Some(GraphEvent::Moved(ids, by / frame.view.zoom))
        }
        Drag::Joining(from) => {
            let to = pointer.and_then(|p| frame.arrow_end_at(boxes, from, p))?;
            Some(GraphEvent::Join(from, to))
        }
        Drag::Picking(from, add) => {
            let r = Rect::from_two_pos(from, pointer?);
            (r.width() > 3.0 || r.height() > 3.0).then(|| {
                let ids = boxes
                    .iter()
                    .filter(|b| r.intersects(frame.rect_of(b)))
                    .map(|b| b.id)
                    .collect();
                GraphEvent::PickBoxes(ids, add)
            })
        }
        _ => None,
    }
}

#[path = "graph_canvas_drawing.rs"]
mod drawing;

#[cfg(test)]
#[path = "graph_canvas_tests.rs"]
mod tests;
