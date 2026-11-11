use super::*;
use crate::test_support::{harness, info};
use egui_kittest::kittest::Queryable;

fn fresh() -> Workbench {
    Workbench::new(info(), None)
}

#[test]
fn a_word_at_the_start_scores_most_then_inside_then_letters_in_order() {
    assert_eq!(score("save", "Save area"), Some(3));
    assert_eq!(score("area", "Save area"), Some(3));
    assert_eq!(score("ave", "Save area"), Some(2));
    assert_eq!(score("svar", "Save area"), Some(1));
    assert_eq!(score("save area", "Save area as…"), Some(6));
    assert_eq!(score("zebra", "Save area"), None);
    assert_eq!(
        score("save zebra", "Save area"),
        None,
        "every word must match"
    );
    assert_eq!(score("", "anything"), Some(0));
    assert_eq!(score("light", "Add point light"), Some(3));
    // Odd text is safe.
    assert_eq!(score("é", "Café"), Some(2));
    assert!(score("x", "").is_none());
}

#[test]
fn the_best_match_comes_first_and_actions_before_entities() {
    let w = fresh();
    let items = w.search_items();
    let found = results(&items, "light");
    assert_eq!(found[0].name, "Add point light");
    assert!(found.iter().any(|i| i.name == "Add spotlight"));
    // The Sun is an entity: found by name.
    let sun = results(&items, "sun");
    assert!(sun.iter().any(|i| i.kind == SearchKind::Entity));
    // Panels too.
    assert!(
        results(&items, "console")
            .iter()
            .any(|i| i.kind == SearchKind::Panel)
    );
    // Nothing typed: the actions, in menu order.
    let all = results(&items, "");
    assert_eq!(all[0].name, "New project…");
    assert!(all.iter().all(|i| i.kind == SearchKind::Action));
    assert!(all.len() <= MAX_SHOWN);
}

#[test]
fn actions_that_cant_be_done_are_left_out() {
    let mut w = fresh();
    let names =
        |w: &Workbench| -> Vec<String> { w.search_items().into_iter().map(|i| i.name).collect() };
    assert!(
        !names(&w).contains(&"Pause".to_string()),
        "not while editing"
    );
    assert!(
        !names(&w).contains(&"Delete".to_string()),
        "nothing selected"
    );
    w.play();
    assert!(names(&w).contains(&"Pause".to_string()));
    assert!(!names(&w).contains(&"Play".to_string()));
    w.stop();
    let id = w.add_point_light();
    w.select(id);
    assert!(names(&w).contains(&"Delete".to_string()));
    // Undo shows what it undoes.
    assert!(names(&w).iter().any(|n| n.starts_with("Undo ")));
}

#[test]
fn space_opens_the_search_and_enter_does_the_first_result() {
    let mut h = harness(fresh());
    h.run();
    assert!(h.query_by_label("Search").is_none());
    h.key_press(egui::Key::Space);
    h.run();
    h.run();
    assert!(h.state().0.search.open);
    let before = h.state().0.document().area().iter().count();
    h.get_by_label("Search").type_text("point light");
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(!h.state().0.search.open);
    assert_eq!(h.state().0.document().area().iter().count(), before + 1);
}

#[test]
fn arrows_choose_esc_closes_and_the_box_opens_it_too() {
    let mut h = harness(fresh());
    h.run();
    h.get_by_label("Search or do anything").click();
    h.run();
    h.run();
    assert!(h.state().0.search.open);
    h.get_by_label("Search").type_text("add");
    h.run();
    h.key_press(egui::Key::ArrowDown);
    h.run();
    assert_eq!(h.state().0.search.chosen, 1);
    h.key_press(egui::Key::ArrowUp);
    h.key_press(egui::Key::ArrowUp);
    h.run();
    assert_eq!(h.state().0.search.chosen, 0, "not past the top");
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(!h.state().0.search.open);
}

#[test]
fn space_while_typing_types_a_space() {
    let mut w = fresh();
    let id = w.add_point_light();
    w.select(id);
    let mut h = harness(w);
    h.run();
    let name = h.get_by_label("Name");
    name.click();
    h.run();
    h.get_by_label("Name").type_text(" lamp");
    h.key_press(egui::Key::Space);
    h.run();
    assert!(!h.state().0.search.open);
}

#[test]
fn picking_an_entity_selects_it_and_a_panel_brings_it_forward() {
    let mut w = fresh();
    let lamp = w.add_point_light();
    w.open.document.selection.clear();
    let ctx = egui::Context::default();
    let mut actions = Vec::new();
    w.do_search(SearchTarget::Entity(lamp), &ctx, &mut actions);
    assert_eq!(w.selection(), Some(lamp));
    w.do_search(SearchTarget::Panel(Panel::Console), &ctx, &mut actions);
    let path = w.dock.find_tab(&Panel::Console).unwrap();
    assert_eq!(
        w.dock[path.surface][path.node].tabs().unwrap()[path.tab.0],
        Panel::Console
    );
    // An entity since deleted is ignored.
    w.delete_selection();
    w.do_search(SearchTarget::Entity(lamp), &ctx, &mut actions);
    assert_eq!(w.selection(), None);
    // A File action goes to the app.
    w.do_search(
        SearchTarget::App(WorkbenchAction::SaveArea),
        &ctx,
        &mut actions,
    );
    assert!(actions.contains(&WorkbenchAction::SaveArea));
}

#[test]
fn workspaces_can_be_switched_from_the_search() {
    let mut w = fresh();
    let items = w.search_items();
    let code = results(&items, "code workspace")[0].clone();
    assert_eq!(code.name, "Code workspace");
    let ctx = egui::Context::default();
    w.do_search(code.target, &ctx, &mut Vec::new());
    assert_eq!(w.workspace(), crate::workspace::Workspace::Code);
    assert!(
        !w.search_items().iter().any(|i| i.name == "Code workspace"),
        "the one in use isn't offered"
    );
}

const ASSETS: &str = "/projects/farm/assets";

/// A project with a file of each kind the search lists.
fn with_files() -> Workbench {
    use crate::library::{LibraryItem, LibraryKind as K, ProjectInfo};
    let mut w = fresh();
    w.set_project(Some(ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: ASSETS.into(),
    }));
    w.set_folders(vec![
        "ai".into(),
        "maps".into(),
        "notes".into(),
        "props".into(),
        "sprites".into(),
    ]);
    w.set_library(
        [
            ("props/ship.glb", K::Model),
            ("props/tower.solassembly", K::Assembly),
            ("sprites/hero.aseprite", K::SpriteSheet),
            ("maps/farm.tilemap.lst", K::TileMap),
            ("props/brick.png", K::Texture),
            ("ai/sparks.lua", K::Code),
            ("main.lst", K::Code),
            ("notes/plan.md", K::Text),
            ("props/brick.png.exe", K::Other),
            ("step.wav", K::Sound),
        ]
        .into_iter()
        .map(|(path, kind)| LibraryItem {
            id: Some(reflect::AssetId::random()),
            kind,
            name: crate::sprite_sheets::name_of(path),
            path: path.into(),
            thumbnail: None,
        })
        .collect(),
    );
    w
}

/// What choosing the file at `path` in the search asks the app for.
fn choose(w: &mut Workbench, path: &str) -> Vec<WorkbenchAction> {
    let item = w
        .search_items()
        .into_iter()
        .find(|i| matches!(&i.target, SearchTarget::File(_, p) if p == path))
        .unwrap_or_else(|| panic!("{path} isn't listed"));
    let mut actions = Vec::new();
    w.do_search(item.target, &egui::Context::default(), &mut actions);
    actions
}

#[test]
fn choosing_a_file_to_place_picks_it_in_the_library_and_asks_the_computer_for_nothing() {
    // SEC-05: a model, assembly, sprite sheet, tile map or picture chosen
    // in the search was handed, file and all, to whatever program the
    // computer has for it, where it was meant to be shown in the Library.
    for (path, folder) in [
        ("props/ship.glb", "props"),
        ("props/tower.solassembly", "props"),
        ("sprites/hero.aseprite", "sprites"),
        ("maps/farm.tilemap.lst", "maps"),
        ("props/brick.png", "props"),
    ] {
        let mut w = with_files();
        // Whatever the Library was showing, the file is found there.
        w.open.library_state.folder = "notes".into();
        w.open.library_state.search = "plan".into();
        w.open.library_state.filter = crate::library::Filter::Sounds;
        w.set_library_pinned(false);
        let actions = choose(&mut w, path);
        assert_eq!(actions, [], "{path}: nothing is asked of the app");
        assert_eq!(w.open.library_state.selected.as_deref(), Some(path));
        assert_eq!(
            w.open.library_state.folder, folder,
            "{path}: its folder is open"
        );
        assert!(w.open.library_state.search.is_empty(), "{path}");
        assert_eq!(
            w.open.library_state.filter,
            crate::library::Filter::All,
            "{path}: no chip hides it"
        );
        assert!(w.drawer_open, "{path}: the Library comes up");
    }
}

#[test]
fn a_file_chosen_to_place_shows_in_properties_as_a_click_in_the_library_would() {
    // A sprite sheet and a tile map have a page in Properties; a model has
    // none, and mustn't be taken for a sound.
    let mut w = with_files();
    choose(&mut w, "sprites/hero.aseprite");
    assert_eq!(
        w.open.sound_shown.as_ref().map(|(path, _)| path.as_str()),
        Some("sprites/hero.aseprite")
    );
    choose(&mut w, "maps/farm.tilemap.lst");
    assert_eq!(
        w.open.sound_shown.as_ref().map(|(path, _)| path.as_str()),
        Some("maps/farm.tilemap.lst")
    );
    let mut w = with_files();
    choose(&mut w, "props/ship.glb");
    assert_eq!(w.open.sound_shown, None);
    // A chip that shows the file is left as it is.
    w.open.library_state.filter = crate::library::Filter::Models;
    choose(&mut w, "props/tower.solassembly");
    assert_eq!(w.open.library_state.filter, crate::library::Filter::Models);
}

#[test]
fn choosing_any_other_file_opens_it_only_if_its_kind_may_open_in_another_program() {
    // The same rule as a double-click in the Library: Lust and text open,
    // a .lua file, a sound and a file that isn't what it seems show their
    // folder.
    let full = |inside: &str| WorkbenchAction::OpenExternally(format!("{ASSETS}/{inside}").into());
    for (path, asked) in [
        ("main.lst", full("main.lst")),
        ("notes/plan.md", full("notes/plan.md")),
        ("ai/sparks.lua", full("ai")),
        ("props/brick.png.exe", full("props")),
        ("step.wav", WorkbenchAction::OpenExternally(ASSETS.into())),
    ] {
        let mut w = with_files();
        assert_eq!(choose(&mut w, path), [asked], "{path}");
    }
}

#[test]
fn enter_on_a_model_in_the_search_shows_it_in_the_library_and_opens_nothing() {
    let mut w = with_files();
    w.open.library_state.folder = "notes".into();
    // Another tab is in front of the Library, which is pinned.
    crate::layout::bring_to_front(&mut w.dock, Panel::Console);
    let mut h = harness(w);
    h.run();
    h.key_press(egui::Key::Space);
    h.run();
    h.run();
    h.get_by_label("Search").type_text("ship");
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    h.run();
    assert!(!h.state().0.search.open);
    assert!(
        !h.state().1.actions.iter().any(|a| matches!(
            a,
            WorkbenchAction::OpenExternally(_) | WorkbenchAction::ShowFolder(_)
        )),
        "{:?}",
        h.state().1.actions
    );
    let w = &h.state().0;
    assert_eq!(
        w.open.library_state.selected.as_deref(),
        Some("props/ship.glb")
    );
    assert_eq!(w.open.library_state.folder, "props");
    // Its tab came to the front, with the file in view.
    let _ = h.get_by_label("ship");
}
