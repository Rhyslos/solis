//! The end of a tick and of the running game's frame (`play_tick.rs`):
//! physics last in the tick, then recording or comparing with a replay,
//! then saves and loads; at the frame's end, what it has to say.

use super::FrameRun;
use crate::console::LogLevel;
use crate::play::{
    MAX_SAVES_PER_TICK, PHYSICS_LIMIT, Playing, RECORDING_TICKS, collapse_saves, flat_broke_note,
};
use crate::workbench::{Workbench, WorkbenchAction};
use std::time::{Duration, Instant};

impl Workbench {
    /// Saves and loads, at the end of the tick, in the order asked
    /// (decision 0038); the same slot asked for again in a tick is done
    /// once, as last asked. Whether a load was asked for: it replaces the
    /// game, so no more ticks run before it.
    pub(super) fn saves_and_loads(
        &mut self,
        saves: Vec<(String, bool)>,
        time: f64,
        seconds: f32,
        run: &mut FrameRun,
    ) -> bool {
        let (saves, dropped) = collapse_saves(saves);
        // What this tick's spawned components' starts spawned in turn
        // would join next tick: it joins now, so a save holds it (a
        // few rounds at most; past that, it's left for the next tick).
        if saves.iter().any(|(_, load)| !load) {
            for _ in 0..8 {
                if self
                    .open
                    .playing
                    .as_ref()
                    .is_none_or(|p| p.pending.is_empty())
                {
                    break;
                }
                run.failures.extend(self.land_edits(time, seconds));
            }
        }
        if dropped > 0 {
            self.log(
                LogLevel::Warning,
                format!(
                    "Game code asked to save or load in more than {MAX_SAVES_PER_TICK} slots \
                     in one tick; the other {dropped} were left out."
                ),
            );
        }
        let mut loading = false;
        let replaying = self
            .open
            .playing
            .as_ref()
            .is_some_and(|p| p.replay.is_some());
        let mut saved_text: Option<Option<String>> = None;
        for (slot, load) in saves {
            if load {
                self.open.outbox.push(WorkbenchAction::LoadGame(slot));
                loading = true;
            } else {
                // Every slot saved this tick holds the same moment:
                // made once.
                let text = match &saved_text {
                    Some(text) => text.clone(),
                    None => {
                        let text = self.save_text();
                        saved_text = Some(text.clone());
                        text
                    }
                };
                // A replay makes the save as the recorded play did (it
                // builds physics again, so the game goes on alike),
                // but doesn't write over the player's saves.
                if let Some(text) = text
                    && !replaying
                {
                    self.open
                        .outbox
                        .push(WorkbenchAction::SaveGame { slot, text });
                }
            }
        }
        loading
    }

    /// The frame's end: the player's settings kept, the frame's times
    /// counted, what game code said told, a quit or a pause taken, the
    /// screens refreshed, and each limit reached said once a play.
    pub(super) fn end_frame(&mut self, started: Instant, ran: u64, mut run: FrameRun) {
        let Some(playing) = &mut self.open.playing else {
            return;
        };
        let refused = playing.world.projectiles().refused() > 0 && !playing.told_too_many;
        if refused {
            playing.told_too_many = true;
        }
        // The player's settings game code changed (decision 0091): kept
        // for the next play; the window's only said, since the editor's
        // window isn't the game's.
        self.player_settings = playing.view.settings;
        let asked = std::mem::take(&mut playing.view.asked);
        let bodies_refused = playing.world.physics_refused() > 0 && !playing.told_too_many_bodies;
        if bodies_refused {
            playing.told_too_many_bodies = true;
        }
        let quit = std::mem::take(&mut playing.view.quit);
        self.say_window_settings(asked, &mut run.said);
        self.profiler
            .add(crate::profiler::Stage::GameCode, run.code);
        self.profiler.add(
            crate::profiler::Stage::World,
            started.elapsed().saturating_sub(run.code),
        );
        if !run.said.is_empty() || !run.failures.is_empty() {
            self.tell_game(run.said, &run.failures);
        }
        // `Window:quit()` (decision 0093): the exported game closes; in
        // the editor, play stops.
        if quit {
            if !self.play_only {
                self.log(
                    LogLevel::Info,
                    "Window:quit(): the exported game would close here, so play stops.",
                );
            }
            self.open.quit_asked = true;
            self.stop();
            return;
        }
        // The game's screens read what this frame's ticks left (decision
        // 0087), and start again when the code changes.
        if ran > 0
            || self
                .open
                .game_screens
                .out_of_date(self.open.program.as_ref())
        {
            self.refresh_game_screens();
        }
        if let Some(why) = run.pause {
            // A replay that matched is good news, not a warning.
            let level = if why.starts_with("The replay matched") {
                LogLevel::Info
            } else {
                LogLevel::Warning
            };
            self.log(level, format!("{why} Press Pause or Play to carry on."));
        }
        self.warn_of_limits(run.dropped_sounds, refused, run.flat_broke, bodies_refused);
    }

    /// What game code asked of the window (decision 0091), said: an
    /// exported game's window changes (the app follows the settings), so
    /// there's nothing to say there.
    fn say_window_settings(&self, asked: Vec<script::settings::Setting>, said: &mut Vec<String>) {
        for asked in asked.into_iter().filter(|_| !self.play_only) {
            use script::settings::Setting;
            match asked {
                Setting::Fullscreen(on) => said.push(format!(
                    "Window:setFullscreen({on}): the exported game would {}. While playing in the \
                     editor, the game stays in the Game panel.",
                    if on {
                        "fill the screen"
                    } else {
                        "sit in a window"
                    }
                )),
                Setting::Resolution(..) => {
                    let [w, h] = self.player_settings.resolution;
                    said.push(format!(
                        "Window:setResolution: the exported game's window would be {w} × {h}. \
                         While playing in the editor, the game stays in the Game panel."
                    ));
                }
                Setting::TextSize(_) | Setting::UiSize(_) => {}
            }
        }
    }

    /// Each limit the frame reached, said once a play: too many sounds
    /// in a tick, projectiles or physics bodies, and 2D physics started
    /// again.
    fn warn_of_limits(
        &mut self,
        dropped_sounds: usize,
        projectiles_refused: bool,
        flat_broke: Option<String>,
        bodies_refused: bool,
    ) {
        if dropped_sounds > 0
            && let Some(playing) = &mut self.open.playing
            && !playing.told_too_many_sounds
        {
            playing.told_too_many_sounds = true;
            self.log(
                LogLevel::Warning,
                format!(
                    "Game code asked for more than {} sounds in one tick; the rest were left out \
                     (said once).",
                    crate::game_sound::MAX_PER_TICK
                ),
            );
        }
        if projectiles_refused {
            self.log(
                LogLevel::Warning,
                format!(
                    "More than {} projectiles were flying at once, so game code's next shots \
                     weren't fired until some ran out of time.",
                    crate::models::grouped(world::MAX_PROJECTILES)
                ),
            );
        }
        if let Some(note) = flat_broke {
            self.log(LogLevel::Warning, note);
        }
        if bodies_refused {
            self.log(
                LogLevel::Warning,
                format!(
                    "The game has the most physics bodies it can ({}), so the rest have no physics: \
                     they don't fall or collide.",
                    crate::models::grouped(world::physics::MAX_BODIES)
                ),
            );
        }
    }
}

/// Physics' step, last in the tick (decision 0036). Why the game must
/// pause: as game code said (`slow`), or physics broke or was too slow.
/// 2D physics starting again is noted for the frame's end.
pub(super) fn step_tick_physics(
    playing: &mut Playing,
    mut slow: Option<String>,
    physics_limit: Duration,
    run: &mut FrameRun,
) -> Option<String> {
    let stepping = Instant::now();
    playing.world.step_physics();
    if playing.world.take_physics_broke() && slow.is_none() {
        slow = Some(
            "Physics stopped working: something in it is out of the range physics can \
             work with (a collider or body thousands of kilometres big or away), so the \
             game has no physics now. Fix its size or place, then Stop and play again."
                .to_string(),
        );
    }
    // 2D physics stopped too, and the world started it again: the
    // game carries on, and the Console says why once a play.
    if let Some(broke) = playing.world.take_flat_broke()
        && !playing.told_flat_broke
    {
        playing.told_flat_broke = true;
        run.flat_broke = Some(flat_broke_note(&playing.world, broke));
    }
    if slow.is_none() && stepping.elapsed() > physics_limit {
        slow = Some(format!(
            "Physics took {:.0} ms for one tick (more than {} ms), so the game is paused \
             rather than falling further behind. Fewer bodies, or simpler colliders, make \
             it quicker.",
            stepping.elapsed().as_secs_f64() * 1000.0,
            PHYSICS_LIMIT.as_millis()
        ));
    }
    slow
}

/// Recording and playing back: the tick's input and checksums kept, or
/// compared with the recording's. Why the game must pause: `slow`, or the
/// recording full, or the replay done (matched or not).
pub(super) fn record_or_compare(
    playing: &mut Playing,
    frame: &input::Frame,
    slow: Option<String>,
) -> Option<String> {
    if playing.recording.is_none() && playing.replay.is_none() {
        return slow;
    }
    let ticks = playing.world.clock().ticks;
    let tick = input::Tick {
        frame: frame.clone(),
        world: playing.world.checksum(),
        // Game code's every second: asking every component
        // costs.
        game: ticks
            .is_multiple_of(60)
            .then(|| playing.game.as_mut().map_or(0, script::Game::checksum)),
    };
    if let Some(recording) = &mut playing.recording {
        let bytes = playing.recording_bytes + tick.written_len();
        // Within what Solis reads back (header and all).
        if recording.ticks.len() < RECORDING_TICKS && bytes + 256 < input::MAX_RECORDING_BYTES {
            playing.recording_bytes = bytes;
            recording.ticks.push(tick);
        } else if slow.is_none() {
            return Some(
                "The recording reached the most Solis keeps (an hour, or 64 MB), so \
                 the game is paused; Stop saves what was recorded."
                    .to_string(),
            );
        }
        return slow;
    }
    let outcome = replay_outcome(playing, &tick);
    slow.or(outcome)
}

/// The replay moves on a tick: whether it stopped matching the recording
/// at this tick, or ended matching it, in words; none while it goes on.
fn replay_outcome(playing: &mut Playing, tick: &input::Tick) -> Option<String> {
    let (recording, at) = playing.replay.as_mut()?;
    let n = *at;
    *at += 1;
    let ended = *at >= recording.ticks.len();
    let count = recording.ticks.len();
    let mismatch = recording.ticks.get(n).is_some_and(|expected| {
        expected.world != tick.world
            || matches!((expected.game, tick.game), (Some(a), Some(b)) if a != b)
    });
    let outcome = if mismatch {
        Some(format!(
            "The replay stopped matching the recording by tick {n} ({:.2} s in; \
             game code's own values are compared once a second, so it may have \
             started a little before): the game came out differently from the \
             same input. Something in it \
             isn't deterministic (frameTime(), which is real time, is a usual \
             one), or the area or code changed since recording.",
            n as f64 * world::TICK
        ))
    } else if ended {
        Some(format!(
            "The replay matched the recording on all {count} ticks: the same \
             input gave exactly the same game."
        ))
    } else {
        None
    };
    if outcome.is_some() {
        playing.replayed = Some(if mismatch { Err(n) } else { Ok(count) });
        playing.replay = None;
    }
    outcome
}
