use super::*;
use crate::test_support::{info, lamp_post_file};
use level::builtin::Light;
use reflect::Value;

/// A workbench knowing the lamp post, with three copies of it placed.
fn three_copies() -> (Workbench, AssetId, Vec<EntityId>) {
    let mut w = Workbench::new(info());
    let (asset, file) = lamp_post_file();
    w.set_prefab(asset, file);
    let mut roots = Vec::new();
    for _ in 0..3 {
        w.place_prefab(asset);
        roots.push(w.selection().unwrap());
    }
    (w, asset, roots)
}

/// The lamp inside the copy whose root is `root`.
fn lamp_of(root: EntityId) -> EntityId {
    level::prefab::derive(root, &[EntityId(reflect::Id::from_raw(2).unwrap())])
}

/// The page in front for a frame's work, as `Workbench::show` has it.
fn in_front(w: &mut Workbench) {
    w.shell.arrangement_mut(RibbonTab::Home).front[0] = Some(Panel::PrefabPage);
    w.page_in();
}

/// The strength of the lamp in each copy.
fn strengths(w: &Workbench, roots: &[EntityId]) -> Vec<f32> {
    roots
        .iter()
        .map(|&r| {
            w.document()
                .level()
                .get(lamp_of(r))
                .unwrap()
                .get::<Light>()
                .unwrap()
                .strength
        })
        .collect()
}

/// What the app does with a save: hands the file back.
fn saved(w: &mut Workbench) {
    let outbox = std::mem::take(&mut w.open.outbox);
    for action in outbox {
        if let WorkbenchAction::SavePrefab { asset, file } = action {
            w.prefab_saved(asset, Arc::new(file), "/p/lamp.prefab.lst".into());
        }
    }
}

#[test]
fn the_page_is_the_prefab_on_its_own_with_an_undo_of_its_own() {
    let (mut w, asset, roots) = three_copies();
    let level_steps = w.document().history().undo_len();
    let copies_before = strengths(&w, &roots);
    w.open_prefab_page(asset);
    assert!(w.open.prefab_page.is_some());
    assert!(w.open_now().prefab_page);
    in_front(&mut w);
    assert!(w.page_in_front());
    let page = w.document().level();
    assert_eq!(page.len(), 2, "the post and its lamp, nothing of the level");
    assert!(
        page.iter().all(|(_, e)| e.origin.is_none()),
        "its own entities"
    );
    // A change on the page is the page's own undo step.
    let lamp = EntityId(reflect::Id::from_raw(2).unwrap());
    w.apply_property(
        crate::properties::PropertyRequest::SetField {
            entity: lamp,
            component: "Light".into(),
            field: "strength".into(),
            value: Value::Float(9.0),
        },
        1,
    );
    assert!(w.prefab_page_dirty());
    w.page_out();
    assert!(
        !w.page_in_front(),
        "between frames the level is the document"
    );
    assert_eq!(
        w.document().history().undo_len(),
        level_steps,
        "the level's undo is its own"
    );
    assert_eq!(
        w.document().level().len(),
        2 + 3 * 2,
        "the level, untouched"
    );
    assert_eq!(
        strengths(&w, &roots),
        copies_before,
        "copies change only when it's saved"
    );
}

#[test]
fn saving_the_page_updates_every_copy_keeping_each_copy_s_own_change() {
    let (mut w, asset, roots) = three_copies();
    let before = strengths(&w, &roots)[0];
    // The second copy changes the strength itself.
    w.apply_property(
        crate::properties::PropertyRequest::SetField {
            entity: lamp_of(roots[1]),
            component: "Light".into(),
            field: "strength".into(),
            value: Value::Float(42.0),
        },
        1,
    );
    w.open_prefab_page(asset);
    in_front(&mut w);
    let lamp = EntityId(reflect::Id::from_raw(2).unwrap());
    w.apply_property(
        crate::properties::PropertyRequest::SetField {
            entity: lamp,
            component: "Light".into(),
            field: "strength".into(),
            value: Value::Float(7.0),
        },
        2,
    );
    w.save_prefab_page();
    saved(&mut w);
    assert!(!w.prefab_page_dirty(), "saved");
    w.page_out();
    assert_ne!(before, 7.0);
    assert_eq!(
        strengths(&w, &roots),
        [7.0, 42.0, 7.0],
        "every copy follows, but the second keeps its own change"
    );
}

#[test]
fn a_saved_value_is_one_undo_step_and_a_saved_level_stays_saved() {
    // Shared 1f's loose ends: copies follow a prefab's new values alone
    // (`level::prefab_follow`), as rebuilding them did.
    let (mut w, asset, roots) = three_copies();
    w.open.document.mark_saved("/p/street.sollevel".into());
    let steps = w.document().history().undo_len();
    let (_, mut file) = lamp_post_file();
    let lamp = EntityId(reflect::Id::from_raw(2).unwrap());
    Arc::make_mut(&mut file)
        .level
        .get_mut(lamp)
        .unwrap()
        .set(&Light {
            strength: 7.0,
            ..Light::point()
        });
    w.set_prefab(asset, file);
    assert_eq!(strengths(&w, &roots), [7.0; 3]);
    assert_eq!(w.document().history().undo_len(), steps + 1, "one step");
    assert!(!w.document().is_dirty(), "the level's file is the same");
    w.undo();
    assert_eq!(strengths(&w, &roots), [Light::point().strength; 3]);
}

#[test]
fn a_part_added_to_the_prefab_comes_to_every_copy() {
    // More than values: the copies are rebuilt the whole way.
    let (mut w, asset, roots) = three_copies();
    let (_, mut file) = lamp_post_file();
    let mut sign = level::EntityData::new("Sign");
    sign.parent = Some(EntityId(reflect::Id::from_raw(1).unwrap()));
    Arc::make_mut(&mut file)
        .level
        .insert(EntityId(reflect::Id::from_raw(3).unwrap()), sign);
    w.set_prefab(asset, file);
    for root in roots {
        let sign = level::prefab::derive(root, &[EntityId(reflect::Id::from_raw(3).unwrap())]);
        assert_eq!(w.document().level().get(sign).unwrap().name, "Sign");
    }
}

#[test]
fn a_page_with_unsaved_changes_asks_before_it_closes() {
    let (mut w, asset, _) = three_copies();
    w.open_prefab_page(asset);
    w.close_prefab_page();
    assert!(w.open.prefab_page.is_none(), "saved: it closes at once");
    w.open_prefab_page(asset);
    in_front(&mut w);
    let root = EntityId(reflect::Id::from_raw(1).unwrap());
    w.select(root);
    w.add_entity();
    w.close_prefab_page();
    assert!(w.open.prefab_page.as_ref().unwrap().asking_to_close);
    // Leaving the project asks the same.
    let mut actions = Vec::new();
    w.request(WorkbenchAction::CloseProject, &mut actions);
    assert!(actions.is_empty(), "held for the question");
}

#[test]
fn ctrl_s_on_the_page_saves_the_prefab_and_play_brings_the_level_back() {
    let (mut w, asset, _) = three_copies();
    w.open_prefab_page(asset);
    in_front(&mut w);
    let mut actions = Vec::new();
    w.request(WorkbenchAction::SaveLevel, &mut actions);
    assert!(actions.is_empty(), "not the level");
    assert!(
        w.open
            .outbox
            .iter()
            .any(|a| matches!(a, WorkbenchAction::SavePrefab { asset: a, .. } if *a == asset))
    );
    w.level_to_front();
    assert!(!w.page_in_front());
    assert_eq!(w.document().level().len(), 2 + 3 * 2, "the level is back");
}
