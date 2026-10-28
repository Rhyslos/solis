//! A sound's loudness over its length, small enough to keep for every
//! sound in a project: what the Library and Properties draw as its
//! waveform. Made while the file is decoded, in one pass, even for sounds
//! too long to hold in memory.

use kira::Frame;

/// The most buckets an overview keeps; past it, pairs are merged.
pub(crate) const MAX_BUCKETS: usize = 2048;

/// Frames per bucket to start with.
const FIRST_BUCKET: u64 = 8;

/// The quietest and loudest sample in each stretch of a sound.
#[derive(Debug, Clone, PartialEq)]
pub struct Overview {
    /// Frames each bucket covers (the last may cover fewer).
    frames_per_bucket: u64,
    /// Each bucket's lowest and highest sample (left and right mixed).
    buckets: Vec<(f32, f32)>,
    /// Frames seen so far in the last, unfinished bucket.
    in_last: u64,
    /// Frames seen in all.
    frames: u64,
}

impl Default for Overview {
    fn default() -> Self {
        Self::new()
    }
}

impl Overview {
    /// An empty overview.
    pub fn new() -> Self {
        Self {
            frames_per_bucket: FIRST_BUCKET,
            buckets: Vec::new(),
            in_last: 0,
            frames: 0,
        }
    }

    /// Frames seen in all.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// Adds the next frames.
    pub fn add(&mut self, frames: &[Frame]) {
        for f in frames {
            let v = (f.left + f.right) * 0.5;
            if self.buckets.is_empty() || self.in_last == self.frames_per_bucket {
                if self.buckets.len() == MAX_BUCKETS {
                    self.halve();
                }
                if self.buckets.is_empty() || self.in_last == self.frames_per_bucket {
                    self.buckets.push((v, v));
                    self.in_last = 0;
                }
            }
            if let Some(last) = self.buckets.last_mut() {
                last.0 = last.0.min(v);
                last.1 = last.1.max(v);
            }
            self.in_last += 1;
            self.frames += 1;
        }
    }

    /// Merges buckets in pairs, so each covers twice as much.
    fn halve(&mut self) {
        let merged: Vec<(f32, f32)> = self
            .buckets
            .chunks(2)
            .map(|pair| {
                pair.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &(a, b)| {
                    (lo.min(a), hi.max(b))
                })
            })
            .collect();
        // The new last bucket holds the old last one or two.
        self.in_last = if self.buckets.len().is_multiple_of(2) {
            self.frames_per_bucket + self.in_last
        } else {
            self.in_last
        };
        self.buckets = merged;
        self.frames_per_bucket *= 2;
    }

    /// The overview as `columns` (lowest, highest) pairs across the whole
    /// length, for drawing; empty for a silent or empty sound.
    pub fn columns(&self, columns: usize) -> Vec<(f32, f32)> {
        if columns == 0 || self.buckets.is_empty() {
            return Vec::new();
        }
        let n = self.buckets.len();
        (0..columns)
            .map(|c| {
                let from = c * n / columns;
                let to = ((c + 1) * n / columns).max(from + 1).min(n);
                self.buckets[from..to]
                    .iter()
                    .fold((f32::MAX, f32::MIN), |(lo, hi), &(a, b)| {
                        (lo.min(a), hi.max(b))
                    })
            })
            .map(|(lo, hi)| if lo > hi { (0.0, 0.0) } else { (lo, hi) })
            .collect()
    }
}
