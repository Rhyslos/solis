//! The Library browser as R4 builds it (decision 0118, board 04), driven
//! through the whole editor: the same browser on Home and the Library
//! tab, the right-click menus, New starting in renaming, renaming by
//! double-click and `F2` with bad names refused, moving by dragging,
//! Home's left side and its folder trees, and deleting.

use crate::library::{LeftShows, LibraryItem, LibraryKind, ProjectInfo};
use crate::panel::Panel;
use crate::shell::RibbonTab;
use crate::test_support::*;
use crate::workbench::{Workbench, WorkbenchAction};
use egui::Pos2;
use egui_kittest::kittest::Queryable;

type H = egui_kittest::Harness<'static, State>;

fn file(path: &str, kind: LibraryKind) -> LibraryItem {
    let name = path.rsplit('/').next().unwrap_or(path);
    LibraryItem {
        id: Some(reflect::AssetId::random()),
        kind,
        name: name.split('.').next().unwrap_or(name).to_owned(),
        path: path.to_owned(),
        thumbnail: None,
    }
}

/// A small farm: two folders at the top (one an older project's
/// `assets`), one inside, the engine's own, and a few files.
fn farm() -> Workbench {
    let mut w = Workbench::new(info());
    w.set_project(Some(ProjectInfo {
        name: "Farm Days".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm".into(),
    }));
    w.set_folders(
        ["assets", "farm", "farm/animals", "solis"]
            .map(String::from)
            .to_vec(),
    );
    w.set_library(vec![
        file("meadow.sollevel", LibraryKind::Level),
        file("main.lst", LibraryKind::Code),
        file("farm/hay.png", LibraryKind::Texture),
        file("farm/cluck.wav", LibraryKind::Sound),
        file("solis/project.lst", LibraryKind::Code),
    ]);
    w
}

fn started(mut w: Workbench, tab: RibbonTab) -> H {
    w.show_tab(tab);
    let mut h = harness(w);
    h.run();
    h.run();
    h
}

fn tile(h: &H, name: &str) -> egui::Rect {
    h.get_by_label(name).rect()
}

/// A point on a tile's picture (its top half), clear of its name.
fn picture_of(rect: egui::Rect) -> Pos2 {
    egui::pos2(rect.center().x, rect.top() + rect.width() / 3.0)
}

/// A point on a tile's name, under its picture.
fn name_of(rect: egui::Rect) -> Pos2 {
    egui::pos2(rect.left() + 8.0, rect.top() + rect.width() + 10.0)
}

fn right_click(h: &mut H, at: Pos2) {
    h.hover_at(at);
    h.run();
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        h.run();
    }
    h.run();
}

fn key(h: &mut H, key: egui::Key) {
    h.key_press(key);
    h.run();
    h.run();
}

fn actions(h: &H) -> &[WorkbenchAction] {
    &h.state().1.actions
}

#[test]
fn the_same_browser_on_both_tabs_shows_the_same_folder() {
    let mut h = started(farm(), RibbonTab::Library);
    // The project's own folders: assets is an ordinary one; the engine's
    // isn't shown.
    assert!(h.query_by_label("assets").is_some());
    assert!(h.query_by_label("solis").is_none());
    assert!(h.query_by_label("Folder solis").is_none());
    assert!(h.query_by_label("4 items \u{b7} 0 selected").is_some());
    let at = picture_of(tile(&h, "farm"));

    double_click(&mut h, at);
    assert_eq!(h.state().0.open.library_state.folder, "farm");
    assert!(h.query_by_label("hay").is_some());
    h.state_mut().0.show_tab(RibbonTab::Home);
    h.run();
    h.run();
    assert!(
        h.query_by_label("hay").is_some(),
        "Home's Library is in the same folder"
    );
    assert!(h.query_by_label("3 items \u{b7} 0 selected").is_some());
    h.get_by_label("Up one folder").click();
    h.run();
    h.run();
    assert_eq!(h.state().0.open.library_state.folder, "");
    h.state_mut().0.show_tab(RibbonTab::Library);
    h.run();
    h.run();
    assert!(h.query_by_label("meadow").is_some(), "and back on the tab");
    h.get_by_label("Back").click();
    h.run();
    assert_eq!(h.state().0.open.library_state.folder, "farm");
}

#[test]
fn the_library_tab_s_left_slot_is_empty_and_details_is_on_the_right() {
    let h = started(farm(), RibbonTab::Library);
    let w = &h.state().0;
    assert!(!in_view(w, Panel::Level) && !in_view(w, Panel::Folders));
    assert!(in_view(w, Panel::Library) && in_view(w, Panel::Properties));
    assert!(h.query_by_label("Details").is_some());
}

#[test]
fn grid_or_list_the_kind_filter_and_search() {
    let mut h = started(farm(), RibbonTab::Library);
    h.get_by_label("Show as a list").click();
    h.run();
    assert!(h.state().0.open.library_state.list);
    h.get_by_label("Show as tiles").click();
    h.run();
    assert!(!h.state().0.open.library_state.list);
    h.get_by_label("Kind of file").click();
    h.run();
    assert!(h.query_by_label("All kinds").is_some(), "its list");
    h.key_press(egui::Key::Escape);
    h.run();
    h.state_mut().0.open.library_state.filter = crate::library::Filter::Code;
    h.run();
    h.run();
    assert!(h.query_by_label("main").is_some());
    assert!(h.query_by_label("meadow").is_none());
    h.state_mut().0.open.library_state.filter = crate::library::Filter::All;
    h.get_by_label("Search the Library").click();
    h.run();
    h.get_by_label("Search the Library").type_text("hay");
    h.run();
    h.run();
    assert!(h.query_by_label("hay").is_some(), "found in another folder");
    assert!(h.query_by_label("meadow").is_none());
}

#[test]
fn right_click_on_space_makes_things_in_the_folder_shown() {
    let mut h = started(farm(), RibbonTab::Library);
    let at = picture_of(tile(&h, "farm"));

    double_click(&mut h, at);
    let files = body_of(&h, Panel::Library);
    right_click(
        &mut h,
        egui::pos2(files.right() - 40.0, files.bottom() - 40.0),
    );
    for row in [
        "New folder",
        "New level",
        "New code file",
        "New sound event…",
        "Import…",
    ] {
        assert!(h.query_by_label(row).is_some(), "{row}");
    }
    assert!(
        h.query_by_label("New prefab").is_some(),
        "unfinished, shown"
    );
    h.get_by_label("New folder").click();
    h.run();
    assert!(
        actions(&h).contains(&WorkbenchAction::NewFolder("farm".into())),
        "in the folder shown"
    );
}

#[test]
fn something_new_starts_in_renaming_and_enter_keeps_its_name() {
    let mut h = started(farm(), RibbonTab::Library);
    h.state_mut().0.open.library_state.open_folder("farm");
    let mut asked = Vec::new();
    h.state_mut().0.library_request(
        crate::library::LibraryRequest::NewLevel("farm".into()),
        &mut asked,
    );
    assert!(matches!(
        asked.as_slice(),
        [WorkbenchAction::NewTextFile { path, .. }] if path.to_str() == Some("farm/level.sollevel")
    ));
    assert!(
        !asked
            .iter()
            .any(|a| matches!(a, WorkbenchAction::OpenLevelFile(_))),
        "not opened: it's named first"
    );
    // The app makes it; the Library lists it, and its name box opens.
    let mut files = h.state().0.open.library.to_vec();
    files.push(file("farm/level.sollevel", LibraryKind::Level));
    h.state_mut().0.set_library(files);
    h.run();
    h.run();
    let renaming = h.state().0.open.library_state.renaming.clone();
    assert_eq!(
        renaming,
        Some(("farm/level.sollevel".into(), "level".into()))
    );
    h.get_by_label("New name").type_text("Barn inside");
    key(&mut h, egui::Key::Enter);
    assert!(actions(&h).contains(&WorkbenchAction::MoveFile {
        from: "farm/level.sollevel".into(),
        to: "farm/Barn inside.sollevel".into(),
    }));
    assert_eq!(
        h.state().0.open.library_state.selected.as_deref(),
        Some("farm/Barn inside.sollevel"),
        "what's picked follows the new name"
    );
}

#[test]
fn a_double_click_on_a_name_renames_and_on_the_picture_opens() {
    let mut h = started(farm(), RibbonTab::Library);
    let meadow = tile(&h, "meadow");
    double_click(&mut h, picture_of(meadow));
    assert!(
        actions(&h)
            .iter()
            .any(|a| matches!(a, WorkbenchAction::OpenLevelFile(_))),
        "the picture opens it"
    );
    assert!(h.state().0.open.library_state.renaming.is_none());
    double_click(&mut h, name_of(meadow));
    assert_eq!(
        h.state().0.open.library_state.renaming,
        Some(("meadow.sollevel".into(), "meadow".into())),
        "the name renames it"
    );
    key(&mut h, egui::Key::Escape);
    assert!(h.state().0.open.library_state.renaming.is_none());
    assert!(
        !actions(&h)
            .iter()
            .any(|a| matches!(a, WorkbenchAction::MoveFile { .. })),
        "Esc keeps the old name"
    );
}

#[test]
fn f2_renames_and_a_bad_name_keeps_the_old_one_and_says_why() {
    let mut h = started(farm(), RibbonTab::Library);
    let main = tile(&h, "main");
    h.hover_at(picture_of(main));
    h.run();
    h.get_by_label("main").click();
    h.run();
    h.hover_at(picture_of(main));
    h.run();
    key(&mut h, egui::Key::F2);
    assert_eq!(
        h.state().0.open.library_state.renaming,
        Some(("main.lst".into(), "main".into()))
    );
    h.get_by_label("New name").type_text("what?");
    h.run();
    assert!(
        h.query_by_label_contains("Windows forbids").is_some(),
        "said under the box while typed"
    );
    key(&mut h, egui::Key::Enter);
    assert!(
        !actions(&h)
            .iter()
            .any(|a| matches!(a, WorkbenchAction::MoveFile { .. })),
        "the old name stays"
    );
    let said = h.state().0.log_lines();
    assert!(
        said.iter()
            .any(|l| l.text.contains("Kept the name main.lst") && l.text.contains("forbids")),
        "the Console says why"
    );
}

#[test]
fn dragging_a_tile_onto_a_folder_moves_it() {
    let mut h = started(farm(), RibbonTab::Library);
    let main = picture_of(tile(&h, "main"));
    let farm = picture_of(tile(&h, "farm"));
    drag_for_frames(&mut h, main, (farm - main) / 10.0, 10);
    h.run();
    assert!(actions(&h).contains(&WorkbenchAction::MoveFile {
        from: "main.lst".into(),
        to: "farm/main.lst".into(),
    }));
}

#[test]
fn several_picked_are_deleted_together_to_the_recycle_bin() {
    let mut h = started(farm(), RibbonTab::Library);
    let main = tile(&h, "main");
    let at = picture_of(tile(&h, "meadow"));

    click(&mut h, at, egui::Modifiers::NONE);
    click(&mut h, picture_of(main), egui::Modifiers::COMMAND);
    assert_eq!(h.state().0.open.library_state.picked().len(), 2);
    assert!(h.query_by_label("4 items \u{b7} 2 selected").is_some());
    h.hover_at(picture_of(main));
    h.run();
    key(&mut h, egui::Key::Delete);
    for path in ["meadow.sollevel", "main.lst"] {
        assert!(
            actions(&h).contains(&WorkbenchAction::DeleteFile(path.into())),
            "{path}"
        );
    }
}

#[test]
fn the_menu_on_one_file_and_on_several() {
    let mut h = started(farm(), RibbonTab::Library);
    let at = picture_of(tile(&h, "main"));

    right_click(&mut h, at);
    for row in ["Open", "Rename", "Duplicate", "Delete", "Show in Explorer"] {
        assert!(h.query_by_label(row).is_some(), "{row}");
    }
    h.get_by_label("Duplicate").click();
    h.run();
    assert!(actions(&h).contains(&WorkbenchAction::DuplicateFile("main.lst".into())));
    h.key_press(egui::Key::Escape);
    h.run();
    let at = picture_of(tile(&h, "meadow"));

    click(&mut h, at, egui::Modifiers::COMMAND);
    let at = picture_of(tile(&h, "meadow"));

    right_click(&mut h, at);
    assert!(h.query_by_label("Rename").is_none(), "not for two at once");
    assert!(h.query_by_label("Delete").is_some());
}

#[test]
fn home_s_left_side_switches_between_the_level_panel_and_the_folders() {
    let mut h = started(farm(), RibbonTab::Home);
    assert!(in_view(&h.state().0, Panel::Level));
    h.get_by_label("Folders").click();
    h.run();
    h.run();
    assert_eq!(h.state().0.open.library_state.left, LeftShows::Folders);
    assert!(in_view(&h.state().0, Panel::Folders));
    assert!(!in_view(&h.state().0, Panel::Level));
    h.get_by_label("Level").click();
    h.run();
    h.run();
    assert!(in_view(&h.state().0, Panel::Level));
    // Showing either from elsewhere switches it too.
    h.state_mut().0.show_panel(Panel::Folders);
    h.run();
    h.run();
    assert!(in_view(&h.state().0, Panel::Folders));
}

#[test]
fn two_trees_side_by_side_move_a_file_from_one_to_the_other() {
    let mut w = farm();
    w.open.library_state.left = LeftShows::Folders;
    w.open.library_state.trees.open_another(0);
    w.open.library_state.trees.list[1].top = "farm".into();
    let mut h = started(w, RibbonTab::Home);
    let trees = body_of(&h, Panel::Folders);
    // The first tree shows the top (main.lst among its files), the second
    // farm (its animals folder).
    let main = h
        .get_all_by_label("main")
        .find(|n| n.rect().center().x < trees.center().x)
        .expect("main in the first tree")
        .rect()
        .center();
    let animals = h
        .get_all_by_label("Folder animals")
        .find(|n| n.rect().center().x > trees.center().x)
        .expect("animals in the second tree")
        .rect()
        .center();
    drag_for_frames(&mut h, main, (animals - main) / 10.0, 10);
    h.run();
    assert!(actions(&h).contains(&WorkbenchAction::MoveFile {
        from: "main.lst".into(),
        to: "farm/animals/main.lst".into(),
    }));
    // A tree closes; the last stays.
    h.state_mut().0.open.library_state.trees.close(1);
    h.state_mut().0.open.library_state.trees.close(0);
    assert_eq!(h.state().0.open.library_state.trees.list.len(), 1);
}
