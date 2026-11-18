//! A project's kind on the frame (decision 0118, R6; board 08): every
//! ribbon walked in a 2D and a 3D project offers nothing of the other
//! kind, the tab bar leaves the other kind's tabs out, a change of kind
//! in Project settings applies at once and back, something of the
//! hidden kind already in the level stays listed with its note, and the
//! note on Home's ribbon comes and goes.

use super::ribbon::{Control, Group, filtered};
use super::{RibbonTab, ShellCommand, ribbons};
use crate::kinds::{self, PartKind};
use crate::launcher::ProjectKind;
use crate::test_support::{harness, info};
use crate::unbuilt::{Entry, UnbuiltId};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;

/// An editor with a project open, of `kind`.
fn project(kind: ProjectKind) -> Workbench {
    let mut w = Workbench::new(info());
    w.set_project(Some(crate::library::ProjectInfo {
        name: "Farm Days".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm".into(),
    }));
    w.apply_project_kind(kind);
    w
}

/// Every command and unfinished part on `groups`, lists' items included.
fn parts(groups: &[Group]) -> (Vec<ShellCommand>, Vec<UnbuiltId>) {
    let (mut commands, mut unbuilt) = (Vec::new(), Vec::new());
    for (_, entries) in groups.iter().flat_map(|g| &g.slots) {
        for entry in entries {
            match entry {
                Entry::Unbuilt(id) => unbuilt.push(*id),
                Entry::Built(Control::Do { command, .. } | Control::Switch { command, .. }) => {
                    commands.push(*command);
                }
                Entry::Built(Control::List { command, items, .. }) => {
                    commands.extend(*command);
                    for item in items {
                        match item {
                            Entry::Built((_, command)) => commands.push(*command),
                            Entry::Unbuilt(id) => unbuilt.push(*id),
                        }
                    }
                }
                Entry::Built(Control::Fact(_)) => {}
            }
        }
    }
    (commands, unbuilt)
}

/// Every ribbon (each tab's, the contextual ones, and Home while placing)
/// as a project of `kind` draws it.
fn every_ribbon(kind: ProjectKind) -> Vec<(String, Vec<Group>)> {
    let mut ribbons = Vec::new();
    for tab in RibbonTab::ALL
        .into_iter()
        .chain(RibbonTab::CONTEXT)
        .filter(|t| *t != RibbonTab::File)
    {
        let w = project(kind);
        let groups = filtered(ribbons::of(tab, &w), kind);
        ribbons.push((format!("{tab:?}"), groups));
    }
    let mut placing = project(kind);
    placing.set_placing(true);
    let groups = filtered(ribbons::of(RibbonTab::Home, &placing), kind);
    ribbons.push(("Home, placing".into(), groups));
    ribbons
}

#[test]
fn no_ribbon_offers_what_the_project_s_kind_hides() {
    for (kind, hidden) in [
        (ProjectKind::TwoD, PartKind::ThreeD),
        (ProjectKind::ThreeD, PartKind::TwoD),
    ] {
        for (tab, groups) in every_ribbon(kind) {
            let (commands, unbuilt) = parts(&groups);
            for command in commands {
                assert_ne!(kinds::command(command), hidden, "{command:?} on {tab}");
            }
            for id in unbuilt {
                assert_ne!(kinds::unbuilt(id), hidden, "{id:?} on {tab}");
            }
            assert!(
                groups.iter().all(|g| !g.slots.is_empty()),
                "{tab} keeps an emptied group"
            );
        }
    }
}

#[test]
fn both_s_ribbons_are_as_they_were() {
    for tab in RibbonTab::ALL
        .into_iter()
        .chain(RibbonTab::CONTEXT)
        .filter(|t| *t != RibbonTab::File)
    {
        let w = project(ProjectKind::Both);
        let all = ribbons::of(tab, &w);
        assert_eq!(filtered(all.clone(), ProjectKind::Both), all, "{tab:?}");
    }
}

#[test]
fn a_2d_project_s_home_is_board_08_s() {
    let w = project(ProjectKind::TwoD);
    let groups = filtered(ribbons::of(RibbonTab::Home, &w), ProjectKind::TwoD);
    let labels: Vec<_> = groups.iter().map(|g| g.label).collect();
    assert_eq!(
        labels,
        [Some("Edit"), Some("Create"), Some("Paint tiles"), None]
    );
    let (commands, unbuilt) = parts(&groups[1..2]);
    use ShellCommand as C;
    for made in [C::AddEntity, C::AddSprite, C::NewTileMap, C::AddCamera] {
        assert!(commands.contains(&made), "Create has {made:?}");
    }
    assert_eq!(unbuilt, [UnbuiltId::HomeCreateTrigger], "the board's Zone");
    let (snaps, _) = parts(&groups[3..]);
    assert_eq!(snaps, [C::SnapToTiles, C::PixelPerfect]);
    // A 3D project's Home has no Tile map, and keeps Lighting.
    let w = project(ProjectKind::ThreeD);
    let groups = filtered(ribbons::of(RibbonTab::Home, &w), ProjectKind::ThreeD);
    let (commands, _) = parts(&groups);
    assert!(!commands.contains(&C::NewTileMap));
    assert!(commands.contains(&C::ImportModel));
    assert!(groups.iter().any(|g| g.label == Some("Lighting")));
}

#[test]
fn the_tab_bar_leaves_out_the_other_kind_s_tabs() {
    for (kind, shading) in [
        (ProjectKind::TwoD, false),
        (ProjectKind::ThreeD, true),
        (ProjectKind::Both, true),
    ] {
        let mut w = project(kind);
        // The note names the Shading tab too.
        w.set_hints_on(false);
        let mut h = harness(w);
        h.run();
        h.run();
        assert_eq!(h.query_by_label("Shading").is_some(), shading, "{kind:?}");
        // A hidden tab can't be brought to the front.
        h.state_mut().0.show_tab(RibbonTab::Shading);
        assert_eq!(
            h.state().0.ribbon_tab() == RibbonTab::Shading,
            shading,
            "{kind:?}"
        );
    }
}

#[test]
fn changing_the_kind_in_project_settings_applies_at_once_and_back() {
    let mut w = project(ProjectKind::ThreeD);
    w.set_hints_on(false);
    w.show_settings(crate::file_pages::Tab::Project);
    let mut h = harness(w);
    h.run();
    h.get_by_role_and_label(Role::RadioButton, "2D").click();
    h.run();
    assert_eq!(h.state().0.project_kind(), ProjectKind::TwoD);
    let asked = WorkbenchAction::SetProjectKind(ProjectKind::TwoD);
    assert!(h.state().1.actions.contains(&asked));
    // What 2D hides is said under the row, and gone on Home at once.
    assert!(
        h.query_by_label_contains("2D hides what only 3D uses")
            .is_some()
    );
    h.state_mut().0.show_tab(RibbonTab::Home);
    h.run();
    h.run();
    assert!(h.query_by_label("Fill tool").is_some(), "the tile painter");
    assert!(
        h.query_by_label("New tile map").is_some(),
        "Create › Tile map"
    );
    assert!(
        h.query_by_label("Import model…").is_none(),
        "no Model in 2D"
    );
    assert!(h.query_by_label("Shading").is_none(), "no Shading tab");
    // And back: Both shows everything again.
    h.state_mut()
        .0
        .show_settings(crate::file_pages::Tab::Project);
    h.run();
    h.get_by_role_and_label(Role::RadioButton, "Both").click();
    h.run();
    assert_eq!(h.state().0.project_kind(), ProjectKind::Both);
    h.state_mut().0.show_tab(RibbonTab::Home);
    h.run();
    h.run();
    assert!(h.query_by_label("Import model…").is_some(), "Model is back");
    assert!(h.query_by_label("Shading").is_some(), "Shading is back");
}

#[test]
fn something_3d_in_a_2d_project_stays_listed_with_its_note() {
    let mut w = project(ProjectKind::Both);
    let model = w.add_model(crate::test_support::crate_model(), &[], 0.1, &[]);
    let before = w.open.document.level().clone();
    w.apply_project_kind(ProjectKind::TwoD);
    w.select(model);
    let mut h = harness(w);
    h.run();
    h.run();
    let name = h
        .state()
        .0
        .open
        .document
        .level()
        .get(model)
        .unwrap()
        .name
        .clone();
    assert!(
        h.query_all_by_label_contains(&name).count() >= 2,
        "still listed, and shown in Properties"
    );
    let entity = h
        .state()
        .0
        .open
        .document
        .level()
        .get(model)
        .unwrap()
        .clone();
    assert_eq!(
        kinds::entity(&entity),
        PartKind::ThreeD,
        "its row says 3D only"
    );
    assert!(
        h.query_by_label("3D only: switch the project to Both to edit it")
            .is_some(),
        "Properties' note"
    );
    // Nothing deleted, nothing changed.
    assert_eq!(*h.state().0.open.document.level(), before);
}

#[test]
fn the_note_shows_the_first_times_and_goes_with_its_x() {
    let mut w = project(ProjectKind::Both);
    w.set_project_kind(ProjectKind::TwoD);
    let mut h = harness(w);
    h.run();
    h.run();
    let note = "No Shading tab and no Lighting group: they are 3D only";
    assert!(h.query_by_label(note).is_some(), "the first time");
    h.get_by_label("Don't show this note again").click();
    h.run();
    h.run();
    assert!(h.query_by_label(note).is_none(), "gone");
    assert_eq!(
        h.state().0.interface_look().kind_note_seen,
        crate::interface_look::KIND_NOTE_GONE
    );
    // Past the first few openings it doesn't come back either.
    let mut w = project(ProjectKind::Both);
    for _ in 0..=crate::interface_look::KIND_NOTE_TIMES {
        w.set_project_kind(ProjectKind::TwoD);
    }
    assert_eq!(w.kind_note(), None);
    // Nor in Both, which hides nothing, nor with hints off.
    let mut w = project(ProjectKind::Both);
    w.set_project_kind(ProjectKind::ThreeD);
    assert!(w.kind_note().is_some());
    w.set_hints_on(false);
    assert_eq!(w.kind_note(), None);
}

#[test]
fn the_status_line_says_the_project_and_its_kind() {
    let w = project(ProjectKind::TwoD);
    assert_eq!(w.status().level, "Farm Days \u{b7} 2D");
    let w = project(ProjectKind::ThreeD);
    assert_eq!(w.status().level, "Farm Days \u{b7} 3D");
    // Both says the level, as it did.
    let w = project(ProjectKind::Both);
    assert!(w.status().level.ends_with(" \u{b7} 3D"));
    assert!(!w.status().level.starts_with("Farm Days"));
}
