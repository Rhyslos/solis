use super::*;
use crate::test_support::info;
use level::builtin::Light;

/// A workbench with a post and a lamp under it, and a fence beside them.
fn yard() -> (Workbench, EntityId, EntityId, EntityId) {
    let mut w = Workbench::new(info());
    let post = w.add_entity();
    let lamp = w.add_point_light();
    w.reparent(&[lamp], Some(post));
    let fence = w.add_entity();
    (w, post, lamp, fence)
}

/// What the app does with a plan: the module known, then the swap.
fn made(w: &mut Workbench, plan: &MakeModulePlan) -> AssetId {
    let id = AssetId::random();
    w.set_module(id, std::sync::Arc::new(plan.file.clone()));
    if let Some(swap) = &plan.swap {
        w.swap_for_copy(id, swap);
    }
    id
}

#[test]
fn one_picked_entity_becomes_a_module_and_a_linked_copy_in_one_undo_step() {
    let (mut w, post, lamp, _) = yard();
    w.select(post);
    w.ask_make_module();
    let mut form = w.open.make_module.clone().expect("the dialog is open");
    assert_eq!(form.count, 2, "the post and its lamp");
    assert!(form.swap, "swapped for a copy, at first");
    form.name = "Lamp post".into();
    let plan = w.make_module_plan(&form).unwrap();
    assert_eq!(plan.file.kind, FileKind::Module { root: post });
    assert_eq!(plan.file.level.get(post).unwrap().name, "Lamp post");
    let steps = w.document().history().undo_len();
    let before = w.document().level().clone();
    made(&mut w, &plan);
    let level = w.document().level();
    let root = level.get(post).unwrap();
    assert!(
        root.origin.as_ref().is_some_and(|o| o.path.is_empty()),
        "the post is a copy's root, keeping its ID"
    );
    assert!(!level.contains(lamp), "its part has the copy's own ID");
    assert_eq!(level.tree(post).len(), 2);
    assert_eq!(
        w.document().history().undo_len(),
        steps + 1,
        "one undo step"
    );
    assert!(
        w.level_file().unwrap().overrides.is_empty(),
        "the copy matches"
    );
    w.undo();
    assert_eq!(*w.document().level(), before, "undo puts the entities back");
}

#[test]
fn several_picked_entities_go_under_a_new_root_and_keep_their_places() {
    let (mut w, post, _, fence) = yard();
    let at = |w: &Workbench, id| w.document().world_matrix(id).w_axis;
    let (post_at, fence_at) = (at(&w, post), at(&w, fence));
    w.open.document.selection.select_all([post, fence], post);
    w.ask_make_module();
    let mut form = w.open.make_module.clone().unwrap();
    assert_eq!(form.count, 3);
    form.name = "Yard".into();
    let plan = w.make_module_plan(&form).unwrap();
    let FileKind::Module { root } = plan.file.kind else {
        panic!("a level")
    };
    assert_eq!(plan.file.level.get(root).unwrap().name, "Yard");
    made(&mut w, &plan);
    let level = w.document().level();
    assert!(!level.contains(post) && !level.contains(fence));
    let copy = level.get(root).expect("the copy stands where they were");
    assert_eq!(copy.name, "Yard");
    let posts: Vec<_> = level
        .iter()
        .filter(|(_, e)| e.origin.is_some() && e.name == "Entity")
        .map(|(id, _)| at(&w, id))
        .collect();
    assert_eq!(posts.len(), 2);
    assert!(
        posts.contains(&post_at) && posts.contains(&fence_at),
        "{posts:?}"
    );
}

#[test]
fn without_the_swap_the_level_stays_as_it_was() {
    let (mut w, post, _, _) = yard();
    w.select(post);
    w.ask_make_module();
    let mut form = w.open.make_module.clone().unwrap();
    form.swap = false;
    let before = w.document().level().clone();
    let plan = w.make_module_plan(&form).unwrap();
    assert!(plan.swap.is_none());
    made(&mut w, &plan);
    assert_eq!(*w.document().level(), before);
}

#[test]
fn nothing_picked_or_no_name_is_said_never_made() {
    let mut w = Workbench::new(info());
    w.open.document.selection.clear();
    w.ask_make_module();
    assert!(w.open.make_module.is_none());
    assert!(
        w.log_lines()
            .last()
            .unwrap()
            .text
            .contains("Pick the entities")
    );
    let (mut w, post, _, _) = yard();
    w.select(post);
    w.ask_make_module();
    let mut form = w.open.make_module.clone().unwrap();
    form.name = "  ".into();
    assert!(
        w.make_module_plan(&form)
            .unwrap_err()
            .contains("needs a name")
    );
}

#[test]
fn the_dialog_starts_in_the_library_s_folder_and_browse_changes_it() {
    let (mut w, post, _, _) = yard();
    w.open.library_state.folder = "farm/animals".into();
    w.select(post);
    w.ask_make_module();
    assert_eq!(w.open.make_module.as_ref().unwrap().folder, "farm/animals");
    w.set_module_folder("/props/");
    assert_eq!(w.open.make_module.as_ref().unwrap().folder, "props");
    let plan = w
        .make_module_plan(w.open.make_module.as_ref().unwrap())
        .unwrap();
    assert_eq!(plan.folder, std::path::PathBuf::from("props"));
    let _ = Light::point();
}
