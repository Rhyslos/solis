//! The Profiler (Phase 5): where each frame's time goes. The CPU's
//! share is split into what the editor does each frame (game code, the
//! game's world, preparing what to draw, handing it to the GPU, and the
//! rest of the editor), and the GPU's own passes are timed where the card
//! can (`ViewportRenderer::take_report`). Counts say how much was drawn.
//! This keeps the frames; `profiler_view.rs` draws them.

use crate::viewport::{DrawReport, ViewportRenderer, ViewportView};
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
    /// The game's sounds: choosing what's heard, placing it, the buses
    /// (Phase 7, S5; the frame before, as the app works it out after the
    /// editor has drawn).
    Sound,
    /// Everything else the editor does: panels, input, the interface.
    Editor,
}

impl Stage {
    /// Every stage, in the order the panel shows them.
    pub const ALL: [Self; 6] = [
        Self::GameCode,
        Self::World,
        Self::Prepare,
        Self::Draw,
        Self::Sound,
        Self::Editor,
    ];

    /// Its name in the panel.
    pub fn name(self) -> &'static str {
        match self {
            Self::GameCode => "Game code",
            Self::World => "World",
            Self::Prepare => "Preparing to draw",
            Self::Draw => "Drawing (CPU)",
            Self::Sound => "Sound",
            Self::Editor => "Editor",
        }
    }
}

/// One frame, as the panel shows it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sample {
    /// Milliseconds per stage, in [`Stage::ALL`]'s order.
    pub cpu: [f64; 6],
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
pub(crate) const KEPT: usize = 240;

/// Records each frame's times.
#[derive(Debug, Default)]
pub struct Profiler {
    started: Option<Instant>,
    spent: [Duration; 6],
    history: VecDeque<Sample>,
    /// Whether the CPU's stages are left out of the panel (Debug, Show).
    pub(crate) hide_cpu: bool,
    /// Whether the GPU's times are left out of the panel (Debug, Show).
    pub(crate) hide_gpu: bool,
    /// How many frames have ended since the editor started: the newest
    /// frame's number.
    counted: u64,
    /// The frame picked in the frame-time graph, by its number (the
    /// newest is shown when none is, or when it's no longer kept).
    pub(crate) picked: Option<u64>,
}

impl Profiler {
    /// A frame starts.
    pub fn begin(&mut self) {
        self.started = Some(Instant::now());
        self.spent = [Duration::ZERO; 6];
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
        // Sound is worked out outside the editor's frame, so it isn't
        // taken from the editor's share.
        let others: Duration = self.spent[..Stage::Sound as usize].iter().sum();
        self.spent[Stage::Editor as usize] = total.saturating_sub(others);
        let mut cpu = [0.0; 6];
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
        self.counted += 1;
    }

    /// The number of the frame kept at `index` (0 the oldest).
    pub fn number_of(&self, index: usize) -> u64 {
        self.counted - self.history.len() as u64 + index as u64 + 1
    }

    /// The frame the timeline shows, with its number: the one picked
    /// while it's kept, else the newest.
    pub fn shown(&self) -> Option<(u64, &Sample)> {
        let oldest = self.number_of(0);
        let picked = self
            .picked
            .filter(|n| *n >= oldest && *n <= self.counted)
            .map(|n| (n, &self.history[(n - oldest) as usize]));
        picked.or_else(|| self.history.back().map(|s| (self.counted, s)))
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
        let mut cpu = [0.0; 6];
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

/// How many copies were drawn at each level of detail: "Levels of detail:
/// 1,200 in full, 3,400 at level 1, 50 at level 3".
pub(crate) fn lod_text(lods: &[u64; 5]) -> String {
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
pub(crate) fn amount(bytes: u64) -> String {
    match bytes {
        0..1_000 => format!("{bytes} bytes"),
        1_000..1_000_000 => format!("{:.1} kB", bytes as f64 / 1e3),
        _ => format!("{:.1} MB", bytes as f64 / 1e6),
    }
}

/// `12,345,678`.
pub(crate) fn thousands(n: u64) -> String {
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
    fn sound_is_shown_without_being_taken_from_the_editor() {
        // Sound is worked out after the editor's frame (by the app), so
        // it isn't part of the time the frame measured.
        let mut p = Profiler::default();
        p.begin();
        p.add(Stage::Sound, Duration::from_millis(4));
        std::thread::sleep(Duration::from_millis(3));
        p.end(DrawReport::default(), 0, 0);
        let sample = p.history().next().unwrap();
        assert_eq!(sample.cpu[Stage::Sound as usize], 4.0);
        assert!(
            sample.cpu[Stage::Editor as usize] >= 2.5,
            "{:?}",
            sample.cpu
        );
        assert_eq!(Stage::Sound.name(), "Sound");
    }

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
