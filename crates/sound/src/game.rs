//! The game's sounds on the sound device (Phase 7, S3): one-shots and
//! sounds on entities, placed in 3D around a listener (the Game panel's
//! camera), paused with the game and faded out on Stop.
//!
//! Everything here is told what to do by the app after each tick; nothing
//! here reads or changes the game (decision 0056: sound never changes the
//! game).
//!
//! **Placement.** Positions and the listener's turn are Solis's world
//! coordinates as they are: Kira places a listener's ears on its local
//! ±X with +Y up, which is how a Camera entity is turned (it looks along
//! its -Z with +Y up), so the camera's world turn is the listener's.
//! Nothing is converted, so nothing can be converted wrongly.

use crate::device::{FileStream, Manager, Preview, memory_sound};
use crate::event::{Fade, Pick};
use crate::read::{Clip, Samples};
use kira::listener::ListenerHandle;
use kira::sound::streaming::StreamingSoundData;
use kira::track::{SpatialTrackBuilder, SpatialTrackDistances, TrackBuilder, TrackHandle};
use kira::{Decibels, Easing, PlaybackRate, Tween};
use std::time::Duration;

/// How long Stop takes to fade the game's sounds out.
pub const STOP_FADE: Duration = Duration::from_millis(250);
/// How long `stopSoundsOn` takes to fade an entity's sounds out.
pub const STOP_ON_FADE: Duration = Duration::from_millis(100);
/// How long pausing and carrying on take, so neither clicks.
const PAUSE_FADE: Duration = Duration::from_millis(40);
/// The most game sounds playing at once; past it, new ones are skipped
/// (S5 chooses which to keep by priority).
pub const MAX_PLAYING: usize = 256;

/// Where a game sound plays from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Placement {
    /// The same everywhere (a 2D event, or one played without a place).
    Flat,
    /// At a place in the world, which stays put.
    At(Spot),
    /// On an entity (by its ID's number), following it: the app moves it
    /// each frame with [`crate::SoundDevice::move_game_sounds`].
    On(u64, Spot),
}

/// A placed sound's place and how it fades with distance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spot {
    /// Where it is, in the world.
    pub place: [f32; 3],
    /// Full volume up to here, in metres.
    pub near_m: f32,
    /// Silent past here, in metres.
    pub far_m: f32,
    /// How it gets quieter between the two.
    pub fade: Fade,
}

/// How many ticks of one-shots are kept after a stall: older ones are
/// dropped rather than all playing at once.
pub const STALL_TICKS: u64 = 3;

/// One game sound playing: its track (for a placed one) and itself.
struct Playing {
    /// The entity it's on, by its ID's number.
    on: Option<u64>,
    /// Its own track, when it's placed (Kira places a track, not a sound).
    track: Option<kira::track::SpatialTrackHandle>,
    /// The sound itself.
    sound: Preview,
}

/// The game's tracks on the device: one the game's sounds play under
/// (paused with the game), one for sounds a Step plays while paused, and
/// the listener.
pub(crate) struct GameTracks {
    track: TrackHandle,
    stepping: TrackHandle,
    listener: ListenerHandle,
    playing: Vec<Playing>,
    paused: bool,
}

impl GameTracks {
    /// The game's tracks and listener on a new manager, or `None` if Kira
    /// has no room (it always does on a fresh one).
    pub(crate) fn new(manager: &mut Manager) -> Option<Self> {
        let origin = mint::Vector3::from([0.0_f32; 3]);
        let facing = mint::Quaternion::from([0.0_f32, 0.0, 0.0, 1.0]);
        // Room for every sound playing at once, and a track each when
        // placed.
        let roomy = || {
            TrackBuilder::new()
                .sound_capacity(MAX_PLAYING)
                .sub_track_capacity(MAX_PLAYING)
        };
        let (track, stepping, listener) = match manager {
            Manager::Card(m) => (
                m.add_sub_track(roomy()).ok()?,
                m.add_sub_track(roomy()).ok()?,
                m.add_listener(origin, facing).ok()?,
            ),
            Manager::Test(m) => (
                m.add_sub_track(roomy()).ok()?,
                m.add_sub_track(roomy()).ok()?,
                m.add_listener(origin, facing).ok()?,
            ),
        };
        Some(Self {
            track,
            stepping,
            listener,
            playing: Vec::new(),
            paused: false,
        })
    }

    fn tidy(&mut self) {
        self.playing
            .retain(|p| p.sound.state() != kira::sound::PlaybackState::Stopped);
    }
}

impl crate::SoundDevice {
    /// Plays one pick of a game's sound event, from `placement`. While
    /// paused (a Step), it plays without waiting for the game to carry on.
    /// Silent devices do nothing.
    pub fn play_game_sound(
        &mut self,
        clip: &Clip,
        pick: &Pick,
        placement: Placement,
    ) -> Result<(), String> {
        let Some(game) = self.game.as_mut() else {
            return Ok(());
        };
        game.tidy();
        if game.playing.len() >= MAX_PLAYING {
            return Err(format!("more than {MAX_PLAYING} game sounds at once"));
        }
        let volume = Decibels(if pick.volume_db.is_finite() {
            pick.volume_db
        } else {
            0.0
        });
        let speed = pick.speed();
        let rate = PlaybackRate(if speed.is_finite() && speed > 0.0 {
            speed
        } else {
            1.0
        });
        let parent = if game.paused {
            &mut game.stepping
        } else {
            &mut game.track
        };
        let (on, spot) = match placement {
            Placement::Flat => (None, None),
            Placement::At(spot) => (None, Some(spot)),
            Placement::On(entity, spot) => (Some(entity), Some(spot)),
        };
        let mut track = match spot {
            None => None,
            Some(spot) => {
                let near = spot.near_m.max(0.0);
                let far = spot.far_m.max(near + 0.01);
                let builder = SpatialTrackBuilder::new()
                    .distances(SpatialTrackDistances {
                        min_distance: near,
                        max_distance: far,
                    })
                    .attenuation_function(easing(spot.fade))
                    .persist_until_sounds_finish(true);
                let place = mint::Vector3::from(finite(spot.place));
                Some(
                    parent
                        .add_spatial_sub_track(game.listener.id(), place, builder)
                        .map_err(|e| format!("couldn't place it ({e})"))?,
                )
            }
        };
        let sound = match &clip.samples {
            Samples::Memory(frames) => {
                let data = memory_sound(clip.info, frames.clone())
                    .volume(volume)
                    .playback_rate(rate);
                let handle = match track.as_mut() {
                    Some(t) => t.play(data),
                    None => parent.play(data),
                };
                Preview::Memory(handle.map_err(|e| format!("couldn't play it ({e})"))?)
            }
            Samples::Streamed(path) => {
                let stream = FileStream::open(path, clip.info).map_err(|e| e.to_string())?;
                let data = StreamingSoundData::from_decoder(stream)
                    .volume(volume)
                    .playback_rate(rate);
                let handle = match track.as_mut() {
                    Some(t) => t.play(data),
                    None => parent.play(data),
                };
                Preview::Stream(handle.map_err(|e| format!("couldn't play it ({e})"))?)
            }
        };
        game.playing.push(Playing { on, track, sound });
        Ok(())
    }

    /// Puts the listener (the Game panel's camera) at `place`, turned by
    /// `turn` (x, y, z, w), smoothly to the next frame.
    pub fn set_listener(&mut self, place: [f32; 3], turn: [f32; 4]) {
        let Some(game) = self.game.as_mut() else {
            return;
        };
        let turn = if turn.iter().all(|v| v.is_finite()) && turn.iter().any(|v| *v != 0.0) {
            turn
        } else {
            [0.0, 0.0, 0.0, 1.0]
        };
        game.listener
            .set_position(mint::Vector3::from(finite(place)), Tween::default());
        game.listener
            .set_orientation(mint::Quaternion::from(turn), Tween::default());
    }

    /// Moves the sounds on entities to where `place_of` says each entity
    /// is now; an entity gone (`None`) leaves its sounds where they were.
    pub fn move_game_sounds(&mut self, place_of: impl Fn(u64) -> Option<[f32; 3]>) {
        let Some(game) = self.game.as_mut() else {
            return;
        };
        game.tidy();
        for playing in &mut game.playing {
            if let (Some(on), Some(track)) = (playing.on, playing.track.as_mut())
                && let Some(place) = place_of(on)
            {
                track.set_position(mint::Vector3::from(finite(place)), Tween::default());
            }
        }
    }

    /// The entities game sounds are playing on, by their IDs' numbers (to
    /// ask where they are each frame).
    pub fn game_sound_entities(&mut self) -> Vec<u64> {
        let Some(game) = self.game.as_mut() else {
            return Vec::new();
        };
        game.tidy();
        let mut on: Vec<u64> = game.playing.iter().filter_map(|p| p.on).collect();
        on.sort_unstable();
        on.dedup();
        on
    }

    /// Stops the sounds on an entity, with a short fade.
    pub fn stop_game_sounds_on(&mut self, entity: u64) {
        let Some(game) = self.game.as_mut() else {
            return;
        };
        for playing in game.playing.iter_mut().filter(|p| p.on == Some(entity)) {
            playing.sound.stop_over(STOP_ON_FADE);
        }
    }

    /// Pauses the game's sounds (the game paused), or carries them on.
    pub fn pause_game_sounds(&mut self, paused: bool) {
        let Some(game) = self.game.as_mut() else {
            return;
        };
        if game.paused == paused {
            return;
        }
        game.paused = paused;
        let fade = Tween {
            duration: PAUSE_FADE,
            ..Tween::default()
        };
        if paused {
            game.track.pause(fade);
        } else {
            game.track.resume(fade);
        }
    }

    /// Fades every game sound out (Stop), and carries on unpaused for the
    /// next play.
    pub fn stop_game_sounds(&mut self) {
        let Some(game) = self.game.as_mut() else {
            return;
        };
        for playing in &mut game.playing {
            playing.sound.stop_over(STOP_FADE);
        }
        game.playing.clear();
        if game.paused {
            game.paused = false;
            game.track.resume(Tween::default());
        }
    }

    /// How many game sounds are playing (tests, and the Profiler later).
    pub fn game_sounds_playing(&mut self) -> usize {
        self.game.as_mut().map_or(0, |g| {
            g.tidy();
            g.playing.len()
        })
    }
}

/// How a fade sounds, as Kira's curve over the distance between near and
/// far (Kira measures it in decibels, from full to silent).
fn easing(fade: Fade) -> Option<Easing> {
    match fade {
        // Even in decibels: quick at first in loudness, then slow, as
        // sound outdoors.
        Fade::Realistic => Some(Easing::Linear),
        // Close to even in loudness itself.
        Fade::Even => Some(Easing::OutPowi(2)),
        // Full volume; the app doesn't start it past the far distance.
        Fade::None => None,
    }
}

/// A place with anything not a number put at 0.
fn finite(p: [f32; 3]) -> [f32; 3] {
    p.map(|v| if v.is_finite() { v } else { 0.0 })
}

/// Whether a sound placed at `place` can be heard from `listener` at all:
/// one past its far distance isn't started (decision 0056: sounds too far
/// away to hear are never started).
pub fn within_reach(place: [f32; 3], listener: [f32; 3], far_m: f32) -> bool {
    let d: f32 = place
        .iter()
        .zip(listener)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>()
        .sqrt();
    d.is_finite() && d <= far_m
}
