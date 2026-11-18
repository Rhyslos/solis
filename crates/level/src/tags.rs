//! The project's tags (stage 1, tags): labels such as Enemy or
//! Collectible that entities carry, so game code can find them ("every
//! enemy within 3 m", "the nearest collectible"). The list is the
//! project's own, made and renamed in Project settings › Tags, and kept in
//! `tags.lst` in the project's folder, a Lust data file
//! ([decision 0081](../../../docs/decisions/0081-what-solis-makes-is-lust.md)),
//! version 1:
//!
//! ```text
//! -- The project's tags (Project settings › Tags). Written by Solis; safe to edit by hand.
//! fetch "solis/data/tags":[Tag];
//!
//! lock int version = 1;
//! lock Tag array tags = [
//!     new Tag { id = "1BWHNR3RV8PYY", name = "Enemy" },
//!     new Tag { id = "0X3CM2V7Q1TWA", name = "Collectible" },
//! ];
//! end
//! ```
//!
//! Each tag has a stable ID, which is what entities keep
//! (`builtin::Tags`): renaming a tag changes only this file, and every
//! level, prefab and save follows. Its name is what game code calls it,
//! `Tag.Enemy`, so it must be one Lust can name: [`TagList::problem`] says
//! why one isn't, in plain words. A tag in the file that can't be used (no
//! ID, a name Lust can't take, a second of one name) is left out with a
//! note; the rest are kept.

use crate::lust_data::{self as data, DataError};
use reflect::TagId;

/// The file's name, in the project's folder.
pub const FILE_NAME: &str = "tags.lst";

/// The word naming its classes' file (`solis/data/tags.lst`).
pub const KIND: &str = "tags";

/// The version this build writes.
pub const VERSION: u32 = 1;

/// The most bytes a tags file may have.
pub const MAX_BYTES: usize = 256 << 10;

/// The most tags a project may have.
pub const MAX_TAGS: usize = 1024;

/// The longest a tag's name may be, in characters.
pub const MAX_NAME_CHARS: usize = 40;

/// What a tags file holds, in Lust: written into every project's
/// `solis/data/tags.lst`, and handed to Lust when the file is read.
pub const CLASSES: &str = "-- What a project's tags file holds (Solis writes this; tags.lst fetches it).\n\
class Tag {\n    has {\n        string id = \"\";\n        string name = \"\";\n    }\n}\n\npass Tag;\nend\n";

/// The tags file's version and upgraders. Version 1 is the first.
pub static FORMAT: reflect::upgrade::Format<TagList> = reflect::upgrade::Format {
    name: "tags",
    current: VERSION,
    version_of: data::version_of,
    steps: &[],
};

const COMMENT: &str =
    "The project's tags (Project settings › Tags). Written by Solis; safe to edit by hand.";

/// Names Lust keeps for its own objects, which no tag may have.
const LUST_OBJECTS: &[&str] = &["Maths", "Files", "Time", "Json", "Web", "Random"];

/// The name a project with no tags gives its one option (Lust's states
/// need one), so no tag may have it.
const NO_TAG: &str = "None";

/// One tag: its stable ID and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    /// What entities keep.
    pub id: TagId,
    /// What it's called, and what game code calls it (`Tag.Enemy`).
    pub name: String,
}

/// The project's tags, in the order they were made.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TagList {
    tags: Vec<Tag>,
}

/// A tags file, read: its tags, and notes on any left out.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Read {
    /// The tags.
    pub tags: TagList,
    /// What was left out, and why, in plain words.
    pub notes: Vec<String>,
}

impl TagList {
    /// Every tag, in order.
    pub fn iter(&self) -> impl Iterator<Item = &Tag> {
        self.tags.iter()
    }

    /// How many there are.
    pub fn len(&self) -> usize {
        self.tags.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    /// The tag with this ID.
    pub fn get(&self, id: TagId) -> Option<&Tag> {
        self.tags.iter().find(|t| t.id == id)
    }

    /// The name of the tag with this ID.
    pub fn name_of(&self, id: TagId) -> Option<&str> {
        self.get(id).map(|t| t.name.as_str())
    }

    /// The tag called exactly `name` (as game code names it).
    pub fn id_of(&self, name: &str) -> Option<TagId> {
        self.tags.iter().find(|t| t.name == name).map(|t| t.id)
    }

    /// Every tag's name, in order.
    pub fn names(&self) -> Vec<String> {
        self.tags.iter().map(|t| t.name.clone()).collect()
    }

    /// Why `name` can't be a tag's (the tag `renaming` keeps its own name),
    /// in plain words; `None` when it can.
    pub fn problem(&self, name: &str, renaming: Option<TagId>) -> Option<String> {
        if let Some(why) = name_problem(name) {
            return Some(why);
        }
        let same = self
            .tags
            .iter()
            .find(|t| Some(t.id) != renaming && t.name.eq_ignore_ascii_case(name))?;
        Some(if same.name == name {
            format!("There's already a tag called {name}.")
        } else {
            format!(
                "There's already a tag called {}: game code can't tell names apart by capitals alone.",
                same.name
            )
        })
    }

    /// Makes a tag called `name`, at the end, and gives back its ID; or
    /// why it can't be made.
    pub fn add(&mut self, name: &str) -> Result<TagId, String> {
        let id = loop {
            let id = TagId::random();
            if self.get(id).is_none() {
                break id;
            }
        };
        self.add_with_id(id, name)?;
        Ok(id)
    }

    /// Makes a tag with this ID, at the end; or why it can't be made.
    pub fn add_with_id(&mut self, id: TagId, name: &str) -> Result<(), String> {
        if self.tags.len() >= MAX_TAGS {
            return Err(format!("A project can have at most {MAX_TAGS} tags."));
        }
        if self.get(id).is_some() {
            return Err("Another tag has that ID.".into());
        }
        if let Some(why) = self.problem(name, None) {
            return Err(why);
        }
        self.tags.push(Tag {
            id,
            name: name.to_owned(),
        });
        Ok(())
    }

    /// Renames the tag `id` to `name`; or why it can't be.
    pub fn rename(&mut self, id: TagId, name: &str) -> Result<(), String> {
        if self.get(id).is_none() {
            return Err("That tag is gone from the project.".into());
        }
        if let Some(why) = self.problem(name, Some(id)) {
            return Err(why);
        }
        if let Some(tag) = self.tags.iter_mut().find(|t| t.id == id) {
            name.clone_into(&mut tag.name);
        }
        Ok(())
    }

    /// Takes the tag `id` out of the list; whether it was there. Entities
    /// keeping its ID then carry nothing by it.
    pub fn remove(&mut self, id: TagId) -> bool {
        let before = self.tags.len();
        self.tags.retain(|t| t.id != id);
        self.tags.len() != before
    }
}

/// Why `name` can't name a tag by itself (without the others), in plain
/// words: game code names it `Tag.<name>`, so it must be a Lust option.
pub fn name_problem(name: &str) -> Option<String> {
    let Some(first) = name.chars().next() else {
        return Some("A tag needs a name.".into());
    };
    if name.chars().count() > MAX_NAME_CHARS {
        return Some(format!(
            "A tag's name can be at most {MAX_NAME_CHARS} characters."
        ));
    }
    if !first.is_ascii_uppercase() {
        return Some(format!(
            "A tag's name starts with a capital letter, as Enemy does: game code calls it \
             Tag.Enemy. Try {}.",
            suggestion(name)
        ));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Some(format!(
            "A tag's name holds only letters, digits and _, with no spaces: game code calls it \
             Tag.<name>. Try {}.",
            suggestion(name)
        ));
    }
    let letters: Vec<char> = name.chars().filter(char::is_ascii_alphabetic).collect();
    if letters.len() >= 2 && letters.iter().all(char::is_ascii_uppercase) {
        return Some(format!(
            "A tag's name can't be all capitals, which Lust keeps for its own: try {}.",
            suggestion(name)
        ));
    }
    if LUST_OBJECTS.contains(&name) {
        return Some(format!(
            "{name} is one of Lust's own names: choose another."
        ));
    }
    if name == NO_TAG {
        return Some(
            "None is what game code has when a project has no tags: choose another.".into(),
        );
    }
    None
}

/// A name like `name` that a tag can have, to suggest: its words joined,
/// each starting with a capital (`big bat` is `BigBat`).
fn suggestion(name: &str) -> String {
    let words = name
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter(|w| !w.is_empty());
    let mut out = String::new();
    for word in words {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            let rest: String = chars.collect();
            if word
                .chars()
                .filter(char::is_ascii_alphabetic)
                .all(|c| c.is_ascii_uppercase())
            {
                out.push_str(&rest.to_ascii_lowercase());
            } else {
                out.push_str(&rest);
            }
        }
    }
    if out.is_empty() || !out.starts_with(|c: char| c.is_ascii_uppercase()) {
        out.insert_str(0, "Tag");
    }
    out.chars().take(MAX_NAME_CHARS).collect()
}

/// `tags` as a tags file that [`read`] reads back.
pub fn write(tags: &TagList) -> String {
    let mut out = data::header(COMMENT, KIND, &["Tag"], VERSION);
    let items: Vec<String> = tags
        .iter()
        .map(|t| {
            data::thing(
                "Tag",
                &[
                    ("id", data::text(&t.id.to_string())),
                    ("name", data::text(&t.name)),
                ],
            )
        })
        .collect();
    data::list(&mut out, "Tag", "tags", &items);
    out.push_str("end\n");
    out
}

/// Reads a tags file. Refused, with its line, only when it can't be read
/// at all or is from a newer version; a tag that can't be used is left
/// out with a note.
pub fn read(text: &str) -> Result<Read, DataError> {
    let version = data::version_of(text).unwrap_or(0);
    if version > VERSION {
        return Err(data::error(
            0,
            format!(
                "it was saved by a newer version (tags {version}; this one reads up to \
                 {VERSION}). Update to open it."
            ),
        ));
    }
    let values = data::read(text, KIND, CLASSES, MAX_BYTES)?;
    let mut read = Read::default();
    let things = data::things(&values, "tags");
    for (n, thing) in things.iter().enumerate() {
        let name = data::field_text(thing, "name");
        let shown: String = name.chars().take(MAX_NAME_CHARS).collect();
        if read.tags.len() >= MAX_TAGS {
            read.notes.push(format!(
                "Only the first {MAX_TAGS} tags are kept; {} more are left out.",
                things.len() - n
            ));
            break;
        }
        let Ok(id) = data::field_text(thing, "id").parse::<TagId>() else {
            read.notes.push(format!(
                "The tag \"{shown}\" is left out: it has no ID, which entities keep it by."
            ));
            continue;
        };
        if let Err(why) = read.tags.add_with_id(id, name) {
            read.notes
                .push(format!("The tag \"{shown}\" is left out: {why}"));
        }
    }
    Ok(read)
}

#[cfg(test)]
#[path = "tags_tests.rs"]
mod tests;
