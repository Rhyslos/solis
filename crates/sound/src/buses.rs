//! The mixer's buses on Kira's tracks (Phase 7, S4): one track per bus,
//! nested as the buses are, each with a volume control (its level from
//! [`crate::mixer::levels`]) and then a meter, so the meter reads what
//! comes out after the fader.
//!
//! The game has two trees of them: one its sounds play on (paused with
//! the game), and one for sounds a Step plays while paused. Both follow
//! the same levels, and a bus's meter is the louder of the two.

use crate::device::Manager;
use crate::mixer::Mixer;
use kira::effect::volume_control::{VolumeControlBuilder, VolumeControlHandle};
use kira::effect::{Effect, EffectBuilder};
use kira::info::Info;
use kira::track::{TrackBuilder, TrackHandle};
use kira::{Decibels, Frame, Tween};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// How long a level change takes on the track, so a fader moved or a bus
/// muted never clicks (ducking's own fades are worked out frame by frame
/// in [`crate::mixer::Ducker`]).
const LEVEL_TWEEN: Duration = Duration::from_millis(25);

/// A meter on a track: the loudest sample since it was last read, as an
/// amplitude (1 is full scale), kept in an atomic so the mixing thread
/// never waits.
#[derive(Debug, Clone, Default)]
pub(crate) struct MeterReading(Arc<AtomicU32>);

impl MeterReading {
    /// The loudest since the last call, and starts again.
    pub(crate) fn take(&self) -> f32 {
        f32::from_bits(self.0.swap(0, Ordering::Relaxed))
    }

    /// Notes a peak (for the effect, and tests).
    fn note(&self, peak: f32) {
        // Non-negative floats order as their bits do.
        self.0.fetch_max(peak.to_bits(), Ordering::Relaxed);
    }
}

/// The meter effect, last on a bus's track.
struct Meter(MeterReading);

impl Effect for Meter {
    fn process(&mut self, input: &mut [Frame], _dt: f64, _info: &Info) {
        let peak = input
            .iter()
            .map(|f| f.left.abs().max(f.right.abs()))
            .filter(|v| v.is_finite())
            .fold(0.0_f32, f32::max);
        self.0.note(peak);
    }
}

struct MeterBuilder(MeterReading);

impl EffectBuilder for MeterBuilder {
    type Handle = ();

    fn build(self) -> (Box<dyn Effect>, Self::Handle) {
        (Box::new(Meter(self.0)), ())
    }
}

/// One tree of bus tracks.
pub(crate) struct BusTree {
    /// Each bus's track, in the mixer's order (a parent before what's
    /// inside it).
    pub(crate) tracks: Vec<TrackHandle>,
    volumes: Vec<VolumeControlHandle>,
    meters: Vec<MeterReading>,
}

/// Room on each bus's track for every game sound at once, and a track
/// each when placed.
fn roomy() -> TrackBuilder {
    TrackBuilder::new()
        .sound_capacity(crate::game::MAX_PLAYING)
        .sub_track_capacity(crate::game::MAX_PLAYING)
}

impl BusTree {
    /// The mixer's buses as tracks under `under`. `None` if Kira has no
    /// room (it has, with the capacities set here).
    pub(crate) fn new(under: &mut TrackHandle, mixer: &Mixer, levels: &[f32]) -> Option<Self> {
        let parents = mixer.parents();
        let mut tracks: Vec<TrackHandle> = Vec::with_capacity(mixer.buses.len());
        let mut volumes = Vec::with_capacity(mixer.buses.len());
        let mut meters = Vec::with_capacity(mixer.buses.len());
        for (i, parent) in parents.iter().enumerate() {
            let meter = MeterReading::default();
            let mut builder = roomy();
            let volume = builder.add_effect(VolumeControlBuilder::new(decibels(
                levels.get(i).copied().unwrap_or(0.0),
            )));
            builder.add_effect(MeterBuilder(meter.clone()));
            // Parents come first (the mixer keeps that order), so the
            // parent's track is already made; Master goes under `under`.
            let track = match parent.filter(|p| *p < i) {
                Some(p) => tracks[p].add_sub_track(builder).ok()?,
                None => under.add_sub_track(builder).ok()?,
            };
            tracks.push(track);
            volumes.push(volume);
            meters.push(meter);
        }
        Some(Self {
            tracks,
            volumes,
            meters,
        })
    }

    /// Sets each bus's level, in decibels (silent below -60).
    pub(crate) fn set_levels(&mut self, levels: &[f32]) {
        let tween = Tween {
            duration: LEVEL_TWEEN,
            ..Tween::default()
        };
        for (volume, level) in self.volumes.iter_mut().zip(levels) {
            volume.set_volume(decibels(*level), tween);
        }
    }

    /// Each bus's loudest sample since the last reading.
    pub(crate) fn take_meters(&self) -> Vec<f32> {
        self.meters.iter().map(MeterReading::take).collect()
    }
}

/// A level in decibels as Kira's, with anything not a number, or quieter
/// than Kira's silence, silent.
fn decibels(level: f32) -> Decibels {
    if level.is_finite() && level > Decibels::SILENCE.0 {
        Decibels(level.min(24.0))
    } else {
        Decibels::SILENCE
    }
}

/// A new container track for a tree on `manager`.
pub(crate) fn container(manager: &mut Manager) -> Option<TrackHandle> {
    match manager {
        Manager::Card(m) => m
            .add_sub_track(TrackBuilder::new().sub_track_capacity(4))
            .ok(),
        Manager::Test(m) => m
            .add_sub_track(TrackBuilder::new().sub_track_capacity(4))
            .ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_meter_keeps_the_loudest_sample_until_read() {
        let reading = MeterReading::default();
        let mut meter = Meter(reading.clone());
        let info = kira::info::MockInfoBuilder::new().build();
        let mut frames = vec![Frame::from_mono(0.25), Frame::new(-0.5, 0.1)];
        meter.process(&mut frames, 1.0 / 48_000.0, &info);
        let mut quiet = vec![Frame::from_mono(0.1)];
        meter.process(&mut quiet, 1.0 / 48_000.0, &info);
        assert_eq!(reading.take(), 0.5, "the loudest, either side");
        assert_eq!(reading.take(), 0.0, "starts again once read");
        // It never changes what it hears.
        assert_eq!(quiet[0], Frame::from_mono(0.1));
        let mut odd = vec![Frame::from_mono(f32::NAN), Frame::from_mono(0.2)];
        meter.process(&mut odd, 1.0 / 48_000.0, &info);
        assert_eq!(reading.take(), 0.2, "not-a-number samples are skipped");
    }

    #[test]
    fn silent_and_odd_levels_are_kira_s_silence() {
        assert_eq!(decibels(f32::NEG_INFINITY), Decibels::SILENCE);
        assert_eq!(decibels(f32::NAN), Decibels::SILENCE);
        assert_eq!(decibels(-80.0), Decibels::SILENCE);
        assert_eq!(decibels(-6.0), Decibels(-6.0));
        assert_eq!(decibels(100.0), Decibels(24.0));
    }
}
