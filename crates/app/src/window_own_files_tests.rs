//! Saving the files the editor changes itself (a clips file, an animator,
//! a tileset, an atlas, a tile map), carried out by the app as the editor
//! asks (the review, BUG-09): the app's functions called in the order a
//! frame calls them, with no window and no watcher. Every file here is
//! small text, read where it's asked for, but tile maps and pictures,
//! which arrive from a worker (BUG-31): `read_sprites` waits for them as
//! the frames would.
//!
//! The saves are made by hand, as the editor would ask for them; what the
//! editor's own tabs and undo steps do with the answers is tested in the
//! editor (`editor/src/own_files_tests.rs`).

use super::*;
use editor::OwnFile;
use editor::sprite_sheets::{
    AnimatorState, AtlasState, FramesState, MapFileState, SpriteSheets, TilesetState,
};
use std::sync::Arc;

/// The editor with a new, empty project open in a folder of its own.
pub(super) fn editor_with_project(name: &str) -> (egui::Context, App, PathBuf) {
    let root = std::env::temp_dir().join(format!("solis-own-files-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let ctx = egui::Context::default();
    // Nothing is asked of the network: checking for updates is off.
    let mut storage = super::tests::Memory::default();
    eframe::set_value(&mut storage, AUTO_UPDATE_KEY, &false);
    let mut cc = eframe::CreationContext::_new_kittest(ctx.clone());
    cc.storage = Some(&storage);
    let mut app = App::new(
        &cc,
        crate::app_info(),
        Vec::new(),
        crate::updates::PendingSlot::default(),
        None,
    );
    app.sound = crate::sound::Sound::with_opener(
        || Err("there's no sound card".to_owned()),
        crate::sound::RETRY,
    );
    app.open_project(&root, true);
    let assets = app.projects.db().expect("opened").project().assets();
    (ctx, app, assets)
}

/// Takes in what the app's workers read, frame by frame, until nothing is
/// left to read (at most ten seconds).
fn read_sprites(app: &mut App) {
    let start = std::time::Instant::now();
    app.follow_sprites();
    while app.sprites.waiting() && start.elapsed().as_secs() < 10 {
        std::thread::sleep(std::time::Duration::from_millis(5));
        app.follow_sprites();
    }
}

/// Closes the editor and takes its project's folder away.
pub(super) fn close(app: App) {
    let root = app.projects.root().map(Path::to_path_buf);
    drop(app);
    if let Some(root) = root {
        let _ = std::fs::remove_dir_all(root);
    }
}

/// A file of one kind: its name, and three versions of it whose texts
/// differ in length (so a change on disk is seen whatever its time says).
struct Kind {
    file: &'static str,
    versions: [OwnFile; 3],
}

fn kinds() -> Vec<Kind> {
    let clips = |name: &str| {
        OwnFile::Frames(Arc::new(animation::FramesFile {
            clips: vec![animation::frames_file::OwnClip {
                name: name.into(),
                speed: 1.0,
                direction: animation::Direction::Forward,
                repeats: 0,
                frames: vec![animation::ClipFrame {
                    frame: 0,
                    ms: 100,
                    events: Vec::new(),
                }],
            }],
            ..animation::FramesFile::default()
        }))
    };
    let id = reflect::Id::from_raw(7).unwrap();
    let animator = |name: &str| {
        OwnFile::Animator(Arc::new(animation::AnimatorFile {
            entry: Some(id),
            states: vec![animation::State {
                id,
                name: name.into(),
                place: [0.0, 0.0],
                plays: animation::Plays::Clip(String::new()),
                speed: 1.0,
                speed_by: String::new(),
            }],
            ..animation::AnimatorFile::default()
        }))
    };
    let tileset = |name: &str| {
        OwnFile::Tileset(Arc::new(tiles::Tileset {
            terrains: vec![tiles::Terrain {
                id,
                name: name.into(),
                code: '.',
                colour: [0.25, 0.5, 0.75],
                kind: tiles::TileKind::Plain,
                solid: false,
            }],
            ..tiles::Tileset::default()
        }))
    };
    let atlas = |grid: u32| {
        let mut atlas = animation::atlas::Atlas::new("hero.png");
        atlas.grid = [grid, grid];
        OwnFile::Atlas(Arc::new(atlas))
    };
    let first = tiles::TileMap::new([4, 4], None, Some(b'.'));
    let mut second = first.clone();
    second.add_layer("Roofs", true).unwrap();
    let mut third = second.clone();
    third.add_layer("Clouds", true).unwrap();
    let map = |map: tiles::TileMap| OwnFile::TileMap(Arc::new(map));
    vec![
        Kind {
            file: "hero.solframes",
            versions: [clips("a"), clips("bb"), clips("ccc")],
        },
        Kind {
            file: "hero.solanim",
            versions: [
                animator("Idle"),
                animator("Resting"),
                animator("Sleeping now"),
            ],
        },
        Kind {
            file: "farm.tileset.lst",
            versions: [tileset("grass"), tileset("meadow"), tileset("pasture")],
        },
        Kind {
            file: "hero.atlas.lst",
            versions: [atlas(8), atlas(16), atlas(128)],
        },
        Kind {
            file: "farm.tilemap.lst",
            versions: [map(first), map(second), map(third)],
        },
    ]
}

/// `file` as the text Solis writes for it.
fn text(file: &OwnFile) -> String {
    match file {
        OwnFile::Frames(file) => animation::frames_file::write(file),
        OwnFile::Animator(file) => animation::animator_file::write(file),
        OwnFile::Tileset(file) => tiles::tileset::write(file),
        OwnFile::Atlas(file) => animation::atlas::write(file),
        OwnFile::TileMap(file) => tiles::map_file::write(file).unwrap(),
    }
}

/// The file at `path` in `sheets`, if it's read.
fn held(sheets: &SpriteSheets, path: &str) -> Option<OwnFile> {
    if let Some(FramesState::Read(file, _)) = sheets.frames.get(path) {
        return Some(OwnFile::Frames(Arc::clone(file)));
    }
    if let Some(AnimatorState::Read(file, _)) = sheets.animators.get(path) {
        return Some(OwnFile::Animator(Arc::clone(file)));
    }
    if let Some(TilesetState::Read(file, _)) = sheets.tilesets.get(path) {
        return Some(OwnFile::Tileset(Arc::clone(file)));
    }
    if let Some(AtlasState::Read(file, _)) = sheets.atlases.get(path) {
        return Some(OwnFile::Atlas(Arc::clone(file)));
    }
    match sheets.tile_maps.get(path) {
        Some(MapFileState::Read(file, _)) => Some(OwnFile::TileMap(Arc::clone(file))),
        _ => None,
    }
}

/// Whether `a` and `b` are the very same file in memory, not two alike.
fn same_one(a: &OwnFile, b: &OwnFile) -> bool {
    match (a, b) {
        (OwnFile::Frames(a), OwnFile::Frames(b)) => Arc::ptr_eq(a, b),
        (OwnFile::Animator(a), OwnFile::Animator(b)) => Arc::ptr_eq(a, b),
        (OwnFile::Tileset(a), OwnFile::Tileset(b)) => Arc::ptr_eq(a, b),
        (OwnFile::Atlas(a), OwnFile::Atlas(b)) => Arc::ptr_eq(a, b),
        (OwnFile::TileMap(a), OwnFile::TileMap(b)) => Arc::ptr_eq(a, b),
        _ => false,
    }
}

/// The file on disk at `path`, read as the app reads a file like `like`.
fn on_disk(path: &Path, like: &OwnFile) -> Option<OwnFile> {
    use crate::sprites::{read_animator, read_atlas, read_frames, read_tile_map, read_tileset};
    let mut sheets = SpriteSheets::default();
    match like {
        OwnFile::Frames(_) => drop(sheets.frames.insert("f".into(), read_frames(path))),
        OwnFile::Animator(_) => drop(sheets.animators.insert("f".into(), read_animator(path))),
        OwnFile::Tileset(_) => drop(sheets.tilesets.insert("f".into(), read_tileset(path))),
        OwnFile::Atlas(_) => drop(sheets.atlases.insert("f".into(), read_atlas(path))),
        OwnFile::TileMap(_) => drop(sheets.tile_maps.insert("f".into(), read_tile_map(path))),
    }
    held(&sheets, "f")
}

/// The file as the editor shows it.
fn shown(app: &App, file: &str) -> Option<OwnFile> {
    held(app.workbench.sprite_sheets(), file)
}

/// The Console's errors about saving (with no window there's one about
/// the graphics card, too).
fn errors(app: &App) -> Vec<String> {
    app.workbench
        .log_lines()
        .iter()
        .filter(|l| l.level == LogLevel::Error && l.text.contains("save"))
        .map(|l| l.text.clone())
        .collect()
}

/// The project with `kind`'s first version on disk, read by the app and
/// shown by the editor.
fn with_file(name: &str, kind: &Kind) -> (egui::Context, App, PathBuf) {
    let (ctx, mut app, assets) = editor_with_project(name);
    let [first, second, third] = &kind.versions;
    // What the tests rest on: each version reads back as it was written,
    // and no two texts are as long as each other.
    for version in &kind.versions {
        std::fs::write(assets.join(kind.file), text(version)).unwrap();
        assert_eq!(
            on_disk(&assets.join(kind.file), version).as_ref(),
            Some(version)
        );
    }
    let lengths = [first, second, third].map(|v| text(v).len());
    assert!(lengths[0] != lengths[1] && lengths[1] != lengths[2] && lengths[0] != lengths[2]);
    std::fs::write(assets.join(kind.file), text(first)).unwrap();
    app.refresh_library();
    read_sprites(&mut app);
    assert_eq!(
        shown(&app, kind.file).as_ref(),
        Some(first),
        "{}",
        kind.file
    );
    (ctx, app, assets)
}

/// Asks the app to save `file` as `name`, as the editor does, and has it
/// carried out.
fn save(ctx: &egui::Context, app: &mut App, name: &str, file: &OwnFile) {
    app.queued.push_back(WorkbenchAction::SaveFile {
        path: PathBuf::from(name),
        file: file.clone(),
        text: text(file),
    });
    app.run_queue(ctx);
}

/// The review, BUG-09 (b) and (c): a save only wrote the file. The app's
/// own copy of it stayed as it was until the watcher had the file read
/// again, and anything else read meanwhile handed that older copy to the
/// editor, over the change just saved.
#[test]
fn a_saved_file_is_the_app_s_at_once_so_nothing_read_later_brings_the_old_one_back() {
    for (n, kind) in kinds().iter().enumerate() {
        let (ctx, mut app, assets) = with_file(&format!("saved-{n}"), kind);
        let [_, second, third] = &kind.versions;
        save(&ctx, &mut app, kind.file, second);
        let file = assets.join(kind.file);
        assert_eq!(on_disk(&file, second).as_ref(), Some(second), "written");
        // A picture nothing here uses is read, before the watcher has
        // said a word about the saved file.
        std::fs::write(
            assets.join("icon.png"),
            assets::tile_art::png_of([2, 2], &[9; 2 * 2 * 4]).unwrap(),
        )
        .unwrap();
        app.sprites.want_pictures(["icon.png".to_owned()], &assets);
        read_sprites(&mut app);
        assert_eq!(
            shown(&app, kind.file).as_ref(),
            Some(second),
            "{}: the file as saved, not the older copy",
            kind.file
        );
        // The watcher's listing: Solis's own save isn't read again, so
        // there's no read of it for an editor to take for a change.
        app.refresh_library();
        read_sprites(&mut app);
        let kept = held(app.sprites.sheets(), kind.file).unwrap();
        assert!(same_one(&kept, second), "{}: not read again", kind.file);
        assert_eq!(shown(&app, kind.file).as_ref(), Some(second));
        // A change made outside Solis afterwards is read as ever.
        std::fs::write(&file, text(third)).unwrap();
        app.refresh_library();
        read_sprites(&mut app);
        assert_eq!(
            shown(&app, kind.file).as_ref(),
            Some(third),
            "{}",
            kind.file
        );
        assert_eq!(errors(&app), Vec::<String>::new(), "{}", kind.file);
        close(app);
    }
}

/// The review, BUG-09 (a): a save that couldn't be written. The Console
/// says which file and why, the app's files and the editor's stay as the
/// file is, and the next save that can be written holds what it's given.
#[test]
fn a_save_that_can_t_be_written_is_said_and_the_next_one_writes_the_file() {
    for (n, kind) in kinds().iter().enumerate() {
        let (ctx, mut app, assets) = with_file(&format!("failed-{n}"), kind);
        let [first, second, third] = &kind.versions;
        // A folder sits where the file was: nothing can be written there.
        let file = assets.join(kind.file);
        std::fs::remove_file(&file).unwrap();
        std::fs::create_dir(&file).unwrap();
        save(&ctx, &mut app, kind.file, second);
        let said = errors(&app);
        assert_eq!(said.len(), 1, "{}: {said:?}", kind.file);
        let begins = format!("Couldn't save {}: ", kind.file);
        assert!(said[0].starts_with(&begins), "{}", said[0]);
        assert!(said[0].len() > begins.len() + 3, "and why: {}", said[0]);
        // Nothing took the change for saved: the app's files, and what
        // the editor shows, are the file as it last was.
        assert_eq!(held(app.sprites.sheets(), kind.file).as_ref(), Some(first));
        read_sprites(&mut app);
        assert_eq!(
            shown(&app, kind.file).as_ref(),
            Some(first),
            "{}",
            kind.file
        );
        // The folder gone, the next save is written, whole.
        std::fs::remove_dir(&file).unwrap();
        save(&ctx, &mut app, kind.file, third);
        assert_eq!(on_disk(&file, third).as_ref(), Some(third), "{}", kind.file);
        read_sprites(&mut app);
        assert_eq!(
            shown(&app, kind.file).as_ref(),
            Some(third),
            "{}",
            kind.file
        );
        assert_eq!(errors(&app).len(), 1, "nothing more went wrong");
        close(app);
    }
}

/// A save still waiting when the queue is dropped (the area's Save window
/// was cancelled) is answered as not made, never left without a word.
#[test]
fn a_save_dropped_with_the_queue_is_said_as_not_saved() {
    let kind = &kinds()[1];
    let (_ctx, mut app, assets) = with_file("dropped", kind);
    let [first, second, _] = &kind.versions;
    app.queued.push_back(WorkbenchAction::SaveFile {
        path: PathBuf::from(kind.file),
        file: second.clone(),
        text: text(second),
    });
    app.drop_queue();
    assert!(app.queued.is_empty());
    assert_eq!(
        errors(&app),
        ["Couldn't save hero.solanim: it waited behind the area's save, which didn't happen."]
    );
    let file = assets.join(kind.file);
    assert_eq!(on_disk(&file, first).as_ref(), Some(first), "not written");
    assert_eq!(shown(&app, kind.file).as_ref(), Some(first));
    close(app);
}
