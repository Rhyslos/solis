//! Trigger zones in a save game (stage 1, triggers; version 10): who is
//! inside each zone, and which zones that fire once have fired. Kept so a
//! loaded game never says "entered" again for someone already inside, and
//! a once-only zone stays quiet.
//!
//! The `zones` section is a line a pair, then a line a spent zone:
//!
//! ```text
//! in ZONE WHO
//! once ZONE
//! ```

use super::{Sections, words_of};
use reflect::EntityId;

/// The most lines a save's zones section is read with: past it, the save
/// is damaged (a million entities in a few zones each).
const MAX_ZONE_LINES: usize = 1 << 22;

/// Who is inside each zone and which fired once, as a save writes them.
pub(super) fn zones_text(inside: &[(EntityId, EntityId)], spent: &[EntityId]) -> String {
    let mut text = String::new();
    for (zone, who) in inside {
        text.push_str(&format!("in {zone} {who}\n"));
    }
    for zone in spent {
        text.push_str(&format!("once {zone}\n"));
    }
    text
}

/// Pairs inside zones, and zones that fired once.
pub(super) type ZoneStates = (Vec<(EntityId, EntityId)>, Vec<EntityId>);

/// A save's zones section read back, or why it can't be.
pub(super) fn read_zones(text: &str) -> Result<ZoneStates, String> {
    if text.lines().count() > MAX_ZONE_LINES {
        return Err("it's damaged".into());
    }
    let mut inside = Vec::new();
    let mut spent = Vec::new();
    let id = |word: Option<&str>| -> Result<EntityId, String> {
        word.and_then(|w| w.parse::<reflect::Id>().ok())
            .map(EntityId)
            .ok_or_else(|| "it's damaged".to_string())
    };
    for line in text.lines() {
        let mut words = words_of(line, 3);
        match words.next() {
            Some("in") => {
                let pair = (id(words.next())?, id(words.next())?);
                if words.next().is_some() {
                    return Err("it's damaged".into());
                }
                inside.push(pair);
            }
            Some("once") => {
                spent.push(id(words.next())?);
                if words.next().is_some() {
                    return Err("it's damaged".into());
                }
            }
            _ => return Err("it's damaged".into()),
        }
    }
    Ok((inside, spent))
}

/// Version 9 to 10: version 9 had no trigger zones (stage 1), so nothing
/// is inside one and none has fired.
pub(super) fn zones_kept(sections: &mut Sections) -> Result<(), String> {
    if sections.list.iter().any(|(n, _)| n == "zones") {
        return Err("it already has zones".into());
    }
    sections.list.push(("zones".into(), String::new()));
    Ok(())
}
