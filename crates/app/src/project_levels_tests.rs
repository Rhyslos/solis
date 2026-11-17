//! A project from before levels (decision 0118) opened by this build: its
//! areas become levels, everything that names them finds them, the older
//! files are kept, and a build from before refuses the project after.

use super::*;
use reflect::upgrade::Versioned;

/// The example files kept for good (`tests/fixtures/`).
fn fixture(path: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures")
            .join(path),
    )
    .unwrap()
}

/// Every file under `folder`, as paths inside it with `/` between names.
fn files_under(folder: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut folders = vec![folder.to_path_buf()];
    while let Some(f) = folders.pop() {
        for entry in std::fs::read_dir(&f).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            } else {
                let inside = path.strip_prefix(folder).unwrap();
                let names: Vec<_> = inside.iter().map(|n| n.to_string_lossy()).collect();
                found.push(names.join("/"));
            }
        }
    }
    found.sort();
    found
}

#[test]
fn a_project_s_areas_become_levels_when_it_opens_and_nothing_is_lost() {
    let root = reflect::test_support::TempFolder::new("levels-upgrade").unwrap();
    let assets = root.join("assets");
    std::fs::create_dir_all(assets.join("levels")).unwrap();
    std::fs::create_dir_all(assets.join("areas")).unwrap();
    std::fs::create_dir_all(assets.join("assemblies")).unwrap();
    let project_file = "name = \"Harbour\"\nformat = 1\n";
    std::fs::write(root.join(asset_db::PROJECT_FILE), project_file).unwrap();
    // Three areas of three versions. The yard places the crate assembly
    // by its ID, which its sidecar keeps.
    let docks = fixture("area/v3/blockout.solarea");
    let yard = fixture("area/v2/yard.solarea");
    let turret = fixture("area/v1/turret.solarea");
    std::fs::write(assets.join("levels/docks.solarea"), &docks).unwrap();
    std::fs::write(assets.join("levels/yard.solarea"), &yard).unwrap();
    std::fs::write(assets.join("areas/turret.solarea"), &turret).unwrap();
    let crate_file = assets.join("assemblies/crate.solassembly");
    std::fs::write(&crate_file, fixture("area/v2/crate.solassembly")).unwrap();
    let crate_id: reflect::AssetId = "5KD8V0Q2MX7TA".parse().unwrap();
    let sidecar = asset_db::Sidecar {
        id: crate_id,
        ..asset_db::Sidecar::new(asset_db::hash_file(&crate_file).unwrap())
    };
    sidecar
        .write(&asset_db::sidecar::path_for(&crate_file))
        .unwrap();
    // The start level the game was last exported with, and a recording
    // of a play in the turret's area.
    std::fs::create_dir_all(root.join(asset_db::CACHE_FOLDER)).unwrap();
    std::fs::write(
        root.join(asset_db::CACHE_FOLDER).join("last export.txt"),
        "playtest 1\nname Harbour\narea levels/docks.solarea\nwindow windowed 1280 720\n",
    )
    .unwrap();
    let recording = fixture("recording/v3/play.solrec");

    let mut projects = Projects::default();
    let opened = projects
        .open(&root, false, assets::Limits::default())
        .unwrap();

    // Every area is a level now, under its old name with the new ending,
    // and reads as exactly what it was.
    let registry = level::builtin::registry().unwrap();
    let read = |text: &str| level::text::read_file(text, &registry).unwrap();
    for (name, old) in [
        ("levels/docks", &docks),
        ("levels/yard", &yard),
        ("areas/turret", &turret),
    ] {
        assert!(!assets.join(format!("{name}.solarea")).exists(), "{name}");
        let new = std::fs::read_to_string(assets.join(format!("{name}.sollevel"))).unwrap();
        assert_eq!(read(&new), read(old), "{name}");
        assert_eq!(
            level::text::FORMAT.version_of(&new),
            Some(level::text::VERSION)
        );
    }
    assert_eq!(
        std::fs::read_to_string(assets.join("levels/docks.sollevel")).unwrap(),
        fixture("area/v4/blockout.sollevel")
    );
    // The yard still finds its crate: the assembly it places is the
    // project's, by the same ID.
    let yard_now = read(&std::fs::read_to_string(assets.join("levels/yard.sollevel")).unwrap());
    let placed: Vec<_> = yard_now
        .level
        .iter()
        .filter_map(|(_, e)| e.origin.as_ref().map(|o| o.assembly))
        .collect();
    assert_eq!(placed, [crate_id]);
    let listing = projects.listing();
    assert!(
        listing
            .files
            .iter()
            .any(|f| f.id == Some(crate_id) && f.path.ends_with("crate.solassembly"))
    );
    let levels: Vec<_> = listing
        .files
        .iter()
        .filter(|f| f.kind == asset_db::FileKind::Level)
        .map(|f| f.path.clone())
        .collect();
    assert_eq!(levels.len(), 3, "{levels:?}");
    assert!(levels.iter().all(|p| p.extension().unwrap() == "sollevel"));
    // The start level and the recording name the levels by their new
    // names, which are there.
    let start = crate::playtest::last_export(&root).expect("read");
    assert_eq!(start.level, "levels/docks.sollevel");
    assert!(assets.join(&start.level).is_file());
    let played = input::Recording::read(&recording).unwrap();
    assert_eq!(played.level.as_deref(), Some("areas/turret.sollevel"));
    assert!(assets.join(played.level.unwrap()).is_file());
    // The project file says format 2 now, keeping its name.
    let now = std::fs::read_to_string(root.join(asset_db::PROJECT_FILE)).unwrap();
    assert_eq!(asset_db::PROJECT_FORMAT.version_of(&now), Some(2));
    assert_eq!(asset_db::Project::summary(&root).unwrap().name, "Harbour");
    // Every older file is in the backups, as it was.
    let backups = asset_db::backup::folder(&root);
    let kept = files_under(&backups);
    let in_backup = |name: &str| {
        let copy = kept.iter().find(|k| k.ends_with(name)).unwrap_or_else(|| {
            panic!("{name} isn't kept: {kept:?}");
        });
        std::fs::read_to_string(backups.join(copy)).unwrap()
    };
    assert_eq!(in_backup("assets/levels/docks.solarea"), docks);
    assert_eq!(in_backup("assets/levels/yard.solarea"), yard);
    assert_eq!(in_backup("assets/areas/turret.solarea"), turret);
    assert_eq!(in_backup(asset_db::PROJECT_FILE), project_file);
    assert_eq!(kept.len(), 4, "{kept:?}");
    // The Console is told both.
    let notes = opened.notes.join("\n");
    assert!(
        notes.contains("3 areas made by an older Solis were renamed"),
        "{notes}"
    );
    assert!(
        notes.contains("project.solis was saved in the current format"),
        "{notes}"
    );

    // Opened again: nothing moves and nothing more is kept.
    drop(projects);
    let mut again = Projects::default();
    let opened = again.open(&root, false, assets::Limits::default()).unwrap();
    assert!(
        !opened.notes.join("\n").contains("renamed"),
        "{:?}",
        opened.notes
    );
    assert_eq!(files_under(&backups).len(), 4);

    // A build from before levels (project format 1 at most) refuses the
    // project as one made by a newer Solis, by its project file's first
    // check, before it would look for any area.
    let found = asset_db::PROJECT_FORMAT.version_of(&now).unwrap();
    assert!(found > 1);
    assert_eq!(
        asset_db::ProjectError::TooNew(found).to_string(),
        "it was made by a newer version (format 2); update to open it"
    );
}
