//! What a click or a box in the viewport selects
//! ([decision 0025](../../../docs/decisions/0025-picking-and-gizmos.md)).
//!
//! The renderer says which placements are under the mouse; these rules
//! turn that into a new selection:
//!
//! - **A click** selects what's under the mouse; with Ctrl it's added or
//!   taken away. A click on nothing selects nothing (Ctrl: no change).
//! - **Prefabs:** a click on a copy of a prefab selects the whole
//!   copy (its root). Once something in that copy is selected, a click
//!   selects the entity itself, so a second click goes inside, as Unity
//!   does with prefabs.
//! - **A box** selects everything with a visible pixel inside it, each
//!   copy of a prefab as a whole; with Ctrl it adds to the selection.

use crate::document::Selection;
use level::Level;
use reflect::EntityId;

/// How a click or box changes the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickMode {
    /// Select only what was found.
    Replace,
    /// Add what was found, or take it out if it's already selected
    /// (Ctrl+click).
    Toggle,
    /// Add what was found (Ctrl+drag).
    Add,
}

/// The root of the outermost prefab copy `id` belongs to, or `id`
/// itself when it isn't part of one.
fn copy_root(level: &Level, id: EntityId) -> EntityId {
    level
        .get(id)
        .and_then(|e| e.origin.as_ref())
        .map_or(id, |o| o.instance)
}

/// Whether anything selected belongs to the copy whose root is `root`.
fn inside_selected_copy(level: &Level, selection: &Selection, root: EntityId) -> bool {
    selection.iter().any(|s| copy_root(level, s) == root)
}

/// What a click on `hit` (or on nothing) selects.
pub fn after_click(
    level: &Level,
    selection: &Selection,
    hit: Option<EntityId>,
    mode: PickMode,
) -> Selection {
    let mut out = selection.clone();
    let Some(hit) = hit.filter(|&h| level.contains(h)) else {
        if mode == PickMode::Replace {
            out.clear();
        }
        return out;
    };
    let root = copy_root(level, hit);
    let target = if root != hit && inside_selected_copy(level, selection, root) {
        hit
    } else {
        root
    };
    match mode {
        PickMode::Replace => out.select_only(target),
        PickMode::Toggle => out.toggle(target),
        PickMode::Add => {
            if !out.contains(target) {
                out.toggle(target);
            }
        }
    }
    out
}

/// What a box around `hits` selects.
pub fn after_box(
    level: &Level,
    selection: &Selection,
    hits: &[EntityId],
    mode: PickMode,
) -> Selection {
    let mut found: Vec<EntityId> = hits
        .iter()
        .filter(|&&h| level.contains(h))
        .map(|&h| copy_root(level, h))
        .collect();
    found.sort_unstable();
    found.dedup();
    let mut out = if mode == PickMode::Replace {
        Selection::default()
    } else {
        selection.clone()
    };
    for id in found {
        if !out.contains(id) {
            out.toggle(id);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use level::{EntityData, Origin};
    use reflect::AssetId;

    /// A level with a loose crate and one copy of a prefab (a root
    /// and a lamp inside it).
    fn level() -> (Level, EntityId, EntityId, EntityId) {
        let mut level = Level::new();
        let loose = level.spawn(EntityData::new("Crate"));
        let root = EntityId::random();
        let prefab = AssetId::random();
        let mut root_data = EntityData::new("Lamp post");
        root_data.origin = Some(Origin {
            prefab,
            instance: root,
            path: Vec::new(),
        });
        level.insert(root, root_data);
        let mut lamp = EntityData::new("Lamp");
        lamp.parent = Some(root);
        lamp.origin = Some(Origin {
            prefab,
            instance: root,
            path: vec![EntityId::random()],
        });
        let lamp = level.spawn(lamp);
        (level, loose, root, lamp)
    }

    #[test]
    fn a_click_selects_what_is_under_the_mouse_and_ctrl_toggles() {
        let (level, loose, root, _) = level();
        let s = after_click(
            &level,
            &Selection::default(),
            Some(loose),
            PickMode::Replace,
        );
        assert_eq!(s.primary(), Some(loose));
        let s = after_click(&level, &s, Some(root), PickMode::Toggle);
        assert!(s.contains(loose) && s.contains(root));
        let s = after_click(&level, &s, Some(loose), PickMode::Toggle);
        assert!(!s.contains(loose) && s.contains(root));
    }

    #[test]
    fn a_click_on_nothing_selects_nothing_unless_ctrl_is_held() {
        let (level, loose, _, _) = level();
        let mut s = Selection::default();
        s.select_only(loose);
        assert_eq!(after_click(&level, &s, None, PickMode::Toggle), s);
        assert_eq!(
            after_click(&level, &s, None, PickMode::Replace).primary(),
            None
        );
        let gone = EntityId::random();
        assert_eq!(
            after_click(&level, &s, Some(gone), PickMode::Replace).primary(),
            None,
            "an entity that's gone counts as nothing"
        );
    }

    #[test]
    fn a_copy_is_selected_whole_first_then_a_second_click_goes_inside() {
        let (level, _, root, lamp) = level();
        let first = after_click(&level, &Selection::default(), Some(lamp), PickMode::Replace);
        assert_eq!(first.primary(), Some(root), "the whole copy");
        let second = after_click(&level, &first, Some(lamp), PickMode::Replace);
        assert_eq!(second.primary(), Some(lamp), "then the lamp itself");
        assert!(!second.contains(root));
    }

    #[test]
    fn a_box_selects_copies_whole_and_ctrl_adds() {
        let (level, loose, root, lamp) = level();
        let s = after_box(
            &level,
            &Selection::default(),
            &[lamp, root, lamp],
            PickMode::Replace,
        );
        assert_eq!(s.iter().collect::<Vec<_>>(), [root]);
        let s = after_box(&level, &s, &[loose], PickMode::Add);
        assert!(s.contains(root) && s.contains(loose));
        let s = after_box(&level, &s, &[], PickMode::Replace);
        assert_eq!(s.iter().count(), 0, "an empty box selects nothing");
    }
}
