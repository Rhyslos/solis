//! The File tab's own pages: Recent projects (where Solis starts,
//! decision 0105), Learn, Updates and About.

use super::*;
use crate::library::ProjectInfo;
use crate::shell::RibbonTab;
use crate::test_support::*;
use egui_kittest::kittest::Queryable;

fn farm() -> ProjectInfo {
    ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }
}

fn known(name: &str, root: &str, last_opened: Option<u64>) -> KnownProject {
    KnownProject {
        root: root.into(),
        name: name.into(),
        problem: None,
        pinned: false,
        last_opened,
        saved_by: None,
    }
}

fn some_projects() -> Vec<KnownProject> {
    let mut gone = known("Old jam", "/jam/old", Some(10));
    gone.problem = Some("Folder not found".into());
    let mut pinned = known("Zebra farm", "/games/zebra", Some(50));
    pinned.pinned = true;
    pinned.saved_by = Some(57);
    vec![
        known("Playground", "/games/playground", Some(100)),
        pinned,
        known("Benchmark", "/games/bench", None),
        gone,
    ]
}

fn page_with(projects: Vec<KnownProject>) -> Workbench {
    let mut w = Workbench::new(info());
    w.set_known_projects(projects);
    w.set_this_build(Some(57));
    w.set_manual_url("https://example.com/manual".into());
    w.start_on_recent_projects();
    w
}

fn asked(h: &egui_kittest::Harness<'static, State>, action: WorkbenchAction) -> bool {
    h.state().1.actions.contains(&action)
}

#[test]
fn pinned_projects_come_first_then_the_order_chosen_and_search_narrows() {
    let projects = some_projects();
    let names =
        |list: &[usize]| -> Vec<&str> { list.iter().map(|&i| projects[i].name.as_str()).collect() };
    let (pinned, rest) = shown(&projects, "", Sort::LastOpened);
    assert_eq!(names(&pinned), ["Zebra farm"]);
    assert_eq!(
        names(&rest),
        ["Playground", "Old jam", "Benchmark"],
        "never-timed last"
    );
    let (_, rest) = shown(&projects, "", Sort::Name);
    assert_eq!(names(&rest), ["Benchmark", "Old jam", "Playground"]);
    // Every word, in the name or the folder, any case.
    let (pinned, rest) = shown(&projects, "GAMES  pla", Sort::Name);
    assert!(pinned.is_empty());
    assert_eq!(names(&rest), ["Playground"]);
    let (pinned, rest) = shown(&projects, "nothing like it", Sort::Name);
    assert!(pinned.is_empty() && rest.is_empty());
    assert_eq!(shown(&[], "", Sort::Name), (vec![], vec![]));
}

#[test]
fn how_long_ago_is_said_in_words() {
    let (minute, hour, day) = (60, 3600, 86_400);
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
fn a_project_s_build_is_compared_with_this_one() {
    assert_eq!(BuildMark::of(None, Some(5)), None);
    assert_eq!(BuildMark::of(Some(5), Some(5)), Some(BuildMark::ThisOne(5)));
    assert_eq!(BuildMark::of(Some(4), Some(5)), Some(BuildMark::Older(4)));
    assert_eq!(BuildMark::of(Some(6), Some(5)), Some(BuildMark::Newer(6)));
    assert_eq!(BuildMark::of(Some(6), None), Some(BuildMark::Some(6)));
}

#[test]
fn all_four_arrows_walk_the_cards() {
    use recent::{Walk, walk_to};
    // Seven cards, three to a row.
    assert_eq!(walk_to(None, 7, 3, Walk::Down), Some(0), "the first");
    assert_eq!(walk_to(None, 0, 3, Walk::Down), None, "no cards");
    assert_eq!(walk_to(Some(0), 7, 3, Walk::Right), Some(1));
    assert_eq!(walk_to(Some(1), 7, 3, Walk::Left), Some(0));
    assert_eq!(
        walk_to(Some(0), 7, 3, Walk::Left),
        Some(0),
        "stays at the edge"
    );
    assert_eq!(walk_to(Some(1), 7, 3, Walk::Down), Some(4));
    assert_eq!(walk_to(Some(5), 7, 3, Walk::Down), Some(6), "to the last");
    assert_eq!(walk_to(Some(4), 7, 3, Walk::Up), Some(1));
    assert_eq!(
        walk_to(Some(1), 7, 3, Walk::Up),
        Some(1),
        "the top row stays"
    );
    assert_eq!(walk_to(Some(6), 7, 3, Walk::Right), Some(6));
    // A row as wide as the cards fit.
    assert_eq!(recent::cards_across(240.0), 1);
    assert_eq!(recent::cards_across(1100.0), 4);
    assert_eq!(recent::cards_across(0.0), 1, "never none");
}

#[test]
fn recent_projects_lists_the_cards_and_does_what_they_offer() {
    let mut h = harness(page_with(some_projects()));
    h.run();
    assert!(h.state().0.file_page_open());
    for label in [
        "Learn",
        "Playground",
        "Zebra farm",
        "Unpin Zebra farm",
        "Pin Playground",
        "Remove Old jam from the list",
        "Carry on without a project",
        "Blank project",
        "Turret sample template",
    ] {
        assert!(h.query_all_by_label(label).next().is_some(), "{label}");
    }
    assert!(h.query_by_label_contains("build 57 · this one").is_some());
    assert!(
        h.query_by_label("Pin Old jam").is_none(),
        "a missing one can only go"
    );
    // Pinning goes to the app, and shows at once.
    h.get_by_label("Pin Playground").click();
    h.run();
    assert!(h.state().0.known_projects[0].pinned);
    assert!(asked(
        &h,
        WorkbenchAction::PinProject("/games/playground".into(), true)
    ));
    h.get_by_label("Remove Old jam from the list").click();
    h.run();
    assert_eq!(h.state().0.known_projects.len(), 3);
    assert!(asked(&h, WorkbenchAction::ForgetProject("/jam/old".into())));
    // A click picks a card; Enter opens it.
    h.get_by_label("Benchmark").click();
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(asked(
        &h,
        WorkbenchAction::OpenRecent("/games/bench".into())
    ));
    // The arrows walk the cards (the search lets go of the keys first).
    h.state_mut().0.file_pages.selected = None;
    h.state_mut().0.file_pages.focus_search = false;
    h.ctx.memory_mut(|m| {
        if let Some(id) = m.focused() {
            m.surrender_focus(id);
        }
    });
    h.run();
    h.key_press(egui::Key::ArrowRight);
    h.run();
    let first = h.state().0.file_pages.selected.clone();
    assert!(first.is_some(), "the first card");
    h.key_press(egui::Key::ArrowRight);
    h.run();
    assert_ne!(h.state().0.file_pages.selected, first, "the next one");
    // A template makes a project.
    h.get_by_label("Blank project").click();
    h.run();
    assert!(asked(&h, WorkbenchAction::NewProject));
    h.get_by_label("Turret sample template").click();
    h.run();
    assert!(asked(&h, WorkbenchAction::MakeSampleGame));
    // Carrying on leaves the page with no project.
    h.get_by_label("Carry on without a project").click();
    h.run();
    assert!(!h.state().0.file_page_open());
}

#[test]
fn a_card_s_menu_renames_and_copies_and_a_missing_folder_can_be_located() {
    let mut h = harness(page_with(some_projects()));
    h.run();
    h.get_by_label("Open… ▾").click();
    h.run();
    h.get_by_label("Scan a folder for projects…").click();
    h.run();
    assert!(asked(&h, WorkbenchAction::ScanForProjects));
    h.get_by_label("Open… ▾").click();
    h.run();
    h.get_by_label("Open a project folder…").click();
    h.run();
    assert!(asked(&h, WorkbenchAction::OpenProject));
    h.get_by_label("Locate Old jam").click();
    h.run();
    assert!(asked(&h, WorkbenchAction::LocateProject("/jam/old".into())));
    for (item, action) in [
        (
            "Open",
            WorkbenchAction::OpenRecent("/games/playground".into()),
        ),
        (
            "Show in Explorer",
            WorkbenchAction::ShowProject("/games/playground".into()),
        ),
        (
            "Make a copy…",
            WorkbenchAction::CopyProject("/games/playground".into()),
        ),
    ] {
        h.get_by_label("Playground").click_secondary();
        h.run();
        h.get_by_label(item).click();
        h.run();
        assert!(asked(&h, action), "{item}");
    }
    // Rename… asks for the name, with the old one to change.
    h.get_by_label("Playground").click_secondary();
    h.run();
    h.get_by_label("Rename…").click();
    h.run();
    h.run();
    let renaming = |h: &egui_kittest::Harness<'static, State>| {
        h.state()
            .0
            .file_pages
            .renaming
            .as_ref()
            .map(|r| r.name.clone())
    };
    assert_eq!(renaming(&h).as_deref(), Some("Playground"));
    h.state_mut().0.file_pages.renaming.as_mut().unwrap().name = "   ".into();
    h.run();
    assert!(h.query_by_label_contains("needs a name").is_some());
    h.state_mut().0.file_pages.renaming.as_mut().unwrap().name = " Play park ".into();
    h.run();
    h.get_by_label("Rename").click();
    h.run();
    assert!(asked(
        &h,
        WorkbenchAction::RenameProject("/games/playground".into(), "Play park".into())
    ));
    assert!(h.state().0.file_pages.renaming.is_none());
    // Esc while renaming leaves the name, not the page.
    h.state_mut().0.file_pages.renaming = Some(Renaming {
        root: "/games/bench".into(),
        name: "Bench".into(),
        focused: true,
    });
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.state().0.file_pages.renaming.is_none());
    assert!(h.state().0.file_page_open());
    // Remove from list forgets it.
    h.get_by_label("Playground").click_secondary();
    h.run();
    h.get_by_label("Remove from list").click();
    h.run();
    assert!(asked(
        &h,
        WorkbenchAction::ForgetProject("/games/playground".into())
    ));
    // What the app did is said until closed.
    h.state_mut()
        .0
        .projects_notice("Found 2: 1 added to the list.");
    h.run();
    assert!(h.query_by_label("Found 2: 1 added to the list.").is_some());
    h.get_by_label("Close the note").click();
    h.run();
    assert!(h.state().0.file_pages.notice.is_none());
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
fn the_updates_page_says_what_s_new_and_does_the_next_step() {
    use crate::updates::{ReleaseNotes, UpdateNotice};
    let mut w = page_with(Vec::new());
    w.set_update(UpdateNotice::Available(60));
    w.set_release_notes(Some(ReleaseNotes {
        build: 60,
        notes: "Build 60 of Test Engine:\n\n> The new pages".into(),
        published: Some("2026-11-05".into()),
    }));
    let mut h = harness(w);
    h.run();
    // A newer build is said in the status bar, which leads to Updates.
    h.get_by_label("Build 60 is out").click();
    h.run();
    assert_eq!(h.state().0.file_pages.page, Page::Updates);
    for label in [
        "What's new in build 60",
        "The new pages",
        "Published 2026-11-05",
        "build 57",
        "Build 60",
    ] {
        assert!(
            h.query_all_by_label_contains(label).next().is_some(),
            "{label}"
        );
    }
    h.get_by_label("Download").click();
    h.run();
    assert!(asked(&h, WorkbenchAction::DownloadUpdate));
    h.get_by_label("Check now").click();
    h.run();
    assert!(asked(&h, WorkbenchAction::CheckForUpdates));
    h.get_by_role_and_label(
        egui::accesskit::Role::CheckBox,
        "Look for updates when Test Engine starts",
    )
    .click();
    h.run();
    assert!(asked(&h, WorkbenchAction::SetAutoUpdate(false)));
    h.get_by_label("Open the changes list").click();
    h.run();
    assert!(asked(
        &h,
        WorkbenchAction::OpenProgramFile(ProgramFile::Changes)
    ));
    // Not now puts the notice away.
    h.get_by_label("Not now").click();
    h.run();
    assert!(h.query_by_label("Download").is_none(), "put away");
    // Downloaded: Restart to update closes to install it.
    h.state_mut().0.set_update(UpdateNotice::Ready(60));
    h.run();
    h.get_all_by_label("Restart to update")
        .next()
        .unwrap()
        .click();
    h.run();
    assert!(h.state().0.restarting());
    assert!(asked(&h, WorkbenchAction::Quit));
    // The question File › Updates asks isn't asked over this page.
    h.state_mut().0.offer_update(crate::splash::UpdateOffer {
        latest: 61,
        current: Some(57),
        installed: true,
    });
    h.run();
    assert!(h.state().0.offer.is_none());
    // Up to date: no notes, nothing to do.
    h.state_mut().0.set_update(UpdateNotice::UpToDate(57));
    h.run();
    assert!(h.query_by_label_contains("You have it").is_some());
    assert!(h.query_by_label_contains("What's new").is_none());
}

#[test]
fn the_learn_page_offers_the_examples_the_manual_and_the_sample_area() {
    let mut h = harness(page_with(Vec::new()));
    h.run();
    assert!(h.query_by_label_contains("No projects yet").is_some());
    h.get_by_label("Learn").click();
    h.run();
    for label in [
        "Make the Playground",
        "Make the Turret sample",
        "Make the Benchmark",
        "Open the manual",
        "Add the sample area",
    ] {
        assert!(h.query_all_by_label(label).next().is_some(), "{label}");
    }
    h.get_by_label("Make the Playground").click();
    h.run();
    assert!(asked(&h, WorkbenchAction::MakePlayground));
    // The sample area needs a project to go into.
    h.get_by_label("Add the sample area").click();
    h.run();
    assert!(!asked(&h, WorkbenchAction::AddSampleArea));
    h.state_mut().0.set_project(Some(farm()));
    h.state_mut().0.show_file_page(Page::Learn);
    h.run();
    h.get_by_label("Add the sample area").click();
    h.run();
    assert!(asked(&h, WorkbenchAction::AddSampleArea));
}

#[test]
fn about_is_a_page_with_the_name_the_build_and_the_licences() {
    let mut h = harness(page_with(Vec::new()));
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
    h.get_by_label("Open the notices").click();
    h.run();
    assert!(asked(
        &h,
        WorkbenchAction::OpenProgramFile(ProgramFile::Notices)
    ));
    h.get_by_label("Go to Updates").click();
    h.run();
    assert_eq!(h.state().0.file_pages.page, Page::Updates);
}

#[test]
fn solis_always_starts_on_recent_projects_and_comes_back_to_it() {
    let mut w = Workbench::new(info());
    assert!(!w.file_page_open());
    w.start_on_recent_projects();
    assert!(w.file_page_open(), "always at start");
    assert_eq!(w.file_pages.page, Page::Projects);
    // A project opening leaves it, for the tab before.
    w.set_project(Some(farm()));
    assert!(!w.file_page_open());
    assert_eq!(w.ribbon_tab(), RibbonTab::Home);
    // Closing the editor's window comes back to it, the project open
    // behind it.
    w.show_recent_projects();
    assert!(w.file_page_open());
    assert!(w.open.project.is_some());
}

#[test]
fn with_no_project_the_other_tabs_are_dimmed_and_say_so() {
    let mut h = harness(page_with(some_projects()));
    h.run();
    assert!(h.query_by_label("No project open").is_some());
    assert!(h.query_by_label("Carry on without a project").is_some());
    // With a project, Back to editor again, and its name.
    h.state_mut().0.set_project(Some(farm()));
    h.state_mut().0.show_recent_projects();
    h.run();
    assert!(h.query_by_label("No project open").is_none());
    assert!(h.query_by_label("Back to editor").is_some());
    let subtitle = "Pick up where you left off, or start from a template.";
    assert!(h.query_by_label(subtitle).is_some());
}

#[test]
fn recent_projects_comes_back_over_an_open_project_and_esc_goes_back() {
    let mut w = Workbench::new(info());
    w.set_project(Some(farm()));
    w.set_known_projects(vec![known("Farm", "/projects/farm", Some(1))]);
    w.show_recent_projects();
    let mut h = harness(w);
    h.run();
    assert!(
        asked(&h, WorkbenchAction::ListProjects),
        "the list is read afresh"
    );
    // The open project, opened again, is simply gone back to.
    h.state_mut().1.actions.clear();
    h.get_by_label("Farm").click();
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(!h.state().0.file_page_open());
    assert!(
        !h.state()
            .1
            .actions
            .iter()
            .any(|a| matches!(a, WorkbenchAction::OpenRecent(_)))
    );
    h.state_mut().0.show_recent_projects();
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(!h.state().0.file_page_open());
}

/// A picked card's border and the notice are the design's: the accent in
/// use mixed into a neutral surface, never a colour of its own (the
/// review, BUG-29, found the launcher's written as numbers).
#[test]
fn the_page_s_tints_are_the_accent_in_use_mixed_into_the_surface() {
    use crate::theme::{self, Accent, InterfaceLook};
    use crate::tokens;
    for chosen in Accent::ALL {
        let look = InterfaceLook {
            accent: chosen,
            ..InterfaceLook::default()
        };
        // The editor holds the look it draws with, as the app hands it
        // over.
        let mut w = page_with(some_projects()).with_interface_look(look);
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
        assert!(borders(tokens::card_border()), "{}", chosen.name());
        for other in Accent::ALL.into_iter().filter(|a| *a != chosen) {
            assert!(!fills(tokens::changes_pill(other)), "{}", chosen.name());
            assert!(
                !borders(tokens::open_card_border(other)),
                "{}",
                chosen.name()
            );
        }
    }
    crate::interface_look::use_look(InterfaceLook::default());
}

/// N-05: Enter to keep a project's new name renames it and opens nothing;
/// Enter on a picked card, nothing being typed, still opens it.
#[test]
fn enter_while_a_name_is_typed_never_opens_the_picked_project() {
    let mut h = harness(page_with(some_projects()));
    h.run();
    h.get_by_label("Benchmark").click();
    h.run();
    h.state_mut().0.file_pages.renaming = Some(Renaming {
        root: "/games/bench".into(),
        name: "Bench".into(),
        focused: false,
    });
    h.run();
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    let opened = |h: &egui_kittest::Harness<'static, State>| {
        h.state()
            .1
            .actions
            .iter()
            .any(|a| matches!(a, WorkbenchAction::OpenRecent(_)))
    };
    assert!(asked(
        &h,
        WorkbenchAction::RenameProject("/games/bench".into(), "Bench".into())
    ));
    assert!(!opened(&h), "renamed, not opened");
    // A card picked, with nothing typed: Enter opens it.
    h.get_by_label("Benchmark").click();
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(asked(
        &h,
        WorkbenchAction::OpenRecent("/games/bench".into())
    ));
}
