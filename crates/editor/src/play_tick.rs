//! One frame of the running game (`Workbench::run_game`): the editor's
//! changes taken in, then each tick due, in its fixed order (decision
//! 0033): land what earlier starts spawned, move the world on (and the
//! trigger zones' overlap pass), the tick's input, game code, physics,
//! recording or replay, then saves and loads. What the frame has to say is said at its end. The order
//! is what keeps a replay matching the live game, so each step's place
//! in `run_tick` matters.

use super::cues_of;
use crate::document::Changes;
use crate::game_code::{PlayWorld, sync_all, sync_entities};
use crate::workbench::Workbench;
use end::{record_or_compare, step_tick_physics};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[path = "play_tick_end.rs"]
mod end;

#[path = "play_zones.rs"]
mod zones;

/// What one frame of the running game gathers as it goes, said at its
/// end.
#[derive(Default)]
struct FrameRun {
    /// Game code's share of the frame's time.
    code: Duration,
    /// What game code said.
    said: Vec<String>,
    /// Game code's failures.
    failures: Vec<script::Failure>,
    /// Sounds asked for past the most a tick plays.
    dropped_sounds: usize,
    /// Why the game paused, if it did.
    pause: Option<String>,
    /// That 2D physics stopped and started again, said once a play.
    flat_broke: Option<String>,
}

/// A tick's input as game code sees it.
struct TickInput {
    /// The devices' (or the recording's) frame.
    frame: Arc<input::Frame>,
    /// The button the menu keys chose, run after game code.
    chosen: Option<(usize, usize)>,
    /// The events the engine sends game code this tick.
    events: Vec<script::Event>,
    /// The screens' typing sounds due this tick (sound events' names).
    typing_sounds: Vec<String>,
}

impl Workbench {
    /// Moves the game on by `seconds`, taking in what changed in the
    /// level: only the entities changed, or, when entities were added,
    /// deleted or moved, everything. Called once a frame.
    pub(crate) fn run_game(&mut self, seconds: f32) {
        let started = Instant::now();
        let physics_limit = self.physics_limit();
        let changes = self.open.document.take_changes();
        if self.open.playing.is_none() {
            return;
        }
        let mut run = FrameRun::default();
        self.take_in_changes(&changes, &mut run);
        self.sync_changes(&changes, seconds, &mut run);
        let ran = self.ticks_due(seconds);
        for _ in 0..ran {
            if !self.run_tick(seconds, physics_limit, &mut run) {
                break;
            }
        }
        self.end_frame(started, ran, run);
    }

    /// The level's changes reach the game's world: rebuilt or updated, and
    /// the fields set by hand taken. An edit while recording is said.
    fn take_in_changes(&mut self, changes: &Changes, run: &mut FrameRun) {
        let Some(playing) = &mut self.open.playing else {
            return;
        };
        let level = self.open.document.level();
        if changes.everything {
            playing.world.rebuild(level);
        } else if !changes.entities.is_empty() {
            playing.world.update(
                level,
                changes
                    .entities
                    .iter()
                    .map(|&id| (id, changes.moved.contains(&id))),
            );
        }
        // A field set by hand takes the level's value even where that's
        // what the level held all along and game code had set its own
        // meanwhile: for Solis's own components here, as for game code's
        // below. Everything else game code set on the entity stays.
        for ((id, component), fields) in &changes.set {
            for field in fields {
                playing.world.set_by_hand(*id, component, field);
            }
        }
        // An edit isn't in the recording, so its replay won't match from
        // here: said once.
        if !changes.is_empty() && playing.recording.is_some() && !playing.told_recording_edited {
            playing.told_recording_edited = true;
            run.said.push(
                "The level changed while recording: a recording keeps only the input, so its \
                 replay will stop matching from here."
                    .to_string(),
            );
        }
    }

    /// Changes made in the editor reach the running components.
    fn sync_changes(&mut self, changes: &Changes, seconds: f32, run: &mut FrameRun) {
        let Some(playing) = &mut self.open.playing else {
            return;
        };
        let level = self.open.document.level();
        let Some(game) = &mut playing.game else {
            return;
        };
        if changes.is_empty() {
            return;
        }
        let time = playing.world.clock().time;
        let (tick, seed) = (playing.world.clock().ticks, playing.world.seed());
        let mut host = PlayWorld::new(&mut playing.world, &mut playing.view, time);
        host.frame_time = f64::from(seconds);
        let syncing = Instant::now();
        // With the fields set by hand, which a running component
        // takes whatever the level held (Properties shows the game's
        // value, so setting it to the level's is a change too).
        if changes.everything {
            run.failures
                .extend(sync_all(game, level, &changes.set, &mut host));
        } else {
            run.failures.extend(sync_entities(
                game,
                level,
                changes.entities.iter().copied(),
                &changes.set,
                &mut host,
            ));
        }
        run.code += syncing.elapsed();
        host.outside_a_tick();
        run.said.append(&mut host.said);
        // Sounds asked for as components start (music, say) play
        // with the first tick's.
        let sounds = std::mem::take(&mut host.sounds);
        run.dropped_sounds +=
            crate::game_sound::gather(&mut self.open.game_sounds, sounds, tick, seed);
        // What their starts spawned lands before the next tick.
        let edits = std::mem::take(&mut host.edits);
        playing.pending.extend(edits);
    }

    /// How many ticks this frame runs: those due, or one for Step while
    /// paused.
    fn ticks_due(&mut self, seconds: f32) -> u64 {
        let Some(playing) = &mut self.open.playing else {
            return 0;
        };
        let ran = if playing.paused {
            // Step: one tick, then paused again.
            u64::from(std::mem::take(&mut playing.step))
        } else {
            playing.world.due(f64::from(seconds))
        };
        if ran == 0 && playing.paused {
            // Presses and mouse movement while paused don't wait for the
            // game to go on (the camera would jump).
            self.devices.forget_changes();
        }
        ran
    }

    /// Runs one tick, in its fixed order. False when no more ticks run
    /// this frame: play stopped, or the game paused, loaded or quit.
    fn run_tick(&mut self, seconds: f32, physics_limit: Duration, run: &mut FrameRun) -> bool {
        let Some(time) = self.begin_tick(seconds, run) else {
            return false;
        };
        let Some((cue_sounds, cue_events)) = self.move_world_on() else {
            return false;
        };
        let Some(TickInput {
            frame,
            chosen,
            events,
            typing_sounds,
        }) = self.tick_input(cue_events)
        else {
            return false;
        };
        let mut cue_sounds = cue_sounds;
        cue_sounds.extend(typing_sounds.into_iter().map(|sound| {
            crate::game_sound::Request::Play(sound, script::SoundAt::Nowhere, None, None)
        }));
        self.play_cue_sounds(cue_sounds, run);
        let (slow, saves) = self.run_tick_code(time, seconds, &frame, events, run);
        // A button the menu keys chose runs now; what it sends is the
        // next tick's.
        if let Some(button) = chosen {
            self.press(button);
        }
        // Physics moves on last, with game code's moves and pushes
        // (decision 0036).
        let Some(playing) = &mut self.open.playing else {
            return false;
        };
        let slow = step_tick_physics(playing, slow, physics_limit, run);
        let slow = record_or_compare(playing, &frame, slow);
        // Too slow: no catching up, which would only be slower; the
        // game waits for the person instead (after this tick's saves).
        if slow.is_some() {
            playing.paused = true;
            run.pause = slow;
        }
        let loading = self.saves_and_loads(saves, time, seconds, run);
        self.typing_after_tick();
        // `Window:quit()`: no more ticks.
        let quit = self.open.playing.as_ref().is_some_and(|p| p.view.quit);
        !(loading || run.pause.is_some() || quit)
    }

    /// The UI's clock as this tick ends (stage 1, text effects); while a
    /// screen types its text out, the screens are read after every tick,
    /// so a text begins typing on the tick it shows at any frame rate.
    fn typing_after_tick(&mut self) {
        let Some(playing) = &self.open.playing else {
            return;
        };
        self.open.game_screens.clock = playing.world.clock().ticks as f64 * world::TICK;
        if self.open.game_screens.typing.active {
            self.refresh_game_screens();
            self.open.game_screens.read_after_tick = true;
        } else {
            self.refresh_captions();
        }
    }

    /// Begins a tick: the clock moves on (so a frame that stops early
    /// never counts ticks that didn't run), what earlier starts spawned
    /// joins, and the screens catch up. The tick's game time, or none
    /// when play has stopped.
    fn begin_tick(&mut self, seconds: f32, run: &mut FrameRun) -> Option<f64> {
        let playing = self.open.playing.as_mut()?;
        playing.world.begin_tick();
        let time = playing.world.clock().time;
        // What earlier starts spawned joins first; then projectiles
        // fly on, then game code sees where they are and fires more.
        let landing = Instant::now();
        run.failures.extend(self.land_edits(time, seconds));
        run.code += landing.elapsed();
        // Game code showed or hid a screen last tick: the screens run
        // again now, so whether a menu has the keys this tick follows
        // the game itself, however many ticks a frame runs (the phase
        // review: a replay could otherwise stop matching).
        if self.screens_out_of_step() {
            self.refresh_game_screens();
        }
        Some(time)
    }

    /// The world moves on before game code's turn: projectiles, the
    /// project's clips, maps, 2D settings and animators taken in, then
    /// animators and sprites stepped, then the trigger zones' overlap
    /// pass. The sounds and events the sprites' cues make and the events
    /// the zones' crossings make, or none when play has stopped.
    fn move_world_on(&mut self) -> Option<(Vec<crate::game_sound::Request>, Vec<script::Event>)> {
        self.open.playing.as_ref()?;
        // The game's text in the player's language, as it is now: a
        // language changed by game code or a menu shows from this tick.
        let texts = self.game_texts();
        let playing = self.open.playing.as_mut()?;
        playing.view.set_texts(texts);
        // Where everything is as the tick starts: what drawing between
        // ticks moves from.
        playing.world.remember();
        let dt = playing.world.tick_time() as f32;
        playing.world.projectiles_mut().tick(dt);
        // Sprites' clips move on (Phase 9, D3): the events they pass
        // play their sound events and reach game code first this tick.
        playing
            .world
            .set_clip_sets(Arc::clone(&self.viewport.sheets.clip_sets));
        // Tile maps (D4): the project's files, and a map for each new
        // TileMap (one game code spawned, say).
        playing
            .world
            .set_tile_maps(Arc::clone(&self.viewport.sheets.map_set));
        playing.world.start_maps();
        // Game code names maps by their files' paths (decision 0079).
        playing.view.set_map_paths(&self.viewport.sheets);
        playing.view.set_tables(&self.viewport.sheets);
        // 2D physics (decision 0071): pixels to metres, and which
        // terrains are solid, as they are now.
        playing.world.set_pixels(self.viewport.pixels);
        playing
            .world
            .set_solid_sets(Arc::new(self.viewport.sheets.solid_sets()));
        // Animators first (D3b): each takes the transition the
        // parameters set last tick call for, and sets its sprite's clip.
        playing
            .world
            .set_animators(Arc::clone(&self.viewport.sheets.animator_set));
        playing.world.step_animators();
        let cues = playing.world.step_sprites(playing.world.tick_time());
        let (sounds, mut events) = cues_of(
            &playing.world,
            cues,
            &self.open.cue_sounds,
            &self.viewport.sheets.paths,
        );
        // Trigger zones (stage 1): who walked in or out since the last
        // tick, on where physics' last step left everything.
        events.extend(zones::zone_events(&mut playing.world));
        // Tags (stage 1): where the tagged entities are as the tick
        // starts, which every phase and core asks of.
        playing.view.tags = crate::tags::tag_view(&self.open.tags, self.open.kind, &playing.world);
        Some((sounds, events))
    }

    /// The tick's input: played back, or sampled from the devices once,
    /// so every core sees the same (decision 0037). None when play has
    /// stopped.
    fn tick_input(&mut self, mut events: Vec<script::Event>) -> Option<TickInput> {
        let playing = self.open.playing.as_mut()?;
        let sampled = self.devices.sample(&playing.bindings, world::TICK as f32);
        let frame = match (&playing.replay, &mut playing.script) {
            // The devices are still sampled (and the sample dropped),
            // so nothing pressed during a replay piles up for after it.
            (Some((recording, at)), _) => recording
                .ticks
                .get(*at)
                .map(|t| t.frame.clone())
                .unwrap_or_default(),
            // A script of input (`solis --play`), the same way.
            (None, Some(script)) => script.next_frame(),
            (None, None) => sampled,
        };
        let playing = self.open.playing.as_ref()?;
        // What screens' buttons sent since the last tick is part of its
        // input (decision 0088), so a replay sends it again; during a
        // replay, the recording's own.
        let mut frame = frame;
        // While a screen with buttons is shown, the menu keys move
        // its focus and choose, and the game's own actions wait; what
        // game code sees (and a recording keeps) is that frame.
        let chosen = self.open.game_screens.take_menu_keys(&mut frame);
        // The screens' text typing out (stage 1, text effects): its
        // sounds, and TypingFinished with this tick's input.
        let now = playing.world.clock().ticks as f64 * world::TICK;
        let (typed, sounds) = self.open.game_screens.typing.tick(now);
        for sent in typed {
            if self.open.game_screens.sent.len() < input::MAX_SENT {
                self.open.game_screens.sent.push(sent);
            }
        }
        let sent = std::mem::take(&mut self.open.game_screens.sent);
        if playing.replay.is_none() {
            frame.sent = sent;
        }
        events.extend(frame.sent.iter().map(|s| script::Event {
            to: s.to,
            name: s.name.clone(),
            values: s.values.clone(),
            only: None,
            during: None,
        }));
        Some(TickInput {
            frame: Arc::new(frame),
            chosen,
            events,
            typing_sounds: sounds,
        })
    }

    /// Plays the sounds the sprites' cues asked for this tick.
    fn play_cue_sounds(&mut self, sounds: Vec<crate::game_sound::Request>, run: &mut FrameRun) {
        let Some(playing) = &self.open.playing else {
            return;
        };
        let (tick, seed) = (playing.world.clock().ticks, playing.world.seed());
        run.dropped_sounds +=
            crate::game_sound::gather(&mut self.open.game_sounds, sounds, tick, seed);
    }

    /// Game code's turn: its updates and events with the tick's input,
    /// then what it spawned landed and its rebinding and seed taken in.
    /// Why the game must pause (code too slow), and the saves and loads
    /// it asked for.
    fn run_tick_code(
        &mut self,
        time: f64,
        seconds: f32,
        frame: &Arc<input::Frame>,
        events: Vec<script::Event>,
        run: &mut FrameRun,
    ) -> (Option<String>, Vec<(String, bool)>) {
        let Some(playing) = &mut self.open.playing else {
            return (None, Vec::new());
        };
        let (sound_tick, sound_seed) = (playing.world.clock().ticks, playing.world.seed());
        // Game code's dt at the game's speed (0 paused, M3c).
        let dt = playing.world.tick_time();
        let Some(game) = &mut playing.game else {
            return (None, Vec::new());
        };
        let mut host = PlayWorld::new(&mut playing.world, &mut playing.view, time);
        host.frame_time = f64::from(seconds);
        host.input = Arc::clone(frame);
        host.bindings = Arc::clone(&playing.bindings);
        let ticking = Instant::now();
        let ticked = game.tick_with_events(dt, &mut host, events);
        run.code += ticking.elapsed();
        run.failures.extend(ticked.failures);
        run.said.append(&mut host.said);
        // What this tick spawned and despawned joins the world now,
        // in the order it was asked for (decision 0035), so the next
        // tick sees it however many ticks this frame runs.
        let edits = std::mem::take(&mut host.edits);
        playing.pending.extend(edits);
        let saves = std::mem::take(&mut host.saves);
        let sounds = std::mem::take(&mut host.sounds);
        run.dropped_sounds +=
            crate::game_sound::gather(&mut self.open.game_sounds, sounds, sound_tick, sound_seed);
        let seed = host.seed.take();
        let typed = std::mem::take(&mut host.typed);
        // Rebinding takes effect from the next tick. The tick's
        // own share of the bindings goes first, so they're changed
        // in place rather than copied.
        let rebinds = std::mem::take(&mut host.rebinds);
        drop(host);
        for (action, input) in rebinds {
            Arc::make_mut(&mut playing.bindings).rebind(&action, &input);
        }
        // `finishTyping`: the text shows whole from now (UI only).
        let clock = self.open.game_screens.clock;
        for (entity, screen) in typed {
            self.open.game_screens.typing.finish(entity, &screen, clock);
        }
        let slow = ticked.pause;
        let landing = Instant::now();
        run.failures.extend(self.land_edits(time, seconds));
        run.code += landing.elapsed();
        // A new seed holds from the next tick (after the starts of
        // what this tick spawned, which belong to this one).
        if let (Some(seed), Some(playing)) = (seed, &mut self.open.playing) {
            playing.world.set_seed(seed);
        }
        (slow, saves)
    }
}
