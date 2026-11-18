//! Tags in the running game (stage 1, tags): each entity's `Tags`, as the
//! level gave them and as game code changes them (`addTag`, `removeTag`,
//! landing at the end of the tick in the tick's order). Tags are game
//! state: in the checksum, the state parts and saves (as the entity's
//! `Tags`, which a save keeps with the level).
//!
//! What game code asks of them ("every enemy within 3 m") is answered
//! from a snapshot made once a tick from [`World::tagged`]: see
//! `script::tag_view`.

use crate::{Global, Id, World};
use glam::Vec3;
use level::builtin::Tags;
use reflect::{EntityId, TagId};

impl World {
    /// The tags `id` carries now (none for an entity without any).
    pub fn tags_of(&self, id: EntityId) -> Tags {
        self.get::<Tags>(id).unwrap_or_default()
    }

    /// Gives `id` the tag `tag` (`on`) or takes it away, giving the entity
    /// a `Tags` if it has none. Nothing happens for an entity not in the
    /// game, a tag it already has (or hasn't), or one past the most an
    /// entity carries. Whether it changed.
    pub fn set_tag(&mut self, id: EntityId, tag: TagId, on: bool) -> bool {
        let Some(&entity) = self.by_id.get(&id) else {
            return false;
        };
        let changed = if let Ok(mut tags) = self.ecs.get::<&mut Tags>(entity) {
            if on { tags.add(tag) } else { tags.remove(tag) }
        } else if on {
            self.ecs.insert_one(entity, Tags::of([tag])).is_ok()
        } else {
            false
        };
        self.touched |= changed;
        changed
    }

    /// Every entity carrying a tag, in the order of their IDs, with where
    /// it is in the world now and its tags: what a tick's questions about
    /// tags are answered from.
    pub fn tagged(&self) -> Vec<(EntityId, Vec3, Vec<TagId>)> {
        let mut tagged: Vec<(EntityId, Vec3, Vec<TagId>)> = self
            .ecs
            .query::<(&Id, &Tags, &Global)>()
            .iter()
            .filter(|(_, tags, _)| !tags.ids.is_empty())
            .map(|(id, tags, global)| (id.0, global.0.w_axis.truncate(), tags.ids.clone()))
            .collect();
        tagged.sort_unstable_by_key(|(id, _, _)| *id);
        tagged
    }

    /// Every entity's tags, in the order of their IDs: those carrying
    /// none are left out.
    pub fn all_tags(&self) -> Vec<(EntityId, Tags)> {
        let mut all: Vec<(EntityId, Tags)> = self
            .ecs
            .query::<(&Id, &Tags)>()
            .iter()
            .filter(|(_, tags)| !tags.ids.is_empty())
            .map(|(id, tags)| (id.0, tags.clone()))
            .collect();
        all.sort_unstable_by_key(|(id, _)| *id);
        all
    }

    /// Adds every entity's tags to a checksum: nothing while no entity
    /// carries one, so older recordings' sums stand.
    pub(crate) fn sum_tags(&self, sum: &mut reflect::Checksum) {
        let all = self.all_tags();
        if all.is_empty() {
            return;
        }
        sum.u64(all.len() as u64);
        for (id, tags) in all {
            sum.entity(id);
            sum.u64(tags.ids.len() as u64);
            for tag in tags.ids {
                sum.u64(tag.0.raw());
            }
        }
    }
}

#[cfg(test)]
#[path = "tags_tests.rs"]
mod tests;
