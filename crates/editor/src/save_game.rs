//! Save games ([decision 0038](../../../docs/decisions/0038-save-games-hold-everything.md)):
//! the whole running game, so loading carries on exactly as the saved
//! game would have, checksum and all.
//!
//! A save is text, in sections, so it can be read (and its area even
//! opened) by a person:
//!
//! ```text
//! solis save 8
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
//! places 1
//! ID x y z qx qy qz qw sx sy sz (exact bits)
//! animators 1
//! ID moving=s1 speed=n3fc00000 facing=d0,3f800000 (exact bits)
//! tilemaps 1234
//! ID 1220
//! …the map as the game has changed it (a tile map file)…
//! objects 140
//! …the game-wide objects' values, one block holding them (area text)…
//! speed 16
//! 3fe0000000000000 (the game's speed's exact bits; empty when 1)
//! day 16
//! 4018000000000000 (the hours game code moved the day on by; empty when 0)
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
use crate::workbench::Workbench;
use area::Area;
use area::builtin::Transform;
use reflect::EntityId;

/// The first line of a save this Solis writes and reads.
const HEADER: &str = "solis save 8";
/// The version in [`HEADER`].
const SAVE_VERSION: u32 = 8;

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
    steps: &[
        reading::physics_as_places_and_motion,
        reading::places_kept_apart,
        reading::animators_kept,
        reading::tile_maps_kept,
        reading::objects_kept,
        reading::speed_kept,
        reading::day_kept,
    ],
};

/// A save's text read and written again: an older save in a slot is
/// replaced only by a save that reads back as itself (Phase 8, C2).
pub fn save_read_back(text: &str) -> Result<String, String> {
    Save::read(text).and_then(|save| save.write())
}

/// Whether `text` reads as a save (a project's check: read, not written
/// again, which would hold a 256 MB save twice).
pub fn save_reads(text: &str) -> Result<(), String> {
    Save::read(text).map(drop)
}

/// The most sections a save has (it writes eleven).
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

/// The game-wide objects' values as a save keeps them: area text, one
/// entity whose components are the objects (none: empty). Written with no
/// registry, so an object named like a component keeps its own fields.
fn objects_text(objects: &script::ObjectValues) -> String {
    if objects.is_empty() {
        return String::new();
    }
    let mut holder = area::EntityData::new("objects");
    holder.components = objects.iter().cloned().collect();
    let mut area = Area::new();
    area.insert(objects_holder(), holder);
    area::text::write(&area, &reflect::Registry::new(), &|_| None)
}

/// The entity a save's objects are kept on.
fn objects_holder() -> EntityId {
    EntityId(reflect::Id::from_raw(1).unwrap_or_else(|| unreachable!("1 is an ID")))
}

/// A save's objects section read back, each value fitted to its field in
/// `schemas` (the objects as the running code has them).
fn objects_of(
    text: &str,
    schemas: &[reflect::ComponentSchema],
) -> Result<script::ObjectValues, String> {
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let area = area::text::read(text, &reflect::Registry::new()).map_err(|e| e.to_string())?;
    let holder = area
        .get(objects_holder())
        .ok_or("its objects are damaged")?;
    Ok(holder
        .components
        .iter()
        .map(|(name, data)| {
            let schema = schemas.iter().find(|s| s.name == *name);
            let mut fitted = reflect::ComponentData::default();
            for (field, value) in &data.fields {
                let value = match schema.and_then(|s| s.fields.iter().find(|f| f.name == *field)) {
                    Some(info) => area::text::fit_value(value.clone(), &info.kind),
                    None => value.clone(),
                };
                fitted.set(field, value);
            }
            (name.clone(), fitted)
        })
        .collect())
}

/// The biggest save read.
pub(crate) const MAX_SAVE_BYTES: usize = 256 << 20;

/// The most ticks a save's clock is read with (2^53: still counted
/// exactly, and centuries of play).
const MAX_SAVED_TICKS: u64 = 1 << 53;

/// The words on a projectile's line: its ten numbers.
const PROJECTILE_WORDS: usize = 10;
/// The words on a body's line: its ID, thirteen numbers, asleep and
/// character.
const BODY_WORDS: usize = 16;
/// The words after the ID on a place's line: its transform's ten numbers.
const PLACE_WORDS: usize = 10;

/// The words of a line that can have `most`, and one more when it has
/// more, so too many are seen (and the line refused) without a list of
/// them all being made first: a hand-made save can put a hundred million
/// words on one line.
fn words_of(line: &str, most: usize) -> impl Iterator<Item = &str> {
    line.split(' ').take(most + 1)
}

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
    /// Each animator's parameters, by name, in entity order (its state is
    /// its Animator's, in the area).
    pub(crate) animators: Vec<(EntityId, Vec<(String, animation::ParamValue)>)>,
    /// Each tile map as the game has changed it, in entity order.
    pub(crate) tile_maps: Vec<(EntityId, tiles::TileMap)>,
    /// The game-wide objects' values (decision 0080), as area text: one
    /// entity, `objects`, whose components are the objects. Empty when
    /// the game has none.
    pub(crate) objects: String,
    /// How fast the game ran (`World:setGameSpeed`; 1 as usual).
    pub(crate) speed: f64,
    /// The hours game code moved the day on by (`World:setTimeOfDay`; 0
    /// as usual).
    pub(crate) day_shift: f64,
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
    /// The save's text, or why it can't be written: a tile map the game
    /// has grown past what a save is loaded from (its file is read back
    /// as any map file is), since a save that couldn't be loaded would be
    /// the player's progress lost.
    pub(crate) fn write(&self) -> Result<String, String> {
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
        let animators: String = self
            .animators
            .iter()
            .map(|(id, values)| {
                let words: String = values
                    .iter()
                    .map(|(name, v)| format!(" {name}={}", world::animators::value_words(*v)))
                    .collect();
                format!("{id}{words}\n")
            })
            .collect();
        section("animators", &animators);
        let mut maps = String::new();
        for (id, map) in &self.tile_maps {
            let text = tiles::map_file::write(map).map_err(|why| {
                format!("the tile map on {id} has grown too big to be loaded again ({why})")
            })?;
            maps.push_str(&format!("{id} {}\n{text}\n", text.len()));
        }
        section("tilemaps", &maps);
        section("objects", &self.objects);
        // Its exact bits, or nothing when it's as usual.
        let speed = if self.speed == 1.0 {
            String::new()
        } else {
            format!("{:x}", self.speed.to_bits())
        };
        section("speed", &speed);
        let day = if self.day_shift == 0.0 {
            String::new()
        } else {
            format!("{:x}", self.day_shift.to_bits())
        };
        section("day", &day);
        let head = format!(
            "project {}\ntick {}\ntime {:x}\nseed {}\n",
            one_line(&self.project),
            self.ticks,
            self.time.to_bits(),
            self.seed,
        );
        let sum = sum_of(&head, &body);
        Ok(format!("{HEADER}\n{head}sum {sum:016x}\n{body}"))
    }
}

impl Workbench {
    /// The running game as a save (decision 0038), or why not.
    pub(crate) fn save_of_game(&mut self) -> Result<Save, String> {
        let project = self
            .open
            .project
            .as_ref()
            .map(|p| p.name.clone())
            .ok_or("save games are kept in a project")?;
        let registry = &self.open.document.registry;
        let playing = self.open.playing.as_mut().ok_or("nothing is running")?;
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
        let mut area = self.open.document.area().clone();
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
                // And a screen game code showed or hid (decision 0087).
                if let Some(screen) = playing
                    .world
                    .values_named_for(id, <area::builtin::Screen as reflect::Component>::NAME)
                    && data
                        .components
                        .get(<area::builtin::Screen as reflect::Component>::NAME)
                        != Some(&screen)
                {
                    data.components.insert(
                        <area::builtin::Screen as reflect::Component>::NAME.to_string(),
                        screen,
                    );
                }
                // And a sprite's clip, as far as it had played (Phase 9,
                // D3).
                if let Some(sprite) = playing.world.get::<area::builtin::Sprite>(id)
                    && data.get::<area::builtin::Sprite>().as_ref() != Some(&sprite)
                {
                    data.set(&sprite);
                }
                // And its animator's state (D3b); its parameters are in
                // their own section.
                if let Some(animator) = playing.world.get::<area::builtin::Animator>(id)
                    && data.get::<area::builtin::Animator>().as_ref() != Some(&animator)
                {
                    data.set(&animator);
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
            animators: playing.world.animator_values(),
            tile_maps: playing.world.tile_maps(),
            objects: playing
                .game
                .as_mut()
                .map(|game| objects_text(&game.object_values()))
                .unwrap_or_default(),
            speed: playing.world.speed(),
            day_shift: playing.world.day_shift(),
            notes: Vec::new(),
        })
    }

    /// The running game as a save's text (game code's `saveGame`; the app
    /// writes it to the project's `saves` folder), or `None` (and why, in
    /// the Console). A tick's saves share one: physics is built again
    /// once.
    pub(crate) fn save_text(&mut self) -> Option<String> {
        match self.save_of_game().and_then(|save| save.write()) {
            Ok(text) => {
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
}

#[path = "save_reading.rs"]
mod reading;

#[path = "save_loading.rs"]
mod loading;

#[cfg(test)]
#[path = "save_game_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "save_game_objects_tests.rs"]
mod objects_tests;
