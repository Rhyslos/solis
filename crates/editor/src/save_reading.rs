//! Reading a save game back (`Save::read`): its header and checksum,
//! its sections (an older save's brought up to date first, by the
//! upgraders here), and each section within its limits. Anything that
//! isn't a save this Solis can load whole is refused, saying why in
//! plain words; nothing panics.

use super::{
    BODY_WORDS, FORMAT, HEADER, MAX_SAVE_BYTES, MAX_SAVED_TICKS, MAX_SECTIONS, PLACE_WORDS,
    PROJECTILE_WORDS, SAVE_VERSION, Save, Sections, sum_of, words_of,
};
use area::builtin::Transform;
use reflect::EntityId;

/// Why a save that can't be read whole is refused.
fn damaged() -> String {
    "it's damaged".to_string()
}

/// A save's header lines, read and checked against its checksum.
struct Head<'a> {
    /// The version it was written in.
    version: u32,
    /// The project that made it.
    project: String,
    /// Ticks run.
    ticks: u64,
    /// Game time, seconds.
    time: f64,
    /// The random seed.
    seed: u64,
    /// Everything after the header: the sections.
    body: &'a str,
}

impl Save {
    /// Reads a save, refusing (with why, in plain words) anything that
    /// isn't one this Solis can load whole. Never panics.
    pub(crate) fn read(text: &str) -> Result<Self, String> {
        let Head {
            version,
            project,
            ticks,
            time,
            seed,
            body,
        } = read_head(text)?;
        let found = split_sections(body)?;
        // An older save's sections are copied and brought up to date; a
        // current one's are read where they are (a save can be 256 MB).
        let mut sections = Sections::default();
        let mut list: Vec<(&str, &str)> = if version == SAVE_VERSION {
            found
        } else {
            sections.list = found
                .into_iter()
                .map(|(n, t)| (n.to_string(), t.to_string()))
                .collect();
            FORMAT
                .upgrade(version, &mut sections)
                .map_err(|e| e.to_string())?;
            sections
                .list
                .iter()
                .map(|(n, t)| (n.as_str(), t.as_str()))
                .collect()
        };
        let mut section = |name: &str| -> Result<&str, String> {
            let at = list
                .iter()
                .position(|(n, _)| *n == name)
                .ok_or_else(damaged)?;
            Ok(list.remove(at).1)
        };
        let area = section("area")?.to_string();
        let values = section("values")?.to_string();
        let bindings = section("bindings")?.to_string();
        let projectiles = read_projectiles(section("projectiles")?)?;
        let physics = read_physics(section("physics")?)?;
        let places = read_places(section("places")?)?;
        let animators = read_animators(section("animators")?)?;
        let tile_maps = read_tile_maps(section("tilemaps")?)?;
        let objects = section("objects")?.to_string();
        let speed = read_speed(section("speed")?)?;
        let day_shift = read_day_shift(section("day")?)?;
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
            animators,
            tile_maps,
            objects,
            speed,
            day_shift,
            notes: {
                // A section no version writes: not a save this Solis made.
                if !list.is_empty() {
                    return Err(damaged());
                }
                std::mem::take(&mut sections.notes)
            },
        })
    }
}

/// A save's header: its size, version and lines checked, and its
/// checksum matched against the rest (unchecked while fuzzing).
fn read_head(text: &str) -> Result<Head<'_>, String> {
    if text.len() > MAX_SAVE_BYTES {
        return Err("it's bigger than 256 MB".to_string());
    }
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
    let head = format!("project {project}\ntick {tick_text}\ntime {time_text}\nseed {seed_text}\n");
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
    Ok(Head {
        version,
        project,
        ticks,
        time,
        seed,
        body,
    })
}

/// Every section as written (`name length`, then that much text),
/// borrowed from the save's text: each name once, at most
/// `MAX_SECTIONS`.
fn split_sections(body: &str) -> Result<Vec<(&str, &str)>, String> {
    let mut found: Vec<(&str, &str)> = Vec::new();
    let mut rest = body;
    while !rest.is_empty() {
        if found.len() >= MAX_SECTIONS {
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
        if found.iter().any(|(n, _)| *n == name) {
            return Err(damaged());
        }
        found.push((name, text));
    }
    Ok(found)
}

/// The projectiles in flight, each its ten numbers' exact bits.
fn read_projectiles(projectiles_text: &str) -> Result<Vec<world::Projectile>, String> {
    // Counted before any is made: a hand-made save could list far more.
    if projectiles_text.lines().count() > world::MAX_PROJECTILES {
        return Err(damaged());
    }
    projectiles_text
        .lines()
        .map(|l| {
            let numbers: Vec<f32> = words_of(l, PROJECTILE_WORDS)
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
        .collect::<Result<Vec<_>, String>>()
}

/// Each body's place and motion, and whether it sleeps or is a
/// character (grounded or not).
fn read_physics(physics_text: &str) -> Result<Vec<(EntityId, physics::BodyState)>, String> {
    if physics_text.lines().count() > physics::MAX_BODIES {
        return Err(damaged());
    }
    physics_text
        .lines()
        .map(|l| {
            let words: Vec<&str> = words_of(l, BODY_WORDS).collect();
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
        .collect::<Result<Vec<_>, String>>()
}

/// Where the game has each entity it moved: its transform's numbers.
fn read_places(places_text: &str) -> Result<Vec<(EntityId, Transform)>, String> {
    if places_text.lines().count() > area::text::MAX_ENTITIES {
        return Err(damaged());
    }
    places_text
        .lines()
        .map(|l| {
            let (id, rest) = l.split_once(' ').ok_or_else(damaged)?;
            let id: reflect::Id = id.parse().map_err(|_| damaged())?;
            let n: Vec<f32> = words_of(rest, PLACE_WORDS)
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
        .collect::<Result<Vec<_>, String>>()
}

/// Each animator's parameters, by name.
fn read_animators(animators_text: &str) -> Result<Vec<SavedAnimator>, String> {
    if animators_text.lines().count() > area::text::MAX_ENTITIES {
        return Err(damaged());
    }
    animators_text
        .lines()
        .map(|l| {
            // Its ID, then a word for each parameter.
            let mut words = words_of(l, 1 + animation::animator_file::MAX_PARAMETERS);
            let id: reflect::Id = words
                .next()
                .and_then(|w| w.parse().ok())
                .ok_or_else(damaged)?;
            let values = words
                .map(|w| {
                    let (name, value) = w.split_once('=').ok_or_else(damaged)?;
                    let name =
                        animation::animator_file::parameter_name(name).map_err(|_| damaged())?;
                    let value = world::animators::value_of_words(value).ok_or_else(damaged)?;
                    Ok((name, value))
                })
                .collect::<Result<Vec<_>, String>>()?;
            if values.len() > animation::animator_file::MAX_PARAMETERS {
                return Err(damaged());
            }
            Ok((EntityId(id), values))
        })
        .collect::<Result<Vec<_>, String>>()
}

/// One animator's parameters in a save: its entity, and each parameter's
/// name and value.
type SavedAnimator = (EntityId, Vec<(String, animation::ParamValue)>);

/// Each tile map as the game changed it: a tile map file for each
/// entity, `ID length` before it.
fn read_tile_maps(mut maps_text: &str) -> Result<Vec<(EntityId, tiles::TileMap)>, String> {
    let mut tile_maps = Vec::new();
    while !maps_text.is_empty() {
        if tile_maps.len() >= area::text::MAX_ENTITIES {
            return Err(damaged());
        }
        let (head, after) = maps_text.split_once('\n').ok_or_else(damaged)?;
        let (id, length) = head.split_once(' ').ok_or_else(damaged)?;
        let id: reflect::Id = id.parse().map_err(|_| damaged())?;
        let length: usize = length.parse().map_err(|_| damaged())?;
        let text = after.get(..length).ok_or_else(damaged)?;
        maps_text = after
            .get(length..)
            .and_then(|a| a.strip_prefix('\n'))
            .ok_or_else(damaged)?;
        let read = tiles::map_file::read(text).map_err(|_| damaged())?;
        if !read.notes.is_empty() {
            return Err(damaged());
        }
        tile_maps.push((EntityId(id), read.map));
    }
    Ok(tile_maps)
}

/// How fast the game ran: its exact bits, or 1 when empty.
fn read_speed(text: &str) -> Result<f64, String> {
    let speed = match text {
        "" => 1.0,
        bits => f64::from_bits(u64::from_str_radix(bits, 16).map_err(|_| damaged())?),
    };
    if !(speed.is_finite() && (0.0..=world::MAX_SPEED).contains(&speed)) {
        return Err(damaged());
    }
    Ok(speed)
}

/// The hours game code moved the day on by: its exact bits, or 0 when
/// empty (24 is midnight again).
fn read_day_shift(text: &str) -> Result<f64, String> {
    let day_shift = match text {
        "" => 0.0,
        bits => f64::from_bits(u64::from_str_radix(bits, 16).map_err(|_| damaged())?),
    };
    // 24 is midnight again: an earlier build could write it (the
    // phase review).
    if !(day_shift.is_finite() && (0.0..=24.0).contains(&day_shift)) {
        return Err(damaged());
    }
    let day_shift = if day_shift >= 24.0 { 0.0 } else { day_shift };
    Ok(day_shift)
}

/// Version 1 to 2: version 1 kept physics as Rapier's own bytes, which
/// Solis no longer reads (a file must never reach Rapier's insides). The
/// bodies start at rest where the save has them.
pub(super) fn physics_as_places_and_motion(sections: &mut Sections) -> Result<(), String> {
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
pub(super) fn places_kept_apart(sections: &mut Sections) -> Result<(), String> {
    if sections.list.iter().any(|(n, _)| n == "places") {
        return Err("it already has places".into());
    }
    sections.list.push(("places".into(), String::new()));
    Ok(())
}

/// Version 3 to 4: version 3 had no animators (Phase 9, D3b), so its
/// animators start with every parameter at its start.
pub(super) fn animators_kept(sections: &mut Sections) -> Result<(), String> {
    if sections.list.iter().any(|(n, _)| n == "animators") {
        return Err("it already has animators".into());
    }
    sections.list.push(("animators".into(), String::new()));
    Ok(())
}

/// Version 4 to 5: version 4 had no tile maps (Phase 9, D4), so its maps
/// start from their files.
pub(super) fn tile_maps_kept(sections: &mut Sections) -> Result<(), String> {
    if sections.list.iter().any(|(n, _)| n == "tilemaps") {
        return Err("it already has tile maps".into());
    }
    sections.list.push(("tilemaps".into(), String::new()));
    Ok(())
}

/// Version 5 to 6: version 5 had no game-wide objects (decision 0080),
/// so they start as the code writes them.
pub(super) fn objects_kept(sections: &mut Sections) -> Result<(), String> {
    if sections.list.iter().any(|(n, _)| n == "objects") {
        return Err("it already has objects".into());
    }
    sections.list.push(("objects".into(), String::new()));
    Ok(())
}

/// Version 6 to 7: version 6 had no game speed (game UI, M3c), so the
/// game runs as usual.
pub(super) fn speed_kept(sections: &mut Sections) -> Result<(), String> {
    if sections.list.iter().any(|(n, _)| n == "speed") {
        return Err("it already has a speed".into());
    }
    sections.list.push(("speed".into(), String::new()));
    Ok(())
}

/// Version 7 to 8: version 7 had no time of day (decision 0092), so the
/// day is where the Environment starts it.
pub(super) fn day_kept(sections: &mut Sections) -> Result<(), String> {
    if sections.list.iter().any(|(n, _)| n == "day") {
        return Err("it already has a day".into());
    }
    sections.list.push(("day".into(), String::new()));
    Ok(())
}
