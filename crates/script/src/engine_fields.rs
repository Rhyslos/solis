//! Values the engine reads from a component's `has` block
//! ([decision 0136](../../../docs/decisions/0136-engine-calls-make-or-set.md)):
//! game code changes what a thing is by setting a field with `=`, not by
//! calling a method. A Lust component that holds `Facing facing` turns its
//! entity's sprite (`facing = Facing.NorthEast;`), and one that holds
//! `Look look` dresses it (`look = Look.KnightMana;`; Fixes 3).
//!
//! Which components hold them is found once, from how their Lust is
//! written (the type before the field's name), when the code is checked.
//! As each tick ends every core reads those fields of its running
//! components and gives back the ones that changed since it last looked,
//! a component started since counting as a change: the sprite then
//! follows the component from its first tick. Reading two fields of the
//! few components that hold them costs far less than a call each time.

use crate::game::Change;
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
    /// `Clip animation`: the clip its sprite plays (U5; was `animate`).
    Animation,
    /// `bool flipped`: its sprite mirrored across (was `setFlip`).
    Flipped,
    /// `bool flippedDown`: its sprite upside down.
    FlippedDown,
    /// `bool visible`: shown, or hidden with everything under it (was
    /// `setVisible`).
    Visible,
    /// `bool solid`: its collider in the way (was `setSolid`).
    Solid,
    /// `bool sounding`: its AudioSource on (was `setSoundPlaying`).
    Sounding,
    /// `Screen screen`: the screen its Screen shows, `Screen.None` none
    /// (was `showScreen` and `hideScreen`).
    Screen,
}

impl Read {
    /// Every field the engine reads.
    pub const ALL: [Self; 9] = [
        Self::Facing,
        Self::Look,
        Self::Animation,
        Self::Flipped,
        Self::FlippedDown,
        Self::Visible,
        Self::Solid,
        Self::Sounding,
        Self::Screen,
    ];

    /// The type written before the field's name.
    pub fn kind(self) -> &'static str {
        match self {
            Self::Facing => "Facing",
            Self::Look => "Look",
            Self::Animation => "Clip",
            Self::Screen => "Screen",
            Self::Flipped | Self::FlippedDown | Self::Visible | Self::Solid | Self::Sounding => {
                "bool"
            }
        }
    }

    /// The field's name.
    pub fn field(self) -> &'static str {
        match self {
            Self::Facing => "facing",
            Self::Look => "look",
            Self::Animation => "animation",
            Self::Flipped => "flipped",
            Self::FlippedDown => "flippedDown",
            Self::Visible => "visible",
            Self::Solid => "solid",
            Self::Sounding => "sounding",
            Self::Screen => "screen",
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

/// What a core last read of each running component's fields: the value,
/// as text (`NorthEast`, `true`), with the component that holds it.
pub(crate) type Seen = HashMap<(EntityId, Read), (String, String)>;

/// A field's value as text: a state's option or a yes or no.
fn text_of(value: &LuaValue) -> Option<String> {
    match value {
        LuaValue::String(s) => s.to_str().ok().map(|s| s.to_string()),
        LuaValue::Boolean(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Reads the `watched` fields of the `running` components and gives back
/// a change for each that isn't what was `seen` last (by entity and field,
/// with the component that holds it), keeping what was read for next
/// time. A component gone, or stopped holding one, is forgotten. `lua`
/// finds a clip's file (its names, as the code was compiled with them).
pub(crate) fn changes(
    watched: &[Watched],
    running: &RunningMap,
    seen: &mut Seen,
    lua: &mlua::Lua,
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
        let mut flip: Option<(bool, bool)> = None;
        for one in watched.iter().filter(|w| w.component == *name) {
            let Some(value) = table
                .raw_get::<LuaValue>(one.read.field())
                .ok()
                .as_ref()
                .and_then(text_of)
            else {
                continue;
            };
            let key = (*entity, one.read);
            let before = seen.get(&key).map(|(_, v)| v.clone());
            let read = (name.clone(), value.clone());
            let changed = seen.get(&key) != Some(&read);
            now.insert(key, read);
            if !changed {
                continue;
            }
            let on = value == "true";
            match one.read {
                Read::Facing | Read::Look
                    if value.is_empty() || value == crate::project_names::NO_OPTION => {}
                Read::Facing => changes.push(Change::Facing(*entity, value)),
                Read::Look => changes.push(Change::Look(*entity, value)),
                Read::Animation => {
                    let clip = crate::project_names::path_for(lua, "Clip", &value)
                        .and_then(|p| p.rsplit('/').next().map(str::to_owned));
                    if let Some(clip) = clip {
                        changes.push(Change::PlayClip(*entity, clip, false));
                    }
                }
                Read::Flipped | Read::FlippedDown => flip = Some((false, false)),
                Read::Visible => changes.push(Change::Visible(*entity, on)),
                Read::Solid => changes.push(Change::Solid(*entity, on)),
                Read::Sounding => changes.push(Change::SoundPlaying(*entity, on)),
                Read::Screen if value == crate::project_names::NO_OPTION => {
                    if let Some(shown) = before.filter(|b| b != crate::project_names::NO_OPTION) {
                        changes.push(Change::ShowScreen(*entity, shown, false));
                    }
                }
                Read::Screen => changes.push(Change::ShowScreen(*entity, value, true)),
            }
        }
        // The two flips land together, as `setFlip` set them.
        if flip.is_some() {
            let is_on = |read: Read| now.get(&(*entity, read)).is_some_and(|(_, v)| v == "true");
            changes.push(Change::Flip(
                *entity,
                is_on(Read::Flipped),
                is_on(Read::FlippedDown),
            ));
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
