//! The launcher's side of the window: places kept apart and checked,
//! folders looked at, New project's Create, opening and closing, and a
//! project named on the command line skipping the launcher.

use super::*;
use editor::ProjectKind;

/// A test folder, emptied first, gone when the test ends.
fn scratch(name: &str) -> reflect::test_support::TempFolder {
    reflect::test_support::TempFolder::new(&format!("launcher-{name}")).unwrap()
}

/// The app as the program starts with `launch`, with nothing saved and
/// no network or sound card.
fn app_with(launch: crate::cli::Launch) -> (egui::Context, App) {
    let ctx = egui::Context::default();
    let mut storage = super::super::tests::Memory::default();
    eframe::set_value(&mut storage, super::super::AUTO_UPDATE_KEY, &false);
    let mut cc = eframe::CreationContext::_new_kittest(ctx.clone());
    cc.storage = Some(&storage);
    let mut app = App::new(
        &cc,
        crate::app_info(),
        launch,
        crate::updates::PendingSlot::default(),
        None,
    );
    app.sound = crate::sound::Sound::with_opener(
        || Err("there's no sound card".to_owned()),
        crate::sound::RETRY,
    );
    (ctx, app)
}

fn plan(folder: &Path, name: &str, kind: ProjectKind, starter: Starter) -> NewProjectPlan {
    NewProjectPlan {
        folder: folder.join(name),
        name: name.into(),
        kind,
        starter,
        create_in: folder.to_path_buf(),
    }
}

#[test]
fn a_place_is_checked_and_kept_on_the_screen() {
    let first = [1100.0, 700.0];
    let min = [760.0, 520.0];
    assert_eq!(
        WindowPlace::checked(None, min, first),
        WindowPlace::centred(first)
    );
    let odd = |size, at| Some(WindowPlace { size, at });
    for bad in [
        [10.0, 10.0],
        [f32::NAN, 700.0],
        [1.0e9, 700.0],
        [f32::INFINITY, 1.0],
    ] {
        assert_eq!(
            WindowPlace::checked(odd(bad, None), min, first),
            WindowPlace::centred(first),
            "{bad:?}"
        );
    }
    let kept = WindowPlace::checked(odd([900.0, 600.0], Some([f32::NAN, 3.0])), min, first);
    assert_eq!(
        kept,
        WindowPlace::centred([900.0, 600.0]),
        "a broken corner goes"
    );
    let monitor = egui::vec2(1920.0, 1080.0);
    let at = WindowPlace {
        size: [900.0, 600.0],
        at: Some([100.0, 50.0]),
    };
    assert_eq!(at.corner_on(monitor), egui::pos2(100.0, 50.0));
    // Left on a screen that has gone: centred on this one.
    for gone in [
        [3000.0, 50.0],
        [100.0, 2000.0],
        [-2000.0, 50.0],
        [100.0, -40.0],
    ] {
        let place = WindowPlace {
            at: Some(gone),
            ..at
        };
        assert_eq!(
            place.corner_on(monitor),
            egui::pos2(510.0, 240.0),
            "{gone:?}"
        );
    }
    let huge = WindowPlace::centred([4000.0, 3000.0]);
    assert_eq!(
        huge.corner_on(monitor),
        egui::Pos2::ZERO,
        "never off the top"
    );
}

#[test]
fn a_folder_is_free_empty_taken_or_a_file() {
    let base = scratch("states");
    assert_eq!(folder_state(&base.join("none")), FolderState::Free);
    std::fs::create_dir_all(base.join("empty")).unwrap();
    assert_eq!(folder_state(&base.join("empty")), FolderState::Empty);
    std::fs::create_dir_all(base.join("full")).unwrap();
    std::fs::write(base.join("full/notes.txt"), "mine").unwrap();
    assert_eq!(folder_state(&base.join("full")), FolderState::Taken);
    std::fs::write(base.join("file"), "x").unwrap();
    assert_eq!(folder_state(&base.join("file")), FolderState::AFile);
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(base.join("empty"), base.join("link")).unwrap();
        assert_eq!(
            folder_state(&base.join("link")),
            FolderState::AFile,
            "never a link"
        );
    }
    // Asked on a worker, answered later.
    let mut looks = FolderLooks::default();
    looks.ask(base.join("full"));
    assert!(looks.waiting());
    let start = std::time::Instant::now();
    let mut answers = Vec::new();
    while answers.is_empty() && start.elapsed().as_secs() < 10 {
        answers = looks.answered();
    }
    assert_eq!(answers, [(base.join("full"), FolderState::Taken)]);
    assert!(!looks.waiting());
}

#[test]
fn the_program_starts_on_the_launcher_unless_a_project_is_named() {
    let (_, app) = app_with(crate::cli::Launch::default());
    assert!(app.workbench.launcher_shown());
    let base = scratch("named");
    let farm = base.join("Farm");
    asset_db::Project::create_kind(&farm, "Farm", asset_db::ProjectKind::TwoD).unwrap();
    let (_, app) = app_with(crate::cli::Launch {
        project: Some(farm.clone()),
        ..Default::default()
    });
    assert!(!app.workbench.launcher_shown(), "straight into the editor");
    assert_eq!(app.workbench.project().unwrap().root, farm);
    assert_eq!(app.workbench.project_kind(), ProjectKind::TwoD);
    // A folder that isn't a project: the launcher, saying why.
    let (_, app) = app_with(crate::cli::Launch {
        project: Some(base.join("nothing here")),
        ..Default::default()
    });
    assert!(app.workbench.launcher_shown());
}

#[test]
fn create_makes_the_folder_and_the_project_and_opens_it() {
    let base = scratch("create");
    let (_, mut app) = app_with(crate::cli::Launch::default());
    app.create_project(plan(&base, "Farm Days", ProjectKind::TwoD, Starter::Empty));
    let root = base.join("Farm Days");
    assert!(root.join(asset_db::PROJECT_FILE).is_file(), "the project");
    assert!(!root.join("assets").exists(), "only what it needs to open");
    let summary = asset_db::Project::summary(&root).unwrap();
    assert_eq!(summary.name, "Farm Days");
    assert_eq!(summary.kind, asset_db::ProjectKind::TwoD);
    assert!(!app.workbench.launcher_shown(), "the editor opened it");
    assert_eq!(app.workbench.project_kind(), ProjectKind::TwoD);
    assert_eq!(app.create_in, base.to_path_buf(), "remembered");
    // Closing it goes back to the launcher, which lists it.
    app.close_project();
    assert!(app.workbench.launcher_shown());
    assert!(app.workbench.project().is_none() && app.projects.db().is_none());
    assert!(app.projects.known().saved().iter().any(|k| k.root == root));
}

#[test]
fn create_never_writes_into_a_folder_with_things_in_it() {
    let base = scratch("taken");
    let taken = base.join("Cave Run");
    std::fs::create_dir_all(&taken).unwrap();
    std::fs::write(taken.join("mine.txt"), "keep me").unwrap();
    let (_, mut app) = app_with(crate::cli::Launch::default());
    app.workbench.open_launcher();
    app.create_project(plan(&base, "Cave Run", ProjectKind::Both, Starter::Empty));
    assert!(!taken.join(asset_db::PROJECT_FILE).exists(), "refused");
    assert_eq!(
        std::fs::read_to_string(taken.join("mine.txt")).unwrap(),
        "keep me"
    );
    assert!(app.workbench.launcher_shown());
    // Nor with a bad name, whatever the editor let through.
    app.create_project(plan(&base, "CON", ProjectKind::Both, Starter::Empty));
    assert!(!base.join("CON").exists());
}

#[test]
fn a_sample_starter_is_made_under_the_name_and_kind_chosen() {
    let base = scratch("starter");
    let (_, mut app) = app_with(crate::cli::Launch::default());
    app.create_project(plan(
        &base,
        "My turret",
        ProjectKind::Both,
        Starter::TurretSample,
    ));
    let root = base.join("My turret");
    let summary = asset_db::Project::summary(&root).unwrap();
    assert_eq!(summary.name, "My turret");
    assert_eq!(summary.kind, asset_db::ProjectKind::Both);
    assert!(root.join("assets").join(crate::sample_game::AREA).is_file());
    assert_eq!(app.workbench.project().unwrap().name, "My turret");
}

#[test]
fn open_folder_opens_a_project_or_looks_for_projects_in_the_folder() {
    let base = scratch("open-folder");
    let farm = base.join("games/Farm");
    asset_db::Project::create(&farm, "Farm").unwrap();
    let (_, mut app) = app_with(crate::cli::Launch::default());
    app.open_folder(base.join("games"));
    assert!(
        app.workbench.launcher_shown(),
        "not a project: looked through"
    );
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < 10 {
        if let Some(done) = app.tidying.done() {
            app.tidied(done);
            break;
        }
    }
    assert!(
        app.projects.known().saved().iter().any(|k| k.root == farm),
        "found"
    );
    app.open_folder(farm.clone());
    assert_eq!(app.workbench.project().unwrap().root, farm);
}

#[test]
fn the_kind_changed_in_project_settings_is_kept_in_the_file() {
    let base = scratch("kind");
    let farm = base.join("Farm");
    asset_db::Project::create(&farm, "Farm").unwrap();
    let (_, mut app) = app_with(crate::cli::Launch {
        project: Some(farm.clone()),
        ..Default::default()
    });
    assert_eq!(
        app.workbench.project_kind(),
        ProjectKind::Both,
        "an old project"
    );
    app.set_project_kind(ProjectKind::ThreeD);
    let summary = asset_db::Project::summary(&farm).unwrap();
    assert_eq!(summary.kind, asset_db::ProjectKind::ThreeD);
}
