//! What a click or a box in the viewport selects
//! ([decision 0025](../../../docs/decisions/0025-picking-and-gizmos.md)).
//!
//! The renderer says which placements are under the mouse; these rules
//! turn that into a new selection:
//!
//! - **A click** selects what's under the mouse; with Ctrl it's added or
//!   taken away. A click on nothing selects nothing (Ctrl: no change).
//! - **Assemblies:** a click on a copy of an assembly selects the whole
//!   copy (its root). Once something in that copy is selected, a click
//!   selects the entity itself, so a second click goes inside, as Unity
//!   does with prefabs.
//! - **A box** selects everything with a visible pixel inside it, each
//!   copy of an assembly as a whole; with Ctrl it adds to the selection.

use crate::document::Selection;
use area::Area;
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

/// The root of the outermost assembly copy `id` belongs to, or `id`
/// itself when it isn't part of one.
fn copy_root(area: &Area, id: EntityId) -> EntityId {
    area.get(id)
        .and_then(|e| e.origin.as_ref())
        .map_or(id, |o| o.instance)
}

/// Whether anything selected belongs to the copy whose root is `root`.
fn inside_selected_copy(area: &Area, selection: &Selection, root: EntityId) -> bool {
    selection.iter().any(|s| copy_root(area, s) == root)
}

/// What a click on `hit` (or on nothing) selects.
pub fn after_click(
    area: &Area,
    selection: &Selection,
    hit: Option<EntityId>,
    mode: PickMode,
) -> Selection {
    let mut out = selection.clone();
    let Some(hit) = hit.filter(|&h| area.contains(h)) else {
        if mode == PickMode::Replace {
            out.clear();
        }
        return out;
    };
    let root = copy_root(area, hit);
    let target = if root != hit && inside_selected_copy(area, selection, root) {
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
    area: &Area,
    selection: &Selection,
    hits: &[EntityId],
    mode: PickMode,
) -> Selection {
    let mut found: Vec<EntityId> = hits
        .iter()
        .filter(|&&h| area.contains(h))
        .map(|&h| copy_root(area, h))
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
    use area::{EntityData, Origin};
    use reflect::AssetId;

    /// An area with a loose crate and one copy of an assembly (a root
    /// and a lamp inside it).
    fn area() -> (Area, EntityId, EntityId, EntityId) {
        let mut area = Area::new();
        let loose = area.spawn(EntityData::new("Crate"));
        let root = EntityId::random();
        let assembly = AssetId::random();
        let mut root_data = EntityData::new("Lamp post");
        root_data.origin = Some(Origin {
            assembly,
            instance: root,
            path: Vec::new(),
        });
        area.insert(root, root_data);
        let mut lamp = EntityData::new("Lamp");
        lamp.parent = Some(root);
        lamp.origin = Some(Origin {
            assembly,
            instance: root,
            path: vec![EntityId::random()],
        });
        let lamp = area.spawn(lamp);
        (area, loose, root, lamp)
    }

    #[test]
    fn a_click_selects_what_is_under_the_mouse_and_ctrl_toggles() {
        let (area, loose, root, _) = area();
        let s = after_click(&area, &Selection::default(), Some(loose), PickMode::Replace);
        assert_eq!(s.primary(), Some(loose));
        let s = after_click(&area, &s, Some(root), PickMode::Toggle);
        assert!(s.contains(loose) && s.contains(root));
        let s = after_click(&area, &s, Some(loose), PickMode::Toggle);
        assert!(!s.contains(loose) && s.contains(root));
    }

    #[test]
    fn a_click_on_nothing_selects_nothing_unless_ctrl_is_held() {
        let (area, loose, _, _) = area();
        let mut s = Selection::default();
        s.select_only(loose);
        assert_eq!(after_click(&area, &s, None, PickMode::Toggle), s);
        assert_eq!(
            after_click(&area, &s, None, PickMode::Replace).primary(),
            None
        );
        let gone = EntityId::random();
        assert_eq!(
            after_click(&area, &s, Some(gone), PickMode::Replace).primary(),
            None,
            "an entity that's gone counts as nothing"
        );
    }

    #[test]
    fn a_copy_is_selected_whole_first_then_a_second_click_goes_inside() {
        let (area, _, root, lamp) = area();
        let first = after_click(&area, &Selection::default(), Some(lamp), PickMode::Replace);
        assert_eq!(first.primary(), Some(root), "the whole copy");
        let second = after_click(&area, &first, Some(lamp), PickMode::Replace);
        assert_eq!(second.primary(), Some(lamp), "then the lamp itself");
        assert!(!second.contains(root));
    }

    #[test]
    fn a_box_selects_copies_whole_and_ctrl_adds() {
        let (area, loose, root, lamp) = area();
        let s = after_box(
            &area,
            &Selection::default(),
            &[lamp, root, lamp],
            PickMode::Replace,
        );
        assert_eq!(s.iter().collect::<Vec<_>>(), [root]);
        let s = after_box(&area, &s, &[loose], PickMode::Add);
        assert!(s.contains(root) && s.contains(loose));
        let s = after_box(&area, &s, &[], PickMode::Replace);
        assert_eq!(s.iter().count(), 0, "an empty box selects nothing");
    }
}
