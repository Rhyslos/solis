//! Save games ([decision 0038](../../../docs/decisions/0038-save-games-hold-everything.md)):
//! the whole running game, so loading carries on exactly as the saved
//! game would have, checksum and all.
//!
//! A save is text, in sections, so it can be read (and its area even
//! opened) by a person:
//!
//! ```text
//! solis save 1
//! project Harbour
//! tick 1234
//! time 4060000000000000
//! seed 98765
//! sum 3f2a…
//! area 5120
//! …the area as it stands, game's places and all (area text)…
//! values 812
//! …every running component's values (area text)…
//! bindings 640
//! …the input actions, with game code's rebinding (TOML)…
//! projectiles 2
//! x y z vx vy vz left r g b (exact bits)
//! physics 10392
//! …physics, as bytes in base64…
//! ```
//!
//! `sum` is a checksum of everything after it, so a damaged save is
//! refused, never half loaded.

use crate::console::LogLevel;
use crate::workbench::{Workbench, WorkbenchAction};
use area::Area;
use area::builtin::Transform;
use base64::Engine as _;
use reflect::EntityId;
use std::sync::Arc;

/// The first line of a save this Solis writes and reads.
const HEADER: &str = "solis save 1";

/// The biggest save read.
pub(crate) const MAX_SAVE_BYTES: usize = 256 << 20;

/// A save game, read.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Save {
    /// The project that made it.
    pub(crate) project: String,
    /// Ticks run.
    pub(crate) ticks: u64,
    /// Game time, seconds.
    pub(crate) time: f64,
    /// The random seed.
    pub(crate) seed: u64,
    /// The area as it stood.
    pub(crate) area: String,
    /// Running components' values.
    pub(crate) values: String,
    /// The input actions.
    pub(crate) bindings: String,
    /// The projectiles in flight.
    pub(crate) projectiles: Vec<world::Projectile>,
    /// Physics, as bytes.
    pub(crate) physics: Vec<u8>,
}

impl Save {
    /// The save's text.
    pub(crate) fn write(&self) -> String {
        let mut body = String::new();
        let mut section = |name: &str, text: &str| {
            body.push_str(&format!("{name} {}\n{text}\n", text.len()));
        };
        section("area", &self.area);
        section("values", &self.values);
        section("bindings", &self.bindings);
        let projectiles: String = self
            .projectiles
            .iter()
            .map(|p| {
                let numbers = p
                    .position
                    .to_array()
                    .into_iter()
                    .chain(p.velocity.to_array())
                    .chain([p.left])
                    .chain(p.colour);
                let words: Vec<String> = numbers.map(|v| format!("{:x}", v.to_bits())).collect();
                words.join(" ") + "\n"
            })
            .collect();
        section("projectiles", &projectiles);
        section(
            "physics",
            &base64::engine::general_purpose::STANDARD.encode(&self.physics),
        );
        let mut sum = reflect::Checksum::new();
        sum.bytes(body.as_bytes());
        format!(
            "{HEADER}\nproject {}\ntick {}\ntime {:x}\nseed {}\nsum {:016x}\n{body}",
            self.project.replace('\n', " "),
            self.ticks,
            self.time.to_bits(),
            self.seed,
            sum.finish()
        )
    }

    /// Reads a save, refusing (with why, in plain words) anything that
    /// isn't one this Solis can load whole. Never panics.
    pub(crate) fn read(text: &str) -> Result<Self, String> {
        if text.len() > MAX_SAVE_BYTES {
            return Err("it's bigger than 256 MB".to_string());
        }
        let damaged = || "it's damaged".to_string();
        let mut rest = text;
        let mut line = || -> Option<&str> {
            let (line, after) = rest.split_once('\n')?;
            rest = after;
            Some(line)
        };
        if line() != Some(HEADER) {
            return Err(if text.starts_with("solis save ") {
                "it was made by a newer Solis".to_string()
            } else {
                "it isn't a save game".to_string()
            });
        }
        let mut field = |name: &str| -> Result<String, String> {
            line()
                .and_then(|l| l.strip_prefix(name))
                .and_then(|l| l.strip_prefix(' '))
                .map(str::to_string)
                .ok_or_else(damaged)
        };
        let project = field("project")?;
        let ticks = field("tick")?.parse().map_err(|_| damaged())?;
        let time = f64::from_bits(u64::from_str_radix(&field("time")?, 16).map_err(|_| damaged())?);
        let seed = field("seed")?.parse().map_err(|_| damaged())?;
        let sum = u64::from_str_radix(&field("sum")?, 16).map_err(|_| damaged())?;
        let body = rest;
        let mut check = reflect::Checksum::new();
        check.bytes(body.as_bytes());
        if check.finish() != sum {
            return Err("it's damaged (its checksum doesn't match)".to_string());
        }
        let mut rest = body;
        let mut section = |name: &str| -> Result<&str, String> {
            let (head, after) = rest.split_once('\n').ok_or_else(damaged)?;
            let length: usize = head
                .strip_prefix(name)
                .and_then(|l| l.strip_prefix(' '))
                .and_then(|l| l.parse().ok())
                .ok_or_else(damaged)?;
            let text = after.get(..length).ok_or_else(damaged)?;
            rest = after
                .get(length..)
                .and_then(|a| a.strip_prefix('\n'))
                .ok_or_else(damaged)?;
            Ok(text)
        };
        let area = section("area")?.to_string();
        let values = section("values")?.to_string();
        let bindings = section("bindings")?.to_string();
        let projectiles = section("projectiles")?
            .lines()
            .map(|l| {
                let numbers: Vec<f32> = l
                    .split(' ')
                    .map(|w| u32::from_str_radix(w, 16).map(f32::from_bits))
                    .collect::<Result<_, _>>()
                    .map_err(|_| damaged())?;
                let [x, y, z, vx, vy, vz, left, r, g, b] = numbers[..] else {
                    return Err(damaged());
                };
                Ok(world::Projectile {
                    position: glam::Vec3::new(x, y, z),
                    velocity: glam::Vec3::new(vx, vy, vz),
                    left,
                    colour: [r, g, b],
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let physics = base64::engine::general_purpose::STANDARD
            .decode(section("physics")?)
            .map_err(|_| damaged())?;
        Ok(Self {
            project,
            ticks,
            time,
            seed,
            area,
            values,
            bindings,
            projectiles,
            physics,
        })
    }
}

impl Workbench {
    /// The running game as a save (decision 0038), or why not.
    pub(crate) fn save_of_game(&mut self) -> Result<Save, String> {
        let project = self
            .project
            .as_ref()
            .map(|p| p.name.clone())
            .ok_or("save games are kept in a project")?;
        let registry = &self.document.registry;
        let playing = self.playing.as_mut().ok_or("nothing is running")?;
        // The area as it stands, each entity where the game has it.
        let mut area = self.document.area().clone();
        let ids: Vec<EntityId> = area.iter().map(|(id, _)| id).collect();
        for id in ids {
            if let (Some(t), Some(data)) = (playing.world.get::<Transform>(id), area.get_mut(id)) {
                data.set(&t);
            }
        }
        // Every running component's values, apart: the area keeps the
        // values its components started from, as the game's world does.
        let mut values = Area::new();
        if let Some(game) = &mut playing.game {
            for (entity, name, data) in game.all_values() {
                if values.get(entity).is_none() {
                    values.insert(entity, area::EntityData::new(""));
                }
                if let Some(holder) = values.get_mut(entity) {
                    holder.components.insert(name, data);
                }
            }
        }
        let clock = playing.world.clock();
        Ok(Save {
            project,
            ticks: clock.ticks,
            time: clock.time,
            seed: playing.world.seed(),
            area: area::text::write(&area, registry, &|_| None),
            values: area::text::write(&values, registry, &|_| None),
            bindings: playing.bindings.write(),
            projectiles: playing.world.projectiles().flying().to_vec(),
            physics: playing.world.physics().snapshot(),
        })
    }

    /// Saves the running game in `slot` (game code's `saveGame`): the app
    /// writes it to the project's `saves` folder.
    pub(crate) fn save_game(&mut self, slot: &str) {
        match self.save_of_game() {
            Ok(save) => {
                self.outbox.push(WorkbenchAction::SaveGame {
                    slot: slot.to_string(),
                    text: save.write(),
                });
            }
            Err(why) => self.log(LogLevel::Warning, format!("The game wasn't saved: {why}.")),
        }
    }

    /// Loads a save game (`text`, from the file called `name`): the game
    /// is put back exactly as it was saved, and carries on from there.
    /// Plays first if nothing is running. Stop still puts the area back
    /// as it was before Play.
    pub fn load_game(&mut self, text: &str, name: &str) {
        let save = match Save::read(text) {
            Ok(save) => save,
            Err(why) => {
                self.log(LogLevel::Error, format!("Couldn't load {name}: {why}."));
                return;
            }
        };
        if let Some(project) = &self.project
            && project.name != save.project
        {
            self.log(
                LogLevel::Error,
                format!(
                    "Couldn't load {name}: it was saved by the project {}, not this one.",
                    save.project
                ),
            );
            return;
        }
        let registry = &self.document.registry;
        let (area, values) = match (
            area::text::read(&save.area, registry),
            area::text::read(&save.values, registry),
        ) {
            (Ok(area), Ok(values)) => (area, values),
            (Err(why), _) | (_, Err(why)) => {
                self.log(LogLevel::Error, format!("Couldn't load {name}: {why}."));
                return;
            }
        };
        let bindings = match input::Bindings::read(&save.bindings) {
            Ok(b) => b,
            Err(why) => {
                self.log(
                    LogLevel::Error,
                    format!("Couldn't load {name}: its input {why}."),
                );
                return;
            }
        };
        // Physics first: if it can't be read, nothing changes.
        let mut world = world::World::from_area(&area);
        if let Err(why) = world.restore_physics(&save.physics) {
            self.log(LogLevel::Error, format!("Couldn't load {name}: {why}."));
            return;
        }
        world.set_clock(save.ticks, save.time);
        world.set_seed(save.seed);
        world.projectiles_mut().restore(save.projectiles);
        if self.playing.is_none() {
            self.play();
        }
        if self.playing.is_none() {
            return;
        }
        // The area as it was saved, for the editor's panels; not undone
        // steps, and Stop still goes back to before Play.
        self.give_back_input();
        self.document.replace_by_game(&area);
        let _ = self.document.take_changes();
        let mut view = crate::game_code::ViewCache::default();
        let (mut game, mut pending) = self.start_game(&mut world, &mut view);
        // The running components take their saved values.
        if let Some(game) = &mut game {
            let mut host = crate::game_code::PlayWorld::new(&mut world, &mut view, save.time);
            let components: Vec<(EntityId, String, reflect::ComponentData)> = values
                .iter()
                .flat_map(|(id, data)| {
                    data.components
                        .iter()
                        .map(move |(n, d)| (id, n.clone(), d.clone()))
                })
                .collect();
            let failures = game.sync_many(
                components.iter().map(|(e, n, d)| (*e, n.as_str(), d)),
                &mut host,
            );
            pending.extend(std::mem::take(&mut host.edits));
            for failure in failures {
                let text = crate::game_code::failure_text(self.document.area(), &failure);
                self.log(LogLevel::Error, text);
            }
        }
        let Some(playing) = &mut self.playing else {
            return;
        };
        playing.world = world;
        playing.view = view;
        playing.game = game;
        playing.pending = pending;
        playing.bindings = Arc::new(bindings);
        // A recording or replay can't carry on across a load.
        let stopped = playing.recording.take().is_some() | playing.replay.take().is_some();
        self.log(
            LogLevel::Info,
            format!(
                "Loaded {name}: the game is back as it was {:.1} s in.",
                save.time
            ),
        );
        if stopped {
            self.log(
                LogLevel::Warning,
                "Loading a save ended the recording or replay: it only plays back from Play.",
            );
        }
    }
}

#[cfg(test)]
#[path = "save_game_tests.rs"]
mod tests;
