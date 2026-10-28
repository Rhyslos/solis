//! The game's sounds, played (Phase 7, S3 to S5): each frame, the sounds
//! game code asked for and the `AudioSource`s turned on are gathered,
//! the most important are chosen to be heard (the project's voices, 64
//! unless the Mixer says otherwise), and they're played around the Game
//! panel's camera, following their entities.
//!
//! - **One-shots** (`playSound…`): one event asked for many times in a
//!   tick plays a few copies, a little louder; each event's "at once"
//!   limit holds; one not chosen isn't played.
//! - **Loops** (`AudioSource`): one not chosen rests (it isn't mixed),
//!   and comes back where it would have been, by a clock that runs while
//!   the game plays.
//! - **Reverb zones** (S6): the zones the camera is in decide how much of
//!   each zone's echo every sound placed in the world gets.
//! - **Buses** (S4): each sound plays through its event's bus, with
//!   ducking worked out here each frame, mute and solo from the Mixer, and
//!   game code's `setBusVolume` shares for the play; the meters go back
//!   to the Mixer.
//!
//! Only what the editor hands out is used: requests, the audio sources
//! and where things are drawn. Nothing goes back into the game (decision
//! 0056).

use editor::game_sound::Request;
use editor::{GameSound, LogLevel, SoundMode, Workbench};
use eframe::egui;
use reflect::{AssetId, EntityId, Id};
use sound::SoundDevice;
use sound::event::{Priority, Space};
use sound::game::{Placement, STALL_TICKS, Spot, Voice};
use sound::voices::{Candidate, Key, choose, copies, loop_position, loudness_at};
use std::collections::{HashMap, HashSet};

/// A loop the game has going: when it started (by the sound clock) and
/// which event, so one coming back carries on where it would be.
#[derive(Debug, Clone)]
struct LoopState {
    /// The event's file, as the sounds know it.
    event: String,
    /// The name the AudioSource gives (as written), so the next frame finds
    /// the event without working its file name out again.
    named: String,
    started: f64,
    /// Its pick (before the AudioSource's own volume), and the sounds'
    /// [`changes`](crate::sounds::Sounds::changes) it was made at: picked
    /// again only when the event or its files change.
    pick: Option<(u64, sound::Pick)>,
}

/// How much louder a loop already playing counts when choosing what's
/// heard: one at the edge of the voice limit doesn't stop and start every
/// frame as it and another trade places (the Phase 7 review).
const KEEP_PLAYING_DB: f32 = 1.0;

/// The most unknown sound event names said once each in a play: past it,
/// no more are said (game code making up names can't grow the list).
const MAX_TOLD: usize = 256;

/// The game's sounds between frames.
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
    /// Game code's `setBusVolume` shares, by bus name, this play.
    shares: HashMap<String, f32>,
    /// Ducking as it goes.
    ducker: sound::mixer::Ducker,
    /// When the last frame was, for ducking's fades.
    last_frame: Option<std::time::Instant>,
    /// Seconds the game's sound has played this play (not while paused).
    clock: f64,
    /// The loops going, by entity (heard or resting).
    loops: HashMap<u64, LoopState>,
    /// A number for each event's name, to count its plays on the device.
    event_numbers: HashMap<String, u64>,
    /// How long the last frame's sound took, in seconds (the Profiler).
    spent: f64,
    /// The play the sounds belong to (the editor's play number): a new one
    /// starts the game's sound afresh, even with no Stop seen between.
    play: Option<u64>,
    /// Loops whose AudioSource changed its event this frame, to start
    /// again with the new one.
    changed_loops: Vec<u64>,
}

/// One sound that wants to play this frame, worked out: what to play,
/// from where, how loud it would be, and how it ranks.
struct Wanted<'a> {
    key: Key,
    name: &'a str,
    event: &'a sound::SoundEvent,
    clip: &'a sound::Clip,
    pick: sound::Pick,
    placement: Placement,
    loudness_db: f32,
    extra_db: f32,
    /// The entity it belongs to, whether or not it's placed.
    on: Option<u64>,
}

impl GameAudio {
    /// One frame: follows the game's mode, chooses and plays what's
    /// heard, and moves the listener and the sounds on entities.
    pub fn frame(
        &mut self,
        workbench: &mut Workbench,
        sounds: &crate::sounds::Sounds,
        device: &mut SoundDevice,
        ctx: &egui::Context,
    ) {
        let began = std::time::Instant::now();
        let dt = self
            .last_frame
            .map_or(0.0, |then| began.duration_since(then).as_secs_f32());
        self.last_frame = Some(began);
        let mode = workbench.game_sound_mode();
        // A new play (a replay, Play again, a loaded game) starts afresh,
        // even when this frame never saw the game stopped.
        let play = workbench.play_number();
        if self.play != Some(play) {
            self.play = Some(play);
            if self.mode.is_some_and(|m| m != SoundMode::Off) && mode != SoundMode::Off {
                self.mode = None;
                self.reset(device);
            }
        }
        if self.mode != Some(mode) {
            match mode {
                SoundMode::Off => self.reset(device),
                SoundMode::Paused => device.pause_game_sounds(true),
                SoundMode::Playing => device.pause_game_sounds(false),
            }
            self.mode = Some(mode);
        }
        let requests = workbench.take_game_sounds();
        device.set_mixer(workbench.mixer());
        if mode == SoundMode::Off {
            device.set_reverb(&[]);
            self.mix(workbench, device, dt, ctx);
            self.spent = began.elapsed().as_secs_f64();
            return;
        }
        if mode == SoundMode::Playing {
            self.clock += f64::from(dt);
        }
        // Who hears (S8): the first Listener with Hears now on, or the
        // camera.
        let hearer = workbench.hearer();
        if let Some(h) = &hearer {
            device.set_listener(h.place.to_array(), h.turn.to_array());
        }
        let hearing = hearer.as_ref().map_or([0.0; 3], |h| h.place.to_array());
        device.set_reverb(&reverbs_heard(&workbench.reverb_zones(), hearing));
        let mut notes = Vec::new();
        let mut plays: Vec<(GameSound, String, script::SoundAt, Option<EntityId>)> = Vec::new();
        // In the order asked for: a stop also stops what was asked to
        // play on that entity earlier this frame.
        for request in fresh(requests) {
            match request.request {
                Request::StopOn(which) => {
                    device.stop_game_sounds_on(which.0.raw());
                    plays.retain(
                        |(_, _, at, _)| !matches!(at, script::SoundAt::On(e) if *e == which),
                    );
                }
                Request::BusVolume(ref bus, share) => {
                    // Only a bus the project has: game code naming made-up
                    // buses every tick can't grow this without end (the
                    // Phase 7 review).
                    if workbench.mixer().index_of(bus).is_some() {
                        self.shares.insert(bus.clone(), share);
                    }
                }
                Request::Play(ref name, at, caller) => {
                    let name = name.clone();
                    plays.push((request, name, at, caller));
                }
            }
        }
        // What wants to be heard: this frame's one-shots and the loops.
        let mut wanted: Vec<Wanted> = Vec::new();
        self.shots(
            &plays,
            (hearing, hearer.as_ref()),
            workbench,
            sounds,
            device,
            &mut wanted,
            &mut notes,
        );
        let sources = workbench.audio_sources();
        self.loop_candidates(
            &sources,
            (hearing, hearer.as_ref()),
            workbench,
            sounds,
            &mut wanted,
            &mut notes,
        );
        let already: HashSet<u64> = device.loops_playing().into_iter().collect();
        let candidates: Vec<Candidate> = wanted
            .iter()
            .map(|w| Candidate {
                key: w.key,
                priority: w.event.priority,
                loudness_db: match w.key {
                    Key::Loop(entity) if already.contains(&entity) => {
                        w.loudness_db + KEEP_PLAYING_DB
                    }
                    _ => w.loudness_db,
                },
            })
            .collect();
        let voices = workbench.mixer().voices as usize;
        let room = voices.saturating_sub(device.one_shots_playing());
        let heard: HashSet<Key> = choose(&candidates, room).into_iter().collect();
        // Loops that went, were turned off or rest stop (resting ones keep
        // their place by the clock).
        let going: HashSet<u64> = sources.iter().map(|(id, _, _)| id.0.raw()).collect();
        self.loops.retain(|entity, _| going.contains(entity));
        for entity in device.loops_playing() {
            if !heard.contains(&Key::Loop(entity)) {
                device.stop_loop_on(entity);
            }
        }
        // An AudioSource given another event starts again with it.
        for entity in std::mem::take(&mut self.changed_loops) {
            device.stop_loop_on(entity);
        }
        let real: HashSet<u64> = device.loops_playing().into_iter().collect();
        for w in wanted.iter().filter(|w| heard.contains(&w.key)) {
            let event_number = self.event_number(w.name);
            let voice = |looping, source, start_s| Voice {
                bus: workbench.mixer().bus_for(&w.event.bus),
                event: event_number,
                looping,
                source,
                start_s,
                extra_db: w.extra_db,
                on: w.on,
            };
            let result = match w.key {
                Key::Loop(entity) if real.contains(&entity) => continue,
                Key::Loop(entity) => {
                    let state = self.loops.get(&entity);
                    // Where it would be: time played, at its pick's speed.
                    let start = state.map_or(0.0, |s| {
                        loop_position(
                            (self.clock - s.started) * w.pick.speed(),
                            w.clip.info.seconds(),
                        )
                    });
                    device.play_game_sound(w.clip, &w.pick, w.placement, voice(true, true, start))
                }
                // An event set to Loop plays on until stopped
                // (`stopSoundsOn`) or the game stops.
                Key::Shot(_) => device.play_game_sound(
                    w.clip,
                    &w.pick,
                    w.placement,
                    voice(w.event.looping, false, 0.0),
                ),
            };
            match result {
                Ok(()) => {
                    if let Key::Shot(_) = w.key {
                        self.last_take.insert(w.name.to_owned(), w.pick.take);
                    }
                }
                Err(why) if !self.told_full => {
                    self.told_full = true;
                    notes.push(format!("A game sound didn't play: {why}. Said once."));
                }
                Err(_) => {}
            }
        }
        for note in notes {
            workbench.log(LogLevel::Warning, note);
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
        self.mix(workbench, device, dt, ctx);
        self.spent = began.elapsed().as_secs_f64();
    }

    /// Everything of a play forgotten, and its sounds faded out.
    fn reset(&mut self, device: &mut SoundDevice) {
        device.stop_game_sounds();
        self.last_take.clear();
        self.told_unknown.clear();
        self.told_full = false;
        self.shares.clear();
        self.loops.clear();
        self.changed_loops.clear();
        self.clock = 0.0;
    }

    /// How long the last frame's sound took, in seconds (the Profiler's
    /// How many bus shares and unknown names it holds (tests).
    #[cfg(test)]
    pub(crate) fn held(&self) -> (usize, usize) {
        (self.shares.len(), self.told_unknown.len())
    }

    /// Sound line).
    pub fn spent(&self) -> f64 {
        self.spent
    }

    /// A number for an event's name, the same all play.
    fn event_number(&mut self, name: &str) -> u64 {
        if let Some(&n) = self.event_numbers.get(name) {
            return n;
        }
        let next = self.event_numbers.len() as u64 + 1;
        self.event_numbers.insert(name.to_owned(), next);
        next
    }

    /// This frame's one-shots, worked out: a pile of one event in one tick
    /// becomes a few louder copies, an event's "at once" limit holds, and
    /// one out of earshot is left out.
    #[allow(clippy::too_many_arguments)]
    fn shots<'a>(
        &mut self,
        plays: &'a [(GameSound, String, script::SoundAt, Option<EntityId>)],
        (hearing, hearer): ([f32; 3], Option<&editor::Hearer>),
        workbench: &Workbench,
        sounds: &'a crate::sounds::Sounds,
        device: &mut SoundDevice,
        wanted: &mut Vec<Wanted<'a>>,
        notes: &mut Vec<String>,
    ) {
        // How many times each event was asked for in each tick.
        let mut asked: HashMap<(u64, &str), usize> = HashMap::new();
        for (request, name, _, _) in plays {
            *asked.entry((request.tick, name.as_str())).or_default() += 1;
        }
        let mut started: HashMap<(u64, &str), usize> = HashMap::new();
        let mut this_frame: HashMap<&str, usize> = HashMap::new();
        for (i, (request, name, at, caller)) in plays.iter().enumerate() {
            let group = (request.tick, name.as_str());
            let (keep, extra_db) = copies(asked[&group]);
            let n = started.entry(group).or_default();
            if *n >= keep {
                continue;
            }
            *n += 1;
            let Some((key, event)) = sounds.event_named(name) else {
                if self.told_unknown.len() < MAX_TOLD && self.told_unknown.insert(name.clone()) {
                    notes.push(format!(
                        "Game code played the sound event \"{name}\", but there's none by \
                         that name (or it couldn't be read): a sound event's name is its file \
                         in assets, without .solsound, such as \"sfx/footstep\". Said once."
                    ));
                }
                continue;
            };
            // Not for this listener: never started.
            if !heard_by(event, *caller, hearer, workbench) {
                continue;
            }
            // The event's own limit, counting what plays and what's
            // starting this frame.
            let number = self.event_number(key);
            let now =
                device.game_sounds_of_event(number) + this_frame.get(key).copied().unwrap_or(0);
            if now >= event.at_once as usize {
                continue;
            }
            let last = self.last_take.get(key).copied();
            let Some((pick, clip)) = pick_of(
                event,
                sounds,
                last,
                request.seed,
                request.tick,
                u64::from(request.order),
            ) else {
                continue;
            };
            let spot = |place: glam::Vec3| Spot {
                place: place.to_array(),
                near_m: event.near_m,
                far_m: event.far_m,
                fade: event.fade,
            };
            let placement = match (event.space, at) {
                (Space::Flat, _) | (_, script::SoundAt::Nowhere) => Placement::Flat,
                (Space::Placed, script::SoundAt::Place(place)) => Placement::At(spot(*place)),
                (Space::Placed, script::SoundAt::On(which)) => {
                    match workbench.place_between(*which) {
                        Some(place) => Placement::On(which.0.raw(), spot(place)),
                        // Not in the game (any more): nothing to play it on.
                        None => continue,
                    }
                }
            };
            let loudness_db = loudness(&placement, pick.volume_db + extra_db, hearing);
            // Too far away to hear: never started (decision 0056).
            if !loudness_db.is_finite() {
                continue;
            }
            *this_frame.entry(key).or_default() += 1;
            wanted.push(Wanted {
                key: Key::Shot(i),
                name: key,
                event,
                clip,
                pick,
                placement,
                loudness_db,
                extra_db,
                on: match at {
                    script::SoundAt::On(which) => Some(which.0.raw()),
                    _ => None,
                },
            });
        }
    }

    /// The loops turned on, worked out: each keeps its first pick (its
    /// take and how it's varied), and a new one starts its clock.
    fn loop_candidates<'a>(
        &mut self,
        sources: &[(EntityId, area::builtin::AudioSource, glam::Vec3)],
        (hearing, hearer): ([f32; 3], Option<&editor::Hearer>),
        workbench: &Workbench,
        sounds: &'a crate::sounds::Sounds,
        wanted: &mut Vec<Wanted<'a>>,
        notes: &mut Vec<String>,
    ) {
        for (id, source, place) in sources {
            let entity = id.0.raw();
            // The same event as last frame: found by its file directly.
            let known = self
                .loops
                .get(&entity)
                .filter(|s| s.named == source.event)
                .and_then(|s| sounds.event_at(&s.event));
            let Some((key, event)) = known.or_else(|| sounds.event_named(&source.event)) else {
                if self.told_unknown.len() < MAX_TOLD
                    && self.told_unknown.insert(source.event.clone())
                {
                    notes.push(format!(
                        "An AudioSource plays the sound event \"{}\", but there's none by that \
                         name (or it couldn't be read). Said once.",
                        source.event
                    ));
                }
                continue;
            };
            // Its entity is who causes it.
            if !heard_by(event, Some(*id), hearer, workbench) {
                continue;
            }
            // The same take each time it comes back: picked from the
            // entity, never the game's random numbers; kept from last
            // frame while nothing changed.
            let changes = sounds.changes();
            let kept = self
                .loops
                .get(&entity)
                .filter(|s| s.event == key)
                .and_then(|s| s.pick)
                .filter(|(at, _)| *at == changes)
                .and_then(|(_, pick)| Some((pick, sounds.clip_of(pick.take)?)));
            let Some((pick, clip)) = kept.or_else(|| pick_of(event, sounds, None, 0, 0, entity))
            else {
                continue;
            };
            let state = self.loops.entry(entity).or_insert_with(|| LoopState {
                event: key.to_owned(),
                named: source.event.clone(),
                started: self.clock,
                pick: None,
            });
            if state.event != key {
                // A different event now: it starts from the top, with the
                // new event.
                *state = LoopState {
                    event: key.to_owned(),
                    named: source.event.clone(),
                    started: self.clock,
                    pick: None,
                };
                self.changed_loops.push(entity);
            } else if state.named != source.event {
                state.named = source.event.clone();
            }
            state.pick = Some((changes, pick));
            let placement = match event.space {
                Space::Flat => Placement::Flat,
                Space::Placed => Placement::On(
                    entity,
                    Spot {
                        place: place.to_array(),
                        near_m: event.near_m,
                        far_m: event.far_m,
                        fade: event.fade,
                    },
                ),
            };
            let pick = sound::Pick {
                volume_db: pick.volume_db + source.volume_db,
                ..pick
            };
            wanted.push(Wanted {
                key: Key::Loop(entity),
                name: key,
                event,
                clip,
                pick,
                placement,
                loudness_db: loudness(&placement, pick.volume_db, hearing),
                extra_db: 0.0,
                on: Some(entity),
            });
        }
    }

    /// Ducking, mute, solo and game code's shares as the buses' levels,
    /// and the meters back to the Mixer.
    fn mix(
        &mut self,
        workbench: &mut Workbench,
        device: &mut SoundDevice,
        dt: f32,
        ctx: &egui::Context,
    ) {
        let mixer = workbench.mixer();
        let busy = device.busy_buses(mixer.buses.len());
        let ducked = self.ducker.step(mixer, &busy, dt).to_vec();
        let shares: Vec<f32> = mixer
            .buses
            .iter()
            .map(|b| self.shares.get(&b.name).copied().unwrap_or(1.0))
            .collect();
        let levels = sound::mixer::levels(mixer, &shares, &ducked, workbench.listening());
        device.set_bus_levels(&levels);
        device.set_reverb_volume(levels.first().copied().unwrap_or(0.0));
        let peaks = device.take_bus_meters();
        workbench.set_bus_meters(&peaks, &ducked, dt, ctx);
    }
}

/// The reverbs heard from `hearing` among `zones` (each with where it's
/// drawn now): each zone's preset and how much of it.
fn reverbs_heard(
    zones: &[(area::builtin::ReverbZone, glam::Mat4)],
    hearing: [f32; 3],
) -> Vec<(sound::reverb::Preset, f32)> {
    use sound::reverb::{Preset, Shape, ZoneHere};
    let here: Vec<ZoneHere> = zones
        .iter()
        .map(|(zone, place)| {
            // The listener in the zone's own space, so a turned or
            // scaled zone is measured as it's drawn.
            let listener = place
                .inverse()
                .transform_point3(glam::Vec3::from_array(hearing))
                .to_array();
            ZoneHere {
                shape: match zone.shape {
                    area::builtin::ZoneShape::Box => Shape::Box(zone.size.to_array()),
                    area::builtin::ZoneShape::Ball => Shape::Ball(zone.radius),
                },
                soft_m: zone.soft_edge,
                preset: Preset::named(&zone.preset).unwrap_or_default(),
                strength: zone.strength,
                listener,
            }
        })
        .collect();
    sound::reverb::mix(&here)
        .into_iter()
        .map(|(i, amount)| (here[i].preset, amount))
        .collect()
}

/// Whether `hearer` hears `event` played by `caller` (decision 0058):
/// everyone hears Everyone's; Only who caused it reaches the listener on
/// the entity that played it, or on what it's part of (or what's part of
/// it); Groups reach a listener in one of the event's groups. With no one
/// hearing, nothing is heard.
fn heard_by(
    event: &sound::SoundEvent,
    caller: Option<EntityId>,
    hearer: Option<&editor::Hearer>,
    workbench: &Workbench,
) -> bool {
    use sound::event::HeardBy;
    let Some(hearer) = hearer else {
        return event.heard_by == HeardBy::Everyone;
    };
    match event.heard_by {
        HeardBy::Everyone => true,
        HeardBy::Caller => caller.is_some_and(|c| workbench.same_family(c, hearer.entity)),
        HeardBy::Groups => event.groups.iter().any(|g| hearer.groups.contains(g)),
    }
}

/// A pick of `event` from takes Solis could read, and its clip.
fn pick_of<'a>(
    event: &sound::SoundEvent,
    sounds: &'a crate::sounds::Sounds,
    last: Option<AssetId>,
    seed: u64,
    tick: u64,
    order: u64,
) -> Option<(sound::Pick, &'a sound::Clip)> {
    // Usually every take can play, and the event is picked from as it is;
    // only when some can't is a copy made without them.
    let pick = if event.takes.iter().all(|t| sounds.clip_of(*t).is_some()) {
        sound::event::pick(event, last, seed, tick, order)?
    } else {
        let playable = sound::SoundEvent {
            takes: event
                .takes
                .iter()
                .copied()
                .filter(|t| sounds.clip_of(*t).is_some())
                .collect(),
            ..event.clone()
        };
        sound::event::pick(&playable, last, seed, tick, order)?
    };
    Some((pick, sounds.clip_of(pick.take)?))
}

/// How loud a sound played from `placement` at `volume_db` is at the
/// listener.
fn loudness(placement: &Placement, volume_db: f32, hearing: [f32; 3]) -> f32 {
    let placed = match placement {
        Placement::Flat => None,
        Placement::At(s) | Placement::On(_, s) => Some((s.place, s.near_m, s.far_m, s.fade)),
    };
    loudness_at(volume_db, placed, hearing)
}

/// Ranks this many loops and one-shots and chooses `room` of them, as each
/// frame does (for the scale check): gives back how many were chosen.
pub fn choose_for_scale(loops: usize, shots: usize, room: usize) -> usize {
    let candidates: Vec<Candidate> = (0..loops)
        .map(|i| Candidate {
            key: Key::Loop(i as u64),
            priority: Priority::Normal,
            loudness_db: loudness_at(
                0.0,
                Some((
                    [(i % 100) as f32, 0.0, (i / 100) as f32],
                    2.0,
                    60.0,
                    sound::event::Fade::Realistic,
                )),
                [0.0; 3],
            ),
        })
        .chain((0..shots).map(|i| Candidate {
            key: Key::Shot(i),
            priority: if i % 10 == 0 {
                Priority::High
            } else {
                Priority::Low
            },
            loudness_db: -((i % 50) as f32),
        }))
        .collect();
    choose(&candidates, room).len()
}

/// The requests worth playing: after a stall (many ticks in one frame),
/// only the last few ticks' sounds play, not a pile-up; stopping and
/// volumes always count.
fn fresh(requests: Vec<GameSound>) -> impl Iterator<Item = GameSound> {
    let newest = requests.iter().map(|r| r.tick).max().unwrap_or(0);
    requests
        .into_iter()
        .filter(move |r| !matches!(r.request, Request::Play(..)) || r.tick + STALL_TICKS >= newest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play(tick: u64) -> GameSound {
        GameSound {
            tick,
            order: 0,
            seed: 1,
            request: Request::Play("step".into(), script::SoundAt::Nowhere, None),
        }
    }

    #[test]
    fn after_a_stall_only_the_last_ticks_play() {
        let stop = GameSound {
            request: Request::StopOn(EntityId(Id::from_raw(5).unwrap())),
            ..play(1)
        };
        let volume = GameSound {
            request: Request::BusVolume("Music".into(), 0.5),
            ..play(1)
        };
        let kept: Vec<u64> = fresh(vec![play(1), play(5), play(9), play(10), stop, volume])
            .map(|r| r.tick)
            .collect();
        assert_eq!(kept, vec![9, 10, 1, 1], "old stops and volumes still count");
        assert_eq!(fresh(Vec::new()).count(), 0);
    }

    #[test]
    fn choosing_keeps_to_the_room() {
        assert_eq!(choose_for_scale(2_000, 256, 64), 64);
        assert_eq!(choose_for_scale(10, 5, 64), 15);
        assert_eq!(choose_for_scale(0, 0, 64), 0);
    }

    /// The scale target's sound (decision 0011): 2,000 units with an
    /// engine loop and 20,000 projectiles each starting a sound in one
    /// tick (game code keeps 256 of a tick's requests), ranked and chosen
    /// in under 0.5 ms.
    #[test]
    #[ignore = "scale check: run by tools/scale/scale-check.sh"]
    fn scale_check_choosing_sounds() {
        let shots = 20_000.min(editor::game_sound::MAX_PER_TICK);
        let runs = 50;
        let began = std::time::Instant::now();
        for _ in 0..runs {
            assert_eq!(choose_for_scale(2_000, shots, 64), 64);
        }
        let each = began.elapsed().as_secs_f64() / f64::from(runs) * 1000.0;
        let slow = each > 0.5;
        println!(
            "Scale check: choosing 64 of 2,000 loops and {shots} sounds: {each:.3} ms (target 0.5 ms){}",
            if slow { " SLOW" } else { "" }
        );
        assert!(!slow);
    }
}

#[cfg(test)]
mod reverb_tests {
    use super::*;
    use area::builtin::{ReverbZone, ZoneShape};
    use glam::{Mat4, Quat, Vec3};
    use sound::reverb::Preset;

    #[test]
    fn the_camera_inside_a_zone_hears_its_echo() {
        let cave = ReverbZone {
            shape: ZoneShape::Ball,
            radius: 10.0,
            soft_edge: 2.0,
            preset: "Cave".into(),
            ..ReverbZone::default()
        };
        let at = Mat4::from_translation(Vec3::new(100.0, 0.0, 0.0));
        let zones = [(cave.clone(), at)];
        assert_eq!(
            reverbs_heard(&zones, [100.0, 0.0, 0.0]),
            [(Preset::Cave, 1.0)]
        );
        assert!(reverbs_heard(&zones, [0.0; 3]).is_empty(), "far outside");
        let edge = reverbs_heard(&zones, [109.0, 0.0, 0.0]);
        assert!((edge[0].1 - 0.5).abs() < 1e-4, "half way across the edge");
        // A turned, scaled box: measured as it's drawn.
        let tunnel = ReverbZone {
            size: Vec3::new(4.0, 40.0, 4.0),
            soft_edge: 0.0,
            preset: "Sewer".into(),
            ..ReverbZone::default()
        };
        let turned = Mat4::from_scale_rotation_translation(
            Vec3::splat(2.0),
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            Vec3::ZERO,
        );
        let zones = [(tunnel, turned)];
        assert_eq!(
            reverbs_heard(&zones, [30.0, 0.0, 0.0]),
            [(Preset::Sewer, 1.0)]
        );
        assert!(reverbs_heard(&zones, [0.0, 10.0, 0.0]).is_empty());
        // A zone scaled to nothing hears nothing, without trouble.
        let flat = [(cave, Mat4::from_scale(Vec3::ZERO))];
        assert!(reverbs_heard(&flat, [0.0; 3]).is_empty());
    }
}
