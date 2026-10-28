//! The editor's icons, drawn in code (no image files), so they stay sharp at
//! any size and take the colour they are given: the tools, and the small
//! symbols on tabs and buttons ([`Icon`]).

use crate::viewport::Tool;
use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};
use std::f32::consts::TAU;

/// Draws `tool`'s icon in `rect` with `color`.
pub fn tool_icon(painter: &Painter, rect: Rect, tool: Tool, color: Color32) {
    let side = rect.width().min(rect.height());
    let square = Rect::from_center_size(rect.center(), vec2(side, side));
    // Points given from 0 to 1 across the icon's square.
    let at = |x: f32, y: f32| pos2(square.left() + x * side, square.top() + y * side);
    let stroke = Stroke::new((side / 11.0).max(1.2), color);
    match tool {
        Tool::Select => select(painter, &at, stroke),
        Tool::Move => four_arrows(painter, &at, stroke, 0.45),
        Tool::Rotate => rotate(painter, square, stroke),
        Tool::Scale => scale(painter, &at, stroke),
        Tool::Transform => {
            painter.circle_stroke(square.center(), side * 0.3, stroke);
            four_arrows(painter, &at, stroke, 0.47);
        }
    }
}

/// A mouse pointer.
fn select(painter: &Painter, at: &dyn Fn(f32, f32) -> Pos2, stroke: Stroke) {
    let outline = vec![
        at(0.28, 0.12),
        at(0.78, 0.55),
        at(0.55, 0.58),
        at(0.68, 0.86),
        at(0.58, 0.9),
        at(0.45, 0.62),
        at(0.28, 0.78),
    ];
    painter.add(Shape::closed_line(outline, stroke));
}

/// A cross with an arrowhead on each end, reaching `reach` from the centre.
fn four_arrows(painter: &Painter, at: &dyn Fn(f32, f32) -> Pos2, stroke: Stroke, reach: f32) {
    let (lo, hi) = (0.5 - reach, 0.5 + reach);
    let head = 0.12;
    painter.line_segment([at(0.5, lo), at(0.5, hi)], stroke);
    painter.line_segment([at(lo, 0.5), at(hi, 0.5)], stroke);
    for (tip, side_a, side_b) in [
        (
            at(0.5, lo),
            at(0.5 - head, lo + head),
            at(0.5 + head, lo + head),
        ),
        (
            at(0.5, hi),
            at(0.5 - head, hi - head),
            at(0.5 + head, hi - head),
        ),
        (
            at(lo, 0.5),
            at(lo + head, 0.5 - head),
            at(lo + head, 0.5 + head),
        ),
        (
            at(hi, 0.5),
            at(hi - head, 0.5 - head),
            at(hi - head, 0.5 + head),
        ),
    ] {
        painter.add(Shape::line(vec![side_a, tip, side_b], stroke));
    }
}

/// Three quarters of a circle with an arrowhead.
fn rotate(painter: &Painter, square: Rect, stroke: Stroke) {
    let centre = square.center();
    let radius = square.width() * 0.32;
    let steps = 24;
    let start = -0.15 * TAU;
    let sweep = 0.75 * TAU;
    let arc: Vec<Pos2> = (0..=steps)
        .map(|i| {
            let angle = start + sweep * i as f32 / steps as f32;
            centre + vec2(angle.cos(), angle.sin()) * radius
        })
        .collect();
    let end = arc.last().copied().unwrap_or(centre);
    painter.add(Shape::line(arc, stroke));
    let head = square.width() * 0.13;
    painter.add(Shape::line(
        vec![end + vec2(-head, 0.0), end, end + vec2(0.0, head)],
        stroke,
    ));
}

/// Two corners and a diagonal: growing outwards.
fn scale(painter: &Painter, at: &dyn Fn(f32, f32) -> Pos2, stroke: Stroke) {
    painter.add(Shape::line(
        vec![at(0.15, 0.55), at(0.15, 0.85), at(0.45, 0.85)],
        stroke,
    ));
    painter.add(Shape::line(
        vec![at(0.55, 0.15), at(0.85, 0.15), at(0.85, 0.45)],
        stroke,
    ));
    painter.line_segment([at(0.17, 0.83), at(0.83, 0.17)], stroke);
}

/// The small symbols on tabs and buttons, after the final mockup
/// ([decision 0004](../../../docs/decisions/0004-editor-layout.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// A screen: the Viewport panel.
    Viewport,
    /// A play symbol in a screen: the Game panel.
    Game,
    /// A tree of lines: the Area panel.
    Area,
    /// Stacked sheets: the Layers panel.
    Layers,
    /// Lines of fields: the Properties panel.
    Properties,
    /// A folder: the Library panel.
    Library,
    /// Angle brackets: the Code panel.
    Code,
    /// A prompt: the Console panel.
    Console,
    /// A warning sign: the Problems panel.
    Problems,
    /// Bars of a graph: the Profiler panel.
    Profiler,
    /// Hills: an area document.
    AreaFile,
    /// A filled triangle: Play.
    Play,
    /// Two people: play with others.
    PlayTogether,
    /// A filled square: Stop.
    Stop,
    /// Two bars: Pause.
    Pause,
    /// A triangle against a bar: Step (one tick).
    Step,
    /// A film camera: a Camera entity.
    Camera,
    /// An "i" in a circle: information.
    Info,
    /// "!" in a triangle: a warning.
    Warning,
    /// A cross: an error.
    Error,
    /// A magnifying glass: search.
    Search,
    /// A box seen from a corner: a model.
    Model,
    /// A small sun with rays: a light.
    Light,
    /// An eye with a line through it: hidden in the editor.
    Hidden,
    /// A padlock: can't be picked in the viewport.
    Lock,
    /// A speaker with sound waves: a sound file.
    Sound,
    /// A diamond: an animator (Phase 9, D3b).
    Animator,
}

/// Draws `icon` in `rect` with `color`. Shapes are given on a 24-unit
/// square, as in the mockup.
pub fn icon(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let side = rect.width().min(rect.height());
    let square = Rect::from_center_size(rect.center(), vec2(side, side));
    let unit = side / 24.0;
    let v = |x: f32, y: f32| pos2(square.left() + x * unit, square.top() + y * unit);
    let stroke = Stroke::new((2.0 * unit).max(1.0), color);
    let line = |points: &[(f32, f32)]| {
        painter.add(Shape::line(
            points.iter().map(|&(x, y)| v(x, y)).collect(),
            stroke,
        ));
    };
    let outline = |points: &[(f32, f32)]| {
        painter.add(Shape::closed_line(
            points.iter().map(|&(x, y)| v(x, y)).collect(),
            stroke,
        ));
    };
    let fill = |points: &[(f32, f32)]| {
        painter.add(Shape::convex_polygon(
            points.iter().map(|&(x, y)| v(x, y)).collect(),
            color,
            Stroke::NONE,
        ));
    };
    match icon {
        Icon::Viewport => outline(&[(3.0, 5.0), (21.0, 5.0), (21.0, 19.0), (3.0, 19.0)]),
        Icon::Game => {
            outline(&[(3.0, 5.0), (21.0, 5.0), (21.0, 19.0), (3.0, 19.0)]);
            fill(&[(10.0, 8.5), (16.0, 12.0), (10.0, 15.5)]);
        }
        Icon::Area => {
            line(&[(4.0, 6.0), (20.0, 6.0)]);
            line(&[(8.0, 12.0), (20.0, 12.0)]);
            line(&[(8.0, 18.0), (20.0, 18.0)]);
            line(&[(4.0, 12.0), (4.0, 18.0)]);
        }
        Icon::Layers => {
            outline(&[(12.0, 4.0), (21.0, 9.0), (12.0, 14.0), (3.0, 9.0)]);
            line(&[(3.0, 14.0), (12.0, 19.0), (21.0, 14.0)]);
        }
        Icon::Properties => {
            line(&[(4.0, 6.0), (20.0, 6.0)]);
            line(&[(4.0, 12.0), (20.0, 12.0)]);
            line(&[(4.0, 18.0), (14.0, 18.0)]);
        }
        Icon::Library => outline(&[
            (3.0, 6.0),
            (9.0, 6.0),
            (11.0, 8.0),
            (21.0, 8.0),
            (21.0, 19.0),
            (3.0, 19.0),
        ]),
        Icon::Code => {
            line(&[(8.0, 7.0), (3.0, 12.0), (8.0, 17.0)]);
            line(&[(16.0, 7.0), (21.0, 12.0), (16.0, 17.0)]);
        }
        Icon::Console => {
            line(&[(4.0, 6.0), (10.0, 12.0), (4.0, 18.0)]);
            line(&[(12.0, 18.0), (20.0, 18.0)]);
        }
        Icon::Profiler => {
            line(&[(3.0, 20.0), (21.0, 20.0)]);
            line(&[(6.0, 17.0), (6.0, 12.0)]);
            line(&[(11.0, 17.0), (11.0, 5.0)]);
            line(&[(16.0, 17.0), (16.0, 9.0)]);
        }
        Icon::Problems | Icon::Warning => {
            outline(&[(12.0, 3.0), (22.0, 20.0), (2.0, 20.0)]);
            line(&[(12.0, 9.0), (12.0, 14.0)]);
            painter.circle_filled(v(12.0, 17.0), 1.2 * unit, color);
        }
        Icon::AreaFile => outline(&[
            (3.0, 20.0),
            (8.0, 4.0),
            (12.0, 14.0),
            (15.0, 9.0),
            (21.0, 20.0),
        ]),
        Icon::Animator => outline(&[(12.0, 3.0), (21.0, 12.0), (12.0, 21.0), (3.0, 12.0)]),
        Icon::Play => fill(&[(7.0, 4.0), (20.0, 12.0), (7.0, 20.0)]),
        Icon::PlayTogether => {
            let thin = Stroke::new((1.6 * unit).max(1.0), color);
            painter.circle_stroke(v(8.0, 9.0), 3.0 * unit, thin);
            painter.circle_stroke(v(16.0, 9.0), 3.0 * unit, thin);
            for x in [0.0, 8.0] {
                let body: Vec<Pos2> = (0..=8)
                    .map(|i| {
                        let t = i as f32 / 8.0;
                        let angle = std::f32::consts::PI * (1.0 + t);
                        v(8.0 + x + 5.0 * angle.cos(), 19.0 + 4.0 * angle.sin())
                    })
                    .collect();
                painter.add(Shape::line(body, thin));
            }
        }
        Icon::Stop => fill(&[(5.0, 5.0), (19.0, 5.0), (19.0, 19.0), (5.0, 19.0)]),
        Icon::Pause => {
            fill(&[(6.0, 5.0), (10.0, 5.0), (10.0, 19.0), (6.0, 19.0)]);
            fill(&[(14.0, 5.0), (18.0, 5.0), (18.0, 19.0), (14.0, 19.0)]);
        }
        Icon::Step => {
            fill(&[(5.0, 5.0), (15.0, 12.0), (5.0, 19.0)]);
            fill(&[(16.0, 5.0), (19.0, 5.0), (19.0, 19.0), (16.0, 19.0)]);
        }
        Icon::Camera => {
            outline(&[(3.0, 8.0), (15.0, 8.0), (15.0, 18.0), (3.0, 18.0)]);
            outline(&[(15.0, 11.0), (21.0, 8.0), (21.0, 18.0), (15.0, 15.0)]);
        }
        Icon::Info => {
            painter.circle_stroke(v(12.0, 12.0), 9.0 * unit, stroke);
            line(&[(12.0, 11.0), (12.0, 17.0)]);
            painter.circle_filled(v(12.0, 7.5), 1.2 * unit, color);
        }
        Icon::Error => {
            line(&[(6.0, 6.0), (18.0, 18.0)]);
            line(&[(18.0, 6.0), (6.0, 18.0)]);
        }
        Icon::Model => {
            outline(&[
                (12.0, 3.0),
                (20.0, 7.5),
                (20.0, 16.5),
                (12.0, 21.0),
                (4.0, 16.5),
                (4.0, 7.5),
            ]);
            line(&[(4.0, 7.5), (12.0, 12.0), (20.0, 7.5)]);
            line(&[(12.0, 12.0), (12.0, 21.0)]);
        }
        Icon::Light => {
            painter.circle_stroke(v(12.0, 12.0), 4.0 * unit, stroke);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::FRAC_PI_4;
                let (s, c) = a.sin_cos();
                line(&[
                    (12.0 + 7.0 * c, 12.0 + 7.0 * s),
                    (12.0 + 10.0 * c, 12.0 + 10.0 * s),
                ]);
            }
        }
        Icon::Search => {
            painter.circle_stroke(v(10.0, 10.0), 6.0 * unit, stroke);
            line(&[(14.5, 14.5), (20.0, 20.0)]);
        }
        Icon::Hidden => {
            // An eye: two arcs and the pupil, crossed out.
            let arc = |top: bool| {
                let points: Vec<Pos2> = (0..=12)
                    .map(|i| {
                        let t = i as f32 / 12.0;
                        let x = 3.0 + 18.0 * t;
                        let bulge = 6.0 * (std::f32::consts::PI * t).sin();
                        v(x, if top { 12.0 - bulge } else { 12.0 + bulge })
                    })
                    .collect();
                painter.add(Shape::line(points, stroke));
            };
            arc(true);
            arc(false);
            painter.circle_filled(v(12.0, 12.0), 2.5 * unit, color);
            line(&[(4.0, 20.0), (20.0, 4.0)]);
        }
        Icon::Lock => {
            outline(&[(5.0, 11.0), (19.0, 11.0), (19.0, 21.0), (5.0, 21.0)]);
            let shackle: Vec<Pos2> = (0..=10)
                .map(|i| {
                    let a = std::f32::consts::PI * (1.0 + i as f32 / 10.0);
                    v(12.0 + 5.0 * a.cos(), 9.0 + 5.0 * a.sin())
                })
                .collect();
            painter.add(Shape::line(shackle, stroke));
            line(&[(7.0, 9.0), (7.0, 11.0)]);
            line(&[(17.0, 9.0), (17.0, 11.0)]);
        }
        Icon::Sound => {
            outline(&[
                (3.0, 9.0),
                (7.0, 9.0),
                (12.0, 4.0),
                (12.0, 20.0),
                (7.0, 15.0),
                (3.0, 15.0),
            ]);
            let arc = |radius: f32| -> Vec<Pos2> {
                (0..=8)
                    .map(|i| {
                        let a = -0.8 + 1.6 * i as f32 / 8.0;
                        v(12.0 + radius * a.cos(), 12.0 + radius * a.sin())
                    })
                    .collect()
            };
            painter.add(Shape::line(arc(4.5), stroke));
            painter.add(Shape::line(arc(8.5), stroke));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_icon_draws_something() {
        let ctx = egui::Context::default();
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let painter = ui.ctx().layer_painter(egui::LayerId::background());
            let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(16.0, 16.0));
            for tool in Tool::ALL {
                tool_icon(&painter, rect, tool, Color32::WHITE);
            }
        });
        let shapes = output.shapes.len();
        // The font atlas upload is not needed here; egui insists it is
        // handled or cleared.
        let mut textures = output.textures_delta;
        textures.clear();
        assert!(shapes >= Tool::ALL.len());
    }

    #[test]
    fn every_icon_draws_something() {
        use Icon::*;
        let all = [
            Viewport,
            Game,
            Area,
            Layers,
            Properties,
            Library,
            Code,
            Console,
            Problems,
            Profiler,
            AreaFile,
            Play,
            PlayTogether,
            Stop,
            Pause,
            Step,
            Camera,
            Info,
            Warning,
            Error,
            Search,
            Model,
            Light,
            Hidden,
            Lock,
        ];
        let mut counts = Vec::new();
        for each in all {
            let ctx = egui::Context::default();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                let painter = ui.ctx().layer_painter(egui::LayerId::background());
                let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(12.0, 12.0));
                icon(&painter, rect, each, Color32::WHITE);
            });
            output.textures_delta.clear();
            counts.push(output.shapes.len());
        }
        assert!(counts.iter().all(|&n| n > 0), "{counts:?}");
    }
}
