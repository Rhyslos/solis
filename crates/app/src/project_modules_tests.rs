//! A project from before modules (decision 0118) opened by this build:
//! its assemblies become modules (`.module.lst`), the copies placed in
//! its levels and the game code that spawns them find them by the same
//! ID and name, the older files are kept, and nothing is lost.

use super::*;

fn fixture(path: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures")
            .join(path),
    )
    .unwrap()
}

/// Writes `text` as `path` inside `assets`, with a sidecar giving it `id`.
fn with_id(assets: &Path, path: &str, text: &str, id: &str) -> reflect::AssetId {
    let at = assets.join(path);
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::fs::write(&at, text).unwrap();
    let id: reflect::AssetId = id.parse().unwrap();
    let sidecar = asset_db::Sidecar {
        id,
        ..asset_db::Sidecar::new(asset_db::hash_file(&at).unwrap())
    };
    sidecar.write(&asset_db::sidecar::path_for(&at)).unwrap();
    id
}

/// A level placing a copy of `module` (as a version 4 level wrote it).
fn level_placing(module: reflect::AssetId, copy: &str) -> String {
    format!(
        "-- A level. Safe to edit by hand: one block per entity, one line per field.\n\
         level version 4;\n\n\
         entity {copy} {{\n    name = \"Copy\";\n    order = \"a0\";\n    \
         assembly = \"{module}\";\n    Transform.position = [4.0, 0.0, 0.0];\n}}\n"
    )
}

#[test]
fn an_old_project_s_assemblies_become_modules_and_every_copy_and_spawn_finds_them() {
    let root = reflect::test_support::TempFolder::new("modules-upgrade").unwrap();
    let assets = root.join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    let project_file = "name = \"Farm\"\nformat = 2\n";
    std::fs::write(root.join(asset_db::PROJECT_FILE), project_file).unwrap();
    // Three assemblies, of two layouts, in two folders.
    let crate_v2 = fixture("area/v2/crate.solassembly");
    let crate_v4 = fixture("area/v4/crate.solassembly");
    let crate_id = with_id(
        &assets,
        "props/crate.solassembly",
        &crate_v2,
        "5KD8V0Q2MX7TA",
    );
    let barrel = with_id(
        &assets,
        "props/barrel.solassembly",
        &crate_v4,
        "6BARRE7000001",
    );
    let chicken = with_id(
        &assets,
        "animals/chicken.solassembly",
        &crate_v4,
        "7CHC4EN000001",
    );
    // Copies of them in two levels, and game code spawning one.
    std::fs::write(
        assets.join("meadow.sollevel"),
        level_placing(chicken, "0J4M7Q2X9T5B1"),
    )
    .unwrap();
    std::fs::write(
        assets.join("yard.sollevel"),
        level_placing(crate_id, "0J4M7Q2X9T5B2"),
    )
    .unwrap();
    let code = "fetch \"solis/world\":World;\nfetch \"solis/project\":Module;\n\n\
                component Coop {\n    method update(float dt) {\n        \
                entity made = World:spawn(Module.Chicken, [0, 0, 0]);\n    }\n}\n\n\
                pass Coop;\nend\n";
    std::fs::write(assets.join("main.lst"), code).unwrap();

    let mut projects = Projects::default();
    let opened = projects
        .open(&root, false, assets::Limits::default())
        .unwrap();

    // Each assembly is a module now, beside where it was, the same module
    // by the same ID.
    let registry = level::builtin::registry().unwrap();
    let read = |text: &str| level::module_file::read(text, &registry).unwrap();
    for (name, old, id) in [
        ("props/crate", &crate_v2, crate_id),
        ("props/barrel", &crate_v4, barrel),
        ("animals/chicken", &crate_v4, chicken),
    ] {
        assert!(
            !assets.join(format!("{name}.solassembly")).exists(),
            "{name}"
        );
        let new_path = assets.join(format!("{name}.module.lst"));
        let new = std::fs::read_to_string(&new_path).unwrap();
        assert_eq!(read(&new), read(old), "{name}");
        assert_eq!(
            (level::module_file::FORMAT.version_of)(&new),
            Some(level::module_file::VERSION)
        );
        let listing = projects.listing();
        assert!(
            listing.files.iter().any(|f| f.id == Some(id)
                && f.kind == asset_db::FileKind::Module
                && f.path
                    .ends_with(format!("{}.module.lst", name.rsplit('/').next().unwrap()))),
            "{name}"
        );
    }
    // The levels' copies name the same IDs, which are the modules'.
    for (level_name, wanted) in [("meadow", chicken), ("yard", crate_id)] {
        let text = std::fs::read_to_string(assets.join(format!("{level_name}.sollevel"))).unwrap();
        let file = level::text::read_file(&text, &registry).unwrap();
        let placed: Vec<_> = file
            .level
            .iter()
            .filter_map(|(_, e)| e.origin.as_ref().map(|o| o.module))
            .collect();
        assert_eq!(placed, [wanted], "{level_name}");
    }
    // Game code names the chicken as before: Module.Chicken.
    let notes = crate::game_code::GameCode::describe(&assets, "Test 1.0");
    assert!(notes.is_empty(), "{notes:?}");
    let names = std::fs::read_to_string(assets.join("solis/project.lst")).unwrap();
    assert!(
        names.contains("state Module = [Barrel, Chicken, Crate];"),
        "{names}"
    );
    // The project file says format 3 now.
    let now = std::fs::read_to_string(root.join(asset_db::PROJECT_FILE)).unwrap();
    assert_eq!(
        reflect::upgrade::Versioned::version_of(&asset_db::PROJECT_FORMAT, &now),
        Some(3)
    );
    // Every older file is in the backups, as it was.
    let backups = asset_db::backup::folder(&root);
    let kept = files_under(&backups);
    assert_eq!(kept.len(), 4, "{kept:?}");
    let in_backup = |name: &str| {
        let copy = kept
            .iter()
            .find(|k| k.ends_with(name))
            .unwrap_or_else(|| panic!("{name} isn't kept: {kept:?}"));
        std::fs::read_to_string(backups.join(copy)).unwrap()
    };
    assert_eq!(in_backup("props/crate.solassembly"), crate_v2);
    assert_eq!(in_backup("props/barrel.solassembly"), crate_v4);
    assert_eq!(in_backup("animals/chicken.solassembly"), crate_v4);
    assert_eq!(in_backup(asset_db::PROJECT_FILE), project_file);
    let said = opened.notes.join("\n");
    assert!(
        said.contains("3 assemblies made by an older Solis were renamed"),
        "{said}"
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
}

#[test]
fn an_assembly_that_can_t_move_stays_and_still_opens() {
    let root = reflect::test_support::TempFolder::new("modules-stay").unwrap();
    let assets = root.join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(
        root.join(asset_db::PROJECT_FILE),
        "name = \"Farm\"\nformat = 2\n",
    )
    .unwrap();
    std::fs::write(
        assets.join("broken.solassembly"),
        "assembly version 1 root X;",
    )
    .unwrap();
    std::fs::write(
        assets.join("crate.solassembly"),
        fixture("area/v4/crate.solassembly"),
    )
    .unwrap();
    std::fs::write(assets.join("crate.module.lst"), "mine").unwrap();
    let mut projects = Projects::default();
    let opened = projects
        .open(&root, false, assets::Limits::default())
        .unwrap();
    let said = opened.notes.join("\n");
    assert_eq!(said.matches("wasn't renamed a module").count(), 2, "{said}");
    assert!(assets.join("broken.solassembly").exists());
    assert_eq!(
        std::fs::read_to_string(assets.join("crate.module.lst")).unwrap(),
        "mine"
    );
    // The one left is still a module in the Library, read as it is.
    let listing = projects.listing();
    assert!(
        listing
            .files
            .iter()
            .any(|f| f.kind == asset_db::FileKind::Module && f.path.ends_with("crate.solassembly"))
    );
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
