//! The Debug tab's Profiler (Screen 6, spec 461-500; round 3,
//! `09-debug-profiler.png`), drawn from what `profiler.rs` keeps: the
//! kept frames as bars with the 60 and 30 frames-a-second lines, the
//! frame picked (a click on a bar, the project lead's answer 25) or the
//! newest; that frame's timeline, the CPU's parts and the GPU's passes
//! as two lanes on a time ruler; and Statistics beside it, averaged over
//! the last 60 frames. The design's other lanes (one a thread) and its
//! memory figures aren't measured: they are unfinished parts.

use crate::profiler::{KEPT, Profiler, Sample, Stage, amount, lod_text, thousands};
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, UnbuiltId};
use egui::{Align2, CornerRadius, Rect, Sense, Shape, Stroke, Ui, UiBuilder, pos2, vec2};

/// The frames averaged in Statistics: about a second at 60 fps.
const AVERAGED: usize = 60;
/// A frame's time at 60 and at 30 frames a second, in milliseconds.
const BUDGETS: [(f64, &str); 2] = [(1000.0 / 60.0, "60 fps"), (1000.0 / 30.0, "30 fps")];
/// The narrowest a block on a lane may be and still show its name.
const NAMED_BLOCK_W: f32 = 36.0;
/// The dashes of the budget lines: drawn, then left out.
const DASH: [f32; 2] = [4.0, 3.0];

/// The Profiler. Returns the frame a click on a bar picked, if one did.
pub(crate) fn profiler_panel(ui: &mut Ui, profiler: &Profiler) -> Option<u64> {
    let whole = ui.max_rect();
    ui.painter().rect_filled(whole, 0.0, tokens::shelf_bar());
    let (Some((number, frame)), Some(average)) = (profiler.shown(), profiler.average(AVERAGED))
    else {
        crate::panels::empty_note(
            ui,
            "Nothing measured yet. Each frame the editor draws is measured here: press Play, or \
             move around the viewport.",
        );
        return None;
    };
    let top = whole.with_max_y(whole.top() + size::PROFILER_TOP_H);
    ui.painter().text(
        pos2(top.right() - size::ROW_PAD, top.center().y),
        Align2::RIGHT_CENTER,
        format!(
            "Frame {} \u{b7} {:.1} ms \u{b7} click a bar to pick its frame",
            thousands(number),
            frame.cpu_total()
        ),
        text::ui(),
        tokens::text_label(),
    );
    let mut picked = None;
    let mut y = top.bottom();
    if !profiler.hide_cpu {
        let graph = Rect::from_x_y_ranges(whole.x_range(), y..=y + size::FRAME_GRAPH_H);
        picked = frame_graph(ui, graph, profiler, number);
        y = graph.bottom();
    }
    tokens::hline(ui.painter(), whole.x_range(), y, tokens::line());
    let below = Rect::from_x_y_ranges(whole.x_range(), y + 1.0..=whole.bottom());
    let split = (below.right() - size::STATS_W).max(below.left() + size::LANE_LABEL_W);
    let timeline_rect = below.with_max_x(split);
    let stats_rect = below.with_min_x(split + 1.0);
    tokens::vline(ui.painter(), split, below.y_range(), tokens::line());
    let mut lanes = ui.new_child(UiBuilder::new().max_rect(timeline_rect).id_salt("timeline"));
    lanes.set_clip_rect(timeline_rect.intersect(ui.clip_rect()));
    timeline(&mut lanes, timeline_rect, profiler, frame);
    let mut stats = ui.new_child(UiBuilder::new().max_rect(stats_rect).id_salt("statistics"));
    stats.set_clip_rect(stats_rect.intersect(ui.clip_rect()));
    statistics(&mut stats, stats_rect, profiler, &average);
    picked
}

/// The kept frames as bars, newest at the right, with the budget lines.
/// Returns the frame a click picked.
fn frame_graph(ui: &mut Ui, rect: Rect, profiler: &Profiler, shown: u64) -> Option<u64> {
    let painter = ui.painter().clone();
    let bars = rect
        .shrink2(vec2(size::ROW_PAD, size::ROW_GAP * 2.0))
        .with_min_x(rect.left() + size::FRAME_GRAPH_LABEL_W);
    let tallest = profiler
        .history()
        .map(Sample::cpu_total)
        .fold(BUDGETS[1].0 * 1.2, f64::max);
    let y = |ms: f64| bars.bottom() - (ms / tallest) as f32 * bars.height();
    for (ms, label) in BUDGETS.iter().copied().chain([(0.0, "0")]) {
        let at = y(ms).round();
        if ms > 0.0 {
            painter.extend(Shape::dashed_line(
                &[pos2(bars.left(), at), pos2(bars.right(), at)],
                Stroke::new(1.0, tokens::stronger_line()),
                DASH[0],
                DASH[1],
            ));
        }
        painter.text(
            pos2(rect.left() + size::ROW_PAD, at),
            Align2::LEFT_CENTER,
            label,
            text::mono_small(),
            tokens::text_meta(),
        );
    }
    let slot = bars.width() / KEPT as f32;
    let gap = if slot >= 4.0 { 1.0 } else { 0.0 };
    let accent = crate::interface_look::in_use().accent.colors().fill;
    let first = KEPT - profiler.history().count();
    for (i, sample) in profiler.history().enumerate() {
        let x = bars.left() + (first + i) as f32 * slot;
        let ms = sample.cpu_total();
        let fill = if profiler.number_of(i) == shown {
            accent
        } else if ms > BUDGETS[0].0 {
            tokens::PROFILER_OVER
        } else {
            tokens::PROFILER_BAR
        };
        let bar = Rect::from_min_max(
            pos2(x, y(ms).min(bars.bottom() - 1.0)),
            pos2(x + slot - gap, bars.bottom()),
        );
        painter.rect_filled(bar, CornerRadius::ZERO, fill);
    }
    let response = ui.interact(bars, ui.id().with("frame bars"), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Frame-time graph")
    });
    let at = response
        .interact_pointer_pos()
        .filter(|_| response.clicked())?;
    let index = ((at.x - bars.left()) / slot).floor() as usize;
    let kept = index.checked_sub(first)?;
    (kept < profiler.history().count()).then(|| profiler.number_of(kept))
}

/// One block on a lane, from `start` to `end` milliseconds, named when
/// it's wide enough.
struct Block<'a> {
    name: &'a str,
    start: f64,
    end: f64,
    fill: egui::Color32,
}

/// The frame's timeline: the ruler, the CPU's lane, the GPU's, the
/// unfinished thread lanes, and the note on reading it.
fn timeline(ui: &mut Ui, rect: Rect, profiler: &Profiler, frame: &Sample) {
    let names = rect.with_max_x(rect.left() + size::LANE_LABEL_W);
    ui.painter().rect_filled(names, 0.0, tokens::panel());
    tokens::vline(ui.painter(), names.right(), rect.y_range(), tokens::line());
    let gpu = frame.drawn.gpu;
    let gpu_total = gpu.map_or(0.0, |g| g.shadows + g.models + g.last_pass);
    let longest = frame.cpu_total().max(gpu_total).max(1.0);
    let span = ruler_span(longest);
    let track = rect
        .with_min_x(names.right() + 1.0)
        .shrink2(vec2(size::GAP, 0.0));
    let x = |ms: f64| track.left() + (ms / span) as f32 * track.width();
    let ruler = Rect::from_x_y_ranges(rect.x_range(), rect.top()..=rect.top() + size::LANE_H);
    let step = ruler_step(span);
    let mut mark = 0.0;
    while mark <= span + 1e-9 {
        let words = if mark == 0.0 {
            "0 ms".to_owned()
        } else if step < 1.0 {
            format!("{mark:.1}")
        } else {
            format!("{mark:.0}")
        };
        ui.painter().text(
            pos2(x(mark), ruler.center().y),
            Align2::LEFT_CENTER,
            words,
            text::mono_small(),
            tokens::text_meta(),
        );
        mark += step;
    }
    let mut y = ruler.bottom();
    let accent = crate::interface_look::in_use().accent.colour();
    let culprit = tokens::mix(accent, tokens::row_hover(), tokens::share::CULPRIT);
    if !profiler.hide_cpu {
        let mut start = 0.0;
        let blocks: Vec<Block<'_>> = Stage::ALL
            .iter()
            .zip(frame.cpu)
            .map(|(stage, ms)| {
                let block = Block {
                    name: stage.name(),
                    start,
                    end: start + ms,
                    fill: if *stage == Stage::GameCode {
                        culprit
                    } else {
                        tokens::stronger_line()
                    },
                };
                start += ms;
                block
            })
            .collect();
        lane(ui, (names, y), "CPU", &blocks, &x);
        y += size::LANE_H;
    }
    if !profiler.hide_gpu {
        let blocks: Vec<Block<'_>> = gpu.map_or_else(Vec::new, |g| {
            let fill = tokens::stronger_line();
            vec![
                Block {
                    name: "Shadows",
                    start: 0.0,
                    end: g.shadows,
                    fill,
                },
                Block {
                    name: "Models",
                    start: g.shadows,
                    end: g.shadows + g.models,
                    fill,
                },
                Block {
                    name: "The last pass",
                    start: g.shadows + g.models,
                    end: g.shadows + g.models + g.last_pass,
                    fill,
                },
            ]
        });
        lane(ui, (names, y), "GPU", &blocks, &x);
        if gpu.is_none() {
            let why = match frame.drawn.gpu_timing {
                Some(false) => "not measured on this graphics card (it has no timestamps)",
                _ => "measured once something is drawn",
            };
            ui.painter().text(
                pos2(track.left(), y + size::LANE_H / 2.0),
                Align2::LEFT_CENTER,
                why,
                text::ui(),
                tokens::text_faint(),
            );
        }
        y += size::LANE_H;
    }
    if !unbuilt::hidden() {
        let label = Rect::from_min_size(
            pos2(names.left() + size::ROW_PAD, y),
            vec2(names.width() - size::ROW_PAD, size::LANE_H),
        );
        let mut label_ui = ui.new_child(UiBuilder::new().max_rect(label).id_salt("thread lanes"));
        unbuilt::status_item(&mut label_ui, UnbuiltId::DebugTimelineThreads);
    }
    let note = "The CPU and the GPU work at the same time, each on its own frame: the slower of \
                the two sets the frame rate, not their sum.";
    let foot = Rect::from_min_max(
        pos2(track.left(), rect.bottom() - 2.0 * size::LANE_H),
        pos2(track.right(), rect.bottom() - size::ROW_GAP),
    );
    let mut foot_ui = ui.new_child(UiBuilder::new().max_rect(foot).id_salt("timeline note"));
    foot_ui.label(
        egui::RichText::new(note)
            .font(text::row())
            .color(tokens::text_label()),
    );
}

/// A lane at `y`: its name in the names' column, then its blocks.
fn lane(
    ui: &Ui,
    (names, y): (Rect, f32),
    name: &str,
    blocks: &[Block<'_>],
    x: &dyn Fn(f64) -> f32,
) {
    let painter = ui.painter();
    painter.text(
        pos2(names.left() + size::ROW_PAD, y + size::LANE_H / 2.0),
        Align2::LEFT_CENTER,
        name,
        text::row(),
        tokens::text_body(),
    );
    for block in blocks.iter().filter(|b| b.end > b.start) {
        let rect = Rect::from_min_max(
            pos2(x(block.start), y + size::LANE_BLOCK_INSET),
            pos2(
                x(block.end) - 1.0,
                y + size::LANE_H - size::LANE_BLOCK_INSET,
            ),
        );
        painter.rect_filled(rect, CornerRadius::same(3), block.fill);
        if rect.width() >= NAMED_BLOCK_W {
            crate::widgets::cut_off(
                ui,
                rect.shrink2(vec2(size::GAP, 0.0)),
                block.name,
                text::ui(),
                tokens::text_primary(),
            );
        }
    }
    tokens::hline(
        painter,
        ui.max_rect().x_range(),
        y + size::LANE_H,
        tokens::line(),
    );
}

/// How many milliseconds the ruler spans for a frame of `longest`: the
/// next whole step above it.
fn ruler_span(longest: f64) -> f64 {
    let step = ruler_step(longest);
    ((longest / step).ceil() * step).max(step)
}

/// The ruler's step: 1, 2, 5, 10 … milliseconds, so about five marks.
fn ruler_step(span: f64) -> f64 {
    let rough = (span / 5.0).max(0.1);
    let power = 10f64.powf(rough.log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|m| m * power)
        .find(|step| *step >= rough)
        .unwrap_or(10.0 * power)
}

/// Statistics: the CPU's and GPU's frame with a bar, then the counts.
fn statistics(ui: &mut Ui, rect: Rect, profiler: &Profiler, average: &Sample) {
    let inside = rect.shrink2(vec2(size::ROW_PAD, size::ROW_GAP * 2.0));
    let mut col = ui.new_child(UiBuilder::new().max_rect(inside).id_salt("stats rows"));
    col.spacing_mut().item_spacing.y = size::ROW_GAP * 2.0;
    col.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Statistics")
                .font(text::header())
                .color(tokens::text_primary()),
        );
        ui.label(
            egui::RichText::new(format!(
                "averaged over the last {} frames",
                profiler.history().count().min(AVERAGED)
            ))
            .font(text::row())
            .color(tokens::text_meta()),
        );
    });
    let budget = BUDGETS[0].0;
    if !profiler.hide_cpu {
        let total = average.cpu_total();
        let fps = if total > 0.0 { 1000.0 / total } else { 0.0 };
        let note = format!("{fps:.0} frames a second at most");
        timed_row(&mut col, "CPU \u{b7} a frame", &note, total, budget);
    }
    if !profiler.hide_gpu
        && let Some(g) = average.drawn.gpu
    {
        timed_row(
            &mut col,
            "GPU \u{b7} a frame",
            "",
            g.shadows + g.models + g.last_pass,
            budget,
        );
    }
    let d = average.drawn;
    let casting = format!("{} casting shadows", thousands(d.casting));
    for (name, note, value) in [
        ("Draw calls", String::new(), thousands(d.batches)),
        ("Triangles", String::new(), thousands(d.triangles)),
        ("Model copies in view", casting, thousands(d.instances)),
        (
            "Sent to the graphics card",
            String::new(),
            amount(d.sent_bytes),
        ),
        (
            "Entities",
            String::new(),
            thousands(average.entities as u64),
        ),
        (
            "Game components running",
            String::new(),
            thousands(average.components as u64),
        ),
    ] {
        count_row(&mut col, name, &note, &value);
    }
    if d.bolts > 0 {
        count_row(&mut col, "Projectiles drawn", "", &thousands(d.bolts));
    }
    if d.lods[1..].iter().any(|&n| n > 0) {
        col.label(
            egui::RichText::new(lod_text(&d.lods))
                .font(text::row())
                .color(tokens::text_meta()),
        );
    }
    if !unbuilt::hidden() {
        unbuilt::link(&mut col, UnbuiltId::DebugStatsMemory);
    }
}

/// A statistic's row: its name, quiet words after it, its value at the
/// right in mono.
fn count_row(ui: &mut Ui, name: &str, note: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(name)
                .font(text::row())
                .color(tokens::text_body()),
        );
        if !note.is_empty() {
            ui.label(
                egui::RichText::new(note)
                    .font(text::ribbon_label())
                    .color(tokens::text_faint()),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(value)
                    .font(text::mono())
                    .color(tokens::text_primary()),
            );
        });
    });
}

/// A time's row, with a bar under it: its share of the 60 fps budget,
/// lighter when it's over.
fn timed_row(ui: &mut Ui, name: &str, note: &str, ms: f64, budget: f64) {
    count_row(ui, name, note, &format!("{ms:.2} ms"));
    let (bar, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), size::STATS_BAR_H),
        Sense::hover(),
    );
    let round = CornerRadius::same(size::radius::GRIP);
    ui.painter().rect_filled(bar, round, tokens::line());
    let share = (ms / budget).clamp(0.0, 1.0) as f32;
    let fill = if ms > budget {
        tokens::PROFILER_OVER
    } else {
        crate::interface_look::in_use().accent.colors().fill
    };
    ui.painter().rect_filled(
        bar.with_max_x(bar.left() + bar.width() * share),
        round,
        fill,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ruler_has_about_five_round_marks() {
        assert_eq!(ruler_step(10.0), 2.0);
        assert_eq!(ruler_step(9.6), 2.0);
        assert_eq!(ruler_step(40.0), 10.0);
        assert_eq!(ruler_step(0.3), 0.1);
        assert_eq!(ruler_span(9.6), 10.0);
        assert_eq!(ruler_span(16.7), 20.0);
        assert!(ruler_span(0.0) > 0.0);
    }
}
