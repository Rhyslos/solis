//! The File tab's own pages: what belongs to the open project and the
//! editor (decision 0118: the projects, Learn and Updates are the
//! launcher's, `crate::launcher`).

use super::*;
use crate::library::ProjectInfo;
use crate::test_support::*;
use egui_kittest::kittest::Queryable;

fn farm() -> ProjectInfo {
    ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }
}

fn asked(h: &egui_kittest::Harness<'static, State>, action: WorkbenchAction) -> bool {
    h.state().1.actions.contains(&action)
}

/// File keeps what belongs to an open project (Save all, Project
/// settings, Preferences, Close project) with About and Exit; the
/// projects, Learn and Updates went to the launcher.
#[test]
fn file_keeps_the_project_s_items_and_close_project_goes_back() {
    let mut w = Workbench::new(info());
    w.set_project(Some(farm()));
    w.show_file_page(Page::ProjectSettings);
    let mut h = harness(w);
    h.run();
    for label in [
        "Back to editor",
        "Save all",
        "Project settings",
        "Preferences",
        "About",
        "Close project",
        "Exit",
    ] {
        assert!(h.query_all_by_label(label).next().is_some(), "{label}");
    }
    for gone in [
        "Recent projects",
        "New project",
        "Open…",
        "Learn",
        "Updates",
    ] {
        assert!(h.query_by_label(gone).is_none(), "{gone}");
    }
    crate::test_support::file_item(&mut h, "Close project");
    assert!(asked(&h, WorkbenchAction::CloseProject));
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(!h.state().0.file_page_open(), "Esc goes back to the editor");
}

#[test]
fn a_project_s_build_is_compared_with_this_one() {
    assert_eq!(BuildMark::of(None, Some(5)), None);
    assert_eq!(BuildMark::of(Some(5), Some(5)), Some(BuildMark::ThisOne(5)));
    assert_eq!(BuildMark::of(Some(4), Some(5)), Some(BuildMark::Older(4)));
    assert_eq!(BuildMark::of(Some(6), Some(5)), Some(BuildMark::Newer(6)));
    assert_eq!(BuildMark::of(Some(6), None), Some(BuildMark::Some(6)));
}

#[test]
fn renaming_the_open_project_changes_its_name_and_nothing_else() {
    let mut w = Workbench::new(info());
    w.set_project(Some(farm()));
    let revision = w.open.document.revision();
    w.rename_open_project("Farm days");
    assert_eq!(w.project().unwrap().name, "Farm days");
    assert_eq!(w.open.document.revision(), revision);
    assert!(w.title().contains("Farm days"));
}

#[test]
fn about_is_a_page_with_the_name_the_build_and_the_licences() {
    let mut w = Workbench::new(info());
    w.set_this_build(Some(57));
    w.set_project(Some(farm()));
    w.show_file_page(Page::ProjectSettings);
    let mut h = harness(w);
    h.run();
    crate::test_support::file_item(&mut h, "About");
    assert_eq!(h.state().0.file_pages.page, Page::About);
    for label in ["Test Engine", "A free, open-source 3D game engine."] {
        assert!(h.query_all_by_label(label).next().is_some(), "{label}");
    }
    assert!(h.query_by_label_contains("build 57").is_some());
    h.get_by_label("Open LICENSE").click();
    h.run();
    assert!(asked(
        &h,
        WorkbenchAction::OpenProgramFile(ProgramFile::Licence)
    ));
    h.get_by_label("Open the user guide").click();
    h.run();
    assert!(asked(&h, WorkbenchAction::OpenGuide(None)));
    h.get_by_label("Open the notices").click();
    h.run();
    assert!(asked(
        &h,
        WorkbenchAction::OpenProgramFile(ProgramFile::Notices)
    ));
    // Updates are the launcher's; About asks GitHub straight away.
    h.get_by_label("Check for updates").click();
    h.run();
    assert!(asked(&h, WorkbenchAction::CheckForUpdates));
}
