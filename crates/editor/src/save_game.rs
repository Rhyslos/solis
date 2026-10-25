//! Save games ([decision 0038](../../../docs/decisions/0038-save-games-hold-everything.md)):
//! the whole running game, so loading carries on exactly as the saved
//! game would have, checksum and all.
//!
//! A save is text, in sections, so it can be read (and its area even
//! opened) by a person:
//!
//! ```text
//! solis save 2
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
//! physics 2
//! ID x y z qx qy qz qw vx vy vz wx wy wz asleep character (exact bits)
//! ```
//!
//! Physics is kept as each body's place and motion, never Rapier's own
//! insides: saving builds the running game's physics again from those
//! (so it carries on exactly as a loaded save will), and loading builds
//! it from them, so nothing in a file can reach Rapier.
//!
//! `sum` is a checksum of everything after it, so a damaged save is
//! refused, never half loaded.

use crate::console::LogLevel;
use crate::workbench::{Workbench, WorkbenchAction};
use area::Area;
use area::builtin::Transform;
use reflect::EntityId;
use std::sync::Arc;

/// The first line of a save this Solis writes and reads.
const HEADER: &str = "solis save 2";

/// The biggest save read.
pub(crate) const MAX_SAVE_BYTES: usize = 256 << 20;

/// The most ticks a save's clock is read with (2^53: still counted
/// exactly, and centuries of play).
const MAX_SAVED_TICKS: u64 = 1 << 53;

/// A save's checksum: of its header's lines and everything after them.
fn sum_of(head: &str, body: &str) -> u64 {
    let mut sum = reflect::Checksum::new();
    sum.bytes(head.as_bytes());
    sum.bytes(body.as_bytes());
    sum.finish()
}

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
    /// Each body's place and motion, in entity order.
    pub(crate) physics: Vec<(EntityId, physics::BodyState)>,
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
        let bodies: String = self
            .physics
            .iter()
            .map(|(id, s)| {
                let numbers = s
                    .position
                    .to_array()
                    .into_iter()
                    .chain(s.rotation.to_array())
                    .chain(s.velocity.to_array())
                    .chain(s.spin.to_array());
                let words: Vec<String> = numbers.map(|v| format!("{:x}", v.to_bits())).collect();
                let character = match s.character {
                    None => "-",
                    Some(c) if c.grounded => "1",
                    Some(_) => "0",
                };
                format!(
                    "{id} {} {} {character}\n",
                    words.join(" "),
                    u8::from(s.sleeping)
                )
            })
            .collect();
        section("physics", &bodies);
        let head = format!(
            "project {}\ntick {}\ntime {:x}\nseed {}\n",
            self.project.replace('\n', " "),
            self.ticks,
            self.time.to_bits(),
            self.seed,
        );
        format!("{HEADER}\n{head}sum {:016x}\n{body}", sum_of(&head, &body))
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
            let version = text
                .strip_prefix("solis save ")
                .and_then(|r| r.split('\n').next())
                .and_then(|v| v.trim().parse::<u32>().ok());
            return Err(if version.is_some_and(|v| v < 2) {
                "it was made by an earlier Solis, whose saves this one can't read".to_string()
            } else if version.is_some() {
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
        let tick_text = field("tick")?;
        let time_text = field("time")?;
        let seed_text = field("seed")?;
        let sum = u64::from_str_radix(&field("sum")?, 16).map_err(|_| damaged())?;
        let body = rest;
        let head =
            format!("project {project}\ntick {tick_text}\ntime {time_text}\nseed {seed_text}\n");
        if sum_of(&head, body) != sum {
            return Err("it's damaged (its checksum doesn't match)".to_string());
        }
        let ticks: u64 = tick_text.parse().map_err(|_| damaged())?;
        let time = f64::from_bits(u64::from_str_radix(&time_text, 16).map_err(|_| damaged())?);
        let seed = seed_text.parse().map_err(|_| damaged())?;
        // A sensible clock: at most about 4,700 years of ticks.
        if ticks > MAX_SAVED_TICKS || !(time.is_finite() && (0.0..1.0e13).contains(&time)) {
            return Err(damaged());
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
        let physics_text = section("physics")?;
        if physics_text.lines().count() > physics::MAX_BODIES {
            return Err(damaged());
        }
        let physics = physics_text
            .lines()
            .map(|l| {
                let words: Vec<&str> = l.split(' ').collect();
                let [id, numbers @ .., asleep, character] = &words[..] else {
                    return Err(damaged());
                };
                let id: reflect::Id = id.parse().map_err(|_| damaged())?;
                let n: Vec<f32> = numbers
                    .iter()
                    .map(|w| u32::from_str_radix(w, 16).map(f32::from_bits))
                    .collect::<Result<_, _>>()
                    .map_err(|_| damaged())?;
                let [x, y, z, qx, qy, qz, qw, vx, vy, vz, wx, wy, wz] = n[..] else {
                    return Err(damaged());
                };
                let state = physics::BodyState {
                    position: glam::Vec3::new(x, y, z),
                    rotation: glam::Quat::from_xyzw(qx, qy, qz, qw),
                    velocity: glam::Vec3::new(vx, vy, vz),
                    spin: glam::Vec3::new(wx, wy, wz),
                    sleeping: match *asleep {
                        "0" => false,
                        "1" => true,
                        _ => return Err(damaged()),
                    },
                    character: match *character {
                        "-" => None,
                        "0" => Some(physics::CharacterState { grounded: false }),
                        "1" => Some(physics::CharacterState { grounded: true }),
                        _ => return Err(damaged()),
                    },
                };
                if !state.is_sensible() {
                    return Err(damaged());
                }
                Ok((EntityId(id), state))
            })
            .collect::<Result<Vec<_>, String>>()?;
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
        // Physics built again from each body's place and motion, as loading
        // will: from here the running game and a loaded one go on alike.
        let bodies = playing.world.physics_states();
        playing.world.rebuild_physics(&bodies, true);
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
            physics: bodies,
        })
    }

    /// Saves the running game in `slot` (game code's `saveGame`): the app
    /// writes it to the project's `saves` folder.
    pub(crate) fn save_game(&mut self, slot: &str) {
        match self.save_of_game() {
            Ok(save) => {
                let text = save.write();
                if text.len() > MAX_SAVE_BYTES {
                    self.log(
                        LogLevel::Warning,
                        format!(
                            "The game wasn't saved: it would be {} MB, more than the {} MB a save \
                             can be loaded from.",
                            text.len() >> 20,
                            MAX_SAVE_BYTES >> 20
                        ),
                    );
                    return;
                }
                self.outbox.push(WorkbenchAction::SaveGame {
                    slot: slot.to_string(),
                    text,
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
        // Physics built from each body's place and motion, as saving built
        // the running game's.
        let mut world = world::World::from_area(&area);
        world.rebuild_physics(&save.physics, true);
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
        // Game code starts on a world of its own, thrown away after: what
        // starts do (spawn, move, set the seed) already happened in the
        // saved game, so they mustn't happen again on top of it.
        let mut scratch = world::World::from_area(&area);
        let mut scratch_view = crate::game_code::ViewCache::default();
        let (mut game, _) = self.start_game(&mut scratch, &mut scratch_view);
        drop(scratch);
        let mut pending = Vec::new();
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
