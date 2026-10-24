//! The Profiler panel (Phase 5): where each frame's time goes. The CPU's
//! share is split into what the editor does each frame (game code, the
//! game's world, preparing what to draw, handing it to the GPU, and the
//! rest of the editor), and the GPU's own passes are timed where the card
//! can (`ViewportRenderer::take_report`). Counts say how much was drawn.

use crate::theme;
use crate::viewport::{DrawReport, ViewportRenderer, ViewportView};
use egui::{Color32, RichText, Sense, Stroke, Ui, vec2};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// What a frame's CPU time is spent on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Lust, Lua and Rust components: ticks, events, taking in changes.
    GameCode,
    /// The game's world: stepping, placing, taking in the area's changes.
    World,
    /// Working out what each view draws.
    Prepare,
    /// Handing it to the GPU.
    Draw,
    /// Everything else the editor does: panels, input, the interface.
    Editor,
}

impl Stage {
    /// Every stage, in the order the panel shows them.
    pub const ALL: [Self; 5] = [
        Self::GameCode,
        Self::World,
        Self::Prepare,
        Self::Draw,
        Self::Editor,
    ];

    /// Its name in the panel.
    pub fn name(self) -> &'static str {
        match self {
            Self::GameCode => "Game code",
            Self::World => "World",
            Self::Prepare => "Preparing to draw",
            Self::Draw => "Drawing (CPU)",
            Self::Editor => "Editor",
        }
    }

    fn color(self) -> Color32 {
        match self {
            Self::GameCode => Color32::from_rgb(0xe0, 0x9a, 0x3e),
            Self::World => Color32::from_rgb(0x5f, 0xa8, 0xd3),
            Self::Prepare => Color32::from_rgb(0x8c, 0xc0, 0x6a),
            Self::Draw => Color32::from_rgb(0xc5, 0x7a, 0xd6),
            Self::Editor => Color32::from_rgb(0x7f, 0x86, 0x91),
        }
    }
}

/// One frame, as the panel shows it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sample {
    /// Milliseconds per stage, in [`Stage::ALL`]'s order.
    pub cpu: [f64; 5],
    /// What was drawn, and the GPU's times if measured.
    pub drawn: DrawReport,
    /// Game components running.
    pub components: usize,
    /// Entities in the game's world (or the area, when not playing).
    pub entities: usize,
}

impl Sample {
    /// The CPU's whole frame.
    pub fn cpu_total(&self) -> f64 {
        self.cpu.iter().sum()
    }
}

/// How many frames the panel remembers.
const KEPT: usize = 240;

/// Records each frame's times.
#[derive(Debug, Default)]
pub struct Profiler {
    started: Option<Instant>,
    spent: [Duration; 5],
    history: VecDeque<Sample>,
}

impl Profiler {
    /// A frame starts.
    pub fn begin(&mut self) {
        self.started = Some(Instant::now());
        self.spent = [Duration::ZERO; 5];
    }

    /// `stage` took `time` more this frame.
    pub fn add(&mut self, stage: Stage, time: Duration) {
        self.spent[stage as usize] += time;
    }

    /// The frame ends: whatever wasn't another stage was the editor's.
    pub fn end(&mut self, drawn: DrawReport, components: usize, entities: usize) {
        let Some(started) = self.started.take() else {
            return;
        };
        let total = started.elapsed();
        let others: Duration = self.spent[..4].iter().sum();
        self.spent[Stage::Editor as usize] = total.saturating_sub(others);
        let mut cpu = [0.0; 5];
        for (ms, spent) in cpu.iter_mut().zip(self.spent) {
            *ms = spent.as_secs_f64() * 1000.0;
        }
        if self.history.len() == KEPT {
            self.history.pop_front();
        }
        self.history.push_back(Sample {
            cpu,
            drawn,
            components,
            entities,
        });
    }

    /// The frames remembered, oldest first.
    pub fn history(&self) -> impl Iterator<Item = &Sample> {
        self.history.iter()
    }

    /// The average of the newest `count` frames (or fewer, if fewer were
    /// drawn), with the newest one's counts.
    pub fn average(&self, count: usize) -> Option<Sample> {
        let newest = self.history.back()?;
        let recent: Vec<&Sample> = self.history.iter().rev().take(count.max(1)).collect();
        let n = recent.len() as f64;
        let mut cpu = [0.0; 5];
        for sample in &recent {
            for (sum, ms) in cpu.iter_mut().zip(sample.cpu) {
                *sum += ms / n;
            }
        }
        let timed: Vec<_> = recent.iter().filter_map(|s| s.drawn.gpu).collect();
        let gpu = (!timed.is_empty()).then(|| {
            let n = timed.len() as f64;
            timed
                .iter()
                .fold(crate::viewport::GpuSplit::default(), |sum, t| {
                    crate::viewport::GpuSplit {
                        shadows: sum.shadows + t.shadows / n,
                        models: sum.models + t.models / n,
                        last_pass: sum.last_pass + t.last_pass / n,
                    }
                })
        });
        Some(Sample {
            cpu,
            drawn: DrawReport {
                gpu,
                ..newest.drawn
            },
            ..newest.clone()
        })
    }
}

/// A renderer that times what it's asked to draw, for [`Stage::Draw`].
pub(crate) struct Timed<'a> {
    pub(crate) inner: &'a mut dyn ViewportRenderer,
    pub(crate) spent: Duration,
}

impl ViewportRenderer for Timed<'_> {
    fn render(&mut self, view: &ViewportView) -> Result<egui::TextureId, String> {
        let start = Instant::now();
        let result = self.inner.render(view);
        self.spent += start.elapsed();
        result
    }

    fn picked(&mut self, image: usize) -> Option<crate::viewport_tools::PickAnswer> {
        self.inner.picked(image)
    }

    fn take_report(&mut self) -> DrawReport {
        self.inner.take_report()
    }
}

/// The frames averaged in the panel's numbers: about a second at 60 fps.
const AVERAGED: usize = 60;

/// Draws the Profiler panel.
pub(crate) fn profiler_panel(ui: &mut Ui, profiler: &Profiler) {
    let Some(average) = profiler.average(AVERAGED) else {
        ui.label(
            RichText::new(
                "Nothing measured yet. Each frame the editor draws is measured here: \
                 press Play, or move around the viewport.",
            )
            .color(theme::TEXT_WEAK),
        );
        return;
    };
    let total = average.cpu_total();
    let fps = if total > 0.0 { 1000.0 / total } else { 0.0 };
    ui.label(
        RichText::new(format!(
            "The CPU takes {total:.2} ms a frame ({fps:.0} frames a second at most), \
             averaged over the last {} frames.",
            profiler.history.len().min(AVERAGED)
        ))
        .strong(),
    );
    graph(ui, profiler);
    ui.horizontal_wrapped(|ui| {
        for (stage, ms) in Stage::ALL.iter().zip(average.cpu) {
            let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
            ui.painter().rect_filled(rect, 2.0, stage.color());
            ui.label(format!("{} {ms:.2} ms", stage.name()));
            ui.add_space(8.0);
        }
    });
    ui.add_space(4.0);
    let gpu = match (average.drawn.gpu, average.drawn.gpu_timing) {
        (Some(t), _) => format!(
            "The GPU takes {:.2} ms a frame: shadows {:.2} ms, models {:.2} ms, the last \
             pass {:.2} ms.",
            t.shadows + t.models + t.last_pass,
            t.shadows,
            t.models,
            t.last_pass
        ),
        (None, Some(false)) => {
            "The GPU's own times aren't measured on this graphics card (it doesn't offer \
             timestamps)."
                .to_string()
        }
        (None, _) => "The GPU's own times arrive once something is drawn.".to_string(),
    };
    ui.label(gpu);
    let d = average.drawn;
    ui.label(
        RichText::new(format!(
            "Drawn: {} model copies in view ({} casting shadows), {} triangles, in {} draw \
             calls; {} sent to the graphics card. {} entities; {} game components running.",
            thousands(d.instances),
            thousands(d.casting),
            thousands(d.triangles),
            thousands(d.batches),
            amount(d.sent_bytes),
            thousands(average.entities as u64),
            thousands(average.components as u64),
        ))
        .color(theme::TEXT_WEAK),
    );
    if d.bolts > 0 {
        ui.label(
            RichText::new(format!("Projectiles drawn: {}.", thousands(d.bolts)))
                .color(theme::TEXT_WEAK),
        );
    }
    if d.lods[1..].iter().any(|&n| n > 0) {
        ui.label(RichText::new(lod_text(&d.lods)).color(theme::TEXT_WEAK))
            .on_hover_text(
                "Far away, simpler versions of models are drawn (levels of detail): level 1 is \
                 about half the triangles, 2 a quarter, 3 an eighth",
            );
    }
}

/// How many copies were drawn at each level of detail: "Levels of detail:
/// 1,200 in full, 3,400 at level 1, 50 at level 3".
fn lod_text(lods: &[u64; 5]) -> String {
    let parts: Vec<String> = lods
        .iter()
        .enumerate()
        .filter(|&(_, &n)| n > 0)
        .map(|(level, &n)| match level {
            0 => format!("{} in full", thousands(n)),
            _ => format!("{} at level {level}", thousands(n)),
        })
        .collect();
    format!("Levels of detail: {}", parts.join(", "))
}

/// A number of bytes as people say it: 512 bytes, 3.4 kB, 12.0 MB.
fn amount(bytes: u64) -> String {
    match bytes {
        0..1_000 => format!("{bytes} bytes"),
        1_000..1_000_000 => format!("{:.1} kB", bytes as f64 / 1e3),
        _ => format!("{:.1} MB", bytes as f64 / 1e6),
    }
}

/// The frames as stacked bars, newest on the right, with lines at 60
/// and 30 frames a second.
fn graph(ui: &mut Ui, profiler: &Profiler) {
    let height = 90.0;
    let width = ui.available_width().max(60.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, theme::FIELD);
    let tallest = profiler
        .history()
        .map(Sample::cpu_total)
        .fold(1000.0 / 30.0 * 1.2, f64::max);
    let y = |ms: f64| rect.bottom() - (ms / tallest) as f32 * rect.height();
    let bar = rect.width() / KEPT as f32;
    let first = KEPT - profiler.history.len();
    for (i, sample) in profiler.history().enumerate() {
        let x = rect.left() + (first + i) as f32 * bar;
        let mut below = 0.0;
        for (stage, ms) in Stage::ALL.iter().zip(sample.cpu) {
            let top = below + ms;
            painter.rect_filled(
                egui::Rect::from_min_max(egui::pos2(x, y(top)), egui::pos2(x + bar, y(below))),
                0.0,
                stage.color(),
            );
            below = top;
        }
    }
    for (ms, label) in [(1000.0 / 60.0, "60 fps"), (1000.0 / 30.0, "30 fps")] {
        let at = y(ms);
        painter.line_segment(
            [egui::pos2(rect.left(), at), egui::pos2(rect.right(), at)],
            Stroke::new(1.0, theme::TEXT_WEAK.gamma_multiply(0.6)),
        );
        painter.text(
            egui::pos2(rect.left() + 4.0, at - 2.0),
            egui::Align2::LEFT_BOTTOM,
            label,
            egui::FontId::proportional(10.0),
            theme::TEXT_WEAK,
        );
    }
}

/// `12,345,678`.
fn thousands(n: u64) -> String {
    let text = n.to_string();
    let mut out = String::new();
    for (i, c) in text.chars().enumerate() {
        if i > 0 && (text.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewport::GpuSplit;

    #[test]
    fn a_frame_s_stages_add_up_to_the_frame() {
        let mut p = Profiler::default();
        p.begin();
        p.add(Stage::GameCode, Duration::from_millis(3));
        p.add(Stage::World, Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(6));
        p.end(DrawReport::default(), 5, 10);
        let sample = p.history().next().unwrap();
        assert_eq!(sample.cpu[0], 3.0);
        assert!(
            sample.cpu[Stage::Editor as usize] >= 1.5,
            "{:?}",
            sample.cpu
        );
        assert!(sample.cpu_total() >= 6.0);
        assert_eq!((sample.components, sample.entities), (5, 10));
    }

    #[test]
    fn averages_keep_the_newest_counts_and_only_measured_gpu_times() {
        let mut p = Profiler::default();
        for (i, gpu) in [None, Some(4.0), Some(2.0)].into_iter().enumerate() {
            p.begin();
            p.add(Stage::World, Duration::from_millis(2 * (i as u64 + 1)));
            p.end(
                DrawReport {
                    instances: i as u64,
                    gpu: gpu.map(|ms| GpuSplit {
                        models: ms,
                        ..GpuSplit::default()
                    }),
                    gpu_timing: Some(true),
                    ..DrawReport::default()
                },
                0,
                0,
            );
        }
        let average = p.average(60).unwrap();
        assert!((average.cpu[Stage::World as usize] - 4.0).abs() < 1e-9);
        assert_eq!(average.drawn.instances, 2);
        assert_eq!(average.drawn.gpu.unwrap().models, 3.0);
        // Only the newest frames are remembered.
        for _ in 0..KEPT + 10 {
            p.begin();
            p.end(DrawReport::default(), 0, 0);
        }
        assert_eq!(p.history().count(), KEPT);
    }

    #[test]
    fn big_numbers_read_easily() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_000), "1,000");
        assert_eq!(thousands(12_345_678), "12,345,678");
        assert_eq!(amount(0), "0 bytes");
        assert_eq!(amount(3_400), "3.4 kB");
        assert_eq!(amount(12_000_000), "12.0 MB");
        assert_eq!(
            lod_text(&[1_200, 3_400, 0, 50, 0]),
            "Levels of detail: 1,200 in full, 3,400 at level 1, 50 at level 3"
        );
    }
}
