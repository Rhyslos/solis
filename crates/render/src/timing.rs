//! How long the GPU takes over a frame, for the Profiler: timestamps
//! written at the start and end of each pass, read back a frame or two
//! later without waiting (CONTRIBUTING rule 10). Only on cards that offer
//! timestamps (`wgpu::Features::TIMESTAMP_QUERY`); elsewhere there are no
//! GPU times, and the Profiler says so.

use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

/// The GPU's time for one frame of one viewport, in milliseconds.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GpuTimes {
    /// The sun's shadow map (0 when there are no shadows).
    pub shadows: f64,
    /// The sky, models, grid and see-through things.
    pub models: f64,
    /// Exposure and tone mapping.
    pub last_pass: f64,
}

impl GpuTimes {
    /// All of it.
    pub fn total(&self) -> f64 {
        self.shadows + self.models + self.last_pass
    }
}

/// What one frame drew.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameStats {
    /// Mesh copies drawn: those in view.
    pub instances: u32,
    /// Mesh copies drawn into the sun's shadow map: those whose shadow
    /// can fall in view.
    pub casting: u32,
    /// Draw calls for models (shadows' included).
    pub batches: u32,
    /// Triangles drawn (shadows not counted).
    pub triangles: u64,
    /// Bytes of copies sent to the graphics card: only what changed.
    pub sent_bytes: u64,
    /// Mesh copies in view drawn at each level of detail: in full, then
    /// each simpler level.
    pub lods: [u32; 5],
    /// Projectiles drawn.
    pub bolts: u32,
    /// Sprites drawn.
    pub sprites: u32,
}

/// The passes timed, each a pair of timestamps.
#[derive(Debug, Clone, Copy)]
pub(crate) enum TimedPass {
    Shadows = 0,
    Models = 1,
    LastPass = 2,
}

const STAMPS: u32 = 6;
const BYTES: u64 = STAMPS as u64 * 8;
/// Frames that may be waiting for their times at once; past it, a frame
/// simply isn't timed.
const IN_FLIGHT: usize = 3;

/// One frame's times on their way back.
#[derive(Debug)]
struct Readback {
    buffer: wgpu::Buffer,
    /// 0 free, 1 waiting, 2 ready, 3 failed.
    state: Arc<AtomicU8>,
    /// Whether that frame drew shadows (else their stamps mean nothing).
    shadows: Cell<bool>,
}

/// One viewport's GPU timer.
#[derive(Debug)]
pub(crate) struct GpuTimer {
    queries: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readbacks: Vec<Readback>,
    /// Nanoseconds per timestamp tick.
    period: f64,
    /// The frame being timed now: its readback.
    current: Cell<Option<usize>>,
    latest: Cell<Option<GpuTimes>>,
}

impl GpuTimer {
    /// A timer, if the device can write timestamps.
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("frame timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: STAMPS,
        });
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame timestamps resolved"),
            size: BYTES,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readbacks = (0..IN_FLIGHT)
            .map(|_| Readback {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("frame timestamps read back"),
                    size: BYTES,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
                state: Arc::new(AtomicU8::new(0)),
                shadows: Cell::new(false),
            })
            .collect();
        Some(Self {
            queries,
            resolve,
            readbacks,
            period: f64::from(queue.get_timestamp_period()),
            current: Cell::new(None),
            latest: Cell::new(None),
        })
    }

    /// Starts timing a frame, if a readback is free.
    pub(crate) fn begin(&self, shadows: bool) {
        let free = self
            .readbacks
            .iter()
            .position(|r| r.state.load(Ordering::Acquire) == 0);
        if let Some(slot) = free {
            self.readbacks[slot].shadows.set(shadows);
        }
        self.current.set(free);
    }

    /// The timestamps for `pass`, when this frame is timed.
    pub(crate) fn writes(&self, pass: TimedPass) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        self.current.get()?;
        let first = pass as u32 * 2;
        Some(wgpu::RenderPassTimestampWrites {
            query_set: &self.queries,
            beginning_of_pass_write_index: Some(first),
            end_of_pass_write_index: Some(first + 1),
        })
    }

    /// Copies the frame's timestamps out, at the end of its commands.
    pub(crate) fn finish(&self, encoder: &mut wgpu::CommandEncoder) {
        let Some(slot) = self.current.get() else {
            return;
        };
        // Stamps not written this frame (no shadows) resolve as 0, and
        // don't count.
        encoder.resolve_query_set(&self.queries, 0..STAMPS, &self.resolve, 0);
        encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.readbacks[slot].buffer, 0, BYTES);
    }

    /// After the frame was submitted: asks for its times back.
    pub(crate) fn submitted(&self) {
        let Some(slot) = self.current.take() else {
            return;
        };
        let readback = &self.readbacks[slot];
        readback.state.store(1, Ordering::Release);
        let done = Arc::clone(&readback.state);
        readback
            .buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                done.store(if result.is_ok() { 2 } else { 3 }, Ordering::Release);
            });
    }

    /// The newest times that have arrived. Never waits.
    pub(crate) fn poll(&self, device: &wgpu::Device) -> Option<GpuTimes> {
        if self
            .readbacks
            .iter()
            .any(|r| r.state.load(Ordering::Acquire) == 1)
        {
            let _ = device.poll(wgpu::PollType::Poll);
        }
        for readback in &self.readbacks {
            match readback.state.load(Ordering::Acquire) {
                2 => {
                    if let Ok(mapped) = readback.buffer.slice(..).get_mapped_range() {
                        let stamps: Vec<u64> = mapped
                            .chunks_exact(8)
                            .map(|b| u64::from_le_bytes(b.try_into().unwrap_or([0; 8])))
                            .collect();
                        self.latest
                            .set(Some(times(&stamps, self.period, readback.shadows.get())));
                    }
                    readback.buffer.unmap();
                    readback.state.store(0, Ordering::Release);
                }
                3 => readback.state.store(0, Ordering::Release),
                _ => {}
            }
        }
        self.latest.get()
    }
}

/// The passes' times from their timestamps (`period` nanoseconds a tick).
/// A pair that went backwards (a card that reset its clock) counts as 0.
fn times(stamps: &[u64], period: f64, shadows: bool) -> GpuTimes {
    let pass = |i: usize| -> f64 {
        match (stamps.get(i * 2), stamps.get(i * 2 + 1)) {
            (Some(&start), Some(&end)) if end >= start => (end - start) as f64 * period / 1e6,
            _ => 0.0,
        }
    };
    GpuTimes {
        shadows: if shadows { pass(0) } else { 0.0 },
        models: pass(1),
        last_pass: pass(2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_become_milliseconds_per_pass() {
        // 1 ns a tick: 2 ms of shadows, 5 of models, 1 for the last pass.
        let stamps = [0, 2_000_000, 2_000_000, 7_000_000, 7_000_000, 8_000_000];
        let t = times(&stamps, 1.0, true);
        assert_eq!(
            (t.shadows, t.models, t.last_pass, t.total()),
            (2.0, 5.0, 1.0, 8.0)
        );
        // No shadows drawn: their stamps don't count.
        assert_eq!(times(&stamps, 1.0, false).shadows, 0.0);
        // A clock that went backwards counts as nothing, not a huge time.
        assert_eq!(times(&[5, 1, 0, 0, 0, 0], 1.0, true).shadows, 0.0);
    }
}
