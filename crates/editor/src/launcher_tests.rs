//! The launcher (decision 0086) and the editor basics (E1): New in the Library, a new
//! Lust file that compiles, a new area, and what isn't built kept out of
//! sight.

use super::*;
use crate::library::{LibraryItem, LibraryKind, LibraryRequest, ProjectInfo};
use crate::test_support::*;
use egui_kittest::kittest::Queryable;

fn farm() -> ProjectInfo {
    ProjectInfo {
        name: "Farm".into(),
        root: "/projects/farm".into(),
        assets: "/projects/farm/assets".into(),
    }
}

fn item(path: &str, kind: LibraryKind) -> LibraryItem {
    LibraryItem {
        id: Some(reflect::AssetId::random()),
        kind,
        name: crate::sprite_sheets::name_of(path),
        path: path.into(),
        thumbnail: None,
    }
}

/// The text of each new file asked for, by path.
fn made(actions: &[WorkbenchAction]) -> Vec<(String, String)> {
    actions
        .iter()
        .filter_map(|a| match a {
            WorkbenchAction::NewTextFile { path, text } => {
                Some((path.display().to_string(), text.clone()))
            }
            _ => None,
        })
        .collect()
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

fn launcher_with(projects: Vec<KnownProject>) -> Workbench {
    let mut w = Workbench::new(info());
    w.set_known_projects(projects);
    w.set_this_build(Some(57));
    w.set_manual_url("https://example.com/manual".into());
    w.launch_launcher();
    w
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
fn the_launcher_lists_projects_and_does_what_it_offers() {
    let mut h = harness(launcher_with(some_projects()));
    h.run();
    assert!(h.state().0.launcher_open());
    for label in [
        "Learn",
        "Playground",
        "Zebra farm",
        "Unpin Zebra farm",
        "Pin Playground",
        "More for Playground",
        "Remove Old jam from the list",
        "Close the launcher",
    ] {
        assert!(h.query_by_label(label).is_some(), "{label}");
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
    assert!(h.state().1.actions.contains(&WorkbenchAction::PinProject(
        "/games/playground".into(),
        true
    )));
    h.get_by_label("Remove Old jam from the list").click();
    h.run();
    assert_eq!(h.state().0.known_projects.len(), 3);
    assert!(
        h.state()
            .1
            .actions
            .contains(&WorkbenchAction::ForgetProject("/jam/old".into()))
    );
    // A click picks a row; Enter opens it.
    h.get_by_label("Benchmark").click();
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(
        h.state()
            .1
            .actions
            .contains(&WorkbenchAction::OpenRecent("/games/bench".into()))
    );
    // ⋯ > Open does the same; Remove from list forgets it.
    h.get_by_label("More for Playground").click();
    h.run();
    h.get_by_label("Remove from list").click();
    h.run();
    assert!(
        h.state()
            .1
            .actions
            .contains(&WorkbenchAction::ForgetProject("/games/playground".into()))
    );
    // The arrows move along the list.
    h.state_mut().0.launcher.selected = None;
    h.key_press(egui::Key::ArrowDown);
    h.run();
    assert_eq!(
        h.state().0.launcher.selected,
        Some("/games/zebra".into()),
        "the pinned one is first"
    );
    h.key_press(egui::Key::ArrowDown);
    h.run();
    assert_eq!(h.state().0.launcher.selected, Some("/games/bench".into()));
    h.get_by_label("Close the launcher").click();
    h.run();
    assert!(!h.state().0.launcher_open());
}

#[test]
fn the_menu_shows_renames_and_copies_and_a_missing_folder_can_be_located() {
    let mut h = harness(launcher_with(some_projects()));
    h.run();
    let asked = |h: &egui_kittest::Harness<'static, State>, a: WorkbenchAction| {
        h.state().1.actions.contains(&a)
    };
    h.get_by_label("Scan a folder…").click();
    h.run();
    assert!(asked(&h, WorkbenchAction::ScanForProjects));
    h.get_by_label("Locate Old jam").click();
    h.run();
    assert!(asked(&h, WorkbenchAction::LocateProject("/jam/old".into())));
    h.get_by_label("More for Playground").click();
    h.run();
    h.get_by_label("Show in Explorer").click();
    h.run();
    assert!(asked(
        &h,
        WorkbenchAction::ShowProject("/games/playground".into())
    ));
    h.get_by_label("More for Playground").click();
    h.run();
    h.get_by_label("Make a copy…").click();
    h.run();
    assert!(asked(
        &h,
        WorkbenchAction::CopyProject("/games/playground".into())
    ));
    // Rename… asks for the name, with the old one to change.
    h.get_by_label("More for Playground").click();
    h.run();
    h.get_by_label("Rename…").click();
    h.run();
    h.run();
    assert_eq!(
        h.state()
            .0
            .launcher
            .renaming
            .as_ref()
            .map(|r| r.name.as_str()),
        Some("Playground")
    );
    h.state_mut().0.launcher.renaming.as_mut().unwrap().name = "   ".into();
    h.run();
    assert!(h.query_by_label_contains("needs a name").is_some());
    h.state_mut().0.launcher.renaming.as_mut().unwrap().name = " Play park ".into();
    h.run();
    h.get_by_label("Rename").click();
    h.run();
    assert!(asked(
        &h,
        WorkbenchAction::RenameProject("/games/playground".into(), "Play park".into())
    ));
    assert!(h.state().0.launcher.renaming.is_none());
    // Esc while renaming leaves the name, not the launcher.
    h.state_mut().0.launcher.renaming = Some(Renaming {
        root: "/games/bench".into(),
        name: "Bench".into(),
        focused: true,
    });
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.state().0.launcher.renaming.is_none());
    assert!(h.state().0.launcher_open());
    // What the app did is said until closed.
    h.state_mut()
        .0
        .launcher_notice("Found 2: 1 added to the list.");
    h.run();
    assert!(h.query_by_label("Found 2: 1 added to the list.").is_some());
    h.get_by_label("Close the note").click();
    h.run();
    assert!(h.state().0.launcher.notice.is_none());
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
    let mut w = launcher_with(Vec::new());
    w.set_update(UpdateNotice::Available(60));
    w.set_release_notes(Some(ReleaseNotes {
        build: 60,
        notes: "Build 60 of Test Engine:\n\n> The launcher".into(),
        published: Some("2026-11-05".into()),
    }));
    let mut h = harness(w);
    h.run();
    // A newer build is marked in the sidebar, at its bottom, and in the
    // bar over the projects.
    assert!(h.query_all_by_label_contains("Build 60 is out").count() >= 2);
    h.get_by_label("Updates").click();
    h.run();
    for label in [
        "What's new in build 60",
        "The launcher",
        "Published 2026-11-05",
        "Build 57",
    ] {
        assert!(h.query_by_label_contains(label).is_some(), "{label}");
    }
    h.get_by_label("Download").click();
    h.run();
    assert!(
        h.state()
            .1
            .actions
            .contains(&WorkbenchAction::DownloadUpdate)
    );
    h.get_by_label("Check now").click();
    h.run();
    assert!(
        h.state()
            .1
            .actions
            .contains(&WorkbenchAction::CheckForUpdates)
    );
    h.get_by_label_contains("Look for updates when").click();
    h.run();
    assert!(
        h.state()
            .1
            .actions
            .contains(&WorkbenchAction::SetAutoUpdate(false))
    );
    // Downloaded: Restart to update closes to install it.
    h.state_mut().0.set_update(UpdateNotice::Ready(60));
    h.run();
    h.get_by_label("Restart to update").click();
    h.run();
    assert!(h.state().0.restarting());
    assert!(h.state().1.actions.contains(&WorkbenchAction::Quit));
    // The question File > Updates asks isn't asked over this page.
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
    assert!(h.query_by_label("You have it").is_some());
    assert!(h.query_by_label_contains("What's new").is_none());
}

#[test]
fn the_learn_page_offers_the_examples_and_the_manual() {
    let mut h = harness(launcher_with(Vec::new()));
    h.run();
    assert!(h.query_by_label_contains("No projects yet").is_some());
    h.get_by_label("Learn").click();
    h.run();
    for label in [
        "Make the Playground",
        "Make the Turret sample",
        "Make the Benchmark",
        "Open the manual ↗",
    ] {
        assert!(h.query_by_label(label).is_some(), "{label}");
    }
    h.get_by_label("Make the Playground").click();
    h.run();
    assert!(
        h.state()
            .1
            .actions
            .contains(&WorkbenchAction::MakePlayground)
    );
    assert!(
        h.query_by_label_contains("Show this page when").is_none(),
        "it always shows: nothing to turn off"
    );
}

#[test]
fn solis_always_starts_on_the_launcher_and_comes_back_to_it() {
    let mut w = Workbench::new(info());
    assert!(!w.launcher_open());
    w.launch_launcher();
    assert!(w.launcher_open(), "always at launch");
    // A project opening closes it.
    w.set_project(Some(farm()));
    assert!(!w.launcher_open());
    // Closing the editor's window comes back to it, the project open
    // behind it.
    w.back_to_launcher();
    assert!(w.launcher_open());
    assert!(w.open.project.is_some());
}

#[test]
fn file_projects_brings_it_back_over_an_open_project_and_esc_goes_back() {
    let mut w = Workbench::new(info());
    w.set_project(Some(farm()));
    w.set_known_projects(vec![known("Farm", "/projects/farm", Some(1))]);
    w.open_launcher();
    let mut h = harness(w);
    h.run();
    assert!(
        h.state().1.actions.contains(&WorkbenchAction::ListProjects),
        "the list is read afresh"
    );
    assert!(
        h.query_by_label("← Back to Farm").is_some()
            || h.query_by_label("Close the launcher").is_some()
    );
    // The open project, opened again, is simply gone back to.
    h.state_mut().1.actions.clear();
    h.get_by_label("Farm").click();
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(!h.state().0.launcher_open());
    assert!(
        !h.state()
            .1
            .actions
            .iter()
            .any(|a| matches!(a, WorkbenchAction::OpenRecent(_)))
    );
    h.state_mut().0.open_launcher();
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(!h.state().0.launcher_open());
}

#[test]
fn the_first_code_file_is_main_lst_and_later_ones_components() {
    let mut w = Workbench::new(info());
    w.set_project(Some(farm()));
    let mut actions = Vec::new();
    w.library_request(LibraryRequest::NewCodeFile("crops".into()), &mut actions);
    let files = made(&actions);
    assert_eq!(files[0].0, "main.lst", "game code starts there");
    assert!(files[0].1.contains("component Game"));
    // It opens in the computer's editor for Lust (Ledger), once written.
    let opened = actions
        .iter()
        .position(|a| matches!(a, WorkbenchAction::OpenExternally(p) if p.ends_with("main.lst")))
        .expect("opened");
    assert!(opened > 0, "after it's made");
    w.set_library(vec![item("main.lst", LibraryKind::Code)]);
    let mut actions = Vec::new();
    w.library_request(LibraryRequest::NewCodeFile("crops".into()), &mut actions);
    let files = made(&actions);
    assert_eq!(files[0].0, "crops/component.lst");
    assert!(files[0].1.contains("fetch \"crops/component\":Component;"));
    assert!(files[0].1.contains("component Component"));
}

#[test]
fn new_code_files_compile_as_they_are() {
    // The game's first file on its own, and a component fetched from it.
    let check = |files: Vec<(String, String)>| {
        let mut all = script::describe::descriptions("Test 1.0", "solis");
        all.extend(files);
        let code = script::Code {
            files: all,
            lua: Vec::new(),
            notes: Vec::new(),
        };
        let problems = script::check(&code);
        assert!(problems.is_empty(), "{problems:?}");
    };
    let main = crate::workbench_library::starter_code("Game", "main.lst");
    check(vec![("main.lst".into(), main)]);
    let wheat = crate::workbench_library::starter_code("Wheat", "crops/wheat.lst");
    check(vec![
        (
            "main.lst".into(),
            "fetch \"crops/wheat\":Wheat;\nend\n".into(),
        ),
        ("crops/wheat.lst".into(), wheat),
    ]);
}

#[test]
fn a_new_area_is_saved_where_it_was_asked_for_and_opened() {
    let mut w = Workbench::new(info());
    w.set_project(Some(farm()));
    w.set_library(vec![item("levels/area.solarea", LibraryKind::Area)]);
    let mut actions = Vec::new();
    w.library_request(LibraryRequest::NewArea("levels".into()), &mut actions);
    let files = made(&actions);
    assert_eq!(files[0].0, "levels/area 2.solarea", "a free name");
    assert!(files[0].1.contains("area version"));
    let read = area::text::read(&files[0].1, &w.open.document.registry).expect("reads");
    assert!(!read.is_empty(), "an environment, as a new area has");
    assert!(actions.iter().any(
        |a| matches!(a, WorkbenchAction::OpenAreaFile(p) if p.ends_with("levels/area 2.solarea"))
    ));
}

#[test]
fn right_clicking_the_library_offers_new_everywhere() {
    let mut w = Workbench::new(info());
    w.set_project(Some(farm()));
    w.set_library(vec![item("tiles.png", LibraryKind::Texture)]);
    let mut h = harness(w);
    h.run();
    crate::test_support::ribbon_tab(&mut h, crate::shell::RibbonTab::Library);
    h.get_by_label("+ New").click();
    h.run();
    for label in [
        "Lust code file",
        "Folder",
        "Tile map",
        "Tileset",
        "Animator",
        "Sound event",
    ] {
        assert!(h.query_by_label(label).is_some(), "{label}");
    }
    assert!(
        h.query_all_by_label("Area").count() >= 1,
        "the New list's item"
    );
    h.get_by_label("Tile map").click();
    h.run();
    assert!(h.state().1.actions.iter().any(|a| matches!(
        a,
        WorkbenchAction::SaveFile {
            file: crate::OwnFile::TileMap(_),
            ..
        }
    )));
}

#[test]
fn what_isn_t_built_isn_t_shown() {
    let mut h = harness(Workbench::new(info()));
    h.run();
    for label in [
        "Build",
        "Build & Play",
        "Play with two players",
        "Layers",
        "Wireframe",
    ] {
        assert!(h.query_by_label(label).is_none(), "{label}");
    }
}

#[test]
fn a_newer_build_is_a_bar_over_the_projects_not_a_question() {
    use crate::updates::UpdateNotice;
    let mut w = launcher_with(Vec::new());
    w.set_update(UpdateNotice::Available(60));
    let mut h = harness(w);
    h.run();
    h.get_by_label("Update now").click();
    h.run();
    assert!(
        h.state()
            .1
            .actions
            .contains(&WorkbenchAction::DownloadUpdate)
    );
    h.get_by_label("See what's new").click();
    h.run();
    assert_eq!(h.state().0.launcher.page, Page::Updates);
    h.state_mut().0.launcher.page = Page::Projects;
    h.run();
    h.get_by_label("Not now").click();
    h.run();
    assert!(h.query_by_label("Update now").is_none(), "put away");
    // Downloaded: Restart to update quits, to install it.
    h.state_mut().0.set_update(UpdateNotice::Ready(60));
    h.run();
    h.get_by_label("Restart to update").click();
    h.run();
    assert!(h.state().1.actions.contains(&WorkbenchAction::Quit));
    assert!(h.state().0.restarting());
}

/// The launcher's cards, its chosen row and its notices are the design's:
/// the accent in use mixed into a neutral surface, never a colour of its
/// own (the review, BUG-29, found them written as numbers).
#[test]
fn the_launcher_s_tints_are_the_accent_in_use_mixed_into_the_surface() {
    use crate::theme::{self, Accent, InterfaceLook};
    use crate::tokens;
    for chosen in Accent::ALL {
        let look = InterfaceLook {
            accent: chosen,
            ..InterfaceLook::default()
        };
        // The editor holds the look it draws with, as the app hands it
        // over.
        let mut w = launcher_with(some_projects()).with_interface_look(look);
        w.launcher_notice("Made a copy.");
        let mut h = harness(w);
        theme::restyle(&h.ctx, look);
        h.run();
        h.get_by_label("Playground").click();
        h.run();
        let drawn = rects_drawn(&h);
        let fills = |colour| drawn.iter().any(|(_, fill, _)| *fill == colour);
        let borders = |colour| drawn.iter().any(|(_, _, border)| *border == colour);
        assert!(fills(tokens::select_item(chosen)), "{}", chosen.name());
        assert!(fills(tokens::changes_pill(chosen)), "{}", chosen.name());
        assert!(
            borders(tokens::open_card_border(chosen)),
            "{}",
            chosen.name()
        );
        assert!(borders(tokens::card_border()), "{}", chosen.name());
        for other in Accent::ALL.into_iter().filter(|a| *a != chosen) {
            assert!(!fills(tokens::select_item(other)), "{}", chosen.name());
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
