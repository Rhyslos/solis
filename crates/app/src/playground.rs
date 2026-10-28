//! The playground (Phase 6, M5): **Help > Make the playground…** writes a
//! project into a folder for trying the character, the cameras, physics
//! and saves: a level with steps, ramps (one too steep to walk up), crates
//! to push and a moving platform, a character with the ready-made
//! controller and third-person camera, and quick saves on F6 and F7. It's
//! the level the Phase 6 gate is tried in.
//!
//! Phase 7, S7 (the sound gate's level): footsteps from the character's
//! game code, a music loop, a radio whose chatter ducks the music, and a
//! cave with a reverb zone and a drip. The sounds are made in code
//! (`playground_sounds.rs`).

use area::builtin::{
    AudioSource, BodyKind, Camera, CharacterBody, Collider, Model, ReverbZone, RigidBody,
    Transform, ZoneShape,
};
use area::{Area, EntityData};
use asset_db::{Project, Sidecar};
use glam::{Quat, Vec3};
use reflect::{AssetId, AssetRef, ComponentData, EntityId, Id, Value};
use std::path::Path;

/// The playground's name, as its project file has it.
pub const NAME: &str = "Playground";

/// The area it opens on.
pub const AREA: &str = "playground.solarea";

/// The game's code, from the very file the project shows.
pub const MAIN: &str = include_str!("../../../samples/playground/assets/main.lst");

/// The models, as (name in `assets::sample::playground_glb`, file in the
/// assets).
const MODELS: [(&str, &str); 4] = [
    ("block", "models/block.glb"),
    ("crate", "models/crate.glb"),
    ("platform", "models/platform.glb"),
    ("character", "models/character.glb"),
];

/// Makes a sound file's bytes.
type MakeSound = fn() -> Vec<u8>;

/// The sound files, as (file in the assets, how it's made).
const SOUNDS: [(&str, MakeSound); 7] = [
    ("sounds/footstep_1.wav", || {
        crate::playground_sounds::footstep(0)
    }),
    ("sounds/footstep_2.wav", || {
        crate::playground_sounds::footstep(1)
    }),
    ("sounds/footstep_3.wav", || {
        crate::playground_sounds::footstep(2)
    }),
    ("sounds/footstep_4.wav", || {
        crate::playground_sounds::footstep(3)
    }),
    ("sounds/music.wav", crate::playground_sounds::music),
    ("sounds/radio.wav", crate::playground_sounds::radio),
    ("sounds/drip.wav", crate::playground_sounds::drip),
];

/// A sound file's ID, fixed so the sound events can name it.
fn sound_id(path: &str) -> AssetId {
    let n = SOUNDS.iter().position(|(p, _)| *p == path).unwrap_or(0) as u64;
    AssetId(Id::from_raw(0x5015_9A7E_0002_0001 + n).unwrap_or_else(Id::random))
}

/// The sound events, as (file in the assets, the event).
fn sound_events() -> Vec<(&'static str, sound::SoundEvent)> {
    use sound::event::{Fade, Space};
    let placed = |takes: Vec<AssetId>, bus: &str, near_m: f32, far_m: f32| sound::SoundEvent {
        takes,
        bus: bus.to_owned(),
        space: Space::Placed,
        near_m,
        far_m,
        fade: Fade::Realistic,
        ..sound::SoundEvent::default()
    };
    vec![
        (
            "sfx/footstep.solsound",
            sound::SoundEvent {
                volume_db: -4.0,
                volume_range_db: 2.0,
                pitch_range_st: 1.0,
                ..placed(
                    (1..=4)
                        .map(|i| sound_id(&format!("sounds/footstep_{i}.wav")))
                        .collect(),
                    "Effects",
                    2.0,
                    25.0,
                )
            },
        ),
        (
            "sfx/drip.solsound",
            sound::SoundEvent {
                looping: true,
                volume_db: -6.0,
                ..placed(vec![sound_id("sounds/drip.wav")], "Ambience", 2.0, 20.0)
            },
        ),
        (
            "voice/radio.solsound",
            sound::SoundEvent {
                looping: true,
                priority: sound::event::Priority::High,
                ..placed(vec![sound_id("sounds/radio.wav")], "Voice", 3.0, 25.0)
            },
        ),
        (
            "music/theme.solsound",
            sound::SoundEvent {
                takes: vec![sound_id("sounds/music.wav")],
                bus: "Music".to_owned(),
                space: Space::Flat,
                looping: true,
                volume_db: -8.0,
                priority: sound::event::Priority::High,
                ..sound::SoundEvent::default()
            },
        ),
    ]
}

/// A model's ID, fixed so the area made here can name it.
fn model(name: &str) -> AssetId {
    let n = MODELS.iter().position(|(m, _)| *m == name).unwrap_or(0) as u64;
    AssetId(Id::from_raw(0x5015_9A7E_0000_0001 + n).unwrap_or_else(Id::random))
}

/// An ID for the playground's own entities, fixed so its files are the
/// same every time.
fn entity(n: u64) -> EntityId {
    EntityId(Id::from_raw(0x5015_9A7E_0001_0000 + n).unwrap_or_else(Id::random))
}

/// The input actions: the ready-made ones, and QuickSave (F6) and
/// QuickLoad (F7), chosen by the project lead.
pub fn bindings() -> input::Bindings {
    let mut bindings = input::Bindings::default();
    bindings
        .actions
        .push(input::Action::button("QuickSave", &["F6"]));
    bindings
        .actions
        .push(input::Action::button("QuickLoad", &["F7"]));
    bindings
}

/// Every file the playground writes, inside its assets: (path, bytes).
pub fn files() -> Vec<(String, Vec<u8>)> {
    let mut files = vec![("main.lst".to_string(), MAIN.as_bytes().to_vec())];
    for (name, path) in MODELS {
        let glb = assets::sample::playground_glb(name);
        let mut hash = asset_db::ContentHash::default();
        hash.update(&glb);
        let mut sidecar = Sidecar::new(hash.finish());
        sidecar.id = model(name);
        files.push((
            format!("{path}.{}", asset_db::sidecar::EXTENSION),
            sidecar.to_text().into_bytes(),
        ));
        files.push((path.to_string(), glb));
    }
    for (path, make) in SOUNDS {
        let bytes = make();
        let mut hash = asset_db::ContentHash::default();
        hash.update(&bytes);
        let mut sidecar = Sidecar::new(hash.finish());
        sidecar.id = sound_id(path);
        files.push((
            format!("{path}.{}", asset_db::sidecar::EXTENSION),
            sidecar.to_text().into_bytes(),
        ));
        files.push((path.to_string(), bytes));
    }
    for (path, event) in sound_events() {
        files.push((path.to_string(), sound::event::write(&event).into_bytes()));
    }
    let registry = area::builtin::registry().ok();
    let empty = reflect::Registry::default();
    let registry = registry.as_ref().unwrap_or(&empty);
    let path_of = |id: AssetId| {
        MODELS
            .iter()
            .find(|(name, _)| model(name) == id)
            .map(|(_, path)| (*path).to_string())
    };
    files.push((
        AREA.to_string(),
        area::text::write(&level(), registry, &path_of).into_bytes(),
    ));
    files
}

/// An entity showing `model`, at `position`, turned `rotation`, scaled to
/// `size` (the models are 1 m).
fn shape(name: &str, model_name: &str, position: Vec3, rotation: Quat, size: Vec3) -> EntityData {
    EntityData::new(name)
        .with(&Transform {
            position,
            rotation,
            scale: size,
        })
        .with(&Model {
            asset: Some(AssetRef {
                asset: model(model_name),
                part: None,
            }),
        })
}

/// A fixed block with a box collider: floors, steps, ramps.
fn block(name: &str, position: Vec3, rotation: Quat, size: Vec3) -> EntityData {
    shape(name, "block", position, rotation, size).with(&Collider::default())
}

/// A ramp `long` metres along +Y, rising at `degrees`, 4 m wide, its low
/// end at `foot` on the floor.
fn ramp(name: &str, foot: Vec3, long: f32, degrees: f32) -> EntityData {
    let turn = Quat::from_rotation_x(degrees.to_radians());
    let thick = 0.2;
    let middle = foot + turn * Vec3::new(0.0, long * 0.5, -thick * 0.5);
    block(name, middle, turn, Vec3::new(4.0, long, thick))
}

/// The level: the sun and sky, a floor, steps up to a ledge, a gentle and
/// a steep ramp, crates, a moving platform, the character and its camera,
/// and the quick-save entity.
pub fn level() -> Area {
    let mut area = area::builtin::starter_area();
    let mut n = 0;
    let mut add = |area: &mut Area, data: EntityData| {
        n += 1;
        area.insert(entity(n), data);
        entity(n)
    };
    add(
        &mut area,
        block(
            "Floor",
            Vec3::new(0.0, 0.0, -0.5),
            Quat::IDENTITY,
            Vec3::new(60.0, 60.0, 1.0),
        ),
    );
    // Four steps of 25 cm (the character climbs up to 35 cm), then a
    // ledge a metre up.
    for i in 0..4 {
        let high = 0.25 * (i + 1) as f32;
        add(
            &mut area,
            block(
                &format!("Step {}", i + 1),
                Vec3::new(6.5 + i as f32, 0.0, high * 0.5),
                Quat::IDENTITY,
                Vec3::new(1.0, 4.0, high),
            ),
        );
    }
    add(
        &mut area,
        block(
            "Ledge",
            Vec3::new(12.0, 0.0, 0.5),
            Quat::IDENTITY,
            Vec3::new(4.0, 4.0, 1.0),
        ),
    );
    // A wall beside the ledge, for the camera to keep out of.
    add(
        &mut area,
        block(
            "Wall",
            Vec3::new(12.0, 3.0, 2.0),
            Quat::IDENTITY,
            Vec3::new(8.0, 0.5, 4.0),
        ),
    );
    add(
        &mut area,
        ramp("Gentle ramp", Vec3::new(0.0, 6.0, 0.0), 8.0, 20.0),
    );
    add(
        &mut area,
        ramp("Steep ramp", Vec3::new(-7.0, 6.0, 0.0), 6.0, 55.0),
    );
    // Crates to push: one alone, and a small stack.
    for (i, at) in [
        Vec3::new(-4.0, -3.0, 0.5),
        Vec3::new(-7.0, -3.0, 0.5),
        Vec3::new(-8.1, -3.0, 0.5),
        Vec3::new(-7.55, -3.0, 1.5),
    ]
    .into_iter()
    .enumerate()
    {
        add(
            &mut area,
            shape(
                &format!("Crate {}", i + 1),
                "crate",
                at,
                Quat::IDENTITY,
                Vec3::ONE,
            )
            .with(&RigidBody {
                mass: 20.0,
                ..RigidBody::default()
            })
            .with(&Collider::default()),
        );
    }
    // A platform low enough to step onto, sliding along x.
    let from = Vec3::new(4.0, -9.0, 0.15);
    let to = Vec3::new(14.0, -9.0, 0.15);
    let mut platform = shape(
        "Moving platform",
        "platform",
        from,
        Quat::IDENTITY,
        Vec3::new(3.0, 3.0, 0.3),
    )
    .with(&RigidBody {
        kind: BodyKind::Kinematic,
        ..RigidBody::default()
    })
    .with(&Collider::default());
    let mut moves = ComponentData::default();
    moves.set("from", Value::vec3(from));
    moves.set("to", Value::vec3(to));
    platform.components.insert("Platform".into(), moves);
    add(&mut area, platform);
    // The cave: a tunnel of blocks with a reverb zone inside, and a drip.
    let cave = Vec3::new(-20.0, -10.0, 0.0);
    for (name, offset, size) in [
        (
            "Cave wall 1",
            Vec3::new(-3.0, 0.0, 2.0),
            Vec3::new(0.5, 12.0, 4.0),
        ),
        (
            "Cave wall 2",
            Vec3::new(3.0, 0.0, 2.0),
            Vec3::new(0.5, 12.0, 4.0),
        ),
        (
            "Cave roof",
            Vec3::new(0.0, 0.0, 4.25),
            Vec3::new(6.5, 12.0, 0.5),
        ),
        (
            "Cave back",
            Vec3::new(0.0, -6.0, 2.0),
            Vec3::new(6.5, 0.5, 4.0),
        ),
    ] {
        add(&mut area, block(name, cave + offset, Quat::IDENTITY, size));
    }
    add(
        &mut area,
        EntityData::new("Cave echo")
            .with(&Transform {
                position: cave + Vec3::new(0.0, 0.0, 2.0),
                ..Transform::default()
            })
            .with(&ReverbZone {
                shape: ZoneShape::Box,
                size: Vec3::new(6.0, 12.0, 4.0),
                soft_edge: 2.0,
                preset: "Cave".to_owned(),
                strength: 1.0,
                radius: 8.0,
            }),
    );
    add(
        &mut area,
        EntityData::new("Drip")
            .with(&Transform {
                position: cave + Vec3::new(1.5, -3.0, 3.5),
                ..Transform::default()
            })
            .with(&AudioSource {
                event: "sfx/drip".to_owned(),
                ..AudioSource::default()
            }),
    );
    // A radio on a crate, chattering on the Voice bus: near it, the
    // music dips (Voice ducks Music).
    add(
        &mut area,
        shape(
            "Radio",
            "crate",
            Vec3::new(10.0, 9.0, 0.3),
            Quat::IDENTITY,
            Vec3::new(0.6, 0.4, 0.6),
        )
        .with(&AudioSource {
            event: "voice/radio".to_owned(),
            ..AudioSource::default()
        }),
    );
    // The music, heard the same everywhere.
    add(
        &mut area,
        EntityData::new("Music").with(&AudioSource {
            event: "music/theme".to_owned(),
            ..AudioSource::default()
        }),
    );
    // The character and its camera, each naming the other.
    let player = entity(100);
    let camera = entity(101);
    let mut character = shape(
        "Player",
        "character",
        Vec3::new(0.0, -4.0, 0.0),
        Quat::IDENTITY,
        Vec3::ONE,
    )
    .with(&CharacterBody::default());
    let mut controller = ComponentData::default();
    controller.set("camera", Value::Entity(Some(camera)));
    character
        .components
        .insert("CharacterController".into(), controller);
    character
        .components
        .insert("Footsteps".into(), ComponentData::default());
    area.insert(player, character);
    let mut eye = EntityData::new("Camera")
        .with(&Transform {
            position: Vec3::new(0.0, -8.0, 2.6),
            rotation: Quat::from_rotation_x(75_f32.to_radians()),
            ..Transform::default()
        })
        .with(&Camera::default());
    let mut follow = ComponentData::default();
    follow.set("target", Value::Entity(Some(player)));
    eye.components.insert("ThirdPersonCamera".into(), follow);
    area.insert(camera, eye);
    let mut game = EntityData::new("Quick saves");
    game.components
        .insert("QuickSave".into(), ComponentData::default());
    area.insert(entity(102), game);
    area
}

/// Makes `root` the playground: a new project with its files and input
/// actions. Refuses a folder that's already a project, or that has a file
/// where the playground's would go (nothing of the person's is ever
/// replaced).
pub fn make(root: &Path) -> Result<(), asset_db::ProjectError> {
    let files = files();
    let taken = files
        .iter()
        .map(|(path, _)| root.join(asset_db::ASSETS_FOLDER).join(path))
        .chain([root.join(input::FILE)])
        .find(|file| file.exists());
    if let Some(file) = taken {
        return Err(asset_db::ProjectError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "there's already a file called {}, which the playground would replace; pick an \
                 empty folder",
                file.strip_prefix(root).unwrap_or(&file).display()
            ),
        )));
    }
    let project = Project::create(root, NAME)?;
    for (path, bytes) in &files {
        let file = project.assets().join(path);
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder)?;
        }
        asset_db::write_atomically(&file, bytes)?;
    }
    asset_db::write_atomically(&root.join(input::FILE), bindings().write().as_bytes())?;
    Ok(())
}

#[cfg(test)]
#[path = "playground_tests.rs"]
mod tests;
