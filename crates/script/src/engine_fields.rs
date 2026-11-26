//! Values the engine reads from a component's `has` block
//! ([decision 0136](../../../docs/decisions/0136-engine-calls-make-or-set.md),
//! [0137](../../../docs/decisions/0137-the-uniformity-pass.md)): game
//! code changes what a thing is by setting a field with `=`, not by
//! calling a method. A Lust component that holds `Facing facing` turns its
//! entity's sprite (`facing = Facing.NorthEast;`), one that holds `Look
//! look` dresses it (`look = Look.KnightMana;`; Fixes 3), and the
//! uniformity pass's U5 adds the rest a component sets on its own entity:
//! `Clip animation`, `Flip flip`, `bool visible`, `bool solid`, `bool
//! soundPlaying` and `Screen screen`.
//!
//! Which components hold them is found once, from how their Lust is
//! written (the type before the field's name), when the code is checked.
//! As each tick ends every core reads those fields of its running
//! components and gives back the ones that changed since it last looked,
//! a component started since counting as a change: the entity then
//! follows the component from its first tick. Reading a few fields of the
//! few components that hold them costs far less than a call each time.

use crate::game::Change;
use crate::project_names::{NO_OPTION, NamePaths, path_in};
use crate::ticking::{Instance, RunningMap};
use mlua::Value as LuaValue;
use reflect::EntityId;
use std::collections::HashMap;

/// What the engine does with a field it reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Read {
    /// `Facing facing`: which way the entity's sprite faces.
    Facing,
    /// `Look look`: which of its clips file's looks the sprite wears.
    Look,
    /// `Clip animation`: the clip the sprite plays (`Clip.None` stops it
    /// where it is).
    Animation,
    /// `Flip flip`: the sprite mirrored across, down, both or neither.
    Flip,
    /// `bool visible`: the entity and everything under it are drawn.
    Visible,
    /// `bool solid`: the entity's collider blocks and is hit.
    Solid,
    /// `bool soundPlaying`: the entity's `AudioSource` plays.
    SoundPlaying,
    /// `Screen screen`: the screen this component shows on the entity's
    /// Screen (`Screen.None` shows none).
    Screen,
}

impl Read {
    /// Every field the engine reads.
    pub const ALL: [Self; 8] = [
        Self::Facing,
        Self::Look,
        Self::Animation,
        Self::Flip,
        Self::Visible,
        Self::Solid,
        Self::SoundPlaying,
        Self::Screen,
    ];

    /// The type written before the field's name.
    pub fn kind(self) -> &'static str {
        match self {
            Self::Facing => "Facing",
            Self::Look => "Look",
            Self::Animation => "Clip",
            Self::Flip => "Flip",
            Self::Visible | Self::Solid | Self::SoundPlaying => "bool",
            Self::Screen => "Screen",
        }
    }

    /// The field's name.
    pub fn field(self) -> &'static str {
        match self {
            Self::Facing => "facing",
            Self::Look => "look",
            Self::Animation => "animation",
            Self::Flip => "flip",
            Self::Visible => "visible",
            Self::Solid => "solid",
            Self::SoundPlaying => "soundPlaying",
            Self::Screen => "screen",
        }
    }

    /// The changes that make the entity what the field now says, `was`
    /// being what it said last time (a screen shown before is hidden).
    /// A clip is named by its option (`FarmerWalk`), found in `paths`.
    fn changes(
        self,
        entity: EntityId,
        now: &str,
        was: Option<&str>,
        paths: &NamePaths,
    ) -> Vec<Change> {
        let on = || now == "true";
        match self {
            Self::Facing if now != NO_OPTION => vec![Change::Facing(entity, now.to_owned())],
            Self::Look if now != NO_OPTION => vec![Change::Look(entity, now.to_owned())],
            Self::Facing | Self::Look => Vec::new(),
            Self::Animation if now == NO_OPTION => vec![Change::StopClip(entity)],
            Self::Animation => path_in(paths, "Clip", now)
                .and_then(|path| path.rsplit('/').next().map(str::to_owned))
                .filter(|clip| !clip.is_empty())
                .map(|clip| Change::PlayClip(entity, clip, false))
                .into_iter()
                .collect(),
            Self::Flip => {
                let (across, down) = match now {
                    "Across" => (true, false),
                    "Down" => (false, true),
                    "Both" => (true, true),
                    _ => (false, false),
                };
                vec![Change::Flip(entity, across, down)]
            }
            Self::Visible => vec![Change::Visible(entity, on())],
            Self::Solid => vec![Change::Solid(entity, on())],
            Self::SoundPlaying => vec![Change::SoundPlaying(entity, on())],
            Self::Screen => {
                let mut changes = Vec::new();
                if let Some(was) = was.filter(|w| *w != NO_OPTION) {
                    changes.push(Change::ShowScreen(entity, was.to_owned(), false));
                }
                if now != NO_OPTION {
                    changes.push(Change::ShowScreen(entity, now.to_owned(), true));
                }
                changes
            }
        }
    }
}

/// A component holding a field the engine reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Watched {
    /// The component: `Knight`.
    pub component: String,
    /// What the engine does with it.
    pub read: Read,
}

/// The most fields read from one file: far more than any component has.
const MOST_FIELDS: usize = 10_000;

/// The Lust components in `files` (path and text) holding fields the
/// engine reads, each field once, in the order written.
pub fn found(files: &[(String, String)]) -> Vec<Watched> {
    let mut watched: Vec<Watched> = Vec::new();
    for (_, text) in files {
        for field in level::lust_fields::fields(text, MOST_FIELDS) {
            let Some(read) = Read::ALL
                .into_iter()
                .find(|r| r.kind() == field.kind && r.field() == field.name)
            else {
                continue;
            };
            let one = Watched {
                component: field.component.to_string(),
                read,
            };
            if !watched.contains(&one) {
                watched.push(one);
            }
        }
    }
    watched
}

/// What a core last read of each running component's fields.
pub(crate) type Seen = HashMap<(EntityId, Read), (String, String)>;

/// A field's value as the engine keeps it: a state's option, or `true`
/// or `false`; `None` for anything else (nothing yet, a wrong type).
fn read_value(value: LuaValue) -> Option<String> {
    match value {
        LuaValue::String(text) => text
            .to_str()
            .ok()
            .map(|t| t.to_string())
            .filter(|t| !t.is_empty()),
        LuaValue::Boolean(on) => Some(on.to_string()),
        _ => None,
    }
}

/// Reads the `watched` fields of the `running` components and gives back
/// the changes for each that isn't what was `seen` last (by entity and
/// field, with the component that holds it), keeping what was read for
/// next time. A component gone, or stopped holding one, is forgotten,
/// and the screen it showed is hidden. Clips are found in `paths`.
pub(crate) fn changes(
    watched: &[Watched],
    running: &RunningMap,
    seen: &mut Seen,
    paths: &NamePaths,
) -> Vec<Change> {
    if watched.is_empty() {
        seen.clear();
        return Vec::new();
    }
    let mut now: Seen = HashMap::with_capacity(seen.len());
    let mut changes = Vec::new();
    for ((entity, name), component) in running {
        let Instance::Lust(table) = &component.instance else {
            continue;
        };
        for one in watched.iter().filter(|w| w.component == *name) {
            let Some(value) = table
                .raw_get::<LuaValue>(one.read.field())
                .ok()
                .and_then(read_value)
            else {
                continue;
            };
            let key = (*entity, one.read);
            let read = (name.clone(), value);
            let before = seen.get(&key);
            if before != Some(&read) {
                let was = before
                    .filter(|(c, _)| *c == read.0)
                    .map(|(_, v)| v.as_str());
                changes.extend(one.read.changes(*entity, &read.1, was, paths));
            }
            now.insert(key, read);
        }
    }
    for ((entity, read), (_, value)) in seen.iter() {
        if *read == Read::Screen && value != NO_OPTION && !now.contains_key(&(*entity, *read)) {
            changes.push(Change::ShowScreen(*entity, value.clone(), false));
        }
    }
    *seen = now;
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components_holding_facing_or_look_are_found_by_how_they_re_written() {
        let text = "fetch \"solis/animation\":Facing;\n\
                    fetch \"solis/project\":Look;\n\
                    component Knight {\n    has {\n        Facing facing = Facing.South;\n        \
                    Look look = Look.KnightMain;\n        Facing lastSeen = Facing.North;\n    }\n}\n\
                    component Orc {\n    has {\n        string facing = \"\";\n    }\n}\n\
                    -- component Ghost { has { Facing facing = Facing.North; } }\n\
                    end\n";
        let found = found(&[("main.lst".into(), text.into())]);
        assert_eq!(
            found,
            [
                Watched {
                    component: "Knight".into(),
                    read: Read::Facing
                },
                Watched {
                    component: "Knight".into(),
                    read: Read::Look
                },
            ],
            "a Facing of another name, a facing of another type, and one in a comment are left"
        );
        assert!(super::found(&[]).is_empty());
        assert!(super::found(&[("x.lst".into(), "component { has { Facing".into())]).is_empty());
    }
}
