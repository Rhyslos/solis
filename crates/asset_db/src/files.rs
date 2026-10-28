//! Every file and folder in the project's `assets/` folder, as the
//! Library lists them, and changing them from inside Solis: renaming,
//! moving, new folders and deleting to the Recycle Bin. A file's sidecar
//! always goes with it, so its ID survives (decision 0021).

use crate::db::{ASSEMBLY_EXTENSION, AssetDb, SOUND_EXTENSIONS};
use crate::sidecar;
use reflect::AssetId;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

/// What kind of file, for the Library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileKind {
    /// A glTF model.
    Model,
    /// An assembly.
    Assembly,
    /// An area (`.solarea`).
    Area,
    /// A picture (PNG, JPEG).
    Texture,
    /// Game code: Lust (`.lst`), Lua or Rust.
    Code,
    /// A sound file (WAV, Ogg Vorbis).
    Sound,
    /// Anything else.
    Other,
}

/// The extension of area files.
pub const AREA_EXTENSION: &str = "solarea";

impl FileKind {
    /// The kind of the file at `path`, by its extension.
    pub fn of(path: &Path) -> Self {
        if assets::is_model_file(path) {
            return Self::Model;
        }
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        match ext.as_str() {
            e if e == ASSEMBLY_EXTENSION => Self::Assembly,
            e if e == AREA_EXTENSION => Self::Area,
            "png" | "jpg" | "jpeg" => Self::Texture,
            "lst" | "lua" | "rs" => Self::Code,
            e if SOUND_EXTENSIONS.contains(&e) => Self::Sound,
            _ => Self::Other,
        }
    }
}

/// One file in the Library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    /// Where it is, inside `assets/`.
    pub path: PathBuf,
    /// What kind it is.
    pub kind: FileKind,
    /// Its asset ID, for models and assemblies.
    pub id: Option<AssetId>,
}

/// Everything in `assets/`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Listing {
    /// Every folder, inside `assets/`, sorted (empty ones too).
    pub folders: Vec<PathBuf>,
    /// Every file, sorted by path.
    pub files: Vec<FileEntry>,
}

/// Why a file couldn't be changed, in plain words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileError(pub String);

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FileError {}

/// `path` only if it's a plain path inside a folder: no `..`, no root,
/// nothing empty.
/// Whether `name` can name a file or folder in the project on every
/// system: not empty or hidden (starting with `.`), at most
/// [`MAX_NAME_CHARS`] characters, none of Windows' forbidden ones
/// (`< > : " / | ? *` and backslash), no control or text-direction characters, and
/// not a device's name (`CON`, `COM1`).
pub fn valid_file_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name.chars().count() <= MAX_NAME_CHARS
        && !name.chars().any(forbidden_in_names)
        && !assets::reserved_on_windows(name)
}

/// The most characters a name Solis makes or accepts has.
pub const MAX_NAME_CHARS: usize = 100;

/// Characters never in a file's name: Windows' forbidden ones, control
/// characters, and those that turn text around (which can make
/// a program's name look like a picture's).
fn forbidden_in_names(c: char) -> bool {
    "<>:\"/\\|?*".contains(c)
        || c.is_control()
        || matches!(c, '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

/// `name` made into a usable file name (see [`valid_file_name`]): other
/// characters become `_`, it's cut to a usable length, and a device's
/// name gets `_` added. `fallback` if nothing's left.
pub fn file_name_from(name: &str, fallback: &str) -> String {
    let mut safe: String = name
        .trim()
        .chars()
        .map(|c| {
            if (c.is_alphanumeric() && !forbidden_in_names(c)) || " -_()".contains(c) {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect();
    safe = safe.trim().trim_start_matches('.').to_string();
    if safe.is_empty() {
        safe = fallback.to_string();
    }
    if !valid_file_name(&safe) {
        safe.push('_');
    }
    if !valid_file_name(&safe) {
        safe = fallback.to_string();
    }
    safe
}

fn inside(path: &Path) -> Result<&Path, FileError> {
    let plain = path.components().all(|c| matches!(c, Component::Normal(_)));
    if plain && path.components().next().is_some() {
        Ok(path)
    } else {
        Err(FileError(format!(
            "{} isn't inside the project's assets folder",
            path.display()
        )))
    }
}

impl AssetDb {
    /// Every file and folder in `assets/`, skipping hidden ones, sidecars
    /// and temporary files.
    pub fn listing(&self) -> Listing {
        let root = self.project().assets();
        let ids: std::collections::HashMap<&Path, AssetId> = self
            .records()
            .values()
            .map(|r| (r.path.as_path(), r.id))
            .collect();
        let mut listing = Listing::default();
        let mut todo = vec![root.clone()];
        let mut seen = HashSet::new();
        while let Some(folder) = todo.pop() {
            let Ok(real) = std::fs::canonicalize(&folder) else {
                continue;
            };
            if !seen.insert(real) {
                continue;
            }
            let Ok(entries) = std::fs::read_dir(&folder) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('.')
                    || name.ends_with(".tmp")
                    || name.ends_with(&format!(".{}", sidecar::EXTENSION))
                {
                    continue;
                }
                let relative = path.strip_prefix(&root).unwrap_or(&path).to_path_buf();
                match entry.file_type() {
                    Ok(t) if t.is_dir() => {
                        listing.folders.push(relative);
                        todo.push(path);
                    }
                    Ok(_) => listing.files.push(FileEntry {
                        id: ids.get(relative.as_path()).copied(),
                        kind: FileKind::of(&relative),
                        path: relative,
                    }),
                    Err(_) => {}
                }
            }
        }
        listing.folders.sort();
        listing.files.sort_by(|a, b| a.path.cmp(&b.path));
        listing
    }

    /// Renames or moves the file or folder at `from` to `to` (both inside
    /// `assets/`), with its sidecar, so its ID stays. Never replaces
    /// anything. The watcher then reports the move.
    pub fn move_path(&self, from: &Path, to: &Path) -> Result<(), FileError> {
        let root = self.project().assets();
        let (from_rel, to_rel) = (inside(from)?, inside(to)?);
        // The new name must work on every system (a moved file keeps its
        // name, which was already allowed).
        if from.file_name() != to.file_name()
            && !to
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(valid_file_name)
        {
            return Err(FileError(format!(
                "{} can't be a file's name: it needs to be at most {MAX_NAME_CHARS} characters, \
                 without < > : \" / \\ | ? * or a device's name such as CON",
                to.file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
            )));
        }
        let (source, target) = (root.join(from_rel), root.join(to_rel));
        if !source.exists() {
            return Err(FileError(format!(
                "{} isn't there any more",
                from.display()
            )));
        }
        // On Windows (and macOS) names ignore capitals, so a rename that
        // only changes them finds the file itself "already there".
        let only_capitals = source != target
            && matches!(
                (source.canonicalize(), target.canonicalize()),
                (Ok(a), Ok(b)) if a == b
            );
        if target.exists() && !only_capitals {
            let name = to.file_name().map_or_else(
                || to.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            );
            return Err(FileError(format!("there's already a {name} there")));
        }
        if source.is_dir() && to_rel.starts_with(from_rel) {
            return Err(FileError("a folder can't go inside itself".into()));
        }
        let is_gltf = source.is_file()
            && source
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("gltf"));
        if is_gltf && from_rel.parent() != to_rel.parent() {
            return Err(FileError(
                "a .gltf file reads the files beside it, so it can't leave its folder on \
                 its own: move the whole folder instead"
                    .into(),
            ));
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| FileError(format!("couldn't make the folder: {e}")))?;
        }
        std::fs::rename(&source, &target)
            .map_err(|e| FileError(format!("couldn't move it: {e}")))?;
        let old_sidecar = sidecar::path_for(&source);
        if source.extension().is_some() && old_sidecar.exists() {
            // If this fails, the database pairs the left-behind sidecar
            // with the moved file by its content on the next rescan.
            let _ = std::fs::rename(&old_sidecar, sidecar::path_for(&target));
        }
        Ok(())
    }

    /// Makes a new folder called "New folder" (or "New folder 2"…) in
    /// `parent` (inside `assets/`; empty for `assets/` itself), and returns
    /// it inside `assets/`.
    pub fn new_folder(&self, parent: &Path) -> Result<PathBuf, FileError> {
        let root = self.project().assets();
        let base = if parent.as_os_str().is_empty() {
            root.clone()
        } else {
            root.join(inside(parent)?)
        };
        for n in 1..1000 {
            let name = if n == 1 {
                "New folder".to_string()
            } else {
                format!("New folder {n}")
            };
            let folder = base.join(&name);
            if !folder.exists() {
                std::fs::create_dir_all(&folder)
                    .map_err(|e| FileError(format!("couldn't make the folder: {e}")))?;
                return Ok(folder.strip_prefix(&root).unwrap_or(&folder).to_path_buf());
            }
        }
        Err(FileError("there are too many new folders here".into()))
    }

    /// Moves the file or folder at `path` (inside `assets/`) to the
    /// Recycle Bin, with its sidecar, so it can be brought back.
    pub fn delete_to_trash(&self, path: &Path) -> Result<(), FileError> {
        let full = self.project().assets().join(inside(path)?);
        if !full.exists() {
            return Err(FileError(format!(
                "{} isn't there any more",
                path.display()
            )));
        }
        let mut items = vec![full.clone()];
        let side = sidecar::path_for(&full);
        if full.is_file() && side.exists() {
            items.push(side);
        }
        trash::delete_all(&items)
            .map_err(|e| FileError(format!("couldn't move it to the Recycle Bin: {e}")))
    }
}

#[cfg(test)]
mod name_tests {
    use super::*;

    #[test]
    fn names_that_work_on_every_system() {
        // The third security audit: names Solis makes or accepts could be
        // a device's (`COM¹`), hide a stream (`a:b`), or turn text around.
        for good in ["brick.png", "Lamp post (2)", "drøne", "ship_01"] {
            assert!(valid_file_name(good), "{good}");
        }
        for bad in [
            "",
            ".hidden",
            "a:b",
            "a/b",
            "a\\b",
            "CON",
            "com1.txt",
            "COM¹",
            "trail.",
            "trail ",
            "photo\u{202E}gnp.exe",
            "tab\there",
            &"x".repeat(101),
        ] {
            assert!(!valid_file_name(bad), "{bad:?}");
        }
        assert_eq!(file_name_from("Ship: Mk II", "Assembly"), "Ship_ Mk II");
        assert_eq!(file_name_from("COM¹", "Assembly"), "COM¹_");
        assert_eq!(file_name_from("CON", "Assembly"), "CON_");
        assert_eq!(file_name_from("   ", "Assembly"), "Assembly");
        assert_eq!(file_name_from("a\u{202E}b", "Assembly"), "a_b");
        assert_eq!(file_name_from(&"y".repeat(500), "A").chars().count(), 64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Project;
    use crate::test_folder;

    fn project(name: &str) -> (AssetDb, PathBuf) {
        let root = test_folder(name);
        let project = Project::create(&root, "Test").unwrap();
        let assets = project.assets();
        std::fs::create_dir_all(assets.join("props/old")).unwrap();
        std::fs::write(assets.join("crate.glb"), sample()).unwrap();
        std::fs::write(assets.join("props/rock.glb"), sample()).unwrap();
        std::fs::write(assets.join("props/bark.png"), b"not really a png").unwrap();
        std::fs::write(assets.join("main.solarea"), "area version 1;\n").unwrap();
        std::fs::write(assets.join("turret.lst"), "object Turret {}\n").unwrap();
        std::fs::write(assets.join("notes.txt"), "hi").unwrap();
        let (db, _, _) = AssetDb::open(project, assets::Limits::default());
        (db, assets)
    }

    fn sample() -> Vec<u8> {
        assets::sample::sample_glb()
    }

    #[test]
    fn the_listing_has_every_kind_and_folder_but_no_sidecars() {
        let (db, _) = project("files-listing");
        let listing = db.listing();
        assert_eq!(
            listing.folders,
            [PathBuf::from("props"), PathBuf::from("props/old")]
        );
        let kinds: Vec<(String, FileKind, bool)> = listing
            .files
            .iter()
            // `/` between folders on every system (Windows shows `\`).
            .map(|f| {
                let path: Vec<_> = f.path.iter().map(|p| p.to_string_lossy()).collect();
                (path.join("/"), f.kind, f.id.is_some())
            })
            .collect();
        assert_eq!(
            kinds,
            [
                ("crate.glb".into(), FileKind::Model, true),
                ("main.solarea".into(), FileKind::Area, false),
                ("notes.txt".into(), FileKind::Other, false),
                ("props/bark.png".into(), FileKind::Texture, false),
                ("props/rock.glb".into(), FileKind::Model, true),
                ("turret.lst".into(), FileKind::Code, false),
            ]
        );
    }

    #[test]
    fn moving_a_model_takes_its_sidecar_and_keeps_its_id() {
        let (mut db, assets) = project("files-move");
        let id = db.id_at(&assets.join("crate.glb")).unwrap();
        db.move_path(Path::new("crate.glb"), Path::new("props/old/box.glb"))
            .unwrap();
        assert!(assets.join("props/old/box.glb.solmeta").exists());
        assert!(!assets.join("crate.glb.solmeta").exists());
        db.rescan();
        assert_eq!(db.id_at(&assets.join("props/old/box.glb")), Some(id));
        // Folders move with everything in them.
        db.move_path(Path::new("props"), Path::new("kit")).unwrap();
        db.rescan();
        assert_eq!(db.id_at(&assets.join("kit/old/box.glb")), Some(id));
    }

    #[test]
    fn moves_that_would_lose_or_break_something_are_refused() {
        let (db, assets) = project("files-refused");
        let refuse = |from: &str, to: &str| {
            db.move_path(Path::new(from), Path::new(to))
                .expect_err(&format!("{from} → {to}"))
                .0
        };
        assert!(refuse("crate.glb", "props/rock.glb").contains("already"));
        assert!(refuse("props", "props/old/props").contains("inside itself"));
        assert!(refuse("../outside.glb", "a.glb").contains("isn't inside"));
        assert!(refuse("crate.glb", "/etc/a.glb").contains("isn't inside"));
        assert!(refuse("gone.glb", "a.glb").contains("isn't there"));
        // The third security audit: a rename can't make a device's name,
        // a hidden stream or text that turns around.
        for bad in [
            "CON.glb",
            "COM¹.glb",
            "a:b.glb",
            "rock\u{202E}blg.exe",
            "trail.",
        ] {
            // On Windows `a:b.glb` is a drive's path, refused as outside.
            let why = refuse("crate.glb", bad);
            assert!(
                why.contains("can't be") || why.contains("isn't inside"),
                "{bad}: {why}"
            );
        }
        std::fs::write(assets.join("ship.gltf"), "{}").unwrap();
        assert!(refuse("ship.gltf", "props/ship.gltf").contains("whole folder"));
        // Renaming in place is fine.
        db.move_path(Path::new("ship.gltf"), Path::new("boat.gltf"))
            .unwrap();
        assert!(assets.join("boat.gltf").exists());
        // Changing only capitals works too (on Windows the file used to
        // count as "already there").
        db.move_path(Path::new("boat.gltf"), Path::new("Boat.gltf"))
            .unwrap();
        let names: Vec<String> = std::fs::read_dir(&assets)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"Boat.gltf".to_string()), "{names:?}");
        assert!(!names.contains(&"boat.gltf".to_string()), "{names:?}");
    }

    #[test]
    fn new_folders_get_free_names() {
        let (db, assets) = project("files-new-folder");
        assert_eq!(
            db.new_folder(Path::new("")).unwrap(),
            PathBuf::from("New folder")
        );
        assert_eq!(
            db.new_folder(Path::new("")).unwrap(),
            PathBuf::from("New folder 2")
        );
        assert_eq!(
            db.new_folder(Path::new("props")).unwrap(),
            PathBuf::from("props/New folder")
        );
        assert!(assets.join("props/New folder").is_dir());
        assert!(db.new_folder(Path::new("../x")).is_err());
    }

    #[test]
    fn kinds_come_from_extensions() {
        assert_eq!(FileKind::of(Path::new("a.GLB")), FileKind::Model);
        assert_eq!(FileKind::of(Path::new("a.solassembly")), FileKind::Assembly);
        assert_eq!(FileKind::of(Path::new("a.JPEG")), FileKind::Texture);
        assert_eq!(FileKind::of(Path::new("a.lua")), FileKind::Code);
        assert_eq!(FileKind::of(Path::new("a")), FileKind::Other);
    }
}
