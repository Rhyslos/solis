//! The launcher (decision 0118): its projects (search, filter by kind,
//! order, the cards and their menus), Learn and Updates, opening and
//! closing, and New project.

use super::*;
use crate::file_pages::ProgramFile;
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

fn known(name: &str, root: &str, last_opened: Option<u64>, kind: ProjectKind) -> KnownProject {
    KnownProject {
        root: root.into(),
        name: name.into(),
        problem: None,
        pinned: false,
        last_opened,
        saved_by: None,
        kind,
    }
}

fn some_projects() -> Vec<KnownProject> {
    let mut gone = known("Old jam", "/jam/old", Some(10), ProjectKind::TwoD);
    gone.problem = Some("Folder not found".into());
    let mut pinned = known("Zebra farm", "/games/zebra", Some(50), ProjectKind::TwoD);
    pinned.pinned = true;
    pinned.saved_by = Some(57);
    vec![
        known(
            "Playground",
            "/games/playground",
            Some(100),
            ProjectKind::ThreeD,
        ),
        pinned,
        known("Benchmark", "/games/bench", None, ProjectKind::Both),
        gone,
    ]
}

/// The launcher as the program starts, with `projects`.
fn launcher_with(projects: Vec<KnownProject>) -> Workbench {
    let mut w = Workbench::new(info());
    w.set_known_projects(projects);
    w.set_this_build(Some(57));
    w.set_create_in("/home/sam/Documents/Test Engine projects".into());
    w.open_launcher();
    w
}

fn asked(h: &egui_kittest::Harness<'static, State>, action: &WorkbenchAction) -> bool {
    h.state().1.actions.contains(action)
}

fn names(projects: &[KnownProject], order: &[usize]) -> Vec<String> {
    order.iter().map(|&i| projects[i].name.clone()).collect()
}

#[test]
fn pinned_projects_come_first_then_the_order_and_search_and_kind_narrow() {
    let projects = some_projects();
    let list = |search, filter, sort| names(&projects, &shown(&projects, search, filter, sort));
    assert_eq!(
        list("", Filter::All, Sort::LastOpened),
        ["Zebra farm", "Playground", "Old jam", "Benchmark"],
        "pinned first; never opened last"
    );
    assert_eq!(
        list("", Filter::All, Sort::Name),
        ["Zebra farm", "Benchmark", "Old jam", "Playground"]
    );
    // Every word, in the name or the folder, any case.
    assert_eq!(list("GAMES  pla", Filter::All, Sort::Name), ["Playground"]);
    assert!(list("nothing like it", Filter::All, Sort::Name).is_empty());
    // The filter keeps one kind; with the search, both must hold.
    let two_d = Filter::Kind(ProjectKind::TwoD);
    assert_eq!(list("", two_d, Sort::Name), ["Zebra farm", "Old jam"]);
    assert_eq!(list("zebra", two_d, Sort::Name), ["Zebra farm"]);
    assert_eq!(list("bench", two_d, Sort::Name), Vec::<String>::new());
    let both = Filter::Kind(ProjectKind::Both);
    assert_eq!(list("", both, Sort::Name), ["Benchmark"]);
    assert!(shown(&[], "", Filter::All, Sort::Name).is_empty());
    // Every filter has its word, as drawn.
    let words: Vec<&str> = Filter::choices().iter().map(|(_, w)| *w).collect();
    assert_eq!(words, ["All", "2D", "3D", "2D + 3D"]);
}

#[test]
fn how_long_ago_is_said_in_words() {
    let (minute, hour, day) = (60_u64, 3600_u64, 86_400_u64);
    let now = 1_000 * day;
    for (then, said) in [
        (now, "Just now"),
        (now + 50, "Just now"),
        (now - minute, "1 minute ago"),
        (now - 5 * minute, "5 minutes ago"),
        (now - 2 * hour, "2 hours ago"),
        (now - day - hour, "Yesterday"),
        (now - 3 * day, "3 days ago"),
        (now - 15 * day, "2 weeks ago"),
        (now - 90 * day, "3 months ago"),
        (now - 800 * day, "2 years ago"),
    ] {
        assert_eq!(ago(then, now), said, "{}", now.abs_diff(then));
    }
}

#[test]
fn all_four_arrows_walk_the_cards() {
    use projects::{Walk, cards_across, walk_to};
    assert_eq!(walk_to(None, 7, 3, Walk::Down), Some(0), "the first");
    assert_eq!(walk_to(None, 0, 3, Walk::Down), None, "no cards");
    assert_eq!(walk_to(Some(0), 7, 3, Walk::Right), Some(1));
    assert_eq!(walk_to(Some(1), 7, 3, Walk::Left), Some(0));
    assert_eq!(walk_to(Some(0), 7, 3, Walk::Left), Some(0), "the edge");
    assert_eq!(walk_to(Some(1), 7, 3, Walk::Down), Some(4));
    assert_eq!(walk_to(Some(5), 7, 3, Walk::Down), Some(6), "to the last");
    assert_eq!(walk_to(Some(4), 7, 3, Walk::Up), Some(1));
    assert_eq!(walk_to(Some(1), 7, 3, Walk::Up), Some(1), "the top row");
    assert_eq!(cards_across(240.0), 1);
    // The launcher's own size fits three, as drawn.
    let page = crate::tokens::launcher::WINDOW[0]
        - crate::tokens::launcher::SIDE_W
        - 2.0 * crate::tokens::launcher::PAGE_PAD[1];
    assert_eq!(cards_across(page), 3);
    assert_eq!(cards_across(0.0), 1, "never none");
}

#[test]
fn the_program_starts_on_the_launcher_and_comes_back_to_it() {
    let mut w = Workbench::new(info());
    assert!(!w.launcher_shown(), "only when the app asks");
    w.open_launcher();
    assert!(w.launcher_shown());
    assert_eq!(w.title(), "Test Engine", "the program's own window");
    assert_eq!(w.launcher.page, Page::Projects);
    assert!(
        w.open.outbox.contains(&WorkbenchAction::ListProjects),
        "the list is read afresh"
    );
    // A project opening closes it: the editor fills the window.
    w.set_project(Some(farm()));
    assert!(!w.launcher_shown());
    assert!(!w.file_page_open());
    // File › Close project asks the app, which closes it and comes back.
    let mut actions = Vec::new();
    crate::shell::ShellCommand::CloseProject.run(&mut w, &egui::Context::default(), &mut actions);
    assert_eq!(actions, [WorkbenchAction::CloseProject]);
    w.set_project(None);
    w.open_launcher();
    assert!(w.launcher_shown());
}

#[test]
fn closing_the_project_asks_about_unsaved_work_first() {
    let mut w = Workbench::new(info());
    w.set_project(Some(farm()));
    w.add_entity();
    assert!(w.open.document.is_dirty());
    let mut actions = Vec::new();
    w.request(WorkbenchAction::CloseProject, &mut actions);
    assert!(actions.is_empty(), "asked first");
    assert_eq!(w.open.unsaved_prompt, Some(WorkbenchAction::CloseProject));
}

#[test]
fn the_launcher_lists_the_cards_and_does_what_they_offer() {
    let mut h = harness(launcher_with(some_projects()));
    h.run();
    for label in [
        "Projects",
        "Learn",
        "Updates",
        "Preferences",
        "User guide",
        "Playground",
        "Zebra farm",
        "Remove Old jam from the list",
        "Search projects",
        "Open folder…",
        "New project",
        "2D + 3D",
    ] {
        assert!(h.query_all_by_label(label).next().is_some(), "{label}");
    }
    assert!(
        h.query_by_label("Back to editor").is_none(),
        "not the editor"
    );
    h.get_by_label("Remove Old jam from the list").click();
    h.run();
    assert_eq!(h.state().0.known_projects.len(), 3);
    assert!(asked(
        &h,
        &WorkbenchAction::ForgetProject("/jam/old".into())
    ));
    // A click picks a card; Enter opens it, as a double-click does.
    h.get_by_label("Benchmark").click();
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(asked(
        &h,
        &WorkbenchAction::OpenRecent("/games/bench".into())
    ));
    h.state_mut().1.actions.clear();
    let card = h.get_by_label("Playground").rect().center();
    double_click(&mut h, card);
    h.run();
    assert!(asked(
        &h,
        &WorkbenchAction::OpenRecent("/games/playground".into())
    ));
    // A picked card's Open opens it too.
    h.get_by_label("Open Playground").click();
    h.run();
    // The filter keeps one kind.
    h.get_by_label("3D").click();
    h.run();
    assert!(h.query_by_label("Zebra farm").is_none());
    assert!(h.query_by_label("Playground").is_some());
    h.get_by_label("All").click();
    h.run();
    assert!(h.query_by_label("Zebra farm").is_some());
    h.get_by_label("Open folder…").click();
    h.run();
    assert!(asked(&h, &WorkbenchAction::OpenProject));
    h.get_by_label("User guide").click();
    h.run();
    assert!(asked(&h, &WorkbenchAction::OpenGuide(None)));
    // Preferences show inside the launcher.
    h.get_by_label("Preferences").click();
    h.run();
    assert_eq!(h.state().0.launcher.page, Page::Preferences);
    assert!(
        h.query_by_label_contains("Saved for you on this computer")
            .is_some()
    );
}

#[test]
fn a_card_s_menu_opens_shows_and_removes_and_never_touches_files() {
    let mut h = harness(launcher_with(some_projects()));
    h.run();
    h.get_by_label("Locate Old jam").click();
    h.run();
    assert!(asked(
        &h,
        &WorkbenchAction::LocateProject("/jam/old".into())
    ));
    let root = std::path::PathBuf::from("/games/playground");
    for (item, action) in [
        ("Open", WorkbenchAction::OpenRecent(root.clone())),
        (
            "Show in Explorer",
            WorkbenchAction::ShowProject(root.clone()),
        ),
        ("Make a copy…", WorkbenchAction::CopyProject(root.clone())),
        (
            "Pin to the top",
            WorkbenchAction::PinProject(root.clone(), true),
        ),
    ] {
        h.get_by_label("More for Playground").click();
        h.run();
        h.get_by_label(item).click();
        h.run();
        assert!(asked(&h, &action), "{item}");
    }
    assert!(h.state().0.known_projects[0].pinned, "pinned at once");
    // The same menu on a right-click.
    h.get_by_label("Playground").click_secondary();
    h.run();
    h.get_by_label("Rename…").click();
    h.run();
    h.run();
    let name = h
        .state()
        .0
        .launcher
        .renaming
        .as_ref()
        .map(|r| r.name.clone());
    assert_eq!(name.as_deref(), Some("Playground"));
    h.state_mut().0.launcher.renaming.as_mut().unwrap().name = " Play park ".into();
    h.run();
    h.get_by_label("Rename").click();
    h.run();
    assert!(asked(
        &h,
        &WorkbenchAction::RenameProject(root.clone(), "Play park".into())
    ));
    // Remove from list only forgets it: the app is asked for nothing
    // that touches a file.
    h.state_mut().1.actions.clear();
    h.get_by_label("More for Playground").click();
    h.run();
    h.get_by_label("Remove from list").click();
    h.run();
    assert_eq!(
        h.state().1.actions,
        [WorkbenchAction::ForgetProject(root)],
        "nothing else"
    );
    assert!(h.query_by_label("Playground").is_none());
    // What the app did is said until closed; a mistake from the app too.
    h.state_mut()
        .0
        .projects_notice("Found 2: 1 added to the list.");
    h.run();
    assert!(h.query_by_label("Found 2: 1 added to the list.").is_some());
    h.get_by_label("Close the note").click();
    h.run();
    assert!(h.state().0.launcher.notice.is_none());
}

#[test]
fn the_updates_page_says_what_s_new_and_does_the_next_step() {
    use crate::updates::{ReleaseNotes, UpdateNotice};
    let mut w = launcher_with(Vec::new());
    w.set_update(UpdateNotice::Available(60));
    w.set_release_notes(Some(ReleaseNotes {
        build: 60,
        notes: "Build 60 of Test Engine:\n\n> The new pages".into(),
        published: Some("2026-11-05".into()),
    }));
    let mut h = harness(w);
    h.run();
    h.get_by_label("Updates").click();
    h.run();
    assert_eq!(h.state().0.launcher.page, Page::Updates);
    for label in [
        "What's new in build 60",
        "The new pages",
        "Published 2026-11-05",
    ] {
        assert!(
            h.query_all_by_label_contains(label).next().is_some(),
            "{label}"
        );
    }
    h.get_by_label("Download").click();
    h.run();
    assert!(asked(&h, &WorkbenchAction::DownloadUpdate));
    h.get_by_label("Check now").click();
    h.run();
    assert!(asked(&h, &WorkbenchAction::CheckForUpdates));
    h.get_by_label("Open the changes list").click();
    h.run();
    assert!(asked(
        &h,
        &WorkbenchAction::OpenProgramFile(ProgramFile::Changes)
    ));
    h.get_by_label("Not now").click();
    h.run();
    assert!(h.query_by_label("Download").is_none(), "put away");
    h.state_mut().0.set_update(UpdateNotice::Ready(60));
    h.run();
    h.get_by_label("Restart to update").click();
    h.run();
    assert!(h.state().0.restarting());
    assert!(asked(&h, &WorkbenchAction::Quit));
    // The question isn't asked over this page, which has its buttons.
    h.state_mut().0.offer_update(crate::splash::UpdateOffer {
        latest: 61,
        current: Some(57),
        installed: true,
    });
    h.run();
    assert!(h.state().0.offer.is_none());
}

#[test]
fn the_learn_page_offers_the_examples_and_the_guide() {
    let mut h = harness(launcher_with(Vec::new()));
    h.run();
    assert!(h.query_by_label_contains("No projects yet").is_some());
    h.get_by_label("Learn").click();
    h.run();
    for label in [
        "Make the Playground",
        "Make the Turret sample",
        "Make the UI sample",
        "Make the Benchmark",
        "Open the user guide",
    ] {
        assert!(h.query_all_by_label(label).next().is_some(), "{label}");
    }
    h.get_by_label("Make the Playground").click();
    h.run();
    assert!(asked(&h, &WorkbenchAction::MakePlayground));
    // The sample area needs an open project: it's in the editor's search.
    assert!(h.query_by_label("Add the sample area").is_none());
}

/// A picked card's border and the notice are the accent in use mixed
/// into a neutral surface, never a colour of their own (the review,
/// BUG-29).
#[test]
fn the_launcher_s_tints_are_the_accent_in_use_mixed_into_the_surface() {
    use crate::theme::{self, Accent, InterfaceLook};
    use crate::tokens;
    for chosen in Accent::ALL {
        let look = InterfaceLook {
            accent: chosen,
            ..InterfaceLook::default()
        };
        let mut w = launcher_with(some_projects()).with_interface_look(look);
        w.projects_notice("Made a copy.");
        let mut h = harness(w);
        theme::restyle(&h.ctx, look);
        h.run();
        h.get_by_label("Playground").click();
        h.run();
        let drawn = rects_drawn(&h);
        let fills = |colour| drawn.iter().any(|(_, fill, _)| *fill == colour);
        let borders = |colour| drawn.iter().any(|(_, _, border)| *border == colour);
        assert!(fills(tokens::changes_pill(chosen)), "{}", chosen.name());
        assert!(
            borders(tokens::open_card_border(chosen)),
            "{}",
            chosen.name()
        );
        for other in Accent::ALL.into_iter().filter(|a| *a != chosen) {
            assert!(!fills(tokens::changes_pill(other)), "{}", chosen.name());
        }
    }
    crate::interface_look::use_look(InterfaceLook::default());
}

/// N-05: Enter to keep a project's new name renames it and opens nothing.
#[test]
fn enter_while_a_name_is_typed_never_opens_the_picked_project() {
    let mut h = harness(launcher_with(some_projects()));
    h.run();
    h.get_by_label("Benchmark").click();
    h.run();
    h.state_mut().0.launcher.renaming = Some(Renaming {
        root: "/games/bench".into(),
        name: "Bench".into(),
        focused: false,
    });
    h.run();
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(asked(
        &h,
        &WorkbenchAction::RenameProject("/games/bench".into(), "Bench".into())
    ));
    let opened = h
        .state()
        .1
        .actions
        .iter()
        .any(|a| matches!(a, WorkbenchAction::OpenRecent(_)));
    assert!(!opened, "renamed, not opened");
}

/// N-29: Enter in the search opens only a picked card it still shows.
#[test]
fn enter_in_the_search_opens_only_a_picked_card_it_shows() {
    let enter_after_typing = |pick: Option<&str>, typed: &str| {
        let mut h = harness(launcher_with(some_projects()));
        h.run();
        if let Some(card) = pick {
            h.get_by_label(card).click();
            h.run();
        }
        h.get_by_label("Search projects").click();
        h.run();
        h.get_by_label("Search projects").type_text(typed);
        h.run();
        h.key_press(egui::Key::Enter);
        h.run();
        h.run();
        h.state()
            .1
            .actions
            .iter()
            .filter(|a| matches!(a, WorkbenchAction::OpenRecent(_)))
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(
        enter_after_typing(Some("Benchmark"), "bench"),
        [WorkbenchAction::OpenRecent("/games/bench".into())]
    );
    assert!(enter_after_typing(Some("Benchmark"), "play").is_empty());
    assert!(enter_after_typing(None, "bench").is_empty());
}

#[path = "new_project_tests.rs"]
mod new_project_tests;
