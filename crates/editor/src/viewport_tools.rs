//! The viewport's tools ([decision 0025](../../../docs/decisions/0025-picking-and-gizmos.md)):
//! picking with a click or a box, dragging the gizmo, and what's drawn
//! over the image for them (the gizmo, the selection's outline, the box,
//! and the snapping corner).
//!
//! The viewport never changes the area: it reports [`ViewportEvent`]s,
//! which the workbench turns into selections and undoable edits.

use crate::area_view::AreaView;
use crate::camera::Camera;
use crate::gizmo::{Change, Drag, Gizmo, Handle, Shape};
use crate::picking::PickMode;
use crate::theme::{self, Accent};
use crate::transforming::{ANGLE_STEPS, GRID_STEPS, Snapping};
use egui::{Align, Color32, Layout, Pos2, Rect, RichText, Stroke, Ui};
use glam::{Mat4, Vec2, Vec3};
use reflect::EntityId;

/// What the renderer found for a pick ([`crate::ViewportRenderer::picked`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PickAnswer {
    /// The rectangle asked about, in image pixels: x, y, width, height.
    pub rect: [u32; 4],
    /// Every pick number with a pixel in it (see
    /// [`crate::ViewModel::pick`]).
    pub found: Vec<u32>,
    /// The pick number at the rectangle's middle pixel, if any.
    pub middle: Option<u32>,
    /// The world point drawn at the middle pixel, if any.
    pub point: Option<Vec3>,
}

/// Something the viewport's tools did, for the workbench to carry out.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ViewportEvent {
    /// A gizmo drag began: remember where everything is.
    GizmoStart,
    /// The drag so far, before snapping (`flip`: Ctrl is held).
    GizmoChange {
        /// The change from where the drag started.
        change: Change,
        /// Whether snapping is turned around for this drag.
        flip: bool,
    },
    /// The drag ended.
    GizmoEnd,
    /// The drag was cancelled (Esc, right click): put everything back.
    GizmoCancel,
    /// A click on `hit` (or on nothing).
    Click {
        /// What was under the mouse.
        hit: Option<EntityId>,
        /// Replace or toggle.
        mode: PickMode,
    },
    /// A box around `hits`.
    Box {
        /// What had a pixel inside it.
        hits: Vec<EntityId>,
        /// Replace or add.
        mode: PickMode,
    },
    /// An asset dragged from the Library was dropped here.
    Drop {
        /// The model or assembly.
        asset: reflect::AssetId,
        /// Where in the world: the surface under the mouse, or the ground.
        at: Vec3,
    },
}

/// What a pick sent to the renderer is for.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PickPurpose {
    Click(PickMode),
    Box {
        mode: PickMode,
        /// Lamps whose markers are inside the box (they have no surface).
        markers: Vec<EntityId>,
    },
    /// Where to put an asset dropped from the Library; `fallback` (the
    /// ground under the mouse) if there's no surface there.
    Drop {
        asset: reflect::AssetId,
        fallback: Vec3,
    },
}

/// A pick waiting to be sent, or sent and waiting for its answer, with
/// the entity behind each pick number as they were when it was asked.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Pick {
    pub(crate) rect: [u32; 4],
    pub(crate) purpose: PickPurpose,
    pub(crate) pickable: Vec<EntityId>,
    /// Frames since it was sent (`None`: not sent yet).
    pub(crate) waited: Option<u32>,
}

/// Frames to wait for a pick's answer before giving up on it (a renderer
/// that stopped drawing never answers).
const PICK_PATIENCE: u32 = 120;
/// How far (points) the mouse may move with the button down and still
/// count as a click rather than a box.
const CLICK_SLOP: f32 = 4.0;
/// How close (points) a click must be to a lamp's or camera's marker to
/// pick it.
const MARKER_REACH: f32 = 10.0;

/// What the left mouse button is doing in the viewport.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) enum Pressing {
    /// Nothing.
    #[default]
    Nothing,
    /// Pressed on empty space: a click, unless it moves.
    Click { at: Vec2 },
    /// Drawing a selection box from `from`.
    Box { from: Vec2 },
    /// Dragging a gizmo handle.
    Gizmo {
        drag: Box<Drag>,
        last: Option<Change>,
    },
}

/// What the selection's outline is drawn around.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outline {
    /// A model's box, placed in the world.
    Box {
        /// Where the entity is.
        world: Mat4,
        /// The box's lowest corner, in the model's own space.
        min: Vec3,
        /// Its highest corner.
        max: Vec3,
    },
    /// A point (an entity without a model, such as a lamp).
    Point(Vec3),
}

/// The tools' state, kept between frames by the viewport.
#[derive(Debug, Clone, Default)]
pub(crate) struct Tools {
    /// The gizmo to show, worked out by the workbench each frame.
    pub(crate) gizmo: Option<Gizmo>,
    /// What to outline, worked out by the workbench each frame.
    pub(crate) outlines: Vec<Outline>,
    pub(crate) pressing: Pressing,
    pub(crate) hovered: Option<Handle>,
    pub(crate) pick: Option<Pick>,
    /// A pick asked for while another was in flight; sent after it.
    pub(crate) next: Option<Pick>,
    pub(crate) events: Vec<ViewportEvent>,
}

/// The mouse and keys this frame, as the tools need them.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ToolInput {
    /// Where the mouse is, in points from the viewport's corner.
    pub(crate) mouse: Option<Vec2>,
    pub(crate) pressed: bool,
    pub(crate) down: bool,
    pub(crate) released: bool,
    pub(crate) right_pressed: bool,
    pub(crate) ctrl: bool,
    pub(crate) shift: bool,
    /// Whether the press was on the viewport itself (not its corner
    /// controls or another panel).
    pub(crate) on_viewport: bool,
}

impl Tools {
    /// Handles this frame's mouse. `size` is the viewport's size in
    /// points, `ppp` points to pixels.
    pub(crate) fn interact(
        &mut self,
        input: ToolInput,
        camera: &Camera,
        area: &AreaView,
        snapping: &Snapping,
        size: Vec2,
        ppp: f32,
    ) {
        let locked = snapping.locked;
        self.hovered = None;
        match std::mem::take(&mut self.pressing) {
            Pressing::Nothing => {
                let Some(mouse) = input.mouse else { return };
                let hovered = self
                    .gizmo
                    .filter(|_| input.on_viewport)
                    .and_then(|g| g.handle_at(camera, size, mouse, locked));
                self.hovered = hovered;
                if input.pressed && input.on_viewport {
                    self.pressing = match (self.gizmo, hovered) {
                        (Some(gizmo), Some(handle)) => {
                            self.events.push(ViewportEvent::GizmoStart);
                            Pressing::Gizmo {
                                drag: Box::new(Drag::new(gizmo, handle, camera, size, mouse)),
                                last: None,
                            }
                        }
                        _ => Pressing::Click { at: mouse },
                    };
                }
            }
            Pressing::Click { at } => {
                let mouse = input.mouse.unwrap_or(at);
                if input.released || !input.down {
                    self.click(at, input.ctrl, camera, area, size, ppp);
                } else if mouse.distance(at) > CLICK_SLOP {
                    self.pressing = Pressing::Box { from: at };
                } else {
                    self.pressing = Pressing::Click { at };
                }
            }
            Pressing::Box { from } => {
                let mouse = input.mouse.unwrap_or(from);
                if input.right_pressed {
                    // Cancelled: nothing selected.
                } else if input.released || !input.down {
                    self.boxed(from, mouse, input.ctrl, camera, area, size, ppp);
                } else {
                    self.pressing = Pressing::Box { from };
                }
            }
            Pressing::Gizmo { mut drag, last } => {
                self.hovered = Some(drag.handle);
                if input.right_pressed {
                    self.events.push(ViewportEvent::GizmoCancel);
                    return;
                }
                let change = match input.mouse {
                    Some(mouse) => drag.update(mouse, input.shift, locked),
                    None => drag.change(locked),
                };
                if last != Some(change) {
                    self.events.push(ViewportEvent::GizmoChange {
                        change,
                        flip: input.ctrl,
                    });
                }
                if input.released || !input.down {
                    self.events.push(ViewportEvent::GizmoEnd);
                } else {
                    self.pressing = Pressing::Gizmo {
                        drag,
                        last: Some(change),
                    };
                }
            }
        }
    }

    /// Cancels a gizmo drag or a box, if one is under way (Esc).
    pub(crate) fn cancel(&mut self) -> bool {
        match std::mem::take(&mut self.pressing) {
            Pressing::Gizmo { .. } => {
                self.events.push(ViewportEvent::GizmoCancel);
                true
            }
            Pressing::Box { .. } => true,
            Pressing::Nothing | Pressing::Click { .. } => false,
        }
    }

    /// Whether a gizmo is being dragged.
    pub(crate) fn dragging(&self) -> bool {
        matches!(self.pressing, Pressing::Gizmo { .. })
    }

    /// A click at `at`: a lamp's marker, or whatever the renderer finds
    /// under the mouse.
    fn click(
        &mut self,
        at: Vec2,
        ctrl: bool,
        camera: &Camera,
        area: &AreaView,
        size: Vec2,
        ppp: f32,
    ) {
        let mode = if ctrl {
            PickMode::Toggle
        } else {
            PickMode::Replace
        };
        let marker = area
            .markers()
            .filter_map(|(entity, position)| {
                let p = camera.project(position, size)?;
                let d = p.distance(at);
                (d <= MARKER_REACH).then_some((d, entity))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((_, entity)) = marker {
            self.events.push(ViewportEvent::Click {
                hit: Some(entity),
                mode,
            });
            return;
        }
        let px = pixel_at(at, size, ppp);
        self.ask(
            [px.x as u32, px.y as u32, 1, 1],
            PickPurpose::Click(mode),
            area,
        );
    }

    /// A box from `a` to `b`.
    #[allow(clippy::too_many_arguments)]
    fn boxed(
        &mut self,
        a: Vec2,
        b: Vec2,
        ctrl: bool,
        camera: &Camera,
        area: &AreaView,
        size: Vec2,
        ppp: f32,
    ) {
        let (min, max) = (a.min(b).max(Vec2::ZERO), a.max(b).min(size));
        let markers = area
            .markers()
            .filter(|(_, position)| {
                camera
                    .project(*position, size)
                    .is_some_and(|p| p.cmpge(min).all() && p.cmple(max).all())
            })
            .map(|(entity, _)| entity)
            .collect();
        let last = ((size * ppp).round() - Vec2::ONE).max(Vec2::ZERO);
        let (lo, hi) = (
            (min * ppp).floor().min(last),
            (max * ppp).ceil().min(last + Vec2::ONE),
        );
        let mode = if ctrl {
            PickMode::Add
        } else {
            PickMode::Replace
        };
        self.ask(
            [
                lo.x as u32,
                lo.y as u32,
                (hi.x - lo.x).max(1.0) as u32,
                (hi.y - lo.y).max(1.0) as u32,
            ],
            PickPurpose::Box { mode, markers },
            area,
        );
    }

    /// An asset dragged from the Library was dropped at `at`: it goes on
    /// the surface there, or on the ground (or where the camera looks, if
    /// the ground isn't under the mouse).
    pub(crate) fn drop_asset(
        &mut self,
        asset: reflect::AssetId,
        at: Vec2,
        camera: &Camera,
        area: &AreaView,
        size: Vec2,
        ppp: f32,
    ) {
        let ray = camera.ray_through(at, size);
        let fallback = if camera.is_flat() {
            // A flat view: the point under the mouse, as deep as the
            // view's middle (in a front view, at the height pointed at).
            ray.at((camera.position - ray.origin).dot(ray.direction))
        } else {
            ray.hit_ground(0.0)
                .filter(|p| p.distance(camera.position) < 1_000.0)
                .unwrap_or_else(|| camera.focus())
        };
        let px = pixel_at(at, size, ppp);
        self.ask(
            [px.x as u32, px.y as u32, 1, 1],
            PickPurpose::Drop { asset, fallback },
            area,
        );
    }

    /// Asks the renderer about `rect`. A newer question replaces one not
    /// yet sent.
    fn ask(&mut self, rect: [u32; 4], purpose: PickPurpose, area: &AreaView) {
        if self.pick.as_ref().is_some_and(|p| p.waited.is_some()) {
            // One is in flight: this one goes after it.
            self.next = Some(Pick {
                rect,
                purpose,
                pickable: area.pickable.clone(),
                waited: None,
            });
            return;
        }
        self.pick = Some(Pick {
            rect,
            purpose,
            pickable: area.pickable.clone(),
            waited: None,
        });
    }

    /// The rectangle to send to the renderer this frame, if a pick is
    /// waiting to be sent; it's marked as sent.
    /// [`Self::rect_to_send`], with the entities behind this frame's pick
    /// numbers: a pick asked for while another was in flight is drawn
    /// frames later, with that frame's numbers, so its list is taken now.
    pub(crate) fn send_with(&mut self, pickable: &[EntityId]) -> Option<[u32; 4]> {
        if let Some(pick) = self.pick.as_mut()
            && pick.waited.is_none()
        {
            pick.pickable = pickable.to_vec();
        }
        self.rect_to_send()
    }

    pub(crate) fn rect_to_send(&mut self) -> Option<[u32; 4]> {
        let pick = self.pick.as_mut()?;
        if pick.waited.is_some() {
            return None;
        }
        pick.waited = Some(0);
        Some(pick.rect)
    }

    /// Takes the renderer's answer to the pick in flight, if it came.
    pub(crate) fn receive(&mut self, answer: Option<PickAnswer>) {
        let Some(pick) = self.pick.as_mut() else {
            return;
        };
        let Some(waited) = pick.waited.as_mut() else {
            return;
        };
        // An answer to an older question (one given up on) isn't this
        // one's: it's waited for still.
        let answer = answer.filter(|a| a.rect == pick.rect);
        let Some(answer) = answer else {
            *waited += 1;
            if *waited > PICK_PATIENCE {
                // A drop still lands, on the ground, rather than being lost.
                if let PickPurpose::Drop { asset, fallback } = pick.purpose {
                    self.events.push(ViewportEvent::Drop {
                        asset,
                        at: fallback,
                    });
                }
                self.pick = self.next.take();
            }
            return;
        };
        let entity = |n: u32| -> Option<EntityId> {
            n.checked_sub(1)
                .and_then(|i| pick.pickable.get(i as usize).copied())
        };
        let event = match &pick.purpose {
            PickPurpose::Click(mode) => ViewportEvent::Click {
                hit: answer.middle.and_then(entity),
                mode: *mode,
            },
            PickPurpose::Box { mode, markers } => {
                let mut hits: Vec<EntityId> =
                    answer.found.iter().filter_map(|&n| entity(n)).collect();
                hits.extend(markers);
                ViewportEvent::Box { hits, mode: *mode }
            }
            PickPurpose::Drop { asset, fallback } => ViewportEvent::Drop {
                asset: *asset,
                at: answer.point.unwrap_or(*fallback),
            },
        };
        self.events.push(event);
        self.pick = self.next.take();
    }
}

/// Draws the tools over the viewport image at `rect`.
/// The image pixel under `at` (points) in a view `size` points across:
/// never past the last pixel, which the image may round away at some
/// display scales (a pick there would ask about nothing, and never be
/// answered).
fn pixel_at(at: Vec2, size: Vec2, ppp: f32) -> Vec2 {
    let last = ((size * ppp).round() - Vec2::ONE).max(Vec2::ZERO);
    (at * ppp).floor().max(Vec2::ZERO).min(last)
}

/// Draws a marker for each camera: a small wire pyramid pointing the way
/// it looks, with a notch on its top edge for which way is up.
pub(crate) fn draw_cameras(ui: &Ui, rect: Rect, cameras: &[(EntityId, Mat4)], camera: &Camera) {
    let painter = ui.painter_at(rect);
    let size = Vec2::new(rect.width(), rect.height());
    let at = |p: Vec2| rect.min + egui::vec2(p.x, p.y);
    let stroke = Stroke::new(1.2, theme::TEXT_CONSOLE);
    for (_, world) in cameras {
        let (_, rotation, position) = world.to_scale_rotation_translation();
        let place = |x: f32, y: f32, z: f32| position + rotation * Vec3::new(x, y, z);
        let (w, h, d) = (0.4, 0.25, 0.6);
        let corners = [
            place(-w, -h, -d),
            place(w, -h, -d),
            place(w, h, -d),
            place(-w, h, -d),
        ];
        let apex = place(0.0, 0.0, 0.0);
        let notch = place(0.0, h + 0.18, -d);
        let project = |p: Vec3| camera.project(p, size).map(at);
        let segment = |a: Vec3, b: Vec3| {
            if let (Some(p), Some(q)) = (project(a), project(b)) {
                painter.line_segment([p, q], stroke);
            }
        };
        for i in 0..4 {
            segment(corners[i], corners[(i + 1) % 4]);
            segment(apex, corners[i]);
        }
        segment(corners[3], notch);
        segment(notch, corners[2]);
    }
}

pub(crate) fn draw(
    ui: &Ui,
    rect: Rect,
    tools: &Tools,
    camera: &Camera,
    locked: [bool; 3],
    accent: Accent,
) {
    let painter = ui.painter_at(rect);
    let size = Vec2::new(rect.width(), rect.height());
    let at = |p: Vec2| rect.min + egui::vec2(p.x, p.y);
    // The selection's outline.
    let outline = Stroke::new(1.5, accent.colors().light);
    for mark in &tools.outlines {
        match *mark {
            Outline::Box { world, min, max } => {
                let corner = |i: usize| {
                    world.transform_point3(Vec3::new(
                        if i & 1 == 0 { min.x } else { max.x },
                        if i & 2 == 0 { min.y } else { max.y },
                        if i & 4 == 0 { min.z } else { max.z },
                    ))
                };
                let screen: Vec<Option<Vec2>> =
                    (0..8).map(|i| camera.project(corner(i), size)).collect();
                for (a, b) in BOX_EDGES {
                    if let (Some(p), Some(q)) = (screen[a], screen[b]) {
                        painter.line_segment([at(p), at(q)], outline);
                    }
                }
            }
            Outline::Point(p) => {
                if let Some(s) = camera.project(p, size) {
                    painter.circle_stroke(at(s), 9.0, outline);
                }
            }
        }
    }
    // The gizmo.
    if let Some(gizmo) = &tools.gizmo {
        let dragging = match &tools.pressing {
            Pressing::Gizmo { drag, .. } => Some(drag.handle),
            _ => None,
        };
        for (handle, shape) in gizmo.shapes(camera, size, locked) {
            let lit = tools.hovered == Some(handle) || dragging == Some(handle);
            let color = if lit {
                HIGHLIGHT
            } else {
                match handle.axis() {
                    Some(0) => theme::axis::X,
                    Some(1) => theme::axis::Y,
                    Some(_) => theme::axis::Z,
                    None => theme::TEXT_STRONG,
                }
            };
            draw_shape(&painter, &shape, color, &at);
        }
    }
    // The box being drawn.
    if let (Pressing::Box { from }, Some(mouse)) =
        (&tools.pressing, ui.input(|i| i.pointer.hover_pos()))
    {
        let box_rect = Rect::from_two_pos(at(*from), mouse);
        painter.rect_filled(box_rect, 0.0, accent.colors().fill.gamma_multiply(0.15));
        painter.rect_stroke(
            box_rect,
            0.0,
            Stroke::new(1.0, accent.colors().light),
            egui::StrokeKind::Inside,
        );
    }
}

/// The twelve edges of a box, by corner number (bit 0: x, 1: y, 2: z).
const BOX_EDGES: [(usize, usize); 12] = [
    (0, 1),
    (2, 3),
    (4, 5),
    (6, 7),
    (0, 2),
    (1, 3),
    (4, 6),
    (5, 7),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

/// The colour of the handle under the mouse or being dragged.
const HIGHLIGHT: Color32 = Color32::from_rgb(0xff, 0xd2, 0x4a);

fn draw_shape(painter: &egui::Painter, shape: &Shape, color: Color32, at: &impl Fn(Vec2) -> Pos2) {
    let stroke = Stroke::new(2.0, color);
    match shape {
        Shape::Line { a, b, arrow } => {
            painter.line_segment([at(*a), at(*b)], stroke);
            if *arrow {
                let along = (*b - *a).normalize_or_zero();
                let side = along.perp();
                let tip = *b + along * 10.0;
                painter.add(egui::Shape::convex_polygon(
                    vec![at(tip), at(*b + side * 5.0), at(*b - side * 5.0)],
                    color,
                    Stroke::NONE,
                ));
            }
        }
        Shape::Ring(points) => {
            let mut closed: Vec<Pos2> = points.iter().map(|p| at(*p)).collect();
            if let Some(first) = closed.first().copied() {
                closed.push(first);
            }
            painter.add(egui::Shape::line(closed, stroke));
        }
        Shape::Quad(corners) => {
            painter.add(egui::Shape::convex_polygon(
                corners.iter().map(|p| at(*p)).collect(),
                color.gamma_multiply(0.45),
                Stroke::new(1.0, color),
            ));
        }
        Shape::Dot { centre, radius } => {
            painter.circle(at(*centre), *radius, color.gamma_multiply(0.5), stroke);
        }
    }
}

/// The text shown by the mouse while dragging: how far, how much.
pub(crate) fn readout(change: Change) -> String {
    match change {
        Change::Translate(v) => {
            let parts: Vec<String> = ["X", "Y", "Z"]
                .iter()
                .zip(v.to_array())
                .filter(|(_, d)| d.abs() > 1e-5)
                .map(|(axis, d)| format!("{axis} {d:+.2} m"))
                .collect();
            if parts.is_empty() {
                "0 m".into()
            } else {
                parts.join("  ")
            }
        }
        Change::Rotate { angle, .. } => format!("{:+.1}°", angle.to_degrees()),
        Change::Scale(f) => {
            if (f.x - f.y).abs() < 1e-5 && (f.y - f.z).abs() < 1e-5 {
                format!("× {:.2}", f.x)
            } else {
                format!("× {:.2}  {:.2}  {:.2}", f.x, f.y, f.z)
            }
        }
    }
}

/// Grid, angle and scale snapping, the axis locks and world or object
/// axes, and the camera speed: the viewport's bottom-right corner, in two
/// rows so it stays clear of the tools in the middle.
pub(crate) fn snapping_corner(
    ui: &mut Ui,
    snapping: &mut Snapping,
    accent: Accent,
    camera_speed: u8,
    upwards: bool,
) {
    // In a bottom corner the rows build upwards; in a top one, downwards.
    let layout = if upwards {
        Layout::bottom_up(Align::Max)
    } else {
        Layout::top_down(Align::Max)
    };
    // Each row hugs the edge the corner builds from.
    let row = Layout::right_to_left(if upwards { Align::Max } else { Align::Min });
    ui.with_layout(layout, |ui| {
        ui.spacing_mut().item_spacing.y = 3.0;
        // Bottom row, laid out from the right edge, so the boxes go in
        // backwards: Grid, Angle, Scale, Camera.
        ui.with_layout(row, |ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            theme::overlay_frame()
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(camera_speed.to_string())
                                .color(accent.colors().light)
                                .strong(),
                        );
                        ui.label("Camera");
                    })
                    .response
                    .on_hover_text("Camera speed: hold the right mouse button and turn the wheel");
                });
            theme::overlay_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    toggle(
                        ui,
                        &mut snapping.scale,
                        "Scale 10%",
                        "Scaling goes in 10% steps",
                        accent,
                    );
                    step_menu(ui, &mut snapping.angle_step, &ANGLE_STEPS, |s| {
                        format!("{s:.0}°")
                    });
                    toggle(
                        ui,
                        &mut snapping.angle,
                        &format!("Angle {:.0}°", snapping.angle_step),
                        "Turning goes in steps (hold Ctrl while dragging to flip this)",
                        accent,
                    );
                    step_menu(ui, &mut snapping.grid_step, &GRID_STEPS, grid_text);
                    toggle(
                        ui,
                        &mut snapping.grid,
                        &format!("Grid {}", grid_text(snapping.grid_step)),
                        "Moves land on the grid (hold Ctrl while dragging to flip this)",
                        accent,
                    );
                });
            });
        });
        // The row above: X, Y, Z locks and world or object axes.
        ui.with_layout(row, |ui| {
            theme::overlay_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let axes = if snapping.object_axes {
                        "Object"
                    } else {
                        "World"
                    };
                    toggle(
                        ui,
                        &mut snapping.object_axes,
                        axes,
                        "The gizmo follows the world's axes or the entity's own (L)",
                        accent,
                    );
                    for (i, name) in ["Z", "Y", "X"].into_iter().enumerate() {
                        let axis = 2 - i;
                        lock_button(ui, &mut snapping.locked[axis], name);
                    }
                });
            });
        });
    });
}

/// A grid step as text: "1 m", "50 cm".
fn grid_text(step: f32) -> String {
    if step >= 1.0 {
        format!("{step:.0} m")
    } else {
        format!("{:.0} cm", step * 100.0)
    }
}

/// A button that switches `on`, filled with the accent while on.
fn toggle(ui: &mut Ui, on: &mut bool, text: &str, hint: &str, accent: Accent) {
    let label = if *on {
        RichText::new(text).color(Color32::WHITE)
    } else {
        RichText::new(text)
    };
    let mut button = egui::Button::new(label).min_size(egui::vec2(0.0, 20.0));
    if *on {
        button = button.fill(accent.colors().fill);
    }
    let response = ui.add(button).on_hover_text(hint);
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, *on, text));
    if response.clicked() {
        *on = !*on;
    }
}

/// An axis lock: the axis's letter, red while locked.
fn lock_button(ui: &mut Ui, locked: &mut bool, name: &str) {
    let text = if *locked {
        RichText::new(name).color(Color32::WHITE).strong()
    } else {
        RichText::new(name)
    };
    let mut button = egui::Button::new(text).min_size(egui::vec2(20.0, 20.0));
    if *locked {
        button = button.fill(theme::axis::X.gamma_multiply(0.8));
    }
    let hint = format!("Lock {name}: drags leave it alone ({name})");
    let response = ui.add(button).on_hover_text(&hint);
    let label = format!("Lock {name}");
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, *locked, &label)
    });
    if response.clicked() {
        *locked = !*locked;
    }
}

/// A small ▴ menu choosing one of `steps`.
fn step_menu(ui: &mut Ui, value: &mut f32, steps: &[f32], text: impl Fn(f32) -> String) {
    let response = ui.add(egui::Button::new("▴").min_size(egui::vec2(14.0, 20.0)));
    egui::Popup::menu(&response)
        .align(egui::RectAlign::TOP_START)
        .show(|ui| {
            for &step in steps {
                if ui
                    .add(egui::Button::new(text(step)).selected(*value == step))
                    .clicked()
                {
                    *value = step;
                }
            }
        });
}

#[cfg(test)]
#[path = "viewport_tools_tests.rs"]
mod tests;
