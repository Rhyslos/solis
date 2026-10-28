//! A clip: frames in order, each shown for some milliseconds and perhaps
//! carrying events, played forward, backward or ping-pong, over and over
//! or a number of times, at a speed.
//!
//! Time is in seconds from the clip's start, as an `f64`, moved on by the
//! running game once per tick by plain additions; everything here is
//! sums and comparisons on it, so every PC finds the same frame and the
//! same events at the same tick.

pub use assets::aseprite::Direction;

/// One frame of a clip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipFrame {
    /// Which of the sheet's frames it shows (from 0).
    pub frame: u32,
    /// How long it shows, in milliseconds (at least 1).
    pub ms: u32,
    /// Its events, in order: each happens as the frame starts.
    pub events: Vec<String>,
}

/// One step of a pass through a clip: which frame, and when it starts.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Step {
    /// The frame's place in [`Clip::frames`].
    index: usize,
    /// When it starts, in milliseconds from the pass's start.
    start: f64,
}

/// A clip of frames.
#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    /// Its name ("walk").
    pub name: String,
    /// Its frames, in the order drawn (before its direction is applied).
    pub frames: Vec<ClipFrame>,
    /// Which way it plays.
    pub direction: Direction,
    /// How many passes it plays: 0 for over and over. A ping-pong pass
    /// goes there and back.
    pub repeats: u32,
    /// How fast it plays: 1 as drawn, 2 twice as fast.
    pub speed: f32,
    /// Whether it's one of the sheet's Aseprite tags (its frames are
    /// changed in Aseprite), rather than a clip made in Solis.
    pub from_aseprite: bool,
    /// One pass's steps, worked out once.
    steps: Vec<Step>,
    /// One pass's length, in milliseconds.
    pass_ms: f64,
}

/// Where a clip is at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moment {
    /// The frame's place in [`Clip::frames`].
    pub index: usize,
    /// The sheet's frame it shows (from 0).
    pub frame: u32,
    /// Whether the clip has played all its passes (it then stays on its
    /// last step's frame).
    pub finished: bool,
}

/// The most passes looked through for events in one call: far more than
/// a tick covers at any sensible speed, so a clip sped up absurdly can't
/// stall the tick.
pub const MAX_PASSES_A_CALL: u64 = 64;

impl Clip {
    /// A clip of `frames`, played `direction`, `repeats` passes (0 for
    /// over and over), at `speed`. Frames shorter than 1 ms are made 1 ms.
    pub fn new(
        name: impl Into<String>,
        mut frames: Vec<ClipFrame>,
        direction: Direction,
        repeats: u32,
        speed: f32,
        from_aseprite: bool,
    ) -> Self {
        for f in &mut frames {
            f.ms = f.ms.max(1);
        }
        let order = order(frames.len(), direction);
        let mut steps = Vec::with_capacity(order.len());
        let mut at = 0.0;
        for index in order {
            steps.push(Step { index, start: at });
            at += f64::from(frames[index].ms);
        }
        Self {
            name: name.into(),
            frames,
            direction,
            repeats,
            speed: if speed.is_finite() {
                speed.max(0.0)
            } else {
                1.0
            },
            from_aseprite,
            steps,
            pass_ms: at,
        }
    }

    /// One pass's length, in seconds (at speed 1).
    pub fn pass_seconds(&self) -> f64 {
        self.pass_ms / 1000.0
    }

    /// The whole clip's length in seconds (at speed 1), or `None` when it
    /// plays over and over.
    pub fn length_seconds(&self) -> Option<f64> {
        (self.repeats > 0).then(|| self.pass_ms * f64::from(self.repeats) / 1000.0)
    }

    /// The frames' places in [`Self::frames`] in the order one pass
    /// shows them (ping-pong: 1 2 3 4 3 2).
    pub fn pass_order(&self) -> impl Iterator<Item = usize> + '_ {
        self.steps.iter().map(|s| s.index)
    }

    /// Where the clip is `seconds` after it started (at speed 1: a
    /// sprite's own speed and the clip's are already in `seconds`), or
    /// `None` for a clip with no frames.
    pub fn at(&self, seconds: f64) -> Option<Moment> {
        let last = self.steps.last()?;
        let ms = if seconds.is_finite() {
            (seconds * 1000.0).max(0.0)
        } else {
            0.0
        };
        let finished = self.repeats > 0 && ms >= self.pass_ms * f64::from(self.repeats);
        let step = if finished {
            last
        } else {
            let into = ms % self.pass_ms;
            // The last step starting at or before `into`.
            let k = self.steps.partition_point(|s| s.start <= into);
            &self.steps[k.saturating_sub(1)]
        };
        Some(Moment {
            index: step.index,
            frame: self.frames[step.index].frame,
            finished,
        })
    }

    /// The events of every frame that starts from `from` up to (not
    /// including) `to` seconds, in order, added to `out`. Stepping a clip
    /// from one tick's time to the next's this way gives each event
    /// exactly once, the first frame's included when it starts at 0.
    pub fn events_between(&self, from: f64, to: f64, out: &mut Vec<String>) {
        if self.steps.is_empty() || !(from.is_finite() && to.is_finite()) || to <= from {
            return;
        }
        if self.frames.iter().all(|f| f.events.is_empty()) {
            return;
        }
        let (from, to) = (from.max(0.0) * 1000.0, to.max(0.0) * 1000.0);
        let first = (from / self.pass_ms).floor() as u64;
        let last = (to / self.pass_ms).floor() as u64;
        let last = last.min(first.saturating_add(MAX_PASSES_A_CALL - 1));
        for pass in first..=last {
            if self.repeats > 0 && pass >= u64::from(self.repeats) {
                break;
            }
            let base = pass as f64 * self.pass_ms;
            for step in &self.steps {
                let at = base + step.start;
                if at >= to {
                    break;
                }
                if at >= from {
                    out.extend(self.frames[step.index].events.iter().cloned());
                }
            }
        }
    }
}

/// The places of `n` frames in the order one pass of `direction` shows
/// them.
fn order(n: usize, direction: Direction) -> Vec<usize> {
    let forward = 0..n;
    let back_inside = (1..n.saturating_sub(1)).rev();
    match direction {
        Direction::Forward => forward.collect(),
        Direction::Reverse => forward.rev().collect(),
        Direction::PingPong => forward.chain(back_inside).collect(),
        Direction::PingPongReverse => {
            let down = (0..n).rev();
            let up_inside = 1..n.saturating_sub(1);
            down.chain(up_inside).collect()
        }
    }
}

#[cfg(test)]
#[path = "clip_tests.rs"]
mod tests;
