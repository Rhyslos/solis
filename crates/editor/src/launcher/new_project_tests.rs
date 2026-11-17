//! New project (board 02): the name makes the folder, which must be free
//! or empty; Create in is the app's, remembered; the kind; the starters
//! greyed by kind; Create asks the app.

use super::*;
use crate::launcher::new_project::NewProject;
use egui_kittest::kittest::NodeT as _;

const CREATE_IN: &str = "/home/sam/Documents/Test Engine projects";

fn open_dialog() -> egui_kittest::Harness<'static, State> {
    let mut h = harness(launcher_with(Vec::new()));
    h.run();
    h.get_by_label("New project").click();
    h.run();
    h.run();
    h
}

fn dialog<'a>(h: &'a egui_kittest::Harness<'static, State>) -> &'a NewProject {
    h.state().0.launcher.new_project.as_ref().expect("open")
}

/// Clicks the kind's card (the filter behind the dialog has words of the
/// same name).
fn kind_card(h: &mut egui_kittest::Harness<'static, State>, name: &str) {
    let card = h
        .get_all_by_label(name)
        .max_by(|a, b| a.rect().height().total_cmp(&b.rect().height()))
        .expect("the kind's card")
        .rect()
        .center();
    click(h, card, egui::Modifiers::NONE);
}

fn type_name(h: &mut egui_kittest::Harness<'static, State>, name: &str) {
    h.state_mut().0.launcher.new_project.as_mut().unwrap().name = name.into();
    h.run();
}

fn looked_at(h: &egui_kittest::Harness<'static, State>) -> Vec<std::path::PathBuf> {
    h.state()
        .1
        .actions
        .iter()
        .filter_map(|a| match a {
            WorkbenchAction::LookAtFolder(f) => Some(f.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_name_makes_the_folder_and_create_asks_for_it() {
    let mut h = open_dialog();
    assert_eq!(dialog(&h).create_in, std::path::Path::new(CREATE_IN));
    assert!(
        h.query_by_label("Remembered for your next project.")
            .is_some()
    );
    type_name(&mut h, "Farm Days");
    let folder = std::path::Path::new(CREATE_IN).join("Farm Days");
    assert_eq!(
        dialog(&h).folder(),
        Some(folder.clone()),
        "the folder it makes"
    );
    h.run();
    assert_eq!(
        looked_at(&h),
        std::slice::from_ref(&folder),
        "asked about once"
    );
    assert!(!dialog(&h).ready(), "not until the app has looked");
    h.state_mut()
        .0
        .set_folder_state(folder.clone(), FolderState::Free);
    h.run();
    assert!(dialog(&h).ready());
    kind_card(&mut h, "2D");
    h.get_by_label("Create").click();
    h.run();
    let plan = NewProjectPlan {
        folder: folder.clone(),
        name: "Farm Days".into(),
        kind: ProjectKind::TwoD,
        starter: Starter::Empty,
        create_in: CREATE_IN.into(),
    };
    assert!(asked(&h, &WorkbenchAction::CreateProject(plan)));
    // The app couldn't: why shows, and the dialog stays.
    h.state_mut().0.new_project_failed("The disk is full.");
    h.run();
    assert!(h.query_by_label("The disk is full.").is_some());
    // An answer about a folder no longer named changes nothing.
    type_name(&mut h, "Farm Days 2");
    h.state_mut().0.set_folder_state(folder, FolderState::Taken);
    h.run();
    assert!(dialog(&h).problem().is_none());
}

#[test]
fn a_folder_that_isn_t_empty_is_never_used() {
    let mut h = open_dialog();
    type_name(&mut h, "Cave Run");
    let folder = std::path::Path::new(CREATE_IN).join("Cave Run");
    h.state_mut()
        .0
        .set_folder_state(folder.clone(), FolderState::Taken);
    h.run();
    assert!(!dialog(&h).ready());
    assert!(
        h.query_by_label_contains("already exists and isn't empty")
            .is_some()
    );
    h.get_by_label("Create").click();
    h.run();
    let created = h
        .state()
        .1
        .actions
        .iter()
        .any(|a| matches!(a, WorkbenchAction::CreateProject(_)));
    assert!(!created, "refused");
    // An empty one is fine.
    h.state_mut().0.set_folder_state(folder, FolderState::Empty);
    h.run();
    assert!(dialog(&h).ready());
}

#[test]
fn bad_names_are_refused_with_a_plain_reason() {
    let mut h = open_dialog();
    for (name, why) in [
        ("CON", "keeps CON"),
        ("Farm: Days", "can't hold :"),
        ("Ends.", "dot at the end"),
        (&"x".repeat(101), "at most 100"),
    ] {
        type_name(&mut h, name);
        assert!(h.query_by_label_contains(why).is_some(), "{name}: {why}");
        assert!(!dialog(&h).ready(), "{name}");
    }
    assert!(
        looked_at(&h).is_empty(),
        "a bad name's folder is never looked at"
    );
    // An empty name says nothing until something is typed, but can't be
    // made.
    type_name(&mut h, "");
    assert!(
        dialog(&h)
            .problem()
            .unwrap()
            .contains("Give the project a name")
    );
    assert!(!dialog(&h).ready());
}

#[test]
fn create_in_comes_from_the_app_and_browse_asks_for_another() {
    let mut h = open_dialog();
    h.get_by_label("Browse…").click();
    h.run();
    assert!(asked(&h, &WorkbenchAction::ChooseCreateIn));
    h.state_mut().0.set_create_in("/games".into());
    h.run();
    assert_eq!(dialog(&h).create_in, std::path::Path::new("/games"));
    type_name(&mut h, "Tiny Kart");
    assert_eq!(looked_at(&h), [std::path::Path::new("/games/Tiny Kart")]);
    // With nowhere chosen, it says to choose.
    let mut none = NewProject::new(std::path::PathBuf::new());
    none.name = "Kart".into();
    assert!(none.problem().unwrap().contains("Browse"));
    // The next dialog starts in the folder the app remembered.
    h.get_by_label("Cancel").click();
    h.run();
    assert!(h.state().0.launcher.new_project.is_none());
    h.get_by_label("New project").click();
    h.run();
    assert_eq!(dialog(&h).create_in, std::path::Path::new("/games"));
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.state().0.launcher.new_project.is_none(), "Esc closes it");
}

#[test]
fn starters_that_don_t_suit_the_kind_are_greyed() {
    for starter in Starter::ALL {
        assert!(starter.suits(ProjectKind::Both), "{starter:?}");
        assert!(starter.suits(ProjectKind::ThreeD), "{starter:?}");
        assert_eq!(
            starter.suits(ProjectKind::TwoD),
            starter == Starter::Empty,
            "{starter:?}"
        );
    }
    let mut h = open_dialog();
    h.get_by_label("Start from Playground").click();
    h.run();
    assert_eq!(dialog(&h).starter, Starter::Playground);
    // 2D can't start from a 3D example: it goes back to Empty, and the
    // examples are greyed (and can't be picked).
    kind_card(&mut h, "2D");
    assert_eq!(dialog(&h).starter, Starter::Empty);
    let playground = h.get_by_label("Start from Playground");
    assert!(playground.accesskit_node().is_disabled(), "greyed");
    h.get_by_label("Start from Playground").click();
    h.run();
    assert_eq!(dialog(&h).starter, Starter::Empty);
    kind_card(&mut h, "Both");
    h.get_by_label("Start from Benchmark").click();
    h.run();
    assert_eq!(dialog(&h).starter, Starter::Benchmark);
}

#[test]
fn project_settings_changes_the_kind() {
    let mut w = Workbench::new(info());
    w.set_project(Some(farm()));
    w.set_project_kind(ProjectKind::ThreeD);
    w.show_settings(crate::file_pages::Tab::Project);
    let mut h = harness(w);
    h.run();
    assert!(h.query_by_label("Kind").is_some());
    h.get_by_role_and_label(egui::accesskit::Role::RadioButton, "2D")
        .click();
    h.run();
    assert_eq!(h.state().0.project_kind(), ProjectKind::TwoD);
    assert!(asked(
        &h,
        &WorkbenchAction::SetProjectKind(ProjectKind::TwoD)
    ));
    // Another project starts as its own file says (both, until told).
    h.state_mut().0.set_project(None);
    assert_eq!(h.state().0.project_kind(), ProjectKind::Both);
}
