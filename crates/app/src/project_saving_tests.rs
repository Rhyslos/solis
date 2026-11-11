//! The editors' savers and the Library's file changes (the 2026-11
//! review, SEC-13): each takes only a plain path inside the assets folder,
//! and a saver writes only a file of its own kind there. One of the nine
//! savers once checked its path more loosely than the rest, so
//! `../x.solsound` was written beside the assets, and a new sound event
//! could be made in any folder a path with `..` reached.

use super::*;
use std::collections::BTreeMap;

/// What a saver does about a file that is, or isn't, there already.
#[derive(Clone, Copy, PartialEq)]
enum Needs {
    /// It only changes a file that's there.
    TheFile,
    /// It only makes a file that isn't.
    NoFile,
    /// It makes the file or changes it.
    Nothing,
}

/// One saver: its name, a file name of its kind, text it saves, what it
/// needs of the file, and the call.
struct Saver {
    name: &'static str,
    file: &'static str,
    text: String,
    needs: Needs,
    save: fn(&Projects, &Path, &str) -> Result<(), String>,
}

/// An example file from `tests/fixtures`.
fn fixture(path: &str) -> String {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    std::fs::read_to_string(fixtures.join(path)).unwrap()
}

/// Every saver, with text of its kind as Solis writes it now.
fn savers() -> Vec<Saver> {
    let event = sound::event::read(&fixture("sound-event/v1/footstep.solsound")).unwrap();
    let frames = animation::frames_file::read(&fixture("frames/v1/walker.solframes")).unwrap();
    let animator = animation::animator_file::read(&fixture("animator/v1/farmer.solanim")).unwrap();
    let tileset = tiles::tileset::read(&fixture("tileset/v4/farm.tileset.lst")).unwrap();
    let map = tiles::map_file::read(&fixture("tilemap/v3/farm.tilemap.lst")).unwrap();
    vec![
        Saver {
            name: "save_sound_event",
            file: "x.solsound",
            text: sound::event::write(&event.event),
            needs: Needs::TheFile,
            save: |p, path, text| p.save_sound_event(path, text).map(drop),
        },
        Saver {
            name: "save_frames",
            file: "x.solframes",
            text: animation::frames_file::write(&frames.file),
            needs: Needs::Nothing,
            save: |p, path, text| p.save_frames(path, text).map(drop),
        },
        Saver {
            name: "save_animator",
            file: "x.solanim",
            text: animation::animator_file::write(&animator.file),
            needs: Needs::Nothing,
            save: |p, path, text| p.save_animator(path, text).map(drop),
        },
        Saver {
            name: "save_tileset",
            file: "x.tileset.lst",
            text: tiles::tileset::write(&tileset.tileset),
            needs: Needs::Nothing,
            save: |p, path, text| p.save_tileset(path, text).map(drop),
        },
        Saver {
            name: "save_tile_map",
            file: "x.tilemap.lst",
            text: tiles::map_file::write(&map.map).unwrap(),
            needs: Needs::Nothing,
            save: |p, path, text| p.save_tile_map(path, text).map(drop),
        },
        Saver {
            name: "save_picture",
            file: "x.png",
            text: "a picture".into(),
            needs: Needs::NoFile,
            save: |p, path, text| p.save_picture(path, text.as_bytes()),
        },
        Saver {
            name: "save_new_text (code)",
            file: "x.lst",
            text: "end\n".into(),
            needs: Needs::NoFile,
            save: |p, path, text| p.save_new_text(path, text),
        },
        Saver {
            name: "save_new_text (an area)",
            file: "x.solarea",
            text: "area version 3;\n".into(),
            needs: Needs::NoFile,
            save: |p, path, text| p.save_new_text(path, text),
        },
        Saver {
            name: "save_atlas",
            file: "x.atlas.lst",
            text: fixture("atlas/v1/tools.atlas.lst"),
            needs: Needs::Nothing,
            save: |p, path, text| p.save_atlas(path, text),
        },
        Saver {
            name: "save_screen",
            file: "x.lst",
            text: "end\n".into(),
            needs: Needs::TheFile,
            save: |p, path, text| p.save_screen(path, text),
        },
    ]
}

/// A project of its own in a scratch folder (`base/project`), with a file
/// of every saver's kind wherever a path that strays could land: in the
/// project's folder, above it, and in a folder beside it. Each holds the
/// saver's text and a line more: in the format Solis writes now (a saver
/// writes straight over such a file, with no copy kept first), yet not
/// what any saver here saves, so a save that lands on one shows.
fn scratch(name: &str) -> (PathBuf, Projects) {
    let base = std::env::temp_dir().join(format!("solis-app-saving-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let root = base.join("project");
    std::fs::create_dir_all(base.join("outside")).unwrap();
    std::fs::create_dir_all(&root).unwrap();
    let mut projects = Projects::default();
    projects
        .open(&root, true, assets::Limits::default())
        .unwrap();
    std::fs::create_dir_all(root.join("assets/a")).unwrap();
    for saver in savers() {
        for folder in [base.clone(), root.clone(), base.join("outside")] {
            std::fs::write(folder.join(saver.file), format!("{}\n", saver.text)).unwrap();
        }
    }
    (base, projects)
}

/// What's under a folder: each file with what it holds, and each folder
/// (holding nothing).
type Found = BTreeMap<PathBuf, Option<Vec<u8>>>;

/// Every file and folder under `base`, but for Solis's own folder in the
/// project (its cache is trimmed on a worker as a project opens).
fn files(base: &Path) -> Found {
    let own = base.join("project").join(asset_db::CACHE_FOLDER);
    let mut found = BTreeMap::new();
    let mut todo = vec![base.to_path_buf()];
    while let Some(folder) = todo.pop() {
        for entry in std::fs::read_dir(&folder).unwrap() {
            let path = entry.unwrap().path();
            if path == own {
                continue;
            }
            if path.is_dir() {
                found.insert(path.clone(), None);
                todo.push(path);
            } else {
                let holds = std::fs::read(&path).unwrap();
                found.insert(path, Some(holds));
            }
        }
    }
    found
}

/// The files and folders that differ between `before` and `after`:
/// changed, made or gone.
fn changed(before: &Found, after: &Found) -> Vec<PathBuf> {
    let gone = before.keys().filter(|path| !after.contains_key(*path));
    after
        .iter()
        .filter(|(path, holds)| before.get(*path) != Some(*holds))
        .map(|(path, _)| path)
        .chain(gone)
        .cloned()
        .collect()
}

/// Paths to a file or folder called `name` that leave the assets folder,
/// or would on some system: up and out, or from a root, a drive, a
/// network share or a folder beside the project.
fn leaving(name: &str, outside: &Path) -> Vec<String> {
    vec![
        "..".into(),
        "../..".into(),
        "../../outside".into(),
        // Onto a file that's there, and where none is.
        format!("../{name}"),
        format!("../fresh-{name}"),
        format!("a/../../{name}"),
        format!("..\\{name}"),
        format!("a\\..\\..\\{name}"),
        format!("./{name}"),
        format!("a//{name}"),
        format!("/{name}"),
        format!("\\{name}"),
        format!("C:{name}"),
        format!("C:\\{name}"),
        format!("C:/{name}"),
        format!("\\\\server\\share\\{name}"),
        format!("//server/share/{name}"),
        outside.join(name).to_string_lossy().into_owned(),
    ]
}

/// Paths a saver of files like `file` must refuse: those that leave the
/// assets, an empty one, and files of another kind.
fn strays(file: &str, outside: &Path) -> Vec<String> {
    let mut strays = leaving(file, outside);
    strays.extend([
        String::new(),
        "x".into(),
        "x.exe".into(),
        format!("{file}.exe"),
        format!("a/{file}/"),
    ]);
    strays
}

#[test]
fn every_saver_refuses_a_path_that_strays_or_isn_t_its_kind_and_writes_nothing() {
    let (base, projects) = scratch("strays");
    let before = files(&base);
    for saver in savers() {
        for path in strays(saver.file, &base.join("outside")) {
            let said = (saver.save)(&projects, Path::new(&path), &saver.text);
            let Err(why) = said else {
                panic!("{} took {path:?}", saver.name);
            };
            assert!(
                why.starts_with("it isn't") && why.ends_with("in the assets"),
                "{} said of {path:?}: {why}",
                saver.name
            );
        }
    }
    let written = changed(&before, &files(&base));
    assert!(written.is_empty(), "a refused save wrote {written:?}");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn every_saver_saves_a_plain_path_of_its_kind_as_before() {
    let (base, projects) = scratch("plain");
    let assets = base.join("project/assets");
    for (n, saver) in savers().iter().enumerate() {
        // A folder each, as two savers write `x.lst`.
        let folder = format!("kept{n}");
        std::fs::create_dir_all(assets.join(&folder)).unwrap();
        let path = PathBuf::from(format!("{folder}/{}", saver.file));
        let file = assets.join(&path);
        let save = |path: &Path| (saver.save)(&projects, path, &saver.text);
        // Its folder gone (or, for one that never makes files, the file).
        let gone = save(&PathBuf::from(format!("gone/{}", saver.file))).unwrap_err();
        assert!(
            gone.contains("isn't there any more"),
            "{}: {gone}",
            saver.name
        );
        if saver.needs == Needs::TheFile {
            let missing = save(&path).unwrap_err();
            assert!(
                missing.contains("isn't there any more"),
                "{}: {missing}",
                saver.name
            );
            assert!(!file.exists(), "{} made a file", saver.name);
            std::fs::write(&file, &saver.text).unwrap();
        }
        save(&path).unwrap_or_else(|why| panic!("{}: {why}", saver.name));
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            saver.text,
            "{}",
            saver.name
        );
        // Once more, over the file it wrote.
        let again = save(&path);
        if saver.needs == Needs::NoFile {
            let why = again.unwrap_err();
            assert!(why.contains("already there"), "{}: {why}", saver.name);
        } else {
            again.unwrap_or_else(|why| panic!("{} again: {why}", saver.name));
        }
        // The name's ending in capitals is the same kind (under another
        // name: where capitals don't count, `X` would be the file `x`).
        if saver.needs == Needs::NoFile {
            let shouted = format!("{folder}/{}", saver.file.replace("x.", "y.").to_uppercase());
            save(Path::new(&shouted)).unwrap_or_else(|why| panic!("{}: {why}", saver.name));
        }
    }
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn no_made_up_path_makes_a_saver_write_outside_the_assets() {
    let (base, projects) = scratch("sweep");
    let assets = base.join("project/assets");
    let outside = |all: Found| -> Found {
        all.into_iter()
            .filter(|(path, _)| !path.starts_with(&assets))
            .collect()
    };
    let before = outside(files(&base));
    // Every path of up to three of these parts under each start, with
    // either slash between them: thousands for each saver.
    let parts = ["..", ".", "", "a", "x", "C:", "assets", "outside"];
    let starts = ["", "/", "\\", "C:", "C:\\", "../", "..\\"];
    let mut folders: Vec<Vec<&str>> = vec![Vec::new()];
    for length in 1..=3 {
        let longer: Vec<Vec<&str>> = folders
            .iter()
            .filter(|f| f.len() == length - 1)
            .flat_map(|f| parts.iter().map(move |p| [f.as_slice(), &[*p]].concat()))
            .collect();
        folders.extend(longer);
    }
    let mut tried = 0;
    for saver in savers() {
        for folder in &folders {
            for (start, slash) in starts.iter().flat_map(|s| [(s, "/"), (s, "\\")]) {
                for name in [saver.file, "x.exe"] {
                    let mut names = folder.clone();
                    names.push(name);
                    let path = format!("{start}{}", names.join(slash));
                    // Whatever it says: only where it wrote matters here.
                    let _ = (saver.save)(&projects, Path::new(&path), &saver.text);
                    tried += 1;
                }
            }
        }
    }
    assert!(tried > 100_000, "{tried}");
    let written = changed(&before, &outside(files(&base)));
    assert!(written.is_empty(), "a saver wrote outside: {written:?}");
    // The sweep did save where it may: plain paths inside the assets.
    assert!(assets.join("x.png").is_file());
    assert!(assets.join("a/x.solframes").is_file());
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn a_plain_path_is_names_only_on_every_system() {
    for plain in ["x.lst", "a/x.lst", "props/tiles/farm.tileset.lst"] {
        assert!(plain_inside(Path::new(plain)), "{plain}");
        assert!(assets_or_inside(Path::new(plain)), "{plain}");
        assert!(named_inside(Path::new(plain)), "{plain}");
    }
    for stray in ["", ".", "..", "/x", "\\x", "C:x", "C:\\x", "a/../x", "a//x"] {
        assert!(!plain_inside(Path::new(stray)), "{stray}");
        // But for the assets folder itself, where a folder is named.
        assert_eq!(assets_or_inside(Path::new(stray)), stray.is_empty());
    }
    // Where a file moves to: its folder by the rule, then a name (whose
    // letters `asset_db` judges).
    for named in ["a:b.png", "props/a:b.png", "props\\x"] {
        assert!(named_inside(Path::new(named)), "{named}");
    }
    for stray in [
        "", ".", "..", "a/", "a/..", "/x", "\\x", "a//x", "./x", "../x", "a/../x", "C:/x", "C:\\x",
        "C:a/x",
    ] {
        assert!(!named_inside(Path::new(stray)), "{stray}");
    }
}

#[test]
fn a_new_sound_event_is_made_only_in_a_folder_inside_the_assets() {
    let (base, mut projects) = scratch("new-event");
    let before = files(&base);
    for folder in leaving("a", &base.join("outside")) {
        let said = projects.new_sound_event(Path::new(&folder), "sound version 1;\n");
        let Err(why) = said else {
            panic!("a sound event was made in {folder:?}");
        };
        assert!(why.ends_with("isn't a folder in the assets"), "{why}");
    }
    let written = changed(&before, &files(&base));
    assert!(written.is_empty(), "a refused folder got {written:?}");
    // In the assets folder itself (no path at all) and in a folder inside
    // it, as before.
    for (folder, made) in [
        ("", "New sound event.solsound"),
        ("a", "a/New sound event.solsound"),
    ] {
        let at = projects
            .new_sound_event(Path::new(folder), "sound version 1;\n")
            .unwrap();
        assert_eq!(at, PathBuf::from(made));
        assert!(base.join("project/assets").join(made).is_file());
    }
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn moving_deleting_and_new_folders_keep_to_plain_paths_inside_the_assets() {
    let (base, projects) = scratch("file-changes");
    let assets = base.join("project/assets");
    std::fs::write(assets.join("a/keep.png"), "a picture").unwrap();
    let before = files(&base);
    let outside = "isn't inside the project's assets folder";
    for stray in leaving("x.png", &base.join("outside")) {
        let stray = Path::new(&stray);
        let moved_from = projects.move_path(stray, Path::new("a/taken.png"));
        assert!(moved_from.unwrap_err().ends_with(outside), "from {stray:?}");
        // Where it would go is refused too: its folder by the same rule,
        // and a new name by `asset_db`'s stricter one for names.
        let moved_to = projects.move_path(Path::new("a/keep.png"), stray);
        assert!(moved_to.is_err(), "to {stray:?}");
        let deleted = projects.delete(stray);
        assert!(deleted.unwrap_err().ends_with(outside), "delete {stray:?}");
        let made = projects.new_folder(stray);
        assert!(made.unwrap_err().ends_with(outside), "folder in {stray:?}");
    }
    assert!(projects.delete(Path::new("")).is_err());
    assert!(projects.move_path(Path::new(""), Path::new("b")).is_err());
    assert!(projects.move_path(Path::new("a"), Path::new("")).is_err());
    let touched = changed(&before, &files(&base));
    assert!(touched.is_empty(), "a refused change touched {touched:?}");

    // Plain paths go through as before. A name that can't be a file's is
    // still explained as a name.
    let named = projects
        .move_path(Path::new("a/keep.png"), Path::new("a/ke:ep.png"))
        .unwrap_err();
    assert!(named.contains("can't be a file's name"), "{named}");
    let gone = projects.delete(Path::new("a/gone.png")).unwrap_err();
    assert!(gone.contains("isn't there any more"), "{gone}");
    projects
        .move_path(Path::new("a/keep.png"), Path::new("a/kept.png"))
        .unwrap();
    projects
        .move_path(Path::new("a/kept.png"), Path::new("kept.png"))
        .unwrap();
    assert!(assets.join("kept.png").is_file());
    assert_eq!(
        projects.new_folder(Path::new("")).unwrap(),
        PathBuf::from("New folder")
    );
    assert_eq!(
        projects.new_folder(Path::new("a")).unwrap(),
        PathBuf::from("a/New folder")
    );
    let _ = std::fs::remove_dir_all(&base);
}
