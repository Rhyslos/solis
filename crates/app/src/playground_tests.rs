//! The playground: its files read back, it's made as a project without
//! replacing anything, and it plays: the character lands, the camera
//! follows it, crates fall into place and the platform slides.

use super::*;

fn compiled(assets: &Path) -> script::Compiled {
    crate::game_code::GameCode::describe(assets, "Test 1.0");
    let code = script::gather(assets);
    match script::compile(&code) {
        Ok(Some(compiled)) => compiled,
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    }
}

fn find(w: &editor::Workbench, name: &str) -> EntityId {
    w.document()
        .area()
        .iter()
        .find(|(_, e)| e.name == name)
        .map(|(id, _)| id)
        .unwrap_or_else(|| panic!("no {name}"))
}

fn at(w: &editor::Workbench, id: EntityId) -> Vec3 {
    w.world().unwrap().global(id).unwrap().w_axis.truncate()
}

#[test]
fn its_files_read_back() {
    let registry = area::builtin::registry().unwrap();
    for (path, bytes) in files() {
        let text = String::from_utf8_lossy(&bytes);
        if path.ends_with(".solarea") {
            let area = area::text::read(&text, &registry).unwrap();
            assert!(area.len() > 15, "{path}");
        }
        if path.ends_with(".solmeta") {
            Sidecar::parse(&text).unwrap();
        }
    }
    let bindings = input::Bindings::read(&bindings().write()).unwrap();
    assert_eq!(bindings.get("QuickSave").unwrap().inputs, ["F6"]);
    assert_eq!(bindings.get("QuickLoad").unwrap().inputs, ["F7"]);
    assert!(bindings.get("Zoom").is_some(), "the cameras' actions too");
}

#[test]
fn the_playground_is_made_as_a_project_and_plays() {
    let root = std::env::temp_dir()
        .join(format!("solis-playground-{}", std::process::id()))
        .join("Playground");
    let _ = std::fs::remove_dir_all(&root);
    make(&root).unwrap();
    assert!(make(&root).is_err(), "not over an existing project");
    let project = Project::open(&root).unwrap();
    assert_eq!(project.name(), NAME);
    let assets = project.assets();
    let compiled = compiled(&assets);
    let mut w = editor::Workbench::new(
        editor::AppInfo {
            name: "Test".into(),
            version: "1.0".into(),
        },
        None,
    );
    w.set_program(Some(std::sync::Arc::new(compiled)));
    for name in [
        "CharacterController",
        "ThirdPersonCamera",
        "FirstPersonCamera",
        "RtsCamera",
        "QuickSave",
        "Platform",
        "CharacterBody",
    ] {
        assert!(
            w.document().registry.get(name).is_some(),
            "{name} in Properties"
        );
    }
    let bindings = std::fs::read_to_string(root.join(input::FILE)).unwrap();
    w.set_bindings(input::Bindings::read(&bindings).unwrap());
    let text = std::fs::read_to_string(assets.join(AREA)).unwrap();
    let area = area::text::read(&text, &w.document().registry).unwrap();
    w.open_area(area, assets.join(AREA));
    let (player, camera, platform, stacked) = (
        find(&w, "Player"),
        find(&w, "Camera"),
        find(&w, "Moving platform"),
        find(&w, "Crate 4"),
    );
    w.play();
    for _ in 0..(60 * 2) {
        w.play_for(1.0 / 60.0);
    }
    let problems: Vec<&str> = w
        .log_lines()
        .iter()
        .map(|l| l.text.as_str())
        .filter(|t| t.contains("stopped") || t.contains("Couldn't"))
        .collect();
    assert!(problems.is_empty(), "{problems:?}");
    // The character stands on the floor where it was put.
    let feet = at(&w, player);
    assert!(feet.z.abs() < 0.05 && (feet.y + 4.0).abs() < 0.05, "{feet}");
    // The camera is behind it (it faces +Y), above, looking its way.
    let eye = at(&w, camera);
    assert!(
        eye.y < feet.y - 2.0 && eye.z > feet.z + 1.0,
        "camera at {eye}"
    );
    // The stacked crate is still on the two below.
    assert!((at(&w, stacked).z - 1.5).abs() < 0.1, "{}", at(&w, stacked));
    // The platform has slid a third of its way in two seconds of six.
    let slid = at(&w, platform);
    assert!(slid.x > 5.0 && slid.x < 13.0, "platform at {slid}");
}

#[test]
fn the_playground_never_replaces_a_file_of_the_person_s() {
    let root = std::env::temp_dir()
        .join(format!("solis-playground-mine-{}", std::process::id()))
        .join("My game");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join(input::FILE), "# mine\n").unwrap();
    let why = make(&root).unwrap_err().to_string();
    assert!(why.contains(input::FILE), "{why}");
    assert_eq!(
        std::fs::read_to_string(root.join(input::FILE)).unwrap(),
        "# mine\n"
    );
    assert!(!root.join(asset_db::PROJECT_FILE).exists());
}
