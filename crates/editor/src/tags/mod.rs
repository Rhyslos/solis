//! Tags (stage 1, tags; the approved mock-up `docs/design/tags/`): the
//! project's own list, made, renamed and deleted in Project settings ›
//! Tags (`page.rs`) and kept in its `tags.lst`, which the app reads and
//! writes; an entity's tags as chips under its name in Properties
//! (`chips.rs`), each change one undo step; and what the running game
//! asks of them, answered from a view made once a tick ([`tag_view`]).
//!
//! An entity keeps its tags by their IDs (`level::builtin::Tags`), so a
//! rename changes only the list: every level, prefab and save follows,
//! and game code's generated name (`Tag.Enemy`) changes with it. A tag
//! that's gone from the list (deleted, or lost in a merge) is carried by
//! nothing: it isn't shown, the game doesn't see it, and the next change
//! to that entity's tags leaves it out.

pub(crate) mod chips;
pub(crate) mod page;

use crate::console::LogLevel;
use crate::workbench::{Workbench, WorkbenchAction};
use level::Edit;
use level::builtin::Tags;
use level::tags::TagList;
use reflect::{Component, EntityId, TagId};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Where the project's saved levels use each tag, as the app counted them
/// (each level expanded, so copies of prefabs count): for each tag, each
/// level's path inside the project and how many of its entities carry it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TagUse {
    /// Each tag's levels and counts.
    pub levels: BTreeMap<TagId, Vec<(String, usize)>>,
}

/// What uses one tag, for its row and for Delete's question.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct UsedBy {
    /// Entities in the project's levels carrying it.
    pub(crate) entities: usize,
    /// The levels they're in, by name.
    pub(crate) levels: Vec<String>,
    /// The prefabs carrying it, by name.
    pub(crate) prefabs: Vec<String>,
}

impl UsedBy {
    /// Whether nothing uses it.
    pub(crate) fn is_empty(&self) -> bool {
        self.entities == 0 && self.prefabs.is_empty()
    }

    /// "Used by 14 entities and 2 prefabs", or "Not used yet".
    pub(crate) fn words(&self) -> String {
        let entities = match self.entities {
            1 => "1 entity".to_owned(),
            n => format!("{n} entities"),
        };
        let prefabs = match self.prefabs.len() {
            1 => "1 prefab".to_owned(),
            n => format!("{n} prefabs"),
        };
        match (self.entities, self.prefabs.len()) {
            (0, 0) => "Not used yet".into(),
            (_, 0) => format!("Used by {entities}"),
            (0, _) => format!("Used by {prefabs}"),
            _ => format!("Used by {entities} and {prefabs}"),
        }
    }
}

/// What a change to an entity's tags asks for (Properties' chips, and a
/// zone's Only tags).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TagAsk {
    /// Give it this tag.
    Add(TagId),
    /// Take this tag off.
    Remove(TagId),
    /// Make a tag of this name in the project, and give it.
    New(String),
    /// A prefab's copy: back to the prefab's tags (↶).
    Revert,
}

/// The view of the tags game code asks about for one tick: the project's
/// tags, and where the world's tagged entities are now. A 2D project
/// measures across the ground.
pub(crate) fn tag_view(
    tags: &TagList,
    kind: crate::launcher::ProjectKind,
    world: &world::World,
) -> Arc<script::TagView> {
    let names: Vec<(TagId, String)> = tags.iter().map(|t| (t.id, t.name.clone())).collect();
    let flat = kind == crate::launcher::ProjectKind::TwoD;
    Arc::new(script::TagView::new(&names, &world.tagged(), flat))
}

/// The tags `entity` carries that the project still has, in order.
pub(crate) fn carried(tags: &TagList, entity: &level::EntityData) -> Vec<TagId> {
    entity
        .get::<Tags>()
        .map(|t| {
            t.ids
                .into_iter()
                .filter(|id| tags.get(*id).is_some())
                .collect()
        })
        .unwrap_or_default()
}

impl Workbench {
    /// Uses the project's tags (as read from its `tags.lst`, or none).
    pub fn set_tags(&mut self, tags: TagList) {
        if tags != self.open.tags {
            self.open.tags = tags;
            self.open.tags_revision += 1;
        }
    }

    /// The project's tags.
    pub fn tags(&self) -> &TagList {
        &self.open.tags
    }

    /// The app's count of where the project's saved levels use each tag.
    pub fn set_tag_use(&mut self, counted: TagUse) {
        self.open.tag_use = Some(counted);
    }

    /// The project's tags changed here: kept, and saved (which writes
    /// game code's names again).
    fn tags_changed(&mut self, tags: TagList) {
        self.set_tags(tags);
        if self.open.project.is_some() {
            self.open
                .outbox
                .push(WorkbenchAction::SaveTags(self.open.tags.clone()));
        } else {
            self.log(
                LogLevel::Info,
                "Tags are kept with a project: open or make one to keep them.",
            );
        }
    }

    /// Makes a tag called `name`; its ID, or why it can't be made.
    pub(crate) fn make_tag(&mut self, name: &str) -> Result<TagId, String> {
        let mut tags = self.open.tags.clone();
        let id = tags.add(name)?;
        self.tags_changed(tags);
        Ok(id)
    }

    /// Renames the tag `id` everywhere: the list alone changes, since
    /// levels and prefabs keep tags by ID, and game code's name follows.
    pub(crate) fn rename_tag(&mut self, id: TagId, name: &str) -> Result<(), String> {
        let old = self.open.tags.name_of(id).unwrap_or_default().to_owned();
        if old == name {
            return Ok(());
        }
        let mut tags = self.open.tags.clone();
        tags.rename(id, name)?;
        self.tags_changed(tags);
        self.log(
            LogLevel::Info,
            format!(
                "Renamed the tag {old} to {name}: every entity and prefab carrying it follows. \
                 Game code says Tag.{name} now; a line still saying Tag.{old} is a mistake in \
                 Problems until it's changed."
            ),
        );
        Ok(())
    }

    /// Takes the tag `id` out of the project: nothing carries it after.
    pub(crate) fn delete_tag(&mut self, id: TagId) {
        let Some(name) = self.open.tags.name_of(id).map(str::to_owned) else {
            return;
        };
        let mut tags = self.open.tags.clone();
        tags.remove(id);
        self.tags_changed(tags);
        self.log(
            LogLevel::Info,
            format!("Deleted the tag {name}: nothing carries it now."),
        );
    }

    /// What uses the tag `id`: the open level as it is now, the project's
    /// other levels as the app counted them saved, and its prefabs.
    pub(crate) fn used_by(&self, id: TagId) -> UsedBy {
        let here = self
            .open
            .document
            .level()
            .iter()
            .filter(|(_, e)| carried(&self.open.tags, e).contains(&id))
            .count();
        let open_file = self.open_level_inside();
        let mut used = UsedBy {
            entities: here,
            ..UsedBy::default()
        };
        if here > 0 {
            used.levels.push(self.open.document.name());
        }
        let counted = self.open.tag_use.as_ref().and_then(|u| u.levels.get(&id));
        for (level, n) in counted.into_iter().flatten() {
            if open_file.as_deref() == Some(level.as_str()) || *n == 0 {
                continue;
            }
            used.entities += n;
            used.levels.push(reflect::data_files::path_stem(level));
        }
        for (asset, file) in &self.open.prefabs {
            if file
                .level
                .iter()
                .any(|(_, e)| e.get::<Tags>().is_some_and(|t| t.has(id)))
            {
                used.prefabs.push(self.asset_name(*asset));
            }
        }
        used.prefabs.sort();
        used
    }

    /// The open level's file inside the project's assets (`/` between
    /// folders), if it has one.
    fn open_level_inside(&self) -> Option<String> {
        let file = self.open.document.file()?;
        let project = self.open.project.as_ref()?;
        let inside = file.strip_prefix(&project.assets).ok()?;
        Some(inside.to_string_lossy().replace('\\', "/"))
    }

    /// Changes the tags of `entities` (Properties' chips), as one undo
    /// step: each keeps the tags the project still has, changed by `ask`.
    pub(crate) fn change_tags(&mut self, entities: &[EntityId], ask: TagAsk) {
        let (id, label) = match ask {
            TagAsk::Revert => {
                for &entity in entities {
                    self.revert_tags(entity);
                }
                return;
            }
            TagAsk::New(name) => match self.make_tag(name.trim()) {
                Ok(id) => (id, format!("Add tag {}", name.trim())),
                Err(why) => {
                    self.log(LogLevel::Warning, format!("Couldn't make the tag: {why}"));
                    return;
                }
            },
            TagAsk::Add(id) => (id, self.tag_label("Add tag", id)),
            TagAsk::Remove(id) => {
                let label = self.tag_label("Remove tag", id);
                self.edit_tags(entities, &label, |tags| {
                    tags.remove(id);
                });
                return;
            }
        };
        let full: Vec<&str> = entities
            .iter()
            .filter_map(|&e| self.open.document.level().get(e))
            .filter(|e| {
                let t = e.get::<Tags>().unwrap_or_default();
                !t.has(id) && t.ids.len() >= level::builtin::MAX_TAGS_PER_ENTITY
            })
            .map(|e| e.name.as_str())
            .collect();
        if let Some(first) = full.first() {
            let first = (*first).to_owned();
            self.log(
                LogLevel::Warning,
                format!(
                    "{first} carries {} tags already, the most an entity can.",
                    level::builtin::MAX_TAGS_PER_ENTITY
                ),
            );
        }
        self.edit_tags(entities, &label, |tags| {
            tags.add(id);
        });
    }

    /// "Add tag Enemy".
    fn tag_label(&self, what: &str, id: TagId) -> String {
        format!("{what} {}", self.open.tags.name_of(id).unwrap_or("?"))
    }

    /// One undo step setting each entity's tags to what `change` makes of
    /// them (the project's own only), adding a `Tags` where there's none.
    fn edit_tags(&mut self, entities: &[EntityId], label: &str, change: impl Fn(&mut Tags)) {
        let mut edits = Vec::new();
        for &entity in entities {
            let Some(data) = self.open.document.level().get(entity) else {
                continue;
            };
            let had = data.get::<Tags>();
            let mut tags = Tags::of(carried(&self.open.tags, data));
            change(&mut tags);
            if had.as_ref() == Some(&tags) || (had.is_none() && tags.ids.is_empty()) {
                continue;
            }
            if let Some(playing) = &mut self.open.playing {
                playing.note_yours(entity, Tags::NAME, "tags");
            }
            edits.push(match had {
                Some(_) => Edit::SetField {
                    entity,
                    component: Tags::NAME.to_owned(),
                    field: "tags".to_owned(),
                    value: Some(reflect::Value::Text(tags.text())),
                },
                None => Edit::AddComponent {
                    entity,
                    component: Tags::NAME.to_owned(),
                    data: tags.to_data(),
                },
            });
        }
        if !edits.is_empty() {
            self.command(label, edits, None, None);
        }
    }

    /// Puts a prefab copy's tags back to its prefab's (↶): the prefab's
    /// own, or none when the prefab has no tags.
    fn revert_tags(&mut self, entity: EntityId) {
        let Some(expected) = self.part_of(entity).and_then(|p| p.expected) else {
            return;
        };
        let has = self
            .open
            .document
            .level()
            .get(entity)
            .is_some_and(|e| e.has(Tags::NAME));
        let edit = match expected.components.get(Tags::NAME) {
            Some(data) => Edit::SetField {
                entity,
                component: Tags::NAME.to_owned(),
                field: "tags".to_owned(),
                value: data.get("tags").cloned(),
            },
            None if has => Edit::RemoveComponent {
                entity,
                component: Tags::NAME.to_owned(),
            },
            None => return,
        };
        self.command("Revert tags", vec![edit], None, None);
    }

    /// Changes a trigger zone's Only tags, as one undo step.
    pub(crate) fn change_zone_tags(&mut self, entity: EntityId, ask: TagAsk) {
        use level::builtin::TriggerZone;
        let Some(zone) = self
            .open
            .document
            .level()
            .get(entity)
            .and_then(|e| e.get::<TriggerZone>())
        else {
            return;
        };
        let mut only = Tags::of(
            zone.only
                .iter()
                .copied()
                .filter(|id| self.open.tags.get(*id).is_some()),
        );
        let label = match ask {
            TagAsk::Add(id) => {
                only.add(id);
                self.tag_label("Only tag", id)
            }
            TagAsk::Remove(id) => {
                only.remove(id);
                self.tag_label("Not only tag", id)
            }
            TagAsk::New(name) => match self.make_tag(name.trim()) {
                Ok(id) => {
                    only.add(id);
                    format!("Only tag {}", name.trim())
                }
                Err(why) => {
                    self.log(LogLevel::Warning, format!("Couldn't make the tag: {why}"));
                    return;
                }
            },
            TagAsk::Revert => return,
        };
        let edit = Edit::SetField {
            entity,
            component: TriggerZone::NAME.to_owned(),
            field: "only_tags".to_owned(),
            value: Some(reflect::Value::Text(only.text())),
        };
        self.command(&label, vec![edit], None, None);
    }

    /// Whether a prefab copy's tags differ from its prefab's (the changed
    /// dot), for the one entity Properties shows.
    pub(crate) fn tags_changed_from_prefab(
        &self,
        entity: &level::EntityData,
        part: Option<&crate::properties::PrefabPart>,
    ) -> bool {
        let Some(expected) = part.and_then(|p| p.expected.as_ref()) else {
            return false;
        };
        carried(&self.open.tags, entity) != carried(&self.open.tags, expected)
    }
}

#[cfg(test)]
mod tests;
