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
use crate::game_code::{PlayWorld, ViewCache, failure_text, new_game, sync_all, sync_entities};
use crate::panel::Panel;
use crate::workbench::{Workbench, WorkbenchAction};
use area::text::AreaFile;
use area::{Edit, EntityData};
use reflect::{AssetId, EntityId};
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
    /// The game's code running, if there is code (and it started), on
    /// every core (decision 0033).
    pub(crate) game: Option<script::Game>,
    /// What game code reads of the world's entities, kept between ticks.
    pub(crate) view: crate::game_code::ViewCache,
    /// Whether the Console was told projectiles hit their limit.
    told_too_many: bool,
    /// Assembly names game code spawned that weren't found, said once.
    told_unknown: std::collections::HashSet<String>,
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
            | WorkbenchAction::MakeSampleGame
            | WorkbenchAction::MakeBenchmark
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
        let mut view = ViewCache::default();
        let game = self.start_game(&mut world, &mut view);
        self.playing = Some(Playing {
            world,
            paused: false,
            before: Box::new(self.document.clone()),
            assemblies_before: self.assemblies.clone(),
            reload: None,
            game,
            view,
            told_too_many: false,
            told_unknown: Default::default(),
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
    /// components (Lust, Lua and Rust) made from the area; `None` with no
    /// code. If the code's start fails (said in the Console), the Rust
    /// components run on their own.
    fn start_game(
        &mut self,
        world: &mut world::World,
        view: &mut ViewCache,
    ) -> Option<script::Game> {
        if self.program.is_none() && self.rust_components.is_empty() {
            return None;
        }
        let mut game = match new_game(&self.rust_components) {
            Ok(game) => game,
            Err(failure) => {
                self.tell_game(Vec::new(), &[failure]);
                return None;
            }
        };
        game.leave_out(self.left_out.clone());
        let mut host = PlayWorld::new(world, view, 0.0);
        let mut failures = Vec::new();
        // If the code's start fails, the rest of the game (its Rust
        // components) still runs.
        if let Some(chunk) = self.program.clone()
            && let Err(mut failure) = game.load(&chunk, &mut host)
        {
            failure.at.message = format!(
                "the game's code stopped while starting, so its Lust and Lua components don't \
                 run: {}",
                failure.at.message
            );
            failures.push(failure);
        }
        failures.extend(sync_all(&mut game, self.document.area(), &mut host));
        let said = std::mem::take(&mut host.said);
        self.tell_game(said, &failures);
        Some(game)
    }

    /// Writes what game code said, and its failures, in the Console.
    pub(crate) fn tell_game(&mut self, said: Vec<String>, failures: &[script::Failure]) {
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
        // The area as it was, but the components known now: code that
        // changed while playing keeps its components in the registry.
        self.replace_document(*playing.before);
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

    /// Moves the running game on by `seconds` (each frame does this; for
    /// tests and tools that run the game without a window).
    pub fn play_for(&mut self, seconds: f32) {
        self.run_game(seconds);
    }

    /// Moves the game on by `seconds`, taking in what changed in the
    /// area: only the entities changed, or, when entities were added,
    /// deleted or moved, everything. Called once a frame.
    pub(crate) fn run_game(&mut self, seconds: f32) {
        let started = std::time::Instant::now();
        // Game code's share, timed apart; the rest is the world's.
        let mut code = std::time::Duration::ZERO;
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
        let mut spawns = Vec::new();
        let mut despawns = Vec::new();
        // Changes made in the editor reach the running components.
        if let Some(game) = &mut playing.game
            && !changes.is_empty()
        {
            let time = playing.world.clock().time;
            let mut host = PlayWorld::new(&mut playing.world, &mut playing.view, time);
            host.frame_time = f64::from(seconds);
            let syncing = std::time::Instant::now();
            if changes.everything {
                failures.extend(sync_all(game, area, &mut host));
            } else {
                failures.extend(sync_entities(
                    game,
                    area,
                    changes.entities.iter().copied(),
                    &mut host,
                ));
            }
            code += syncing.elapsed();
            said.append(&mut host.said);
            spawns.append(&mut host.spawns);
            despawns.append(&mut host.despawns);
        }
        let mut pause = None;
        if !playing.paused {
            let ran = playing.world.step(f64::from(seconds));
            let now = playing.world.clock().time;
            for i in 0..ran {
                // Projectiles fly on first, then game code sees where
                // they are and fires more.
                playing.world.projectiles_mut().tick(world::TICK as f32);
                let Some(game) = &mut playing.game else {
                    continue;
                };
                let time = now - world::TICK * (ran - 1 - i) as f64;
                let mut host = PlayWorld::new(&mut playing.world, &mut playing.view, time);
                host.frame_time = f64::from(seconds);
                let ticking = std::time::Instant::now();
                let ticked = game.tick(world::TICK, &mut host);
                code += ticking.elapsed();
                failures.extend(ticked.failures);
                said.append(&mut host.said);
                spawns.append(&mut host.spawns);
                despawns.append(&mut host.despawns);
                // Too slow: no catching up, which would only be slower;
                // the game waits for the person instead.
                if ticked.pause.is_some() {
                    playing.paused = true;
                    pause = ticked.pause;
                    break;
                }
            }
        }
        let refused = playing.world.projectiles().refused() > 0 && !playing.told_too_many;
        if refused {
            playing.told_too_many = true;
        }
        // Copies game code made, and entities it took out, join the area
        // (and so the world, next frame).
        self.land_spawns(spawns, despawns);
        self.profiler.add(crate::profiler::Stage::GameCode, code);
        self.profiler.add(
            crate::profiler::Stage::World,
            started.elapsed().saturating_sub(code),
        );
        if !said.is_empty() || !failures.is_empty() {
            self.tell_game(said, &failures);
        }
        if let Some(why) = pause {
            self.log(
                LogLevel::Warning,
                format!("{why} Press Pause or Play to carry on."),
            );
        }
        if refused {
            self.log(
                LogLevel::Warning,
                format!(
                    "More than {} projectiles were flying at once, so game code's next shots \
                     weren't fired until some ran out of time.",
                    crate::models::grouped(world::MAX_PROJECTILES)
                ),
            );
        }
    }

    /// Makes the copies of assemblies game code spawned this frame, and
    /// takes out the entities it despawned: changes to the area that
    /// aren't undo steps (Stop puts the area back). A name that isn't an
    /// assembly in the project is said once.
    fn land_spawns(
        &mut self,
        spawns: Vec<(EntityId, String, glam::Vec3)>,
        despawns: Vec<EntityId>,
    ) {
        if spawns.is_empty() && despawns.is_empty() {
            return;
        }
        let mut edits = Vec::new();
        let mut unknown = Vec::new();
        for (id, name, place) in spawns {
            let Some((asset, shown)) = self.assembly_named(&name) else {
                unknown.push(name);
                continue;
            };
            let data = EntityData::new(&shown).with(&area::builtin::Transform {
                position: place,
                ..Default::default()
            });
            let lookup = |a| self.assemblies.get(&a).cloned();
            if let Ok(mut entities) = area::assembly::Expander::new(&lookup).place(asset, id, &data)
            {
                // The copy stands where it was spawned, turned and sized as
                // the assembly made it.
                if let Some((_, root)) = entities.iter_mut().find(|(e, _)| *e == id) {
                    let made = root.get::<area::builtin::Transform>().unwrap_or_default();
                    root.set(&area::builtin::Transform {
                        position: place,
                        ..made
                    });
                }
                edits.push(Edit::Spawn { entities });
            }
        }
        let mut gone = std::collections::HashSet::new();
        for id in despawns {
            if gone.insert(id) && self.document.area().get(id).is_some() {
                edits.push(Edit::Despawn { root: id });
            }
        }
        self.document.edit_by_game(edits);
        unknown.sort();
        unknown.dedup();
        for name in unknown {
            let Some(playing) = &mut self.playing else {
                break;
            };
            if playing.told_unknown.insert(name.clone()) {
                self.log(
                    LogLevel::Warning,
                    format!(
                        "Game code spawned \"{name}\", but there's no assembly by that name in \
                         the project's assets, so nothing was made."
                    ),
                );
            }
        }
    }

    /// The assembly game code names `name`: by its file in the assets
    /// (`ships/fighter.solasm`), that without the extension, or just its
    /// name (`fighter`, when only one has it); its asset and name.
    fn assembly_named(&self, name: &str) -> Option<(AssetId, String)> {
        let assemblies = || {
            self.library
                .iter()
                .filter(|i| i.kind == crate::library::LibraryKind::Assembly)
                .filter_map(|i| i.id.map(|id| (id, i)))
                .filter(|(id, _)| self.assemblies.contains_key(id))
        };
        let wanted = name.trim_start_matches('/');
        let by_path = assemblies().find(|(_, i)| {
            i.path.eq_ignore_ascii_case(wanted)
                || i.path
                    .rsplit_once('.')
                    .is_some_and(|(stem, _)| stem.eq_ignore_ascii_case(wanted))
        });
        let found = by_path.or_else(|| {
            let mut named = assemblies().filter(|(_, i)| i.name.eq_ignore_ascii_case(wanted));
            let first = named.next();
            first.filter(|_| named.next().is_none())
        });
        found.map(|(id, i)| (id, i.name.clone()))
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
