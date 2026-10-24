use super::*;
use reflect::Value;

/// Writes the sample's area (run once with SOLIS_WRITE_SAMPLE=1 to make
/// `samples/turret/assets/turret.solarea`; the file is committed).
#[test]
#[ignore = "makes the sample's area file"]
fn write_the_sample_area() {
    use area::builtin::{Camera, Transform};
    use area::{Area, EntityData};
    use glam::{Mat3, Quat, Vec3};
    let mut area = Area::new();
    let starter = area::builtin::starter_area();
    // Each after the last, so the Area panel lists them as written here.
    let add = |area: &mut Area, mut data: EntityData| {
        data.order = area.order_at_end(None);
        area.spawn(data)
    };
    for (_, entity) in starter.iter() {
        add(&mut area, entity.clone());
    }
    let place = |x: f32, y: f32, z: f32| Transform {
        position: Vec3::new(x, y, z),
        ..Transform::default()
    };
    let mut drone = EntityData::new("Drone").with(&place(0.0, 10.0, 4.0));
    for component in ["Health", "Armour", "Sparks"] {
        drone
            .components
            .insert(component.into(), reflect::ComponentData::default());
    }
    let drone = add(&mut area, drone);
    let mut turret = EntityData::new("Turret").with(&place(0.0, 0.0, 1.0));
    let mut gun = reflect::ComponentData::default();
    gun.set("target", Value::Entity(Some(drone)));
    turret.components.insert("Turret".into(), gun);
    add(&mut area, turret);
    // A camera behind and above the turret, looking at the drone.
    let eye = Vec3::new(8.0, -10.0, 6.0);
    let ahead = (Vec3::new(0.0, 10.0, 4.0) - eye).normalize();
    let right = ahead.cross(Vec3::Z).normalize();
    let up = right.cross(ahead);
    let rotation = Quat::from_mat3(&Mat3::from_cols(right, up, -ahead));
    add(
        &mut area,
        EntityData::new("Camera")
            .with(&Transform {
                position: eye,
                rotation,
                ..Transform::default()
            })
            .with(&Camera::default()),
    );
    let registry = area::builtin::registry().unwrap();
    let text = area::text::write(&area, &registry, &|_| None);
    if std::env::var_os("SOLIS_WRITE_SAMPLE").is_some() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/turret/assets")
            .join(AREA);
        std::fs::write(path, text).unwrap();
    }
}

#[test]
fn the_sample_is_made_as_a_project_and_plays_through_all_three_languages() {
    let root = std::env::temp_dir()
        .join(format!("solis-sample-{}", std::process::id()))
        .join("Turret sample");
    let _ = std::fs::remove_dir_all(&root);
    make(&root).unwrap();
    assert!(make(&root).is_err(), "not over an existing project");
    let project = Project::open(&root).unwrap();
    assert_eq!(project.name(), NAME);
    let assets = project.assets();
    crate::game_code::GameCode::describe(&assets, "Test 1.0");
    let code = script::gather(&assets);
    let compiled = match script::compile(&code) {
        Ok(Some(compiled)) => compiled,
        Ok(None) => panic!("no code"),
        Err(problems) => panic!("{}", problems[0]),
    };
    let mut w = editor::Workbench::new(
        editor::AppInfo {
            name: "Test".into(),
            version: "1.0".into(),
        },
        None,
    );
    let (rust, note) = rust_components(&assets);
    assert!(note.is_none(), "the file is the built-in one");
    w.set_rust_components(rust);
    w.set_program(Some(std::sync::Arc::new(compiled)));
    for name in ["Turret", "Health", "Armour", "Sparks"] {
        assert!(
            w.document().registry.get(name).is_some(),
            "{name} in Properties"
        );
    }
    let text = std::fs::read_to_string(assets.join(AREA)).unwrap();
    let area = area::text::read(&text, &w.document().registry).unwrap();
    let drone = area
        .iter()
        .find(|(_, e)| e.name == "Drone")
        .map(|(id, _)| id)
        .unwrap();
    w.open_area(area, assets.join(AREA));
    w.play();
    // Eight hits at half a second each: about four seconds of game.
    for _ in 0..(60 * 5) {
        w.play_for(1.0 / 60.0);
    }
    let said: Vec<&str> = w.log_lines().iter().map(|l| l.text.as_str()).collect();
    let count = |start: &str| said.iter().filter(|t| t.starts_with(start)).count();
    assert_eq!(count("Sparks fly from Drone"), 3, "{said:?}");
    assert_eq!(count("Drone is destroyed!"), 1, "{said:?}");
    let x = w.world().unwrap().global(drone).unwrap().w_axis;
    assert_eq!(x.z, 0.0, "it dropped to the ground");
    // A changed armour.rs is noted: changes don't run until Phase 10.
    std::fs::write(assets.join("armour.rs"), "// mine\n").unwrap();
    assert!(rust_components(&assets).1.is_some());
}

#[test]
fn the_sample_never_replaces_a_file_of_the_person_s() {
    let root = std::env::temp_dir()
        .join(format!("solis-sample-mine-{}", std::process::id()))
        .join("My game");
    let _ = std::fs::remove_dir_all(&root);
    let assets = root.join(asset_db::ASSETS_FOLDER);
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(assets.join("main.lst"), "-- mine\nend\n").unwrap();
    let why = make(&root).unwrap_err().to_string();
    assert!(why.contains("main.lst"), "{why}");
    assert_eq!(
        std::fs::read_to_string(assets.join("main.lst")).unwrap(),
        "-- mine\nend\n"
    );
    assert!(
        !root.join(asset_db::PROJECT_FILE).exists(),
        "nothing was made"
    );
}

#[test]
fn armour_rs_with_windows_line_endings_is_still_the_built_in_one() {
    let assets = std::env::temp_dir()
        .join(format!("solis-sample-crlf-{}", std::process::id()))
        .join("assets");
    let _ = std::fs::remove_dir_all(&assets);
    std::fs::create_dir_all(&assets).unwrap();
    let built_in = FILES.iter().find(|(p, _)| *p == "armour.rs").unwrap().1;
    let windows = built_in.replace("\r\n", "\n").replace('\n', "\r\n");
    std::fs::write(assets.join("armour.rs"), windows).unwrap();
    assert!(rust_components(&assets).1.is_none());
}
