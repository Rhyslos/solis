//! The game's state written out part by part (stage 1, the same game on
//! two PCs): everything the world's checksum covers, as readable text
//! under a name for each part. Two runs whose checksums differ at a tick
//! are compared part by part, so the difference is named: which entity,
//! and which of its components or which part of the game.
//!
//! Numbers are written as Rust writes them, the shortest text that reads
//! back as exactly the same number, so two parts are the same text only
//! when they hold the same bits.

use crate::{Global, Id, Name, Parent, Values, World};
use level::builtin::AudioSource;
use reflect::{ComponentData, EntityId, Value};
use std::collections::BTreeMap;

/// The game's state, part by part: each part's name and what it holds.
pub type StateParts = BTreeMap<String, String>;

impl World {
    /// Every part of the game's state the world's checksum covers (game
    /// code's own values are added by whoever runs it).
    pub fn state_parts(&self) -> StateParts {
        let mut parts = StateParts::new();
        let mut game = |part: &str, value: String| parts.insert(format!("game {part}"), value);
        game("seed", self.seed.to_string());
        game("speed", self.speed.to_string());
        game("day moved on by", self.day_shift.to_string());
        game("zones fired once", ids(&self.zone_states().1));
        // What game code switched off (Fixes 2c), when it has.
        for (name, list) in [
            ("hidden", &self.switches.hidden),
            ("colliders off", &self.switches.unsolid),
        ] {
            if !list.is_empty() {
                game(name, ids(&list.iter().copied().collect::<Vec<_>>()));
            }
        }
        let flat = self.flat.state();
        game("2D walls", flat.wall_count().to_string());
        let flying: Vec<String> = self
            .projectiles
            .flying()
            .iter()
            .map(|p| {
                format!(
                    "{:?} {:?} {} {:?}",
                    p.position, p.velocity, p.left, p.colour
                )
            })
            .collect();
        game("projectiles", flying.join("; "));
        self.entity_parts(&mut parts);
        parts
    }

    /// Each entity's parts: what it is in the tree, its place, the values
    /// the level gave it, and what its built-in components hold now.
    fn entity_parts(&self, parts: &mut StateParts) {
        let mut names: BTreeMap<EntityId, String> = BTreeMap::new();
        for &(entity, parent) in &self.order {
            let Ok(e) = self.ecs.entity(entity) else {
                continue;
            };
            let Some(id) = e.get::<&Id>().map(|id| id.0) else {
                continue;
            };
            let name = e.get::<&Name>().map(|n| n.0.clone()).unwrap_or_default();
            let key = format!("{id} {name:?}");
            let up = e.get::<&Parent>().map(|p| p.0.to_string());
            parts.insert(
                format!("{key} entity"),
                format!(
                    "under {} (number {})",
                    up.unwrap_or_else(|| "the level".into()),
                    parent.map_or(-1, |p| p as i64)
                ),
            );
            if let Some(values) = e.get::<&Values>() {
                for (component, data) in &values.0 {
                    parts.insert(format!("{key} {component} (as placed)"), data_text(data));
                }
            }
            if let Some(global) = e.get::<&Global>() {
                parts.insert(
                    format!("{key} Transform (in the world)"),
                    matrix_text(global.0),
                );
            }
            names.insert(id, key);
        }
        let mut part = |id: EntityId, what: &str, value: String| {
            let key = names.get(&id).cloned().unwrap_or_else(|| id.to_string());
            parts.insert(format!("{key} {what}"), value);
        };
        for (id, source) in self.ecs.query::<(&Id, &AudioSource)>().iter() {
            part(id.0, "AudioSource playing", source.playing.to_string());
        }
        for (id, clip, playing, time) in self.sprite_states() {
            let time = f32::from_bits(time);
            part(
                id,
                "Sprite",
                format!("clip {clip:?}, playing {playing}, at {time}"),
            );
        }
        for (id, state, values) in self.animator_states() {
            part(id, "Animator", format!("{state:?} {}", values.join(" ")));
        }
        for (id, sum) in self.map_sums() {
            part(id, "TileMap cells (checksum)", format!("{sum:016x}"));
        }
        for (id, tags) in self.all_tags() {
            part(id, "Tags", tags.text());
        }
        for (zone, who) in self.zone_states().0 {
            part(zone, &format!("TriggerZone has {who} inside"), "yes".into());
        }
        for (id, body) in self.physics.state().body_states() {
            part(id, "3D physics body", format!("{body:?}"));
        }
        let flat = self.flat.state();
        for id in names.keys().copied().collect::<Vec<_>>() {
            if let Some(place) = flat.place(id) {
                let kind = if flat.is_character(id) {
                    "character"
                } else {
                    "solid"
                };
                part(id, "2D physics", format!("{kind} at {place:?}"));
            }
        }
    }
}

/// A component's values as one line: `name = value` for each field, in
/// name order.
pub fn data_text(data: &ComponentData) -> String {
    data.fields
        .iter()
        .map(|(name, value)| format!("{name} = {}", value_text(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A value as text, on one line (text is quoted, its line breaks written
/// as `\n`).
pub fn value_text(value: &Value) -> String {
    match value {
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Text(t) => format!("{t:?}"),
        Value::List(items) => {
            let items: Vec<String> = items.iter().map(value_text).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Asset(None) | Value::Entity(None) => "none".into(),
        Value::Asset(Some(a)) => match a.part {
            Some(part) => format!("asset {}#{part}", a.asset),
            None => format!("asset {}", a.asset),
        },
        Value::Entity(Some(e)) => format!("entity {e}"),
    }
}

/// A place in the world: the matrix's top three rows (the bottom is
/// always 0 0 0 1).
fn matrix_text(m: glam::Mat4) -> String {
    [m.row(0), m.row(1), m.row(2)]
        .iter()
        .map(|r| format!("{} {} {} {}", r.x, r.y, r.z, r.w))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Entity IDs, space between.
fn ids(ids: &[EntityId]) -> String {
    ids.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
#[path = "state_parts_tests.rs"]
mod tests;
