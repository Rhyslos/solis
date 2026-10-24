//! Play mode: the area running as a game, beside the editor.
//!
//! Play keeps a copy of the document (the area, its undo history and the
//! selection; the history's steps are shared, so this is cheap) and builds the game's world from the area
//! ([`world::World`]). While it runs, the editor keeps working: changes
//! reach the game, which is built again from the area when it changes.
//! Stop throws the world away and puts the copy back, so everything is
//! exactly as it was before Play. Saving and opening wait until then.

use crate::console::LogLevel;
use crate::document::Document;
use crate::game_code::{WorldHost, failure_text, sync_all, sync_entity};
use crate::panel::Panel;
use crate::workbench::{Workbench, WorkbenchAction};
use area::text::AreaFile;
use reflect::AssetId;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Whether the game is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayState {
    /// Not playing: editing the area.
    Editing,
    /// The game is running.
    Playing,
    /// The game is running but its clock is stopped.
    Paused,
}

/// The game while it runs.
pub(crate) struct Playing {
    /// The game's entities.
    pub(crate) world: world::World,
    /// Whether its clock is stopped.
    pub(crate) paused: bool,
    /// The document as it was when Play was pressed, put back on Stop.
    before: Box<Document>,
    /// The assemblies as they were when Play was pressed, to rebuild
    /// copies from on Stop if any changed meanwhile.
    assemblies_before: BTreeMap<AssetId, Arc<AreaFile>>,
    /// The area as its file holds it now, if the file changed on disk
    /// while playing: offered again after Stop.
    pub(crate) reload: Option<area::Area>,
    /// The game's code running, if there is code (and it started).
    pub(crate) game: Option<script::Runtime>,
}

impl std::fmt::Debug for Playing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Playing")
            .field("world", &self.world)
            .field("paused", &self.paused)
            .finish_non_exhaustive()
    }
}

/// Whether `action` must wait until the game stops: it saves or replaces
/// the area (or writes an assembly), which would save the game's changes
/// or lose the copy kept for Stop.
pub(crate) fn waits_for_stop(action: &WorkbenchAction) -> bool {
    matches!(
        action,
        WorkbenchAction::NewArea
            | WorkbenchAction::OpenArea
            | WorkbenchAction::OpenAreaFile(_)
            | WorkbenchAction::SaveArea
            | WorkbenchAction::SaveAreaAs
            | WorkbenchAction::NewProject
            | WorkbenchAction::OpenProject
            | WorkbenchAction::OpenRecent(_)
            | WorkbenchAction::AddSampleArea
            | WorkbenchAction::CreateAssembly(_)
            | WorkbenchAction::ApplyToAssembly { .. }
    )
}

impl Workbench {
    /// Whether the game is running, and paused.
    pub fn play_state(&self) -> PlayState {
        match &self.playing {
            None => PlayState::Editing,
            Some(p) if p.paused => PlayState::Paused,
            Some(_) => PlayState::Playing,
        }
    }

    /// The running game's world, if it's running.
    pub fn world(&self) -> Option<&world::World> {
        self.playing.as_ref().map(|p| &p.world)
    }

    /// Play: runs the area as a game (or carries on, if paused). The
    /// Game panel comes to the front.
    pub fn play(&mut self) {
        if let Some(playing) = &mut self.playing {
            if playing.paused {
                playing.paused = false;
                self.log(LogLevel::Info, "Carrying on.");
            }
            return;
        }
        // A drag or a half-typed number ends here, so it isn't part of
        // both the editor's history and the game's.
        self.moving = None;
        self.document.close_step();
        let mut world = world::World::from_area(self.document.area());
        let _ = self.document.take_changes();
        let cameras = world.placed::<area::builtin::Camera>().len();
        let game = self.start_game(&mut world);
        self.playing = Some(Playing {
            world,
            paused: false,
            before: Box::new(self.document.clone()),
            assemblies_before: self.assemblies.clone(),
            reload: None,
            game,
        });
        if let Some(found) = self.dock.find_tab(&Panel::Game) {
            let _ = self.dock.set_active_tab(found);
        }
        let note = if cameras == 0 {
            " There's no camera in the area, so the Game panel shows the editor's view \
             (Entity > Add camera adds one)."
        } else {
            ""
        };
        self.log(
            LogLevel::Info,
            format!("Playing. Changes made now are undone when you press Stop.{note}"),
        );
    }

    /// The game's code started on `world`, with every entity's game
    /// components made from the area; `None` with no code, or if its
    /// start failed (said in the Console).
    fn start_game(&mut self, world: &mut world::World) -> Option<script::Runtime> {
        let chunk = self.program.clone()?;
        let mut runtime = script::Runtime::new();
        let mut host = WorldHost::new(world, 0.0);
        let mut failures = Vec::new();
        match runtime.load(&chunk, &mut host) {
            Ok(()) => failures.extend(sync_all(&mut runtime, self.document.area(), &mut host)),
            Err(failure) => {
                let said = host.said;
                self.tell_game(said, &[failure]);
                return None;
            }
        }
        let said = std::mem::take(&mut host.said);
        world.settle();
        self.tell_game(said, &failures);
        Some(runtime)
    }

    /// Writes what game code said, and its failures, in the Console.
    fn tell_game(&mut self, said: Vec<String>, failures: &[script::Failure]) {
        for words in said {
            self.log(LogLevel::Info, words);
        }
        for failure in failures {
            let text = failure_text(self.document.area(), failure);
            self.log(LogLevel::Error, text);
        }
    }

    /// Pause: stops the game's clock, or starts it again.
    pub fn pause(&mut self) {
        let Some(playing) = &mut self.playing else {
            self.log(LogLevel::Info, "Nothing is running.");
            return;
        };
        playing.paused = !playing.paused;
        let text = if playing.paused {
            "Paused."
        } else {
            "Carrying on."
        };
        self.log(LogLevel::Info, text);
    }

    /// Stop: throws the game away and puts the area, its undo history
    /// and the selection back as they were when Play was pressed.
    /// Assemblies changed on disk meanwhile update their copies again,
    /// and an area file changed on disk is offered again.
    pub fn stop(&mut self) {
        let Some(playing) = self.playing.take() else {
            self.log(LogLevel::Info, "Nothing is running.");
            return;
        };
        let seconds = playing.world.clock().time;
        self.moving = None;
        self.document = *playing.before;
        self.document.touch();
        let now = std::mem::replace(&mut self.assemblies, playing.assemblies_before);
        self.log(
            LogLevel::Info,
            format!("Stopped after {seconds:.1} s; the area is back as it was."),
        );
        for (id, file) in now {
            let changed = self
                .assemblies
                .get(&id)
                .is_none_or(|old| !Arc::ptr_eq(old, &file));
            if changed {
                self.set_assembly(id, file);
            }
        }
        if let Some(area) = playing.reload {
            self.offer_reload(area);
        }
    }

    /// Moves the game on by `seconds`, taking in what changed in the
    /// area: only the entities changed, or, when entities were added,
    /// deleted or moved, everything. Called once a frame.
    pub(crate) fn run_game(&mut self, seconds: f32) {
        let changes = self.document.take_changes();
        let Some(playing) = &mut self.playing else {
            return;
        };
        let area = self.document.area();
        if changes.everything {
            playing.world.rebuild(area);
        } else if !changes.entities.is_empty() {
            playing.world.update(
                area,
                changes
                    .entities
                    .iter()
                    .map(|&id| (id, changes.moved.contains(&id))),
            );
        }
        let mut said = Vec::new();
        let mut failures = Vec::new();
        // Changes made in the editor reach the running components.
        if let Some(game) = &mut playing.game
            && !changes.is_empty()
        {
            let time = playing.world.clock().time;
            let mut host = WorldHost::new(&mut playing.world, time);
            if changes.everything {
                failures.extend(sync_all(game, area, &mut host));
            } else {
                for &id in &changes.entities {
                    failures.extend(sync_entity(game, area, id, &mut host));
                }
            }
            said.append(&mut host.said);
            playing.world.settle();
        }
        if !playing.paused {
            let ran = playing.world.step(f64::from(seconds));
            if let Some(game) = &mut playing.game {
                let now = playing.world.clock().time;
                for i in 0..ran {
                    let time = now - world::TICK * (ran - 1 - i) as f64;
                    let mut host = WorldHost::new(&mut playing.world, time);
                    failures.extend(game.tick(world::TICK, &mut host));
                    said.append(&mut host.said);
                    playing.world.settle();
                }
            }
        }
        if !said.is_empty() || !failures.is_empty() {
            self.tell_game(said, &failures);
        }
    }

    /// Takes out of `actions` what must wait until the game stops, saying
    /// so in the Console. Quitting stops the game first.
    pub(crate) fn hold_while_playing(&mut self, actions: &mut Vec<WorkbenchAction>) {
        if self.playing.is_none() {
            return;
        }
        if actions.contains(&WorkbenchAction::Quit) {
            self.stop();
            return;
        }
        let before = actions.len();
        actions.retain(|a| !waits_for_stop(a));
        if actions.len() < before {
            self.log(
                LogLevel::Warning,
                "Stop playing first: saving, opening and making assemblies wait until the \
                 game stops, so its changes are never saved by mistake.",
            );
        }
    }
}

#[cfg(test)]
#[path = "play_tests.rs"]
mod tests;
