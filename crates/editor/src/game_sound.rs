//! Game code's sounds on their way out of the game (Phase 7, S3).
//!
//! Each tick's requests (`playSound…`, `stopSoundsOn`) are gathered on the
//! host in the tick's fixed order, like moves (decision 0033), and handed
//! to the app after the tick ([`crate::Workbench::take_game_sounds`]),
//! which plays them. Nothing comes back into the game, and nothing here is
//! in a save or a checksum: sound never changes the game (decision 0056).

use reflect::EntityId;

/// The most sound requests kept from one tick; past it they're left out
/// and the Console says so once (decision 0056's limits).
pub const MAX_PER_TICK: usize = 256;

/// What game code asked for.
#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// Play the sound event named here, from there; and the entity whose
    /// code asked (who caused it), if any.
    Play(String, script::SoundAt, Option<EntityId>),
    /// Stop the sounds on this entity.
    StopOn(EntityId),
    /// Set this bus to this share of its fader (0 to 1) for the rest of
    /// the play.
    BusVolume(String, f32),
}

/// One request, with where it stands in the game: the app picks the take
/// from the seed, the tick and the order, so the same game sounds the
/// same every time, and it never touches the game's own random numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct GameSound {
    /// The tick it was asked for in.
    pub tick: u64,
    /// Its place among the tick's requests.
    pub order: u32,
    /// The game's seed.
    pub seed: u64,
    /// What was asked for.
    pub request: Request,
}

/// What the game's sounds should be doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundMode {
    /// No game: fade them out.
    Off,
    /// The game plays.
    Playing,
    /// The game is paused: hold them.
    Paused,
}

/// Adds one tick's `requests` to `out`, numbered in order, keeping at
/// most [`MAX_PER_TICK`]. Returns how many were left out.
pub(crate) fn gather(
    out: &mut Vec<GameSound>,
    requests: Vec<Request>,
    tick: u64,
    seed: u64,
) -> usize {
    let dropped = requests.len().saturating_sub(MAX_PER_TICK);
    out.extend(
        requests
            .into_iter()
            .take(MAX_PER_TICK)
            .enumerate()
            .map(|(order, request)| GameSound {
                tick,
                order: order as u32,
                seed,
                request,
            }),
    );
    dropped
}

impl crate::Workbench {
    /// Game code's sound requests since the last call, in the order asked
    /// for. The app plays them; a long stall's older ticks it may drop.
    pub fn take_game_sounds(&mut self) -> Vec<GameSound> {
        std::mem::take(&mut self.game_sounds)
    }

    /// What the game's sounds should be doing: playing, held while the
    /// game is paused, or faded out when there's no game.
    pub fn game_sound_mode(&self) -> SoundMode {
        match &self.playing {
            None => SoundMode::Off,
            Some(playing) if playing.paused => SoundMode::Paused,
            Some(_) => SoundMode::Playing,
        }
    }

    /// Where the game hears from: its camera (the first Camera entity),
    /// where it's drawn between ticks, as a place and a turn. `None` when
    /// there's no game or no camera.
    pub fn listener(&self) -> Option<(glam::Vec3, glam::Quat)> {
        let playing = self.playing.as_ref()?;
        let t = playing.world.progress();
        let (_, _, global) = playing.world.camera_between(t)?;
        let (_, turn, place) = global.to_scale_rotation_translation();
        Some((place, turn.normalize()))
    }

    /// The game's sounds that keep playing (`AudioSource`s turned on, with
    /// an event named), with where each is drawn now: the app keeps them
    /// going, and lets the far ones rest (Phase 7, S5). Empty with no
    /// game.
    pub fn audio_sources(
        &self,
    ) -> Vec<(reflect::EntityId, area::builtin::AudioSource, glam::Vec3)> {
        let Some(playing) = self.playing.as_ref() else {
            return Vec::new();
        };
        let t = playing.world.progress();
        playing
            .world
            .audio_sources()
            .into_iter()
            .filter(|(_, s)| s.playing && !s.event.is_empty())
            .filter_map(|(id, s)| {
                let place = playing.world.global_between(id, t)?.w_axis.truncate();
                Some((id, s, place))
            })
            .collect()
    }

    /// The game's reverb zones, with where each is drawn now (its place,
    /// turn and scale): the app works out which the listener is in
    /// (Phase 7, S6). Empty with no game.
    pub fn reverb_zones(&self) -> Vec<(area::builtin::ReverbZone, glam::Mat4)> {
        let Some(playing) = self.playing.as_ref() else {
            return Vec::new();
        };
        let t = playing.world.progress();
        playing
            .world
            .placed_between::<area::builtin::ReverbZone>(t)
            .into_iter()
            .map(|(_, zone, place)| (zone, place))
            .collect()
    }

    /// How long the game's sound took last frame, for the Profiler's
    /// Sound line.
    pub fn set_sound_time(&mut self, spent: std::time::Duration) {
        self.sound_spent = spent;
    }

    /// Where an entity of the game is drawn now, between ticks: where
    /// sounds on it are heard from. `None` when it isn't in the game.
    pub fn place_between(&self, id: reflect::EntityId) -> Option<glam::Vec3> {
        let playing = self.playing.as_ref()?;
        let t = playing.world.progress();
        playing
            .world
            .global_between(id, t)
            .map(|m| m.w_axis.truncate())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tick_s_requests_are_numbered_in_order_and_capped() {
        let mut out = Vec::new();
        let many: Vec<Request> = (0..300)
            .map(|i| Request::Play(format!("s{i}"), script::SoundAt::Nowhere, None))
            .collect();
        assert_eq!(gather(&mut out, many, 7, 42), 300 - MAX_PER_TICK);
        assert_eq!(out.len(), MAX_PER_TICK);
        assert_eq!(out[3].order, 3);
        assert_eq!((out[3].tick, out[3].seed), (7, 42));
        assert_eq!(
            out[3].request,
            Request::Play("s3".into(), script::SoundAt::Nowhere, None)
        );
        assert_eq!(gather(&mut out, Vec::new(), 8, 42), 0);
    }
}
