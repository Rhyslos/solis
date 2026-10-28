//! The benchmark (Phase 5, decision 0030): **Help > Make the benchmark…**
//! writes a project into a folder: two fleets of ships (a model Solis
//! makes, nothing proprietary) flying loops and firing lasers, written in
//! Lust, in three areas: the target level (2,000 ships and 20,000
//! lasers), the stress level (10,000 and 100,000) and the ceiling run,
//! which adds ships until 60 frames a second ends and says where.

use area::builtin::{Camera, Model, Transform};
use area::text::{AreaFile, FileKind};
use area::{Area, EntityData};
use asset_db::{Project, Sidecar};
use glam::{Quat, Vec3};
use reflect::{AssetId, AssetRef, ComponentData, EntityId, Id, Value};
use std::path::Path;

/// The benchmark's name, as its project file has it.
pub const NAME: &str = "Benchmark";

/// The areas, as (file in the assets, ships made; 0 for the ceiling run).
pub const AREAS: [(&str, u32); 3] = [
    ("target.solarea", 2_000),
    ("stress.solarea", 10_000),
    ("ceiling.solarea", 0),
];

/// The area the benchmark opens on.
pub const FIRST_AREA: &str = AREAS[0].0;

/// The ship's model, its assembly, and the game's code, inside the assets.
const SHIP_MODEL: &str = "ships/fighter.glb";
const SHIP_ASSEMBLY: &str = "ships/fighter.solassembly";
const CODE: &str = "main.lst";

/// The game's code, from the very file the project shows.
pub const MAIN: &str = include_str!("../../../samples/benchmark/assets/main.lst");

/// The ship model's ID, fixed so the assembly made here can name it.
fn ship_model() -> AssetId {
    AssetId(Id::from_raw(0x5015_BE4C_0000_0001).unwrap_or_else(Id::random))
}

/// An ID for the benchmark's own entities, fixed so its files are the
/// same every time.
fn entity(n: u64) -> EntityId {
    EntityId(Id::from_raw(0x5015_BE4C_0001_0000 + n).unwrap_or_else(Id::random))
}

/// Every file the benchmark writes, inside its assets: (path, bytes).
pub fn files() -> Vec<(String, Vec<u8>)> {
    let mut files = vec![
        (CODE.to_string(), MAIN.as_bytes().to_vec()),
        (SHIP_MODEL.to_string(), assets::sample::ship_glb()),
    ];
    let model = assets::sample::ship_glb();
    let mut hash = asset_db::ContentHash::default();
    hash.update(&model);
    let mut sidecar = Sidecar::new(hash.finish());
    sidecar.id = ship_model();
    files.push((
        format!("{SHIP_MODEL}.{}", asset_db::sidecar::EXTENSION),
        sidecar.to_text().into_bytes(),
    ));
    let registry = area::builtin::registry().ok();
    let empty = reflect::Registry::default();
    let registry = registry.as_ref().unwrap_or(&empty);
    let path_of = |id: AssetId| (id == ship_model()).then(|| SHIP_MODEL.to_string());
    files.push((
        SHIP_ASSEMBLY.to_string(),
        area::text::write_file(&fighter(), registry, &path_of).into_bytes(),
    ));
    for (name, ships) in AREAS {
        files.push((
            name.to_string(),
            area::text::write(&level(ships), registry, &path_of).into_bytes(),
        ));
    }
    files
}

/// The fighter assembly: one entity with the ship's model and its Lust
/// component.
fn fighter() -> AreaFile {
    let root = entity(1);
    let mut ship = EntityData::new("Fighter").with(&Transform::default());
    ship.set(&Model {
        asset: Some(AssetRef {
            asset: ship_model(),
            part: None,
        }),
    });
    ship.components
        .insert("Ship".into(), ComponentData::default());
    let mut area = Area::new();
    area.insert(root, ship);
    AreaFile {
        kind: FileKind::Assembly { root },
        area,
        overrides: Default::default(),
    }
}

/// An area for a level making `ships` ships (the ceiling run for 0): the
/// sun and sky, a camera looking over the field from one end, and the
/// entity whose component makes the fleets.
fn level(ships: u32) -> Area {
    let mut area = area::builtin::starter_area();
    // Looking along +Y, tilted 35° down, from behind the field and above.
    let camera = EntityData::new("Camera")
        .with(&Transform {
            position: Vec3::new(0.0, -330.0, 150.0),
            rotation: Quat::from_rotation_x(55_f32.to_radians()),
            ..Transform::default()
        })
        .with(&Camera {
            far: 3000.0,
            ..Camera::default()
        });
    area.insert(entity(10), camera);
    let mut fleets = EntityData::new("Fleets");
    let mut data = ComponentData::default();
    if ships == 0 {
        fleets.components.insert("Ceiling".into(), data);
    } else {
        data.set("ships", Value::Int(i64::from(ships)));
        fleets.components.insert("Fleets".into(), data);
    }
    area.insert(entity(11), fleets);
    area
}

/// Makes `root` the benchmark: a new project with its files. Refuses a
/// folder that's already a project, or that has a file where the
/// benchmark's would go (nothing of the person's is ever replaced).
pub fn make(root: &Path) -> Result<(), asset_db::ProjectError> {
    let files = files();
    for (path, _) in &files {
        let file = root.join(asset_db::ASSETS_FOLDER).join(path);
        if file.exists() {
            return Err(asset_db::ProjectError::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!(
                    "there's already a file called {}, which the benchmark would replace; pick \
                     an empty folder",
                    file.strip_prefix(root).unwrap_or(&file).display()
                ),
            )));
        }
    }
    let project = Project::create(root, NAME)?;
    for (path, bytes) in &files {
        let file = project.assets().join(path);
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder)?;
        }
        asset_db::write_atomically(&file, bytes)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compiled() -> script::Compiled {
        let mut files = script::describe::descriptions("Test Engine 9.9.9", "solis");
        files.push(("main.lst".into(), MAIN.to_string()));
        let code = script::Code {
            files,
            lua: Vec::new(),
            notes: Vec::new(),
        };
        match script::compile(&code) {
            Ok(Some(c)) => c,
            Ok(None) => panic!("no code"),
            Err(problems) => panic!("{}", problems[0]),
        }
    }

    #[test]
    fn the_benchmark_s_code_is_sound_and_has_its_components() {
        let code = compiled();
        let names: Vec<&str> = code.schemas.iter().map(|s| s.name.as_str()).collect();
        for wanted in ["Ship", "Fleets", "Ceiling"] {
            assert!(names.contains(&wanted), "{names:?}");
        }
    }

    #[test]
    fn its_files_read_back() {
        let registry = area::builtin::registry().unwrap();
        for (path, bytes) in files() {
            let text = String::from_utf8_lossy(&bytes);
            if path.ends_with(".solarea") || path.ends_with(".solassembly") {
                let file = area::text::read_file(&text, &registry)
                    .unwrap_or_else(|e| panic!("{path}: {e}"));
                assert!(!file.area.is_empty(), "{path}");
            }
            if path.ends_with(".solmeta") {
                let sidecar = Sidecar::parse(&text).unwrap();
                assert_eq!(sidecar.id, ship_model());
            }
        }
    }

    #[test]
    fn making_it_twice_in_one_folder_is_refused() {
        let folder = std::env::temp_dir().join(format!("solis-benchmark-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        make(&folder).unwrap();
        assert!(folder.join("assets/ships/fighter.glb").exists());
        assert!(folder.join("assets/ceiling.solarea").exists());
        assert!(make(&folder).is_err());
        let _ = std::fs::remove_dir_all(&folder);
    }
}

#[cfg(test)]
mod play_tests {
    use super::*;

    /// The benchmark made in a fresh folder, its code running in a
    /// workbench that knows its fighter, with `area` open and its fleets
    /// (or ceiling run) changed by `tweak`.
    pub(super) fn playing(
        name: &str,
        area: &str,
        tweak: impl Fn(&mut ComponentData),
    ) -> editor::Workbench {
        let root = std::env::temp_dir().join(format!("solis-bench-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        make(&root).unwrap();
        let assets = root.join(asset_db::ASSETS_FOLDER);
        crate::game_code::GameCode::describe(&assets, "Test 1.0");
        let compiled = match script::compile(&script::gather(&assets)) {
            Ok(Some(c)) => c,
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
        w.set_program(Some(std::sync::Arc::new(compiled)));
        let registry = w.document().registry.clone();
        let fighter = area::text::read_file(
            &std::fs::read_to_string(assets.join(SHIP_ASSEMBLY)).unwrap(),
            &registry,
        )
        .unwrap();
        let id = AssetId::random();
        w.set_assembly(id, std::sync::Arc::new(fighter));
        w.set_library(vec![editor::LibraryItem {
            id: Some(id),
            kind: editor::LibraryKind::Assembly,
            name: "fighter".into(),
            path: SHIP_ASSEMBLY.into(),
            thumbnail: None,
        }]);
        let mut level = area::text::read(
            &std::fs::read_to_string(assets.join(area)).unwrap(),
            &registry,
        )
        .unwrap();
        let fleets = level
            .iter()
            .find(|(_, e)| e.name == "Fleets")
            .map(|(id, _)| id)
            .unwrap();
        for data in level.get_mut(fleets).unwrap().components.values_mut() {
            tweak(data);
        }
        w.open_area(level, assets.join(area));
        let _ = std::fs::remove_dir_all(&root);
        w
    }

    #[test]
    fn the_target_level_makes_its_fleets_and_they_fly_and_fire() {
        let mut w = playing("target", "target.solarea", |d| {
            d.set("ships", Value::Int(60));
        });
        w.play();
        // A tick to spawn, one to land, then a second and a half of flying.
        for _ in 0..92 {
            w.play_for(1.0 / 60.0);
        }
        let said: Vec<&str> = w.log_lines().iter().map(|l| l.text.as_str()).collect();
        assert!(
            !said
                .iter()
                .any(|l| l.contains("stopped") || l.contains("spawned \"")),
            "{said:?}"
        );
        let world = w.world().unwrap();
        let ships = world.placed::<area::builtin::Model>();
        assert_eq!(ships.len(), 60, "{said:?}");
        // Ten lasers a ship once they've been flying long enough; after a
        // second and a half, nearly that.
        let lasers = world.projectiles().len();
        assert!((480..=600).contains(&lasers), "{lasers}");
        // Both fleets: red and green lasers.
        let flying = world.projectiles().flying();
        assert!(flying.iter().any(|p| p.colour[0] > 1.0));
        assert!(flying.iter().any(|p| p.colour[1] > 1.0));
    }

    #[test]
    fn the_ceiling_run_adds_ships_until_60_fps_ends_and_says_where() {
        let mut w = playing("ceiling", "ceiling.solarea", |d| {
            d.set("step", Value::Int(20));
        });
        w.play();
        // At 60 frames a second the first 20 ships pass: 20 more are made.
        for _ in 0..200 {
            w.play_for(1.0 / 60.0);
        }
        // Then this "computer" slows to 30 frames a second.
        for _ in 0..200 {
            w.play_for(1.0 / 30.0);
        }
        let said: Vec<&str> = w.log_lines().iter().map(|l| l.text.as_str()).collect();
        assert!(
            said.iter().any(|l| l.contains("starts with 20 ships")),
            "{said:?}"
        );
        assert!(
            said.iter()
                .any(|l| l.contains("20 ships and 200 lasers: 60 frames a second")),
            "{said:?}"
        );
        assert!(
            said.iter()
                .any(|l| l.contains("keeps 60 frames a second up to about 20 ships and 200 lasers")),
            "{said:?}"
        );
    }
}

#[cfg(test)]
mod picture_tests {
    use super::play_tests::playing;
    use eframe::wgpu;

    /// Plays the target level for two seconds and draws the Game panel's
    /// view to `DUMP_DIR/benchmark.png`, to look at by hand.
    #[test]
    #[ignore = "draws a picture to look at: DUMP_DIR=… cargo test -p app benchmark_picture -- --ignored"]
    fn benchmark_picture() {
        let Some(dir) = std::env::var_os("DUMP_DIR") else {
            return;
        };
        let mut w = playing("picture", "target.solarea", |_| {});
        w.play();
        for _ in 0..150 {
            w.play_for(1.0 / 60.0);
        }
        let world = w.world().unwrap();
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle().with_env());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .unwrap();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let renderer = render::ViewportRenderer::new(&device, render::best_sample_count(&adapter));
        let size = [1280, 720];
        let mut target = renderer.create_target(&device, size);
        let model = assets::load_bytes(
            &assets::sample::ship_glb(),
            None,
            "Ship",
            &assets::Limits::default(),
        )
        .unwrap();
        let ship = renderer.upload_model(&device, &queue, &model);
        let placed: Vec<render::Placement<'_>> = world
            .placed::<area::builtin::Model>()
            .into_iter()
            .map(|(_, _, transform)| render::Placement {
                model: &ship,
                node: None,
                transform,
                pick: 0,
            })
            .collect();
        // As the Game panel makes them (the same layout as the renderer's).
        let made: Vec<editor::ViewBolt> = world
            .projectiles()
            .flying()
            .iter()
            .map(editor::ViewBolt::of)
            .collect();
        let bolts: &[render::Bolt] = bytemuck::cast_slice(&made);
        let (_, camera, global) = world.camera().unwrap();
        let (_, rotation, position) = global.to_scale_rotation_translation();
        let view = glam::Mat4::from_rotation_translation(rotation, position).inverse();
        let params = render::FrameParams {
            view_projection: camera.projection(size[0] as f32 / size[1] as f32) * view,
            camera_position: position,
            focus: position,
            sky_top: [0.29, 0.29, 0.31],
            sky_horizon: [0.24, 0.24, 0.26],
            ground: [0.22, 0.22, 0.22],
            shading: render::Shading::default(),
            size_px: size,
            sun: render::Sun {
                direction: glam::Vec3::new(0.4, 0.3, -0.85).normalize(),
                ..render::Sun::default()
            },
            environment: render::Environment::default(),
            editor_lines: false,
            grid_plane: render::GridPlane::Ground,
            grid_step: 1.0,
        };
        let stats =
            renderer.render_with_bolts(&device, &queue, &mut target, &params, &placed, &[], bolts);
        let pixels = render::read_pixels(&device, &queue, &target).unwrap();
        let image = image::RgbaImage::from_raw(size[0], size[1], pixels).unwrap();
        image
            .save(std::path::Path::new(&dir).join("benchmark.png"))
            .unwrap();
        println!("{} ships, {} lasers: {stats:?}", placed.len(), bolts.len());
    }
}
