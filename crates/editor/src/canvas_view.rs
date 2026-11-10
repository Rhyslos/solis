//! The UI editor's canvas (decision 0089): the open screen drawn by the
//! real renderer at a preview size, its pieces picked with a click (Shift
//! for more), a box dragged round them, or the Elements panel; a piece
//! dragged to another place among its neighbours (or into another
//! container), dragged by its handles to change its size, or, on a
//! `canvas`, dragged to move it. The wheel zooms and the middle button
//! drags the screen, as in the viewport.

use crate::screen_doc::{ScreenDoc, node_at_mut};
use crate::theme;
use crate::viewport::{ViewportRenderer, ViewportView};
use egui::{Color32, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use std::sync::Arc;

/// The canvas's image among the renderer's.
pub(crate) const CANVAS_IMAGE: usize = crate::viewport::GAME_IMAGE + 1;

/// How near a handle the pointer must be, in points.
const HANDLE: f32 = 7.0;

/// How near another's edge a size snaps to, in the screen's pixels.
const SNAP: f32 = 6.0;

/// The part of the screen kept clear of TVs' and phones' edges.
const SAFE: f32 = 0.05;

/// What the canvas needs besides the screen.
pub(crate) struct CanvasUi<'a> {
    /// The screen open.
    pub(crate) doc: &'a mut ScreenDoc,
    /// Draws it.
    pub(crate) renderer: &'a mut dyn ViewportRenderer,
    /// Measures its text.
    pub(crate) fonts: &'a mut ui::Fonts,
    /// The game's screens' pictures.
    pub(crate) pictures: Option<Arc<ui::Pictures>>,
    /// The accent colour.
    pub(crate) accent: Color32,
    /// The project's pictures (paths inside the assets), for the Image
    /// tool.
    pub(crate) picture_files: &'a [String],
    /// What the screen's entity can be read for (Preview values).
    pub(crate) readable: &'a [crate::screen_ties::Readable],
}

/// What the canvas asks of the workbench.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct CanvasOut {
    /// The screen changed: save it.
    pub(crate) changed: bool,
    /// The pointer is over the canvas (Ctrl+Z is its).
    pub(crate) hovered: bool,
    /// Open the file in a code editor.
    pub(crate) open_code: bool,
    /// Something to say in the Console.
    pub(crate) said: Option<String>,
}

/// A drag going on.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Drag {
    /// A box round pieces, from here.
    Box(Pos2),
    /// The picked piece to another place.
    Move,
    /// The picked piece's right edge, bottom edge or corner, by how much.
    Size { right: bool, bottom: bool },
}

/// The screen as drawn, with each piece's box in points.
struct Drawn {
    /// The screen's rect on the canvas.
    rect: Rect,
    /// Design pixels per point.
    design_per_point: f32,
    /// Each matched piece: its place in the file, in the view, and its box.
    pieces: Vec<(Vec<usize>, Vec<usize>, Rect)>,
    /// Pieces made by code: their boxes.
    code: Vec<Rect>,
}

/// The canvas tab.
pub(crate) fn canvas_tab(ui: &mut Ui, c: CanvasUi<'_>) -> CanvasOut {
    let mut out = CanvasOut::default();
    let full = ui.max_rect();
    let bar = Rect::from_min_size(full.min, Vec2::new(full.width(), 30.0));
    let area = Rect::from_min_max(Pos2::new(full.min.x, bar.max.y), full.max);
    let mut top = ui.new_child(egui::UiBuilder::new().max_rect(bar.shrink2(Vec2::new(8.0, 3.0))));
    toolbar(&mut top, c.doc, c.readable, &mut out);
    let mut main = ui.new_child(egui::UiBuilder::new().max_rect(area));
    canvas(&mut main, c, &mut out);
    ui.advance_cursor_after_rect(full);
    out
}

/// The preview size, the safe area, snapping, zoom and Open as code.
fn toolbar(
    ui: &mut Ui,
    doc: &mut ScreenDoc,
    readable: &[crate::screen_ties::Readable],
    out: &mut CanvasOut,
) {
    ui.horizontal(|ui| {
        let presets = ui::presets::PRESETS;
        let shown = presets.get(doc.preset).unwrap_or(&presets[0]);
        egui::ComboBox::from_id_salt("canvas preview size")
            .selected_text(format!("{} × {}", shown.size[0], shown.size[1]))
            .show_ui(ui, |ui| {
                for (i, p) in presets.iter().enumerate() {
                    ui.selectable_value(
                        &mut doc.preset,
                        i,
                        format!("{} × {}  {} · {}", p.size[0], p.size[1], p.name, p.shape),
                    );
                }
            })
            .response
            .on_hover_text("The screen size the canvas shows: check the screen on each");
        egui::ComboBox::from_id_salt("canvas text size")
            .selected_text(format!("Text {}%", doc.text_percent))
            .show_ui(ui, |ui| {
                for p in [100u16, 150, 200] {
                    ui.selectable_value(&mut doc.text_percent, p, format!("{p}%"));
                }
            })
            .response
            .on_hover_text(
                "The player's text size: check the screen still fits when players make text bigger",
            );
        ui.toggle_value(&mut doc.safe_area, "Safe area")
            .on_hover_text("A guide 5% in from each edge: keep text and buttons inside it");
        ui.toggle_value(&mut doc.snap, "Snap")
            .on_hover_text("Sizes snap to whole pixels and to other pieces' edges");
        let zoom = match doc.zoom {
            Some(z) => format!("{:.0}%", z * 100.0),
            None => "Fit".into(),
        };
        if ui
            .button(zoom)
            .on_hover_text("Click to fit the canvas; the wheel zooms")
            .clicked()
        {
            doc.zoom = None;
            doc.pan = Vec2::ZERO;
        }
        preview_values(ui, doc, readable);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button("Open as code ‹›")
                .on_hover_text("Opens the screen's file in your code editor")
                .clicked()
            {
                out.open_code = true;
            }
            if doc.behind {
                ui.label(RichText::new("Updating…").color(theme::text_weak()));
            }
        });
    });
}

/// Preview values: other numbers for the components the view reads,
/// tried on the canvas (a full bar, an empty one) and never saved.
fn preview_values(ui: &mut Ui, doc: &mut ScreenDoc, readable: &[crate::screen_ties::Readable]) {
    let reads = doc.reads();
    let read: Vec<_> = readable
        .iter()
        .filter(|r| reads.iter().any(|(c, _)| *c == r.component))
        .collect();
    let tried = !doc.preview.is_empty();
    let label = if tried {
        "Preview values •"
    } else {
        "Preview values"
    };
    ui.add_enabled_ui(!read.is_empty(), |ui| {
        ui.menu_button(label, |ui| {
            ui.label(
                RichText::new("Try other values on the canvas. They're never saved.")
                    .color(theme::text_weak())
                    .small(),
            );
            for r in &read {
                ui.label(RichText::new(&r.component).strong());
                for (field, kind, start) in &r.fields {
                    if *kind != crate::screen_ties::Kind::Number {
                        continue;
                    }
                    let key = (r.component.clone(), field.clone());
                    let mut n = doc.preview.get(&key).copied().unwrap_or(*start);
                    let top = (start.abs() * 2.0).max(100.0);
                    let moved = ui
                        .add(egui::Slider::new(&mut n, 0.0..=top).text(field.as_str()))
                        .changed();
                    if moved {
                        doc.preview.insert(key, n);
                        doc.redraw = true;
                    }
                }
            }
            if ui
                .add_enabled(tried, egui::Button::new("Back to the starting values"))
                .clicked()
            {
                doc.preview.clear();
                doc.redraw = true;
            }
        })
        .response
        .on_hover_text(if read.is_empty() {
            "Tie a piece to a component in Properties, and its values can be tried here"
        } else {
            "Try other values for what the screen reads"
        });
    });
}

/// The screen, the guides, and picking, dragging and sizing.
fn canvas(ui: &mut Ui, c: CanvasUi<'_>, out: &mut CanvasOut) {
    let CanvasUi {
        doc,
        renderer,
        fonts,
        pictures,
        accent,
        picture_files,
        readable: _,
    } = c;
    let area = ui.max_rect();
    let response = ui.allocate_rect(area, Sense::click_and_drag());
    out.hovered = response.hovered() || response.dragged();
    let painter = ui.painter_at(area);
    painter.rect_filled(area, 0.0, theme::backdrop::horizon());
    let preset = ui::presets::PRESETS
        .get(doc.preset)
        .copied()
        .unwrap_or(ui::presets::PRESETS[0]);
    let [pw, ph] = preset.size.map(|v| v as f32);
    // Zoom and move.
    let fit = ((area.width() - 40.0) / pw)
        .min((area.height() - 40.0) / ph)
        .max(0.01);
    let zoom = doc.zoom.unwrap_or(fit);
    if response.hovered() {
        let wheel = ui.input(|i| i.smooth_scroll_delta.y);
        if wheel != 0.0 {
            let new = (zoom * (1.0 + wheel * 0.002)).clamp(0.05, 8.0);
            if let Some(at) = response.hover_pos() {
                let from = at - area.center() - doc.pan;
                doc.pan -= from * (new / zoom - 1.0);
            }
            doc.zoom = Some(new);
        }
    }
    if response.dragged_by(egui::PointerButton::Middle) {
        doc.pan += response.drag_delta();
    }
    let size = Vec2::new(pw, ph) * zoom;
    let rect = Rect::from_center_size(area.center() + doc.pan, size);
    let Some(view) = doc.view.clone() else {
        let why = doc
            .view_problem
            .clone()
            .unwrap_or_else(|| "Reading the screen…".into());
        crate::panels::placeholder(ui, &why);
        return;
    };
    // The view as drawn: hidden pieces see-through.
    let matches = doc.matches();
    let mut drawn_view = view.clone();
    for (file, seen) in &matches {
        if doc.hidden.contains(file)
            && let Some(node) = node_at_mut(&mut drawn_view, seen)
        {
            node.styles
                .push(("opacity".into(), ui::view::Style::Number(0.0)));
        }
    }
    let ppp = ui.ctx().pixels_per_point();
    let size_px = [
        (size.x * ppp).round().clamp(1.0, 8192.0) as u32,
        (size.y * ppp).round().clamp(1.0, 8192.0) as u32,
    ];
    let screen = Arc::new(ui::Screen {
        name: doc.screen.clone(),
        design: ui::view::design_of(&drawn_view),
        scale: ui::ScaleRule::Grow,
        elements: Vec::new(),
        view: Some(drawn_view),
        looks: ui::view::Looks {
            text_percent: doc.text_percent,
            ..Default::default()
        },
        pictures,
        ui_size: 1.0,
        look: ui::ScreenLook::default(),
    });
    let image = canvas_image(size_px, Arc::clone(&screen));
    match renderer.render(&image) {
        Ok(texture) => {
            painter.image(
                texture,
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        Err(why) => {
            painter.rect_filled(rect, 0.0, Color32::from_gray(40));
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("The screen can't be drawn: {why}"),
                egui::FontId::proportional(13.0),
                theme::text_weak(),
            );
        }
    }
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, theme::line()),
        egui::StrokeKind::Outside,
    );
    if doc.safe_area {
        let inset = Vec2::new(rect.width(), rect.height()) * SAFE;
        dashed(&painter, rect.shrink2(inset), Color32::from_white_alpha(60));
    }
    // Each piece's box, in points.
    let boxes = ui::layout::boxes(&screen, size_px, None, fonts);
    let to_points = |r: [f32; 4]| {
        Rect::from_min_max(
            rect.min + Vec2::new(r[0], r[1]) / ppp,
            rect.min + Vec2::new(r[2], r[3]) / ppp,
        )
    };
    let mut drawn = Drawn {
        rect,
        design_per_point: screen.design[1] / rect.height().max(1.0),
        pieces: Vec::new(),
        code: Vec::new(),
    };
    for (path, r) in &boxes {
        if path.is_empty() {
            continue;
        }
        match matches.iter().find(|(_, seen)| seen == path) {
            Some((file, seen)) => drawn
                .pieces
                .push((file.clone(), seen.clone(), to_points(*r))),
            None => drawn.code.push(to_points(*r)),
        }
    }
    // While the tool ring is open, a click is for it alone.
    let ring_open = ui.data(|d| {
        d.get_temp::<Pos2>(egui::Id::new("canvas tool ring"))
            .is_some()
    });
    if !ring_open {
        if doc.tool == crate::canvas_tools::CanvasTool::Select {
            interact(ui, &response, doc, &drawn, accent, out);
        } else {
            draw_with_tool(ui, &response, doc, &drawn, &painter, accent, out);
        }
    }
    tools_bar(ui, area, doc, picture_files, accent);
    tool_keys(ui, &response, doc);
    // Outlines: what's under the pointer, and what's picked with handles.
    let hover = response
        .hover_pos()
        .and_then(|p| piece_at(doc, &drawn, p))
        .map(|i| drawn.pieces[i].2);
    if let Some(r) = hover {
        painter.rect_stroke(
            r,
            0.0,
            Stroke::new(1.0, accent.gamma_multiply(0.6)),
            egui::StrokeKind::Outside,
        );
    }
    for r in &drawn.code {
        if response.hover_pos().is_some_and(|p| r.contains(p)) {
            painter.rect_stroke(
                *r,
                0.0,
                Stroke::new(1.0, Color32::from_rgb(255, 216, 77)),
                egui::StrokeKind::Outside,
            );
            badge(&painter, *r, "code");
        }
    }
    let picked: Vec<Rect> = drawn
        .pieces
        .iter()
        .filter(|(f, _, _)| doc.picked.contains(f))
        .map(|(_, _, r)| *r)
        .collect();
    for r in &picked {
        painter.rect_stroke(*r, 0.0, Stroke::new(2.0, accent), egui::StrokeKind::Outside);
    }
    if picked.len() == 1 {
        for h in handles(picked[0]) {
            painter.rect_filled(
                Rect::from_center_size(h.0, Vec2::splat(7.0)),
                1.0,
                Color32::WHITE,
            );
            painter.rect_stroke(
                Rect::from_center_size(h.0, Vec2::splat(7.0)),
                1.0,
                Stroke::new(1.0, accent),
                egui::StrokeKind::Inside,
            );
        }
    }
    // What a drag is doing.
    let drag: Option<Drag> = ui
        .data(|d| d.get_temp::<Option<Drag>>(egui::Id::new("canvas drag")))
        .flatten();
    if let Some(at) = response.interact_pointer_pos() {
        match drag {
            Some(Drag::Box(from)) => {
                let r = Rect::from_two_pos(from, at);
                painter.rect_filled(r, 0.0, accent.gamma_multiply(0.12));
                painter.rect_stroke(r, 0.0, Stroke::new(1.0, accent), egui::StrokeKind::Inside);
            }
            Some(Drag::Move) => {
                if let Some(drop) = drop_at(doc, &drawn, at) {
                    painter.line_segment(drop.line, Stroke::new(3.0, accent));
                    painter.rect_stroke(
                        drop.container,
                        0.0,
                        Stroke::new(1.0, accent.gamma_multiply(0.5)),
                        egui::StrokeKind::Outside,
                    );
                }
            }
            Some(Drag::Size { .. }) | None => {}
        }
    }
    if let Some(p) = response.hover_pos()
        && drawn.code.iter().any(|r| r.contains(p))
        && hover.is_none()
    {
        response.on_hover_text(
            "Made by code (a loop or a question): drawn as the game's starting values make it. \
             Edit it as code.",
        );
    }
}

/// The renderer's view for the canvas: the screen over a plain backdrop.
fn canvas_image(size_px: [u32; 2], screen: Arc<ui::Screen>) -> ViewportView {
    ViewportView {
        size_px,
        view_projection: glam::Mat4::IDENTITY,
        camera_position: glam::Vec3::ZERO,
        focus: glam::Vec3::ZERO,
        sky_top: [0.08, 0.09, 0.11],
        sky_horizon: [0.13, 0.15, 0.18],
        ground: [0.13, 0.15, 0.18],
        models: Vec::new(),
        look: crate::look::Look::default(),
        outline_px: 0.0,
        sun: None,
        lamps: Vec::new(),
        environment: area::builtin::Environment::default(),
        hour: 12.0,
        pick: None,
        image: CANVAS_IMAGE,
        editor_lines: false,
        kind: crate::camera::ViewKind::Perspective,
        bolts: Arc::default(),
        grid_step: 1.0,
        sprites: Arc::default(),
        tile_pictures: Arc::default(),
        pixel: None,
        layers: Vec::new(),
        ui: vec![screen],
    }
}

/// A dashed rectangle.
fn dashed(painter: &egui::Painter, r: Rect, colour: Color32) {
    let stroke = Stroke::new(1.0, colour);
    for (a, b) in [
        (r.left_top(), r.right_top()),
        (r.right_top(), r.right_bottom()),
        (r.right_bottom(), r.left_bottom()),
        (r.left_bottom(), r.left_top()),
    ] {
        painter.extend(egui::Shape::dashed_line(&[a, b], stroke, 6.0, 4.0));
    }
}

/// A small label on a box's corner.
fn badge(painter: &egui::Painter, r: Rect, text: &str) {
    let at = r.left_top() + Vec2::new(0.0, -15.0);
    let galley = painter.layout_no_wrap(
        text.into(),
        egui::FontId::proportional(10.0),
        Color32::BLACK,
    );
    let bg = Rect::from_min_size(at, galley.size() + Vec2::new(8.0, 2.0));
    painter.rect_filled(bg, 3.0, Color32::from_rgb(255, 216, 77));
    painter.galley(at + Vec2::new(4.0, 1.0), galley, Color32::BLACK);
}

/// A picked box's handles: right, bottom and the corner, with which edges
/// each moves.
fn handles(r: Rect) -> [(Pos2, bool, bool); 3] {
    [
        (Pos2::new(r.right(), r.center().y), true, false),
        (Pos2::new(r.center().x, r.bottom()), false, true),
        (r.right_bottom(), true, true),
    ]
}

/// The deepest piece under `p` that can be picked (not hidden or locked).
fn piece_at(doc: &ScreenDoc, drawn: &Drawn, p: Pos2) -> Option<usize> {
    drawn
        .pieces
        .iter()
        .enumerate()
        .filter(|(_, (file, _, r))| {
            r.contains(p) && !doc.hidden.contains(file) && !doc.locked.contains(file)
        })
        .max_by_key(|(_, (file, _, r))| (file.len(), -(r.area() as i64)))
        .map(|(i, _)| i)
}

/// Where a moved piece would land: the container, its place in the file
/// and the line to draw.
struct Drop {
    into: Vec<usize>,
    index: usize,
    line: [Pos2; 2],
    container: Rect,
}

/// Where the picked piece would land if dropped at `p`.
fn drop_at(doc: &ScreenDoc, drawn: &Drawn, p: Pos2) -> Option<Drop> {
    place_at(doc, drawn, p, Some(doc.picked.first()?))
}

/// Where a piece goes at `p`: the deepest container under it (not
/// `moving` or inside it), and its place among that container's pieces.
fn place_at(doc: &ScreenDoc, drawn: &Drawn, p: Pos2, moving: Option<&Vec<usize>>) -> Option<Drop> {
    let none = Vec::new();
    let moving = moving.unwrap_or(&none);
    let block = doc.block()?;
    // The deepest container under the pointer that isn't the piece or
    // inside it; the view itself otherwise.
    let container = drawn
        .pieces
        .iter()
        .filter(|(file, _, r)| {
            r.contains(p)
                && (moving.is_empty() || !file.starts_with(moving))
                && block.at(file).is_some_and(|e| e.children.is_some())
        })
        .max_by_key(|(file, _, _)| file.len());
    let (into, rect, kind) = match container {
        Some((file, _, r)) => (file.clone(), *r, block.at(file)?.kind.clone()),
        None => (Vec::new(), drawn.rect, "column".to_string()),
    };
    let target = block.block_at(&into)?;
    let across = kind == "row";
    // Its children's boxes, by their place in the file.
    let children: Vec<(usize, Rect)> = drawn
        .pieces
        .iter()
        .filter(|(file, _, _)| file.len() == into.len() + 1 && file.starts_with(&into))
        .map(|(file, _, r)| (file[into.len()], *r))
        .collect();
    let after = children.iter().find(|(_, r)| {
        if across {
            p.x < r.center().x
        } else {
            p.y < r.center().y
        }
    });
    let (index, line) = match after {
        Some((i, r)) => (
            *i,
            if across {
                [
                    Pos2::new(r.left() - 2.0, r.top()),
                    Pos2::new(r.left() - 2.0, r.bottom()),
                ]
            } else {
                [
                    Pos2::new(r.left(), r.top() - 2.0),
                    Pos2::new(r.right(), r.top() - 2.0),
                ]
            },
        ),
        None => {
            let last = children.last().map(|(_, r)| *r);
            let line = match last {
                Some(r) if across => [
                    Pos2::new(r.right() + 2.0, r.top()),
                    Pos2::new(r.right() + 2.0, r.bottom()),
                ],
                Some(r) => [
                    Pos2::new(r.left(), r.bottom() + 2.0),
                    Pos2::new(r.right(), r.bottom() + 2.0),
                ],
                None => [
                    Pos2::new(rect.left() + 4.0, rect.center().y),
                    Pos2::new(rect.right() - 4.0, rect.center().y),
                ],
            };
            (target.pieces.len(), line)
        }
    };
    Some(Drop {
        into,
        index,
        line,
        container: rect,
    })
}

/// The edges near `x` (or `y`) of others' boxes, for snapping, in points.
fn snapped(value: f32, edges: &[f32], within: f32) -> f32 {
    edges
        .iter()
        .copied()
        .filter(|e| (e - value).abs() <= within)
        .min_by(|a, b| (a - value).abs().total_cmp(&(b - value).abs()))
        .unwrap_or(value)
}

/// Clicks, Shift-clicks, box picks, moves, sizes and keys.
fn interact(
    ui: &mut Ui,
    response: &egui::Response,
    doc: &mut ScreenDoc,
    drawn: &Drawn,
    _accent: Color32,
    out: &mut CanvasOut,
) {
    let id = egui::Id::new("canvas drag");
    let shift = ui.input(|i| i.modifiers.shift);
    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(at) = response.interact_pointer_pos()
    {
        let start = at - response.drag_delta();
        let picked_box = (doc.picked.len() == 1)
            .then(|| drawn.pieces.iter().find(|(f, _, _)| *f == doc.picked[0]))
            .flatten()
            .map(|(_, _, r)| *r);
        let on_handle = picked_box.and_then(|r| {
            handles(r)
                .into_iter()
                .find(|(h, _, _)| h.distance(start) <= HANDLE)
                .map(|(_, right, bottom)| Drag::Size { right, bottom })
        });
        let drag = match on_handle {
            Some(d) => Some(d),
            None => match piece_at(doc, drawn, start) {
                Some(i) => {
                    // The piece dragged is the one moved: first of those
                    // picked (or the only one, if it wasn't picked).
                    let file = drawn.pieces[i].0.clone();
                    match doc.picked.iter().position(|p| *p == file) {
                        Some(k) => {
                            let it = doc.picked.remove(k);
                            doc.picked.insert(0, it);
                        }
                        None => doc.picked = vec![file],
                    }
                    Some(Drag::Move)
                }
                None => Some(Drag::Box(start)),
            },
        };
        ui.data_mut(|d| d.insert_temp(id, drag));
    }
    let drag: Option<Option<Drag>> = ui.data(|d| d.get_temp(id));
    let drag = drag.flatten();
    if response.drag_stopped_by(egui::PointerButton::Primary) {
        ui.data_mut(|d| d.remove::<Option<Drag>>(id));
        let Some(at) = response.interact_pointer_pos() else {
            return;
        };
        match drag {
            Some(Drag::Box(from)) => {
                let r = Rect::from_two_pos(from, at);
                let inside: Vec<Vec<usize>> = drawn
                    .pieces
                    .iter()
                    .filter(|(f, _, b)| {
                        r.contains_rect(*b) && !doc.hidden.contains(f) && !doc.locked.contains(f)
                    })
                    .map(|(f, _, _)| f.clone())
                    .collect();
                // The outermost of those inside.
                let outer: Vec<Vec<usize>> = inside
                    .iter()
                    .filter(|p| !inside.iter().any(|q| q != *p && p.starts_with(q)))
                    .cloned()
                    .collect();
                if shift {
                    for p in outer {
                        if !doc.picked.contains(&p) {
                            doc.picked.push(p);
                        }
                    }
                } else {
                    doc.picked = outer;
                }
            }
            Some(Drag::Move) => {
                let parent_canvas = doc.picked.first().and_then(|p| {
                    let parent = &p[..p.len().saturating_sub(1)];
                    doc.block()?.at(parent).map(|e| e.kind == "canvas")
                });
                if parent_canvas == Some(true) {
                    // On a canvas: moved by its x and y (the one dragged).
                    doc.picked.truncate(1);
                    let total = at - drag_origin(ui, response, at);
                    let path = doc.picked[0].clone();
                    let (x, y) = doc
                        .piece(&path)
                        .map(|e| {
                            let num = |n: &str| {
                                e.style_text(&doc.text, n)
                                    .and_then(|v| v.trim().parse::<f32>().ok())
                                    .unwrap_or(0.0)
                            };
                            (num("x"), num("y"))
                        })
                        .unwrap_or((0.0, 0.0));
                    let nx = (x + total.x * drawn.design_per_point).round();
                    let ny = (y + total.y * drawn.design_per_point).round();
                    let a = doc.set_style("x", Some(&format!("{nx}")));
                    let b = doc.set_style("y", Some(&format!("{ny}")));
                    out.changed |= a || b;
                } else if let Some(drop) = drop_at(doc, drawn, at) {
                    out.changed |= doc.move_picked(&drop.into, drop.index);
                }
            }
            Some(Drag::Size { right, bottom }) => {
                let Some((_, _, r)) = drawn
                    .pieces
                    .iter()
                    .find(|(f, _, _)| Some(f) == doc.picked.first())
                else {
                    return;
                };
                let edges_x: Vec<f32> = drawn
                    .pieces
                    .iter()
                    .flat_map(|(_, _, b)| [b.left(), b.right()])
                    .collect();
                let edges_y: Vec<f32> = drawn
                    .pieces
                    .iter()
                    .flat_map(|(_, _, b)| [b.top(), b.bottom()])
                    .collect();
                let within = SNAP / drawn.design_per_point.max(0.001);
                let mut x = at.x;
                let mut y = at.y;
                if doc.snap {
                    x = snapped(x, &edges_x, within);
                    y = snapped(y, &edges_y, within);
                }
                let mut changed = false;
                if right {
                    let w = ((x - r.left()).max(1.0) * drawn.design_per_point).round();
                    changed |= doc.set_style("width", Some(&format!("{w}")));
                }
                if bottom {
                    let h = ((y - r.top()).max(1.0) * drawn.design_per_point).round();
                    changed |= doc.set_style("height", Some(&format!("{h}")));
                }
                out.changed |= changed;
            }
            None => {}
        }
        return;
    }
    if response.clicked()
        && let Some(at) = response.interact_pointer_pos()
    {
        match piece_at(doc, drawn, at) {
            Some(i) => {
                let file = drawn.pieces[i].0.clone();
                if shift {
                    if let Some(k) = doc.picked.iter().position(|p| *p == file) {
                        doc.picked.remove(k);
                    } else {
                        doc.picked.push(file);
                    }
                } else {
                    doc.picked = vec![file];
                }
            }
            None if !shift => doc.picked.clear(),
            None => {}
        }
    }
    if response.double_clicked()
        && piece_at(
            doc,
            drawn,
            response.interact_pointer_pos().unwrap_or_default(),
        )
        .is_none()
    {
        doc.zoom = None;
        doc.pan = Vec2::ZERO;
    }
    if out.hovered && !ui.ctx().egui_wants_keyboard_input() {
        let (delete, duplicate, escape) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace),
                i.modifiers.command && i.key_pressed(egui::Key::D),
                i.key_pressed(egui::Key::Escape),
            )
        });
        if delete && !doc.picked.is_empty() {
            out.changed |= doc.delete_picked();
        }
        if duplicate && !doc.picked.is_empty() {
            out.changed |= doc.duplicate_picked();
        }
        if escape {
            doc.picked.clear();
        }
    }
}

/// The screen's design pixel at point `p`, or, with `within`, relative to
/// that piece's box.
fn design_at(drawn: &Drawn, p: Pos2, within: Option<Rect>) -> [f32; 2] {
    let origin = within.map_or(drawn.rect.min, |r| r.min);
    let d = (p - origin) * drawn.design_per_point;
    [d.x, d.y]
}

/// The canvas piece's box a new piece goes into, when it does.
fn canvas_box(doc: &ScreenDoc, drawn: &Drawn, drop: &Drop) -> Option<Rect> {
    let piece = doc.block()?.at(&drop.into)?;
    if piece.kind != "canvas" {
        return None;
    }
    drawn
        .pieces
        .iter()
        .find(|(f, _, _)| *f == drop.into)
        .map(|(_, _, r)| *r)
}

/// Drawing with a tool: a drag for a box, an ellipse or a line, clicks
/// for words, a picture, a button or the pen's corners.
fn draw_with_tool(
    ui: &mut Ui,
    response: &egui::Response,
    doc: &mut ScreenDoc,
    drawn: &Drawn,
    painter: &egui::Painter,
    accent: Color32,
    out: &mut CanvasOut,
) {
    use crate::canvas_tools::{CanvasTool, piece_text};
    let tool = doc.tool;
    let id = egui::Id::new("canvas tool drag");
    let stroke = Stroke::new(2.0, accent);
    if tool.drags() {
        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some(at) = response.interact_pointer_pos()
        {
            let start = drag_origin(ui, response, at);
            ui.data_mut(|d| d.insert_temp(id, start));
        }
        let start: Option<Pos2> = ui.data(|d| d.get_temp(id));
        if let (Some(start), Some(at)) = (start, response.interact_pointer_pos()) {
            let r = Rect::from_two_pos(start, at);
            match tool {
                CanvasTool::Box => {
                    painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Inside);
                }
                CanvasTool::Ellipse => {
                    painter.circle_stroke(r.center(), r.width().min(r.height()) / 2.0, stroke);
                }
                _ => {
                    painter.line_segment([start, at], stroke);
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                ui.data_mut(|d| d.remove::<Pos2>(id));
                if let Some(drop) = place_at(doc, drawn, start, None) {
                    let within = canvas_box(doc, drawn, &drop);
                    let a = design_at(drawn, start, within);
                    let b = design_at(drawn, at, within);
                    if let Some(text) = piece_text(tool, a, b, within.is_some(), None) {
                        out.changed |= doc.insert(&drop.into, drop.index, &text);
                    }
                }
            }
        }
        return;
    }
    // Clicks.
    if tool == CanvasTool::Pen {
        for w in doc.pen.windows(2) {
            let p = |d: [f32; 2]| drawn.rect.min + Vec2::new(d[0], d[1]) / drawn.design_per_point;
            painter.line_segment([p(w[0]), p(w[1])], stroke);
        }
        if let (Some(last), Some(at)) = (doc.pen.last(), response.hover_pos()) {
            let p = drawn.rect.min + Vec2::new(last[0], last[1]) / drawn.design_per_point;
            painter.line_segment([p, at], Stroke::new(1.0, accent));
        }
        let finish = response.double_clicked()
            || (response.hovered()
                && !ui.ctx().egui_wants_keyboard_input()
                && ui.input(|i| i.key_pressed(egui::Key::Enter)));
        if finish {
            out.changed |= finish_pen(doc, drawn);
        } else if response.clicked()
            && let Some(at) = response.interact_pointer_pos()
        {
            doc.pen.push(design_at(drawn, at, None));
        }
        return;
    }
    if response.clicked()
        && let Some(at) = response.interact_pointer_pos()
        && let Some(drop) = place_at(doc, drawn, at, None)
    {
        let within = canvas_box(doc, drawn, &drop);
        let a = design_at(drawn, at, within);
        match piece_text(tool, a, a, within.is_some(), doc.picture.as_deref()) {
            Some(text) => out.changed |= doc.insert(&drop.into, drop.index, &text),
            None if tool == CanvasTool::Image => {
                out.said = Some("Choose a picture beside the tools first.".into());
            }
            None => {}
        }
    }
    if let Some(at) = response.hover_pos()
        && let Some(drop) = place_at(doc, drawn, at, None)
        && canvas_box(doc, drawn, &drop).is_none()
    {
        painter.line_segment(drop.line, Stroke::new(2.0, accent.gamma_multiply(0.7)));
    }
}

/// Ends the pen's line: its corners become lines, in the canvas under
/// the first or in a new one.
fn finish_pen(doc: &mut ScreenDoc, drawn: &Drawn) -> bool {
    let points = std::mem::take(&mut doc.pen);
    let Some(first) = points.first() else {
        return false;
    };
    let p0 = drawn.rect.min + Vec2::new(first[0], first[1]) / drawn.design_per_point;
    let Some(drop) = place_at(doc, drawn, p0, None) else {
        return false;
    };
    let within = canvas_box(doc, drawn, &drop);
    let shifted: Vec<[f32; 2]> = match within {
        Some(r) => {
            let o = (r.min - drawn.rect.min) * drawn.design_per_point;
            points.iter().map(|p| [p[0] - o.x, p[1] - o.y]).collect()
        }
        None => points,
    };
    match crate::canvas_tools::pen_text(&shifted, within.is_some()) {
        Some(text) => doc.insert(&drop.into, drop.index, &text),
        None => false,
    }
}

/// The tools along the canvas's bottom, and the picture for Image.
fn tools_bar(ui: &mut Ui, area: Rect, doc: &mut ScreenDoc, pictures: &[String], accent: Color32) {
    use crate::canvas_tools::{CanvasTool, paint_tool};
    let n = CanvasTool::ALL.len() as f32;
    let width = n * 30.0
        + 8.0
        + if doc.tool == CanvasTool::Image {
            220.0
        } else {
            0.0
        };
    let bar = Rect::from_center_size(
        Pos2::new(area.center().x, area.bottom() - 26.0),
        Vec2::new(width, 34.0),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(bar));
    egui::Frame::new()
        .fill(theme::panel())
        .corner_radius(6.0)
        .inner_margin(4.0)
        .show(&mut child, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for tool in CanvasTool::ALL {
                    let on = doc.tool == tool;
                    let (rect, r) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::click());
                    let fill = if on {
                        accent
                    } else if r.hovered() {
                        theme::hover()
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(rect, 4.0, fill);
                    paint_tool(
                        ui.painter(),
                        rect,
                        tool,
                        if on { Color32::WHITE } else { theme::text() },
                    );
                    r.widget_info(|| {
                        egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, tool.name())
                    });
                    if r.on_hover_text(tool.hint()).clicked() {
                        doc.tool = tool;
                        doc.pen.clear();
                    }
                }
                if doc.tool == CanvasTool::Image {
                    egui::ComboBox::from_id_salt("canvas picture")
                        .width(200.0)
                        .selected_text(
                            doc.picture
                                .clone()
                                .unwrap_or_else(|| "Choose a picture".into()),
                        )
                        .show_ui(ui, |ui| {
                            for p in pictures {
                                ui.selectable_value(&mut doc.picture, Some(p.clone()), p);
                            }
                            if pictures.is_empty() {
                                ui.label("No pictures in the project yet.");
                            }
                        });
                }
            });
        });
}

/// Keys 1 to 7 pick tools, E opens them in a ring, Esc goes back to
/// picking.
fn tool_keys(ui: &mut Ui, response: &egui::Response, doc: &mut ScreenDoc) {
    use crate::canvas_tools::CanvasTool;
    let ring = egui::Id::new("canvas tool ring");
    let mut ring_at: Option<Pos2> = ui.data(|d| d.get_temp(ring));
    if (response.hovered() || ring_at.is_some()) && !ui.ctx().egui_wants_keyboard_input() {
        for tool in CanvasTool::ALL {
            if let Some(key) = tool.key()
                && ui.input(|i| i.key_pressed(key) && i.modifiers.is_none())
            {
                doc.tool = tool;
                doc.pen.clear();
                ring_at = None;
            }
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            if doc.tool == CanvasTool::Pen && !doc.pen.is_empty() {
                doc.pen.clear();
            } else {
                doc.tool = CanvasTool::Select;
            }
            ring_at = None;
        }
        if response.hovered() && ui.input(|i| i.key_pressed(egui::Key::E) && i.modifiers.is_none())
        {
            ring_at = response.hover_pos();
        }
    }
    // The ring: every tool round the pointer; a click picks one.
    if let Some(c) = ring_at {
        let painter = ui
            .ctx()
            .layer_painter(egui::LayerId::new(egui::Order::Foreground, ring));
        let n = CanvasTool::ALL.len() as f32;
        let mut chosen = None;
        for (i, tool) in CanvasTool::ALL.into_iter().enumerate() {
            let a = std::f32::consts::TAU * (i as f32 / n) - std::f32::consts::FRAC_PI_2;
            let p = c + Vec2::new(a.cos(), a.sin()) * 52.0;
            let r = Rect::from_center_size(p, Vec2::splat(30.0));
            let hover = ui
                .input(|i| i.pointer.hover_pos())
                .is_some_and(|h| r.contains(h));
            painter.circle_filled(
                p,
                17.0,
                if hover || doc.tool == tool {
                    theme::hover()
                } else {
                    theme::panel()
                },
            );
            crate::canvas_tools::paint_tool(&painter, r, tool, theme::text());
            if hover && ui.input(|i| i.pointer.primary_clicked()) {
                chosen = Some(tool);
            }
        }
        if let Some(tool) = chosen {
            doc.tool = tool;
            doc.pen.clear();
            ring_at = None;
        } else if ui.input(|i| i.pointer.primary_clicked()) {
            ring_at = None;
        }
    }
    match ring_at {
        Some(p) => ui.data_mut(|d| {
            d.insert_temp(ring, p);
        }),
        None => ui.data_mut(|d| {
            d.remove::<Pos2>(ring);
        }),
    }
}

/// Where the drag that ends at `at` began.
fn drag_origin(ui: &Ui, response: &egui::Response, at: Pos2) -> Pos2 {
    ui.input(|i| i.pointer.press_origin())
        .unwrap_or(at - response.drag_delta())
}

#[cfg(test)]
#[path = "canvas_view_tests.rs"]
mod tests;
