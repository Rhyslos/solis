//! The game's sounds, played (Phase 7, S3): each frame, the sounds game
//! code asked for are picked and played, placed around the Game panel's
//! camera, and the sounds on entities follow them.
//!
//! Only what the editor hands out is used: requests and where things are
//! drawn. Nothing goes back into the game (decision 0056).

use editor::game_sound::Request;
use editor::{GameSound, LogLevel, SoundMode, Workbench};
use reflect::{AssetId, EntityId, Id};
use sound::SoundDevice;
use sound::event::Space;
use sound::game::{Placement, STALL_TICKS, Spot, within_reach};
use std::collections::{HashMap, HashSet};

/// The game's sounds between frames: what each event played last (so it
/// isn't played twice running), and what's been said once.
#[derive(Debug, Default)]
pub struct GameAudio {
    /// The mode last frame.
    mode: Option<SoundMode>,
    /// Each event's last take.
    last_take: HashMap<String, AssetId>,
    /// Names already said to be unknown, this play.
    told_unknown: HashSet<String>,
    /// Whether "too many sounds at once" was said, this play.
    told_full: bool,
}

impl GameAudio {
    /// One frame: follows the game's mode, plays what game code asked for,
    /// and moves the listener and the sounds on entities.
    pub fn frame(
        &mut self,
        workbench: &mut Workbench,
        sounds: &crate::sounds::Sounds,
        device: &mut SoundDevice,
    ) {
        let mode = workbench.game_sound_mode();
        if self.mode != Some(mode) {
            match mode {
                SoundMode::Off => {
                    device.stop_game_sounds();
                    self.last_take.clear();
                    self.told_unknown.clear();
                    self.told_full = false;
                }
                SoundMode::Paused => device.pause_game_sounds(true),
                SoundMode::Playing => device.pause_game_sounds(false),
            }
            self.mode = Some(mode);
        }
        let requests = workbench.take_game_sounds();
        if mode == SoundMode::Off {
            return;
        }
        let listener = workbench.listener();
        if let Some((place, turn)) = listener {
            device.set_listener(place.to_array(), turn.to_array());
        }
        let hearing = listener.map_or([0.0; 3], |(p, _)| p.to_array());
        for request in fresh(requests) {
            match request.request {
                Request::StopOn(which) => device.stop_game_sounds_on(which.0.raw()),
                Request::Play(ref name, at) => {
                    let notes = self.play(&request, name, at, hearing, workbench, sounds, device);
                    for note in notes {
                        workbench.log(LogLevel::Warning, note);
                    }
                }
            }
        }
        let on = device.game_sound_entities();
        if !on.is_empty() {
            let places: HashMap<u64, [f32; 3]> = on
                .into_iter()
                .filter_map(|raw| {
                    let id = EntityId(Id::from_raw(raw)?);
                    Some((raw, workbench.place_between(id)?.to_array()))
                })
                .collect();
            device.move_game_sounds(|raw| places.get(&raw).copied());
        }
    }

    /// Plays one request; returns anything to say in the Console.
    #[allow(clippy::too_many_arguments)]
    fn play(
        &mut self,
        request: &GameSound,
        name: &str,
        at: script::SoundAt,
        hearing: [f32; 3],
        workbench: &Workbench,
        sounds: &crate::sounds::Sounds,
        device: &mut SoundDevice,
    ) -> Vec<String> {
        let Some((key, event)) = sounds.event_named(name) else {
            return if self.told_unknown.insert(name.to_owned()) {
                vec![format!(
                    "Game code played the sound event \"{name}\", but there's none by that name \
                     (or it couldn't be read): a sound event's name is its file in assets, \
                     without .solsound, such as \"sfx/footstep\". Said once."
                )]
            } else {
                Vec::new()
            };
        };
        // Only takes that are sound files Solis could read.
        let playable = sound::SoundEvent {
            takes: event
                .takes
                .iter()
                .copied()
                .filter(|t| sounds.clip_of(*t).is_some())
                .collect(),
            ..event.clone()
        };
        let last = self.last_take.get(key).copied();
        let Some(pick) = sound::event::pick(
            &playable,
            last,
            request.seed,
            request.tick,
            u64::from(request.order),
        ) else {
            return Vec::new();
        };
        let Some(clip) = sounds.clip_of(pick.take) else {
            return Vec::new();
        };
        let spot = |place: glam::Vec3| Spot {
            place: place.to_array(),
            near_m: event.near_m,
            far_m: event.far_m,
            fade: event.fade,
        };
        let placement = match (event.space, at) {
            (Space::Flat, _) | (_, script::SoundAt::Nowhere) => Placement::Flat,
            (Space::Placed, script::SoundAt::Place(place)) => Placement::At(spot(place)),
            (Space::Placed, script::SoundAt::On(which)) => match workbench.place_between(which) {
                Some(place) => Placement::On(which.0.raw(), spot(place)),
                // Not in the game (any more): nothing to play it on.
                None => return Vec::new(),
            },
        };
        // Too far away to hear: never started (decision 0056).
        if let Placement::At(s) | Placement::On(_, s) = placement
            && !within_reach(s.place, hearing, s.far_m)
        {
            return Vec::new();
        }
        let key = key.to_owned();
        match device.play_game_sound(clip, &pick, placement) {
            Ok(()) => {
                self.last_take.insert(key, pick.take);
                Vec::new()
            }
            Err(why) if !self.told_full => {
                self.told_full = true;
                vec![format!("A game sound didn't play: {why}. Said once.")]
            }
            Err(_) => Vec::new(),
        }
    }
}

/// The requests worth playing: after a stall (many ticks in one frame),
/// only the last few ticks' sounds play, not a pile-up; stopping always
/// counts.
fn fresh(requests: Vec<GameSound>) -> impl Iterator<Item = GameSound> {
    let newest = requests.iter().map(|r| r.tick).max().unwrap_or(0);
    requests
        .into_iter()
        .filter(move |r| matches!(r.request, Request::StopOn(_)) || r.tick + STALL_TICKS >= newest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play(tick: u64) -> GameSound {
        GameSound {
            tick,
            order: 0,
            seed: 1,
            request: Request::Play("step".into(), script::SoundAt::Nowhere),
        }
    }

    #[test]
    fn after_a_stall_only_the_last_ticks_play() {
        let stop = GameSound {
            request: Request::StopOn(EntityId(Id::from_raw(5).unwrap())),
            ..play(1)
        };
        let kept: Vec<u64> = fresh(vec![play(1), play(5), play(9), play(10), stop])
            .map(|r| r.tick)
            .collect();
        assert_eq!(kept, vec![9, 10, 1], "the old stop still counts");
        assert_eq!(fresh(Vec::new()).count(), 0);
    }
}
