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
const HEADER: &str = "solis save 3";
/// The version in [`HEADER`].
const SAVE_VERSION: u32 = 3;

/// Save games' version and upgraders (Phase 8, C2; decision 0053): a save
/// made by an earlier Solis loads in a later one, never refused just for
/// being older. Each step turns one version's sections into the next's,
/// before they're read; what a step couldn't carry over goes in the
/// notes, which the Console shows. Every version keeps an example save in
/// `tests/fixtures/save/`.
pub static FORMAT: reflect::upgrade::Format<Sections> = reflect::upgrade::Format {
    name: "save game",
    current: SAVE_VERSION,
    version_of: |text| {
        text.split('\n')
            .next()?
            .strip_prefix("solis save ")?
            .trim()
            .parse()
            .ok()
    },
    steps: &[physics_as_places_and_motion, places_kept_apart],
};

/// A save's text read and written again: an older save in a slot is
/// replaced only by a save that reads back as itself (Phase 8, C2).
pub fn save_read_back(text: &str) -> Result<String, String> {
    Save::read(text).map(|save| save.write())
}

/// The most sections a save has (it writes six).
const MAX_SECTIONS: usize = 16;

/// A save's sections as written (`name length`, then that much text),
/// before they're read, with notes on what upgrading changed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sections {
    /// Each section's name and text, in order.
    list: Vec<(String, String)>,
    /// What upgrading couldn't carry over, in plain words.
    notes: Vec<String>,
}

impl Sections {
    /// The text of the section called `name`; an error when it's missing.
    fn take(&mut self, name: &str) -> Result<String, String> {
        let at = self
            .list
            .iter()
            .position(|(n, _)| n == name)
            .ok_or_else(|| "it's damaged".to_string())?;
        Ok(self.list.remove(at).1)
    }
}

/// Version 1 to 2: version 1 kept physics as Rapier's own bytes, which
/// Solis no longer reads (a file must never reach Rapier's insides). The
/// bodies start at rest where the save has them.
fn physics_as_places_and_motion(sections: &mut Sections) -> Result<(), String> {
    let physics = sections
        .list
        .iter_mut()
        .find(|(n, _)| n == "physics")
        .ok_or("it has no physics")?;
    if !physics.1.is_empty() {
        sections.notes.push(
            "it was made by an earlier Solis whose physics can't be carried over, so things \
             that were moving start at rest where they were"
                .to_string(),
        );
    }
    physics.1.clear();
    Ok(())
}

/// Version 2 to 3: version 2 kept where the game had moved each entity in
/// the area itself, which loads the same with no places apart.
fn places_kept_apart(sections: &mut Sections) -> Result<(), String> {
    if sections.list.iter().any(|(n, _)| n == "places") {
        return Err("it already has places".into());
    }
    sections.list.push(("places".into(), String::new()));
    Ok(())
}

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
    /// The area as the editor had it (copies of assemblies kept as plain
    /// entities): where the game moved things is in `places`, as the
    /// running game keeps them apart.
    pub(crate) area: String,
    /// Running components' values.
    pub(crate) values: String,
    /// The input actions.
    pub(crate) bindings: String,
    /// The projectiles in flight.
    pub(crate) projectiles: Vec<world::Projectile>,
    /// Each body's place and motion, in entity order.
    pub(crate) physics: Vec<(EntityId, physics::BodyState)>,
    /// Where the game has each entity it moved (its transform), where
    /// that differs from the area's, in entity order.
    pub(crate) places: Vec<(EntityId, Transform)>,
    /// What upgrading an older save couldn't carry over, for the Console
    /// (never written).
    pub(crate) notes: Vec<String>,
}

/// A transform's ten numbers, exactly, as hex words.
fn transform_words(t: &Transform) -> String {
    t.position
        .to_array()
        .into_iter()
        .chain(t.rotation.to_array())
        .chain(t.scale.to_array())
        .map(|v| format!("{:x}", v.to_bits()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The project's name on one line, as a save keeps it (a name with a
/// line break in it would otherwise break the save's lines).
fn one_line(name: &str) -> String {
    name.replace(['\n', '\r'], " ")
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
        let places: String = self
            .places
            .iter()
            .map(|(id, t)| format!("{id} {}\n", transform_words(t)))
            .collect();
        section("places", &places);
        let head = format!(
            "project {}\ntick {}\ntime {:x}\nseed {}\n",
            one_line(&self.project),
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
        debug_assert_eq!(HEADER, format!("solis save {SAVE_VERSION}"));
        let version = (FORMAT.version_of)(text);
        let first = line();
        match version {
            Some(v) if v > SAVE_VERSION => return Err("it was made by a newer Solis".to_string()),
            Some(0) | None => return Err("it isn't a save game".to_string()),
            // The header, with something after it (a line ending changed
            // by an editor, say).
            Some(v) if first != Some(format!("solis save {v}").as_str()) => return Err(damaged()),
            Some(_) => {}
        }
        let version = version.unwrap_or(SAVE_VERSION);
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
        // A fuzzer can't make a matching checksum, so while fuzzing it's
        // left unchecked and the rest of the reader is tried (decision
        // 0042); everywhere else a damaged save is refused here.
        // Saves before version 3 summed only what follows the header
        // lines, in their first builds.
        let body_only = || {
            let mut check = reflect::Checksum::new();
            check.bytes(body.as_bytes());
            check.finish()
        };
        let matches = sum_of(&head, body) == sum || (version < 3 && body_only() == sum);
        if !matches && !cfg!(fuzzing) {
            return Err("it's damaged (its checksum doesn't match)".to_string());
        }
        let ticks: u64 = tick_text.parse().map_err(|_| damaged())?;
        let time = f64::from_bits(u64::from_str_radix(&time_text, 16).map_err(|_| damaged())?);
        let seed = seed_text.parse().map_err(|_| damaged())?;
        // A sensible clock: at most about 4,700 years of ticks.
        if ticks > MAX_SAVED_TICKS || !(time.is_finite() && (0.0..1.0e13).contains(&time)) {
            return Err(damaged());
        }
        // Every section as written, then brought up to date.
        let mut sections = Sections::default();
        let mut rest = body;
        while !rest.is_empty() {
            if sections.list.len() >= MAX_SECTIONS {
                return Err(damaged());
            }
            let (head, after) = rest.split_once('\n').ok_or_else(damaged)?;
            let (name, length) = head.split_once(' ').ok_or_else(damaged)?;
            let length: usize = length.parse().map_err(|_| damaged())?;
            let text = after.get(..length).ok_or_else(damaged)?;
            rest = after
                .get(length..)
                .and_then(|a| a.strip_prefix('\n'))
                .ok_or_else(damaged)?;
            if sections.list.iter().any(|(n, _)| n == name) {
                return Err(damaged());
            }
            sections.list.push((name.to_string(), text.to_string()));
        }
        FORMAT
            .upgrade(version, &mut sections)
            .map_err(|e| e.to_string())?;
        let mut section = |name: &str| sections.take(name);
        let area = section("area")?;
        let values = section("values")?;
        let bindings = section("bindings")?;
        let projectiles_text = &section("projectiles")?;
        // Counted before any is made: a hand-made save could list far more.
        if projectiles_text.lines().count() > world::MAX_PROJECTILES {
            return Err(damaged());
        }
        let projectiles = projectiles_text
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
        let physics_text = &section("physics")?;
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
        let places_text = &section("places")?;
        if places_text.lines().count() > area::text::MAX_ENTITIES {
            return Err(damaged());
        }
        let places = places_text
            .lines()
            .map(|l| {
                let (id, rest) = l.split_once(' ').ok_or_else(damaged)?;
                let id: reflect::Id = id.parse().map_err(|_| damaged())?;
                let n: Vec<f32> = rest
                    .split(' ')
                    .map(|w| u32::from_str_radix(w, 16).map(f32::from_bits))
                    .collect::<Result<_, _>>()
                    .map_err(|_| damaged())?;
                let [x, y, z, qx, qy, qz, qw, sx, sy, sz] = n[..] else {
                    return Err(damaged());
                };
                let t = Transform {
                    position: glam::Vec3::new(x, y, z),
                    rotation: glam::Quat::from_xyzw(qx, qy, qz, qw),
                    scale: glam::Vec3::new(sx, sy, sz),
                };
                let sensible = t.position.is_finite()
                    && t.rotation.is_finite()
                    && t.scale.is_finite()
                    && (t.rotation.length_squared() - 1.0).abs() < 1.0e-3;
                if !sensible {
                    return Err(damaged());
                }
                Ok((EntityId(id), t))
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
            places,
            notes: {
                // A section no version writes: not a save this Solis made.
                if !sections.list.is_empty() {
                    return Err(damaged());
                }
                std::mem::take(&mut sections.notes)
            },
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
        // The area as the editor has it, and apart from it where the
        // game has moved each entity: the running game keeps the two apart
        // (an edit while playing starts from the editor's, and a rebuild
        // keeps the game's), so a loaded game must too, or it goes on
        // differently from the saved one (found by the hardening
        // milestone's invariant tests, decision 0042).
        let mut area = self.document.area().clone();
        let mut places = Vec::new();
        let ids: Vec<EntityId> = area.iter().map(|(id, _)| id).collect();
        for id in ids {
            if let Some(data) = area.get_mut(id) {
                if let Some(t) = playing.world.get::<Transform>(id)
                    && data.get::<Transform>() != Some(t)
                {
                    places.push((id, t));
                }
                // A sound game code turned on or off stays so after a
                // load (Phase 7, S5).
                if let Some(source) = playing.world.get::<area::builtin::AudioSource>(id)
                    && data.get::<area::builtin::AudioSource>().as_ref() != Some(&source)
                {
                    data.set(&source);
                }
                // Every entity whole: a copy of an assembly (placed, or
                // spawned by game code) is kept as the entities it is
                // now, not as "a copy of" (which loses what's on its root,
                // and can't be read back on its own). Found by the
                // hardening milestone's invariant tests (decision 0042).
                data.origin = None;
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
            places,
            notes: Vec::new(),
        })
    }

    /// The running game as a save's text (game code's `saveGame`; the app
    /// writes it to the project's `saves` folder), or `None` (and why, in
    /// the Console). A tick's saves share one: physics is built again
    /// once.
    pub(crate) fn save_text(&mut self) -> Option<String> {
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
                    return None;
                }
                Some(text)
            }
            Err(why) => {
                self.log(LogLevel::Warning, format!("The game wasn't saved: {why}."));
                None
            }
        }
    }

    /// Loads a save game (`text`, from the file called `name`): the game
    /// is put back exactly as it was saved, and carries on from there.
    /// Plays first if nothing is running. Stop still puts the area back
    /// as it was before Play. The editor gets the mouse and keys back (a
    /// load from the Play menu).
    pub fn load_game(&mut self, text: &str, name: &str) {
        self.load_game_with(text, name, false);
    }

    /// As [`Self::load_game`], for game code's `loadGame` (a quick-load
    /// key): the game keeps the mouse and keys, so playing goes on.
    pub fn load_game_from_code(&mut self, text: &str, slot: &str) {
        self.load_game_with(text, slot, true);
    }

    fn load_game_with(&mut self, text: &str, name: &str, keep_input: bool) {
        let name_shown = name.to_string();
        let save = match Save::read(text) {
            Ok(save) => save,
            Err(why) => {
                self.log(LogLevel::Error, format!("Couldn't load {name}: {why}."));
                return;
            }
        };
        if let Some(project) = &self.project
            && one_line(&project.name) != save.project
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
                    format!("Couldn't load {name}: its input actions can't be read: {why}."),
                );
                return;
            }
        };
        // An older save loads, saying what couldn't be carried over.
        for note in &save.notes {
            self.log(LogLevel::Warning, format!("Loading {name}: {note}."));
        }
        // The loaded game sounds afresh: what played before the load fades
        // out, and nothing queued from before it plays after.
        self.plays += 1;
        self.game_sounds.clear();
        // Physics built from each body's place and motion, as saving built
        // the running game's.
        let mut world = world::World::from_area_without_physics(&area);
        // Each entity where the game had it, noted as the game's move, as
        // in the saved game.
        for (id, t) in &save.places {
            world.set_transform(*id, *t);
        }
        world.settle();
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
        if keep_input {
            // What's held now isn't held in the loaded game.
            self.devices.release_all();
        } else {
            self.give_back_input();
        }
        self.document.replace_by_game(&area);
        let _ = self.document.take_changes();
        let mut view = crate::game_code::ViewCache::default();
        // Game code starts on a world of its own, thrown away after: what
        // starts do (spawn, move, set the seed) already happened in the
        // saved game, so they mustn't happen again on top of it.
        let mut scratch = world::World::from_area_without_physics(&area);
        let mut scratch_view = crate::game_code::ViewCache::default();
        let (mut game, _) = self.start_game_saying(&mut scratch, &mut scratch_view, false);
        drop(scratch);
        let mut pending = Vec::new();
        // The running components take their saved values.
        if let Some(game) = &mut game {
            let mut host = crate::game_code::PlayWorld::new(&mut world, &mut view, save.time);
            // Only values for components the saved area has on that entity
            // (which are what's running): a hand-made save can't give an
            // entity a component this way.
            let components: Vec<(EntityId, String, reflect::ComponentData)> = values
                .iter()
                .flat_map(|(id, data)| {
                    let has = area.get(id).map(|e| &e.components);
                    data.components
                        .iter()
                        .filter(move |(n, _)| has.is_some_and(|c| c.contains_key(*n)))
                        .map(move |(n, d)| (id, n.clone(), d.clone()))
                })
                .collect();
            // Values of components the game's code no longer has (the code
            // changed since the save): left out, and said once each.
            let mut gone: Vec<&str> = components
                .iter()
                .map(|(_, n, _)| n.as_str())
                .filter(|n| !game.has_component(n))
                .collect();
            gone.sort_unstable();
            gone.dedup();
            for name in gone {
                self.console.push(
                    LogLevel::Warning,
                    format!(
                        "Loading {name_of_save}: the game's code has no component {name} any \
                         more, so its saved values were left out.",
                        name_of_save = name_shown
                    ),
                );
            }
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
        // A recording ends at a load (it plays back from Play), but what
        // was recorded up to it is kept, as Stop keeps it.
        let recorded = playing.recording.take();
        let stopped = recorded.is_some() | playing.replay.take().is_some();
        if let Some(recording) = recorded.filter(|r| !r.ticks.is_empty()) {
            self.outbox
                .push(WorkbenchAction::SaveRecording(recording.write()));
        }
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
                "Loading a save ended the recording or replay (a recording plays back from \
                 Play; what was recorded up to the load is saved).",
            );
        }
    }
}

#[cfg(test)]
#[path = "save_game_tests.rs"]
mod tests;
