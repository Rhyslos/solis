//! M15's screens: the Build tab with Export for playtesting as its page
//! (round 3, `10-build-tab`), and the model viewer, an unbuilt screen
//! reached from a model's right-click in the Library
//! (`11-unbuilt-model-viewer`), there while it is in front.

use crate::Workbench;
use crate::interface_look::InterfaceLook;
use crate::library::LibraryRequest;
use crate::panel::Panel;
use crate::shell::{RibbonTab, ShellCommand as C};
use crate::test_support::*;
use egui_kittest::kittest::Queryable;

fn with_project() -> Workbench {
    let mut w = Workbench::new(info());
    w.set_project(Some(crate::ProjectInfo {
        name: "Harbour".into(),
        root: "/projects/harbour".into(),
        assets: "/projects/harbour/assets".into(),
    }));
    w
}

#[test]
fn build_s_page_is_export_for_playtesting_with_the_console_on_its_shelf() {
    let mut w = with_project();
    assert!(
        C::OpenExportFolder.why_not(&w).is_some(),
        "nothing exported yet"
    );
    // Export for playtesting… (the search, Home's Build) opens the tab.
    w.open_export();
    assert_eq!(w.ribbon_tab(), RibbonTab::Build);
    let mut h = harness(w);
    h.run();
    h.run();
    assert!(in_view(&h.state().0, Panel::Export));
    assert_eq!(
        h.state().0.shelf_tab(),
        Some(crate::shell::ShelfTab::Console)
    );
    for words in ["Export for playtesting", "Game's name", "Window"] {
        assert!(h.query_all_by_label(words).next().is_some(), "{words}");
    }
    // The last export's folder opens from the ribbon once there is one.
    h.state_mut()
        .0
        .export_finished(Ok("/home/me/exports/Harbour".into()));
    assert_eq!(C::OpenExportFolder.why_not(&h.state().0), None);
}

#[test]
fn the_model_viewer_comes_from_a_model_and_goes_when_left() {
    let mut w = with_project();
    let mut asked = Vec::new();
    w.library_request(
        LibraryRequest::ViewModel("props/crate.glb".into()),
        &mut asked,
    );
    assert_eq!(w.ribbon_tab(), RibbonTab::Model);
    assert_eq!(
        w.open.library_state.selected.as_deref(),
        Some("props/crate.glb")
    );
    let mut h = harness(w);
    h.run();
    h.run();
    assert!(h.query_by_label("Library tools \u{b7} Model").is_some());
    assert!(h.query_all_by_label("Model viewer").next().is_some());
    // Choosing another tab leaves it, and its tab goes.
    ribbon_tab(&mut h, RibbonTab::Library);
    h.run();
    assert!(h.state().0.shell.viewing_model.is_none());
    assert!(h.query_by_label("Library tools \u{b7} Model").is_none());
}

#[test]
fn with_unfinished_parts_hidden_the_model_viewer_isn_t_there() {
    let hiding = InterfaceLook {
        hide_unfinished: true,
        ..InterfaceLook::default()
    };
    let mut w = with_project().with_interface_look(hiding);
    crate::interface_look::use_look(hiding);
    let mut asked = Vec::new();
    w.library_request(
        LibraryRequest::ViewModel("props/crate.glb".into()),
        &mut asked,
    );
    let mut h = harness(w);
    crate::theme::restyle(&h.ctx, hiding);
    h.run();
    h.run();
    let gone = h.state().0.ribbon_tab() != RibbonTab::Model
        && h.query_by_label("Library tools \u{b7} Model").is_none();
    crate::interface_look::use_look(InterfaceLook::default());
    assert!(gone);
}
