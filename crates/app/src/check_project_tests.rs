//! A project's check: a playground that's unchanged checks out, its
//! recording plays back the same, and a changed or damaged one fails.

use super::*;

const MADE_BY: &str = "Test 1.0";

/// A new playground in its own folder, with a recording of a few seconds
/// of play kept as a check.
fn playground_with_a_check(name: &str) -> PathBuf {
    let root = std::env::temp_dir()
        .join(format!("solis-check-{name}-{}", std::process::id()))
        .join("Playground");
    let _ = std::fs::remove_dir_all(root.parent().unwrap());
    crate::playground::make(&root).unwrap();
    let project = asset_db::Project::open(&root).unwrap();
    let mut quiet = Report::default();
    let program = check_code(&project.assets(), MADE_BY, &mut quiet);
    assert!(program.is_some(), "{:?}", quiet.lines);
    let (db, _, _) = asset_db::AssetDb::open(project, assets::Limits::default());
    let mut w = workbench_for(&root, &db, program, MADE_BY, crate::playground::AREA).unwrap();
    w.play_and_record();
    for _ in 0..90 {
        w.play_for(1.0 / 60.0);
    }
    let text = w.recording_text().expect("recording");
    assert!(text.contains(&format!("\narea {}\n", crate::playground::AREA)));
    let checks = root.join(RECORDINGS).join(CHECKS);
    std::fs::create_dir_all(&checks).unwrap();
    std::fs::write(checks.join("walk.solrec"), text).unwrap();
    root
}

#[test]
fn an_unchanged_project_checks_out() {
    let root = playground_with_a_check("same");
    let report = check(&root, MADE_BY);
    let text = report.text();
    assert_eq!(report.failed, 0, "{text}");
    assert!(text.contains("files read"), "{text}");
    assert!(text.contains("the game's code checks"), "{text}");
    assert!(
        text.contains("plays back the same on all 90 ticks"),
        "{text}"
    );
    assert!(text.ends_with("nothing changed.\n"), "{text}");
    let _ = std::fs::remove_dir_all(root.parent().unwrap());
}

#[test]
fn a_changed_area_or_a_damaged_file_fails() {
    let root = playground_with_a_check("changed");
    // A crate moved in the area: the recording no longer plays the same.
    let area = root.join("assets").join(crate::playground::AREA);
    let text = std::fs::read_to_string(&area).unwrap();
    let moved = text.replacen("name = \"Crate", "name = \"Crate moved", 1);
    let moved = {
        let at = moved.find("name = \"Crate moved").unwrap();
        let line = moved[at..].find("Transform.position = [").unwrap() + at;
        let start = line + "Transform.position = [".len();
        format!("{}9.5, {}", &moved[..start], &moved[start..])
    };
    std::fs::write(&area, moved).unwrap();
    // A damaged sound event, and a recording that names no area.
    std::fs::write(
        root.join("assets/broken.solsound"),
        "sound version 1;\nvolume = \"loud\";",
    )
    .unwrap();
    std::fs::write(
        root.join(RECORDINGS).join(CHECKS).join("old.solrec"),
        "solis recording 1\nseed 1\nstart 0\n",
    )
    .unwrap();
    let report = check(&root, MADE_BY);
    let text = report.text();
    assert!(report.failed >= 3, "{text}");
    assert!(text.contains("FAIL  assets/broken.solsound"), "{text}");
    assert!(text.contains("doesn't say which area"), "{text}");
    assert!(text.contains("walk.solrec"), "{text}");
    assert!(text.contains("isn't as it was when this was recorded"), "{text}");
    assert!(text.contains("Don't ship it"), "{text}");
    let _ = std::fs::remove_dir_all(root.parent().unwrap());
}

#[test]
fn a_folder_that_isn_t_a_project_fails_at_once() {
    let report = check(Path::new("/nowhere/at/all"), MADE_BY);
    assert_eq!(report.failed, 1);
    assert!(report.text().contains("can't be opened as a project"));
}

#[test]
fn an_area_outside_the_assets_is_refused() {
    let root = std::env::temp_dir().join(format!("solis-check-outside-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    asset_db::Project::create(&root, "Out").unwrap();
    let project = asset_db::Project::open(&root).unwrap();
    let (db, _, _) = asset_db::AssetDb::open(project, assets::Limits::default());
    for bad in [
        "../project.solis",
        "a//b.solarea",
        "",
        "a\\..\\..\\x.solarea",
        "C:/x.solarea",
    ] {
        let refused = workbench_for(&root, &db, None, MADE_BY, bad).err().unwrap();
        assert!(refused.contains("isn't in the assets"), "{bad}: {refused}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_samples_check_out() {
    // Each sample project Solis makes, as a new build would open it
    // (Phase 8, C4: the samples are checked on every pull request).
    type Make = fn(&Path) -> Result<(), asset_db::ProjectError>;
    for (name, make) in [
        ("turret", crate::sample_game::make as Make),
        ("benchmark", crate::benchmark::make as Make),
        ("playground", crate::playground::make as Make),
    ] {
        let root = std::env::temp_dir()
            .join(format!("solis-check-sample-{}", std::process::id()))
            .join(name);
        let _ = std::fs::remove_dir_all(&root);
        make(&root).unwrap();
        let report = check(&root, MADE_BY);
        assert_eq!(report.failed, 0, "{name}:\n{}", report.text());
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[test]
fn a_file_bigger_than_its_kind_allows_fails_without_being_read() {
    // Regression (the Phase 8 review): each kind of file is read within the
    // editor's own limit, so a planted huge sidecar can't use up memory.
    let root = std::env::temp_dir().join(format!("solis-check-big-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    asset_db::Project::create(&root, "Big").unwrap();
    let big = vec![b'#'; asset_db::sidecar::MAX_BYTES as usize + 1];
    std::fs::write(root.join("assets/huge.glb.solmeta"), big).unwrap();
    let report = check(&root, MADE_BY);
    let text = report.text();
    assert!(text.contains("FAIL  assets/huge.glb.solmeta"), "{text}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_recording_that_saves_plays_back_the_same_and_writes_nothing() {
    // The Phase 8 review: the check didn't answer game code's saves and
    // loads. A recording ends at a load (it plays back from Play), so only
    // saves come up while one plays back: they're kept by the check, never
    // written into the project.
    const CODE: &str = "get \"solis/world\":[saveGame, loadGame];\n\
        get \"solis/entity\":[positionOf, moveTo];\n\
        component Hopper {\n\
            has { int ticks = 0; }\n\
            method update(float dt) {\n\
                ticks = ticks + 1;\n\
                moveTo(me, positionOf(me) + [0.1, 0, 0]);\n\
                is ticks == 10? then { saveGame(\"quick\"); }\n\
                is ticks == 20? then { loadGame(\"quick\"); }\n\
            }\n\
        }\n\
        pass Hopper;\nend\n";
    let root = std::env::temp_dir().join(format!("solis-check-hop-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    asset_db::Project::create(&root, "Hop").unwrap();
    let assets = root.join("assets");
    std::fs::write(assets.join("main.lst"), CODE).unwrap();
    let mut area = area::Area::new();
    let mut hopper = area::EntityData::new("Hopper").with(&area::builtin::Transform::default());
    hopper
        .components
        .insert("Hopper".into(), reflect::ComponentData::default());
    area.spawn(hopper);
    let registry = area::builtin::registry().unwrap();
    std::fs::write(
        assets.join("hop.solarea"),
        area::text::write(&area, &registry, &|_| None),
    )
    .unwrap();
    let project = asset_db::Project::open(&root).unwrap();
    let mut quiet = Report::default();
    let program = check_code(&project.assets(), MADE_BY, &mut quiet);
    assert!(program.is_some(), "{:?}", quiet.lines);
    let (db, _, _) = asset_db::AssetDb::open(project, assets::Limits::default());
    let mut w = workbench_for(&root, &db, program, MADE_BY, "hop.solarea").unwrap();
    w.play_and_record();
    let mut saved = Saves::default();
    for _ in 0..15 {
        w.play_for(1.0 / 60.0);
        saved.answer(&mut w, &root).unwrap();
    }
    let checks = root.join(RECORDINGS).join(CHECKS);
    std::fs::create_dir_all(&checks).unwrap();
    std::fs::write(checks.join("hop.solrec"), w.recording_text().unwrap()).unwrap();
    let report = check(&root, MADE_BY);
    assert_eq!(report.failed, 0, "{}", report.text());
    assert!(!root.join("saves").exists(), "the check writes no saves");
    let _ = std::fs::remove_dir_all(&root);
}
