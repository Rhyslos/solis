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
    // Every sound event reads, and each take is a sound file it writes
    // (by the ID in that file's sidecar).
    let files = files();
    let ids: Vec<reflect::AssetId> = files
        .iter()
        .filter(|(p, _)| p.ends_with(".wav.solmeta"))
        .map(|(_, b)| Sidecar::parse(&String::from_utf8_lossy(b)).unwrap().id)
        .collect();
    assert_eq!(ids.len(), 7);
    let mut events = 0;
    for (path, bytes) in &files {
        if path.ends_with(".solsound") {
            let read = sound::event::read(&String::from_utf8_lossy(bytes)).unwrap();
            assert!(read.notes.is_empty(), "{path}: {:?}", read.notes);
            assert!(!read.event.takes.is_empty(), "{path}");
            assert!(read.event.takes.iter().all(|t| ids.contains(t)), "{path}");
            assert!(
                sound::Mixer::default().index_of(&read.event.bus).is_some(),
                "{path}"
            );
            events += 1;
        }
        if path.ends_with(".wav") {
            sound::read_bytes(bytes).unwrap();
        }
    }
    assert_eq!(events, 4);
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
    // Its sounds (S7): the music, the radio and the drip keep playing;
    // the cave echoes; no footsteps while the character stands still.
    let mut playing: Vec<String> = w
        .audio_sources()
        .into_iter()
        .map(|(_, s, _)| s.event)
        .collect();
    playing.sort();
    assert_eq!(playing, ["music/theme", "sfx/drip", "voice/radio"]);
    assert_eq!(w.reverb_zones().len(), 1);
    let steps = w
        .take_game_sounds()
        .iter()
        .filter(|s| matches!(&s.request, editor::game_sound::Request::Play(name, _) if name == "sfx/footstep"))
        .count();
    assert_eq!(steps, 0, "standing still");
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

#[test]
fn entity_fields_survive_the_area_opening_before_the_code_is_checked() {
    // The user test: making the playground opens its area at once, before
    // its new code is checked, so the camera's target and the
    // character's camera were read as text and showed as nothing.
    let root = std::env::temp_dir()
        .join(format!("solis-playground-order-{}", std::process::id()))
        .join("Playground");
    let _ = std::fs::remove_dir_all(&root);
    make(&root).unwrap();
    let assets = Project::open(&root).unwrap().assets();
    let compiled = compiled(&assets);
    let mut w = editor::Workbench::new(
        editor::AppInfo {
            name: "Test".into(),
            version: "1.0".into(),
        },
        None,
    );
    // The area first, as the app does.
    let text = std::fs::read_to_string(assets.join(AREA)).unwrap();
    let area = area::text::read(&text, &w.document().registry).unwrap();
    w.open_area(area, assets.join(AREA));
    w.set_program(Some(std::sync::Arc::new(compiled)));
    let (player, camera) = (find(&w, "Player"), find(&w, "Camera"));
    let field = |entity: EntityId, component: &str, field: &str| {
        w.document()
            .area()
            .get(entity)
            .unwrap()
            .components
            .get(component)
            .unwrap()
            .get(field)
            .cloned()
    };
    assert_eq!(
        field(camera, "ThirdPersonCamera", "target"),
        Some(Value::Entity(Some(player)))
    );
    assert_eq!(
        field(player, "CharacterController", "camera"),
        Some(Value::Entity(Some(camera)))
    );
    assert!(!w.document().is_dirty(), "not an unsaved change");
    let _ = std::fs::remove_dir_all(root.parent().unwrap());
}

#[test]
fn footsteps_follow_ground_covered_and_never_while_still() {
    let root = std::env::temp_dir()
        .join(format!("solis-playground-steps-{}", std::process::id()))
        .join("Playground");
    let _ = std::fs::remove_dir_all(&root);
    make(&root).unwrap();
    let assets = Project::open(&root).unwrap().assets();
    let mut w = editor::Workbench::new(
        editor::AppInfo {
            name: "Test".into(),
            version: "1.0".into(),
        },
        None,
    );
    w.set_program(Some(std::sync::Arc::new(compiled(&assets))));
    // A walker carried 10 m along the ground in 3 s (the platform's
    // code moves it), and one standing still.
    let mut area = area::Area::new();
    let mut walk = ComponentData::default();
    walk.set("from", Value::vec3(Vec3::ZERO));
    walk.set("to", Value::vec3(Vec3::new(10.0, 0.0, 0.0)));
    walk.set("seconds", Value::Float(6.0));
    let mut walker = EntityData::new("Walker").with(&Transform::default());
    walker.components.insert("Platform".into(), walk);
    walker
        .components
        .insert("Footsteps".into(), ComponentData::default());
    let walker = area.spawn(walker);
    let mut still = EntityData::new("Still").with(&Transform::default());
    still
        .components
        .insert("Footsteps".into(), ComponentData::default());
    let still = area.spawn(still);
    w.open_area(area, assets.join("steps.solarea"));
    w.play();
    for _ in 0..(60 * 3) {
        w.play_for(1.0 / 60.0);
    }
    let sounds = w.take_game_sounds();
    let on = |who: EntityId| {
        sounds
            .iter()
            .filter(|s| match &s.request {
                editor::game_sound::Request::Play(name, script::SoundAt::On(e)) => {
                    name == "sfx/footstep" && *e == who
                }
                _ => false,
            })
            .count()
    };
    let steps = on(walker);
    // 10 m at a stride of 0.8 m: about 12 steps (the start is gentle).
    assert!((10..=13).contains(&steps), "{steps} steps");
    assert_eq!(on(still), 0, "none while standing");
    let _ = std::fs::remove_dir_all(&root);
}
