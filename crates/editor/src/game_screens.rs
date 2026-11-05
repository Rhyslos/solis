//! The game's screens while it plays ([decision 0087](../../../docs/decisions/0087-screens-through-the-engine.md)):
//! the screens' own Luau started with the program, their views run after
//! each tick (the game-wide objects copied in first), and each entity's
//! `Screen` that's shown drawn over the Game panel in its order. A screen
//! that fails keeps its last picture, and the Console says where, once
//! for each new message.

use crate::console::LogLevel;
use crate::workbench::Workbench;
use reflect::Component;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// The screens' side of playing.
#[derive(Default)]
pub(crate) struct GameScreens {
    /// The screens running, and the program they were started from.
    running: Option<(script::Screens, Arc<script::Compiled>)>,
    /// What's drawn over the Game panel now, in order.
    pub(crate) pictures: Vec<Arc<ui::Screen>>,
    /// Each screen's last good picture, by name.
    last: BTreeMap<String, Arc<ui::Screen>>,
    /// Messages said already, so a failing screen isn't said every tick.
    told: BTreeSet<String>,
}

impl std::fmt::Debug for GameScreens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameScreens")
            .field("running", &self.running.is_some())
            .field("pictures", &self.pictures.len())
            .finish_non_exhaustive()
    }
}

impl GameScreens {
    /// Forgets everything (Stop, or no program).
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    /// Whether the screens need starting (again) for `program`: its
    /// screens aren't the ones running, or there are none and pictures
    /// are left.
    pub(crate) fn out_of_date(&self, program: Option<&Arc<script::Compiled>>) -> bool {
        match (program.filter(|p| p.screens.is_some()), &self.running) {
            (Some(program), Some((_, from))) => !Arc::ptr_eq(program, from),
            (Some(_), None) => self.told.is_empty(),
            (None, _) => self.running.is_some() || !self.pictures.is_empty(),
        }
    }
}

/// The screens shown in `world`, in drawing order: each entity's Screen
/// that's shown and names one, by its order, then the area's order.
pub(crate) fn shown_in(world: &world::World) -> Vec<(reflect::EntityId, String)> {
    let mut shown: Vec<(i64, usize, reflect::EntityId, String)> = world
        .values_named(area::builtin::Screen::NAME)
        .into_iter()
        .enumerate()
        .map(|(i, (id, data))| (i, id, area::builtin::Screen::from_data(&data)))
        .filter(|(_, _, s)| s.shown && !s.screen.is_empty())
        .map(|(i, id, s)| (s.order, i, id, s.screen))
        .collect();
    shown.sort_by_key(|(order, i, _, _)| (*order, *i));
    shown
        .into_iter()
        .map(|(_, _, id, name)| (id, name))
        .collect()
}

/// What screens read: the running game's components.
struct GameReader<'a>(Option<&'a mut script::Game>);

impl script::ScreenReader for GameReader<'_> {
    fn values(&mut self, entity: reflect::EntityId, name: &str) -> Option<reflect::ComponentData> {
        self.0.as_deref_mut()?.values(entity, name)
    }
}

impl Workbench {
    /// Runs the shown screens' views again (after ticks, or when the code
    /// changed) and keeps their pictures for the Game panel.
    pub(crate) fn refresh_game_screens(&mut self) {
        let Some(playing) = &mut self.playing else {
            self.game_screens.clear();
            return;
        };
        let program = self.program.clone().filter(|p| p.screens.is_some());
        let Some(program) = program else {
            self.game_screens.clear();
            return;
        };
        let screens = &mut self.game_screens;
        // Started with the program, and again when it changes.
        let fresh = !matches!(&screens.running, Some((_, from)) if Arc::ptr_eq(from, &program));
        let mut said = Vec::new();
        let mut warned = Vec::new();
        if fresh {
            screens.last.clear();
            screens.running = None;
            if let Some(code) = &program.screens {
                warned.extend(code.notes.iter().cloned());
                match script::Screens::start(code) {
                    Ok(started) => screens.running = Some((started, Arc::clone(&program))),
                    Err(failure) => {
                        said.push(format!("The game's screens couldn't start: {}", failure.at))
                    }
                }
            }
        }
        if let Some((running, _)) = &screens.running {
            if let Some(game) = &mut playing.game {
                running.set_objects(&game.object_values());
            }
            let mut pictures = Vec::new();
            let mut reader = GameReader(playing.game.as_mut());
            for (entity, name) in shown_in(&playing.world) {
                match running.view(&name, Some(entity), &mut reader) {
                    Ok(view) => {
                        let picture = Arc::new(ui::Screen {
                            name: name.clone(),
                            design: ui::view::design_of(&view),
                            scale: ui::ScaleRule::Grow,
                            elements: Vec::new(),
                            view: Some(view),
                        });
                        screens.last.insert(name, Arc::clone(&picture));
                        pictures.push(picture);
                    }
                    Err(problem) => {
                        said.push(format!("The screen {name} stopped: {}", problem));
                        if let Some(last) = screens.last.get(&name) {
                            pictures.push(Arc::clone(last));
                        }
                    }
                }
            }
            screens.pictures = pictures;
        }
        for line in warned {
            if self.game_screens.told.insert(line.clone()) {
                self.log(LogLevel::Warning, line);
            }
        }
        for line in said {
            if self.game_screens.told.insert(line.clone()) {
                self.log(LogLevel::Error, line);
            }
        }
    }

    /// The game's screens drawn over the Game panel now, in order.
    pub fn game_screens(&self) -> &[Arc<ui::Screen>] {
        &self.game_screens.pictures
    }

    /// The names of the game's screens, for a Screen's choices.
    pub(crate) fn screen_names(&self) -> Vec<String> {
        self.program
            .as_ref()
            .and_then(|p| p.screens.as_ref())
            .map(|s| s.screens.iter().map(|(n, _)| n.clone()).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
#[path = "game_screens_tests.rs"]
mod tests;
