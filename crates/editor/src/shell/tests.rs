use super::*;
use crate::test_support::{harness, info, ribbon_tab, run};
use crate::workbench::WorkbenchAction;
use std::collections::BTreeSet;

fn fresh() -> Workbench {
    Workbench::new(info(), None)
}

#[test]
fn every_command_has_a_name_of_its_own_and_a_place() {
    let w = fresh();
    let mut names = BTreeSet::new();
    for command in ShellCommand::ALL {
        let name = command.name(&w);
        assert!(!name.is_empty(), "{command:?} has no name");
        assert!(!command.place().is_empty(), "{command:?} has no place");
        assert!(names.insert(name.clone()), "two commands are called {name}");
        assert_eq!(ShellCommand::named(&name, &w), Some(command));
    }
    let listed: BTreeSet<String> = ShellCommand::ALL.iter().map(|c| format!("{c:?}")).collect();
    assert_eq!(
        listed.len(),
        ShellCommand::ALL.len(),
        "a command is listed twice"
    );
}

#[test]
fn a_switch_is_named_for_what_choosing_it_does_now() {
    let mut h = harness(fresh());
    h.run();
    run(&mut h, "Snap to the grid");
    assert!(h.state().0.viewport.snapping.grid);
    run(&mut h, "Stop snapping to the grid");
    assert!(!h.state().0.viewport.snapping.grid);
    run(&mut h, "Hide unfinished parts");
    assert!(h.state().0.interface_look().hide_unfinished);
    run(&mut h, "Show unfinished parts");
    assert!(!h.state().0.interface_look().hide_unfinished);
}

#[test]
fn a_command_that_cant_be_done_says_why_and_does_nothing() {
    let mut w = fresh();
    let ctx = egui::Context::default();
    let mut actions = Vec::new();
    assert_eq!(
        ShellCommand::Pause.why_not(&w).as_deref(),
        Some("Press Play first")
    );
    assert_eq!(
        ShellCommand::Delete.why_not(&w).as_deref(),
        Some("Select an entity first")
    );
    ShellCommand::Step.run(&mut w, &ctx, &mut actions);
    assert!(actions.is_empty());
    w.play();
    assert_eq!(
        ShellCommand::SaveArea.why_not(&w).as_deref(),
        Some("Stop the game first")
    );
    ShellCommand::SaveArea.run(&mut w, &ctx, &mut actions);
    assert!(actions.is_empty(), "nothing is saved while the game runs");
    w.stop();
    ShellCommand::SaveArea.run(&mut w, &ctx, &mut actions);
    assert_eq!(actions, [WorkbenchAction::SaveArea]);
}

#[test]
fn a_hover_text_gives_the_name_the_shortcut_and_the_reason() {
    let w = fresh();
    let save = ShellCommand::SaveArea.hover_text(&w);
    assert!(save.starts_with("Save area (Ctrl+S)\n"), "{save}");
    let undo = ShellCommand::Undo.hover_text(&w);
    assert_eq!(undo, "Undo (Ctrl+Z)\nNothing to undo");
}

#[test]
fn each_tab_shows_what_its_workspace_showed() {
    let mut h = harness(fresh());
    h.run();
    assert_eq!(h.state().0.ribbon_tab(), RibbonTab::Home);
    for (tab, workspace, front) in [
        (RibbonTab::Animate, Workspace::Animate, None),
        (RibbonTab::Debug, Workspace::Code, Some(Panel::Profiler)),
        (RibbonTab::Ui, Workspace::Ui, None),
        (RibbonTab::Library, Workspace::Level, Some(Panel::Library)),
        (RibbonTab::Audio, Workspace::Level, Some(Panel::Mixer)),
        (RibbonTab::Home, Workspace::Level, None),
    ] {
        ribbon_tab(&mut h, tab);
        h.run();
        let w = &h.state().0;
        assert_eq!(w.ribbon_tab(), tab);
        assert_eq!(w.workspace(), workspace, "{tab:?}");
        if let Some(panel) = front {
            assert!(w.in_front.contains(&panel), "{tab:?} shows {panel:?}");
        }
    }
}

#[test]
fn a_tab_that_isnt_built_cant_be_shown_while_unfinished_parts_are_hidden() {
    let mut h = harness(fresh());
    ribbon_tab(&mut h, RibbonTab::Shading);
    assert_eq!(h.state().0.ribbon_tab(), RibbonTab::Shading);
    run(&mut h, "Hide unfinished parts");
    h.run();
    assert_eq!(
        h.state().0.ribbon_tab(),
        RibbonTab::Home,
        "the tab in front went with the unfinished parts"
    );
    ribbon_tab(&mut h, RibbonTab::Vfx);
    assert_eq!(h.state().0.ribbon_tab(), RibbonTab::Home);
}

#[test]
fn a_saved_arrangement_opens_on_the_tab_that_shows_it() {
    for workspace in Workspace::ALL {
        let mut w = fresh();
        let mut saved = w.workspaces();
        saved.current = workspace;
        saved.layouts.insert(workspace, workspace.standard());
        w.set_workspaces(saved);
        assert_eq!(w.ribbon_tab(), RibbonTab::of_workspace(workspace));
        assert_eq!(w.workspace(), workspace);
    }
}
