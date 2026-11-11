use super::*;
use crate::test_support::{harness, info};
use egui_kittest::kittest::Queryable;

/// A layout as text, to compare two.
fn text(dock: &DockState<Panel>) -> String {
    ron::to_string(dock).unwrap()
}

fn fresh() -> Workbench {
    Workbench::new(info(), None)
}

#[test]
fn every_workspace_s_standard_layout_is_whole_and_draws() {
    for workspace in Workspace::ALL {
        let dock = workspace.standard();
        assert!(layout::is_complete(&dock), "{workspace:?}");
        // A saved copy reads back unchanged (it draws and has every panel).
        let back = layout::restore_with(Some(dock.clone()), workspace.fallback());
        assert_eq!(
            back.iter_all_tabs().count(),
            dock.iter_all_tabs().count(),
            "{workspace:?}"
        );
    }
}

#[test]
fn the_code_workspace_has_the_game_in_front_and_console_beside_problems() {
    let dock = Workspace::Code.standard();
    let front = |panel| {
        let path = dock.find_tab(&panel).unwrap();
        dock[path.surface][path.node].tabs().unwrap()[0] == panel
    };
    assert!(front(Panel::Game));
    assert!(front(Panel::Console));
    assert!(front(Panel::Problems), "its own group, beside the Console");
    assert!(front(Panel::Code));
}

#[test]
fn switching_remembers_each_workspace_s_layout() {
    let mut w = fresh();
    assert_eq!(w.workspace(), Workspace::Level);
    // A change to Level's layout…
    layout::bring_to_front(&mut w.dock, Panel::Console);
    let level = w.dock.clone();
    w.switch_workspace(Workspace::Code);
    assert_eq!(w.workspace(), Workspace::Code);
    assert_ne!(text(&w.dock), text(&level));
    // …is there again on coming back.
    w.switch_workspace(Workspace::Level);
    assert_eq!(text(&w.dock), text(&level));
    // Switching to the one in use changes nothing.
    w.switch_workspace(Workspace::Level);
    assert_eq!(text(&w.dock), text(&level));
}

#[test]
fn a_file_s_panels_go_with_you() {
    let mut w = fresh();
    layout::open_beside(&mut w.dock, Panel::Animator, Panel::Viewport);
    layout::open_beside(&mut w.dock, Panel::Frames, Panel::Console);
    w.switch_workspace(Workspace::Animate);
    assert!(w.dock.find_tab(&Panel::Animator).is_some());
    assert!(w.dock.find_tab(&Panel::Frames).is_some());
    // And they don't stay behind in the one left.
    let level = &w.other_layouts[&Workspace::Level];
    assert!(level.find_tab(&Panel::Animator).is_none());
}

#[test]
fn reset_layout_resets_only_the_workspace_in_use() {
    let mut w = fresh();
    w.switch_workspace(Workspace::Code);
    layout::bring_to_front(&mut w.dock, Panel::Library);
    w.reset_layout();
    assert_eq!(text(&w.dock), text(&Workspace::Code.standard()));
}

#[test]
fn workspaces_are_saved_and_read_back() {
    let mut w = fresh();
    w.switch_workspace(Workspace::Animate);
    let saved = w.workspaces();
    assert_eq!(saved.current, Workspace::Animate);
    assert_eq!(saved.layouts.len(), 2, "Level and Animate so far");
    let written = ron::to_string(&saved).unwrap();
    let mut again = fresh();
    again.set_workspaces(ron::from_str(&written).unwrap());
    assert_eq!(again.workspace(), Workspace::Animate);
    assert_eq!(text(&again.dock), text(&w.dock));
    // A damaged layout becomes that workspace's standard.
    let mut broken = saved.clone();
    broken
        .layouts
        .insert(Workspace::Code, DockState::new(vec![]));
    broken.current = Workspace::Code;
    again.set_workspaces(broken);
    assert_eq!(text(&again.dock), text(&Workspace::Code.standard()));
}

#[test]
fn an_unpinned_library_stays_out_of_every_workspace() {
    let mut w = fresh();
    w.set_library_pinned(false);
    w.switch_workspace(Workspace::Code);
    assert!(w.dock.find_tab(&Panel::Library).is_none());
    w.reset_layout();
    assert!(w.dock.find_tab(&Panel::Library).is_none());
}

#[test]
fn the_tabs_in_the_tab_bar_switch_workspaces() {
    let mut h = harness(fresh());
    h.run();
    for name in ["Home", "Animate", "UI", "Debug"] {
        let _ = h.get_by_label(&format!("{name} tab"));
    }
    h.get_by_label("Debug tab").click();
    h.run();
    assert_eq!(h.state().0.workspace(), Workspace::Code);
    h.get_by_label("Animate tab").click();
    h.run();
    assert_eq!(h.state().0.workspace(), Workspace::Animate);
}
