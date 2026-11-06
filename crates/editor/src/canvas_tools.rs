//! The canvas's drawing tools (decision 0089, M5): box, ellipse, line,
//! pen, text, image and button. Each puts a new piece into the screen's
//! file where it's drawn: into the `canvas` under the pointer at its x and
//! y, or among the pieces of the column or row under it, in their order.
//! Keys 1 to 7 pick the tools while the canvas is under the pointer, E
//! opens them in a ring at the pointer, and Esc goes back to picking.

use crate::screen_doc::ScreenDoc;

/// A tool on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub(crate) enum CanvasTool {
    /// Picking, moving and sizing pieces.
    #[default]
    Select,
    /// Drag a box.
    Box,
    /// Drag an ellipse (a circle, as big as the drag's shorter side).
    Ellipse,
    /// Drag a line.
    Line,
    /// Click corners; Enter or a double-click ends the line.
    Pen,
    /// Click to put words.
    Text,
    /// Click to put the picture chosen in the bar.
    Image,
    /// Click to put a button.
    Button,
}

impl CanvasTool {
    /// Every tool, in the bar's order.
    pub(crate) const ALL: [Self; 8] = [
        Self::Select,
        Self::Box,
        Self::Ellipse,
        Self::Line,
        Self::Pen,
        Self::Text,
        Self::Image,
        Self::Button,
    ];

    /// Its key (1 to 7), if it has one.
    pub(crate) fn key(self) -> Option<egui::Key> {
        use egui::Key;
        Some(match self {
            Self::Select => Key::Num1,
            Self::Box => Key::Num2,
            Self::Ellipse => Key::Num3,
            Self::Line => Key::Num4,
            Self::Pen => Key::Num5,
            Self::Text => Key::Num6,
            Self::Image => Key::Num7,
            Self::Button => return None,
        })
    }

    /// Its name on the bar.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Box => "Box",
            Self::Ellipse => "Ellipse",
            Self::Line => "Line",
            Self::Pen => "Pen",
            Self::Text => "Text",
            Self::Image => "Image",
            Self::Button => "Button",
        }
    }

    /// What it does, on hover.
    pub(crate) fn hint(self) -> &'static str {
        match self {
            Self::Select => "Pick, move and size pieces (1)",
            Self::Box => "Drag a box (2)",
            Self::Ellipse => "Drag a circle (3)",
            Self::Line => "Drag a line (4)",
            Self::Pen => "Click corners; Enter or double-click to end (5)",
            Self::Text => "Click to put words (6)",
            Self::Image => "Click to put the picture chosen beside the tools (7)",
            Self::Button => "Click to put a button",
        }
    }

    /// Whether it's drawn by dragging (otherwise by clicking).
    pub(crate) fn drags(self) -> bool {
        matches!(self, Self::Box | Self::Ellipse | Self::Line)
    }
}

/// A number for a style: whole when it is, else to two places.
pub(crate) fn num(v: f32) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r.fract() == 0.0 {
        format!("{}", r as i64)
    } else {
        format!("{r}")
    }
}

/// The new piece's text for `tool`, from design point `a` to `b` (rounded
/// to whole pixels; a
/// click has them the same), with `on_canvas` when it goes into a
/// `canvas` at those places (relative to it), and the picture for Image.
/// `None` if there's nothing to draw (a drag too small).
pub(crate) fn piece_text(
    tool: CanvasTool,
    a: [f32; 2],
    b: [f32; 2],
    on_canvas: bool,
    picture: Option<&str>,
) -> Option<String> {
    let (a, b) = (a.map(f32::round), b.map(f32::round));
    let (x, y) = (a[0].min(b[0]), a[1].min(b[1]));
    let (w, h) = ((a[0] - b[0]).abs(), (a[1] - b[1]).abs());
    let at = |more: &str| {
        if on_canvas {
            format!("x {}, y {}{more}", num(x), num(y))
        } else {
            more.trim_start_matches(", ").to_string()
        }
    };
    Some(match tool {
        CanvasTool::Select | CanvasTool::Pen => return None,
        CanvasTool::Box => {
            if w < 2.0 || h < 2.0 {
                return None;
            }
            if on_canvas {
                format!(
                    "box [x {}, y {}, width {}, height {}];",
                    num(x),
                    num(y),
                    num(w),
                    num(h)
                )
            } else {
                // Shapes are drawn in a canvas: one just round it.
                format!(
                    "canvas [width {w}, height {h}] {{\n    box [width {w}, height {h}];\n}}",
                    w = num(w),
                    h = num(h)
                )
            }
        }
        CanvasTool::Ellipse => {
            let r = w.min(h) / 2.0;
            if r < 1.0 {
                return None;
            }
            if on_canvas {
                format!(
                    "circle [x {}, y {}, radius {}];",
                    num(x + w / 2.0),
                    num(y + h / 2.0),
                    num(r)
                )
            } else {
                format!(
                    "canvas [width {d}, height {d}] {{\n    circle [x {r}, y {r}, radius {r}];\n}}",
                    d = num(r * 2.0),
                    r = num(r)
                )
            }
        }
        CanvasTool::Line => {
            if w < 2.0 && h < 2.0 {
                return None;
            }
            if on_canvas {
                format!(
                    "line [x {}, y {}, toX {}, toY {}];",
                    num(a[0]),
                    num(a[1]),
                    num(b[0]),
                    num(b[1])
                )
            } else {
                // A line needs a canvas to sit in: one just round it.
                let (lx, ly) = (a[0] - x, a[1] - y);
                let (tx, ty) = (b[0] - x, b[1] - y);
                format!(
                    "canvas [width {}, height {}] {{\n    line [x {}, y {}, toX {}, toY {}];\n}}",
                    num(w.max(2.0)),
                    num(h.max(2.0)),
                    num(lx),
                    num(ly),
                    num(tx),
                    num(ty)
                )
            }
        }
        CanvasTool::Text => format!("text \"Text\" [{}];", at(", size 24")),
        CanvasTool::Image => {
            let picture = picture?;
            format!(
                "image {} [{}];",
                ui::source::quoted(picture),
                at(", width 128")
            )
        }
        CanvasTool::Button => {
            if on_canvas {
                format!("button \"Button\" [{}];", at(""))
            } else {
                "button \"Button\";".to_string()
            }
        }
    })
}

/// The pen's line through `points` (design pixels, rounded to whole ones): a line per corner to
/// corner, in the canvas they're on (relative to it), or in a new canvas
/// just round them.
pub(crate) fn pen_text(points: &[[f32; 2]], on_canvas: bool) -> Option<String> {
    if points.len() < 2 {
        return None;
    }
    let points: Vec<[f32; 2]> = points.iter().map(|p| p.map(f32::round)).collect();
    let (x0, y0) = points
        .iter()
        .fold((f32::MAX, f32::MAX), |(x, y), p| (x.min(p[0]), y.min(p[1])));
    let (x1, y1) = points
        .iter()
        .fold((f32::MIN, f32::MIN), |(x, y), p| (x.max(p[0]), y.max(p[1])));
    let (ox, oy) = if on_canvas { (0.0, 0.0) } else { (x0, y0) };
    let lines: Vec<String> = points
        .windows(2)
        .map(|w| {
            format!(
                "line [x {}, y {}, toX {}, toY {}];",
                num(w[0][0] - ox),
                num(w[0][1] - oy),
                num(w[1][0] - ox),
                num(w[1][1] - oy)
            )
        })
        .collect();
    if on_canvas {
        Some(lines.join("\n"))
    } else {
        Some(format!(
            "canvas [width {}, height {}] {{\n{}\n}}",
            num((x1 - x0).max(2.0)),
            num((y1 - y0).max(2.0)),
            lines
                .iter()
                .map(|l| format!("    {l}"))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }
}

/// Draws `tool`'s icon in `rect`.
pub(crate) fn paint_tool(
    painter: &egui::Painter,
    rect: egui::Rect,
    tool: CanvasTool,
    colour: egui::Color32,
) {
    use egui::{Pos2, Stroke, pos2};
    let r = rect.shrink(rect.width() * 0.22);
    let s = Stroke::new(1.5, colour);
    let at = |x: f32, y: f32| pos2(r.left() + x * r.width(), r.top() + y * r.height());
    match tool {
        CanvasTool::Select => {
            let arrow: Vec<Pos2> = vec![
                at(0.2, 0.0),
                at(0.2, 0.85),
                at(0.42, 0.65),
                at(0.62, 1.0),
                at(0.75, 0.93),
                at(0.56, 0.6),
                at(0.85, 0.6),
            ];
            painter.add(egui::Shape::convex_polygon(arrow, colour, Stroke::NONE));
        }
        CanvasTool::Box => {
            painter.rect_stroke(
                r.shrink2(egui::vec2(0.0, r.height() * 0.15)),
                1.0,
                s,
                egui::StrokeKind::Inside,
            );
        }
        CanvasTool::Ellipse => {
            painter.circle_stroke(r.center(), r.width().min(r.height()) / 2.0, s);
        }
        CanvasTool::Line => {
            painter.line_segment([at(0.0, 1.0), at(1.0, 0.0)], s);
        }
        CanvasTool::Pen => {
            painter.add(egui::Shape::line(
                vec![at(0.0, 0.8), at(0.3, 0.2), at(0.6, 0.75), at(1.0, 0.15)],
                s,
            ));
        }
        CanvasTool::Text => {
            painter.text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                "T",
                egui::FontId::proportional(r.height() * 1.2),
                colour,
            );
        }
        CanvasTool::Image => {
            painter.rect_stroke(r, 1.0, s, egui::StrokeKind::Inside);
            painter.add(egui::Shape::line(
                vec![
                    at(0.0, 0.85),
                    at(0.35, 0.45),
                    at(0.6, 0.7),
                    at(0.8, 0.5),
                    at(1.0, 0.7),
                ],
                s,
            ));
        }
        CanvasTool::Button => {
            painter.rect_stroke(
                r.shrink2(egui::vec2(0.0, r.height() * 0.22)),
                r.height() * 0.3,
                s,
                egui::StrokeKind::Inside,
            );
        }
    }
}

impl ScreenDoc {
    /// Puts `piece_text` (one piece or more, one after another) into the
    /// block at `into` (a piece's place, `[]` for the view) at `index`,
    /// as one undo step, and picks the first piece put in.
    pub(crate) fn insert(&mut self, into: &[usize], index: usize, piece_text: &str) -> bool {
        let Some(block) = self.block().and_then(|b| b.block_at(into)).cloned() else {
            return false;
        };
        let index = index.min(block.pieces.len());
        let text = ui::source::insert_piece(&self.text, &block, index, piece_text);
        let changed = self.change(text);
        if changed {
            self.hidden.clear();
            self.locked.clear();
            let mut at = into.to_vec();
            at.push(index);
            self.picked = vec![at];
            self.behind = true;
        }
        changed
    }
}

#[cfg(test)]
#[path = "canvas_tools_tests.rs"]
mod tests;
