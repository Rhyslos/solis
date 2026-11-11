//! The open project, as the window holds it: its asset database, the
//! watcher on its assets folder, and the list of projects Solis knows
//! ([decision 0021](../../../docs/decisions/0021-asset-database-and-models.md),
//! [decision 0086](../../../docs/decisions/0086-the-launcher.md)).
//!
//! It also saves the files the editors make and change. Every saver is
//! told where by a path inside the assets, and finds the file through one
//! function (`Projects::place`), which takes only a plain path there, of
//! the saver's own kind: a path can't make one write outside the assets
//! folder, or over another kind of file. The Library's file changes (a
//! move, a delete, a new folder or sound event) keep to the same rule
//! (`plain_inside`). Only [`Projects::open`] and [`Projects::bring_in`]
//! take a path from anywhere on the computer: the user chose it there, in
//! the system's own window or by dropping a file.

use asset_db::{AssetDb, Change, FileKind, Listing, Project, Watcher};
use editor::{LibraryItem, LibraryKind, ProjectInfo};
use reflect::AssetId;
use std::path::{Path, PathBuf};

/// The most projects File > Recent projects lists.
pub const MAX_RECENT: usize = 8;

/// The project side of the window.
#[derive(Debug, Default)]
pub struct Projects {
    db: Option<AssetDb>,
    watcher: Option<Watcher>,
    known: crate::known_projects::KnownProjects,
    /// This copy's build number, recorded in each project it opens.
    build: Option<u64>,
    /// Whether the assets folder changed since last asked (any file, not
    /// only assets: the Library lists them all).
    changed: bool,
}

/// What opening a project found.
#[derive(Debug)]
pub struct Opened {
    /// The project, for the editor.
    pub info: ProjectInfo,
    /// Things worth telling the user (damaged sidecars, copies).
    pub notes: Vec<String>,
    /// Whether its folder can be watched; if not, changes are only seen
    /// when it's opened again.
    pub watching: bool,
}

impl Projects {
    /// Remembers the projects `known`; `build` is this copy's number,
    /// written into each project it opens (none for a copy built from
    /// source).
    pub fn with_known(known: crate::known_projects::KnownProjects, build: Option<u64>) -> Self {
        Self {
            known,
            build,
            ..Self::default()
        }
    }

    /// Opens the project in `root`, or makes one there first when `create`
    /// (named after the folder; an existing project is simply opened).
    pub fn open(
        &mut self,
        root: &Path,
        create: bool,
        limits: assets::Limits,
    ) -> Result<Opened, asset_db::ProjectError> {
        let project = match Project::open(root) {
            Err(asset_db::ProjectError::NotAProject(_)) if create => {
                let name = root.file_name().map_or_else(
                    || "Project".to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                Project::create(root, &name)?
            }
            other => other?,
        };
        // Older files Solis made move to Lust first (decision 0081), so
        // the asset database finds them under their new names.
        let moved = crate::to_lust::move_older_files(root, &project.assets());
        let watcher = Watcher::new(&project.assets()).ok();
        let (db, _, mut notes) = AssetDb::open(project, limits);
        notes.splice(0..0, moved);
        // The cache keeps within its size, on a worker: a model whose entry
        // goes meanwhile is simply imported again.
        let cache = db.project().cache();
        let _ = std::thread::Builder::new()
            .name("cache trim".into())
            .spawn(move || {
                let freed = asset_db::cache::trim(&cache, asset_db::cache::MAX_BYTES);
                if freed > 0 {
                    log::info!(
                        "Freed {} MB of the import cache (models not used lately).",
                        freed >> 20
                    );
                }
            });
        let info = ProjectInfo {
            name: db.project().name().to_string(),
            root: root.to_path_buf(),
            assets: db.project().assets(),
        };
        if let Some(build) = self.build
            && let Err(why) = db.project().record_build(build)
        {
            notes.push(format!(
                "Couldn't note this build in {}: {why}.",
                asset_db::PROJECT_FILE
            ));
        }
        self.known
            .opened(root, db.project().name(), crate::known_projects::now());
        let watching = watcher.is_some();
        self.db = Some(db);
        self.watcher = watcher;
        Ok(Opened {
            info,
            notes,
            watching,
        })
    }

    /// The asset database, if a project is open.
    pub fn db(&self) -> Option<&AssetDb> {
        self.db.as_ref()
    }

    /// The import cache's folder, if a project is open.
    pub fn cache(&self) -> Option<PathBuf> {
        self.db.as_ref().map(|db| db.project().cache())
    }

    /// The projects opened lately, newest first, for File > Recent
    /// projects.
    pub fn recent(&self) -> Vec<PathBuf> {
        self.known.recent(MAX_RECENT)
    }

    /// Every project Solis knows.
    pub fn known(&self) -> &crate::known_projects::KnownProjects {
        &self.known
    }

    /// Every project Solis knows, to pin, forget or list.
    pub fn known_mut(&mut self) -> &mut crate::known_projects::KnownProjects {
        &mut self.known
    }

    /// The open project's folder.
    pub fn root(&self) -> Option<&Path> {
        self.db.as_ref().map(|db| db.project().root())
    }

    /// What changed in the assets folder since last asked, with notes in
    /// plain words. Never waits: with nothing reported by the watcher,
    /// the folder isn't looked at.
    pub fn poll(&mut self) -> (Vec<Change>, Vec<String>) {
        let changed = self.watcher.as_ref().is_some_and(Watcher::changed);
        // A new file left waiting for its sidecar is looked at again once
        // it has settled, even if nothing else happens.
        let due = self
            .db
            .as_ref()
            .and_then(AssetDb::look_again_at)
            .is_some_and(|at| std::time::Instant::now() >= at);
        if !changed && !due {
            return (Vec::new(), Vec::new());
        }
        self.changed = true;
        self.rescan()
    }

    /// Whether a new file is waiting for its sidecar, to be looked at
    /// again shortly.
    pub fn waiting(&self) -> bool {
        self.db
            .as_ref()
            .is_some_and(|db| db.look_again_at().is_some())
    }

    /// Whether anything in the assets folder changed since last asked.
    pub fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    /// Gives a file Solis just wrote into the assets its sidecar now (see
    /// [`AssetDb::adopt`]).
    pub fn adopt(&self, path: &Path) -> std::io::Result<()> {
        self.db.as_ref().map_or(Ok(()), |db| db.adopt(path))
    }

    /// Looks at the whole assets folder again.
    pub fn rescan(&mut self) -> (Vec<Change>, Vec<String>) {
        self.db
            .as_mut()
            .map_or_else(|| (Vec::new(), Vec::new()), AssetDb::rescan)
    }

    /// Every file and folder in the assets folder (nothing without a
    /// project).
    pub fn listing(&self) -> Listing {
        self.db.as_ref().map(AssetDb::listing).unwrap_or_default()
    }

    /// The Library's list: every file, by name, with its thumbnail if
    /// `thumbnail` has one; and every folder, `/` between names.
    pub fn library(
        listing: &Listing,
        thumbnail: impl Fn(&Path) -> Option<eframe::egui::TextureId>,
    ) -> (Vec<LibraryItem>, Vec<String>) {
        let mut items: Vec<LibraryItem> = listing
            .files
            .iter()
            .map(|f| LibraryItem {
                id: f.id,
                kind: library_kind(f.kind),
                // Files Solis doesn't know keep their whole name, so
                // "brick.png.exe" never passes for "brick.png".
                name: if matches!(f.kind, FileKind::Other | FileKind::Text) {
                    f.path.file_name()
                } else {
                    f.path.file_stem()
                }
                .map_or_else(|| slashed(&f.path), |s| s.to_string_lossy().into_owned()),
                path: slashed(&f.path),
                thumbnail: thumbnail(&f.path),
            })
            .collect();
        items.sort_by_cached_key(|item| item.name.to_lowercase());
        let folders = listing.folders.iter().map(|f| slashed(f)).collect();
        (items, folders)
    }

    /// Renames or moves a file or folder inside the assets, with its
    /// sidecar. `from` is a plain path there (`plain_inside`), and `to`
    /// a name in a folder there (`named_inside`).
    pub fn move_path(&self, from: &Path, to: &Path) -> Result<(), String> {
        let db = self.db.as_ref().ok_or("no project is open")?;
        if !plain_inside(from) {
            return Err(outside(from));
        }
        if !named_inside(to) {
            return Err(outside(to));
        }
        db.move_path(from, to).map_err(|e| e.to_string())
    }

    /// Makes a new folder inside `parent` (inside the assets; empty for
    /// the assets folder itself).
    pub fn new_folder(&self, parent: &Path) -> Result<PathBuf, String> {
        let db = self.db.as_ref().ok_or("no project is open")?;
        if !assets_or_inside(parent) {
            return Err(outside(parent));
        }
        db.new_folder(parent).map_err(|e| e.to_string())
    }

    /// Makes a new sound event in `folder` (inside the assets; empty for
    /// the assets folder itself) holding `text`, named "New sound event"
    /// (with a number if that's taken), and gives it its ID at once.
    /// Returns where it is, inside the assets.
    pub fn new_sound_event(&mut self, folder: &Path, text: &str) -> Result<PathBuf, String> {
        let db = self.db.as_mut().ok_or("no project is open")?;
        let assets = db.project().assets();
        let dir = assets.join(folder);
        // The path is checked before the disk is asked about it.
        if !assets_or_inside(folder) || !dir.is_dir() {
            return Err(format!("{} isn't a folder in the assets", folder.display()));
        }
        let ext = asset_db::SOUND_EVENT_EXTENSION;
        let name = (1..1000)
            .map(|n| {
                if n == 1 {
                    format!("New sound event.{ext}")
                } else {
                    format!("New sound event {n}.{ext}")
                }
            })
            .find(|name| !dir.join(name).exists())
            .ok_or("there are already 999 new sound events in that folder")?;
        let file = dir.join(&name);
        asset_db::write_atomically(&file, text.as_bytes()).map_err(|e| e.to_string())?;
        // Made by Solis just now: taken at once, not left to settle.
        db.adopt(&file).map_err(|e| e.to_string())?;
        db.rescan();
        Ok(folder.join(name))
    }

    /// Where the file at `path` (inside the assets) is on disk, for saving
    /// `what` ("a clips file"). Every saver finds its file here, so none can
    /// write outside the assets folder or over a file of another kind:
    /// `path` must be a plain path inside the assets ([`plain_inside`])
    /// whose name `is_kind` takes for that kind.
    fn place(
        &self,
        path: &Path,
        what: &str,
        is_kind: impl FnOnce(&Path) -> bool,
    ) -> Result<(&AssetDb, PathBuf), String> {
        let db = self.db.as_ref().ok_or("no project is open")?;
        if !plain_inside(path) || !is_kind(path) {
            return Err(format!("it isn't {what} in the assets"));
        }
        Ok((db, db.project().assets().join(path)))
    }

    /// Saves `text` as the file of `kind` at `path` (inside the assets). A
    /// file there in an older format is copied to the backups first
    /// (Phase 8, C1); gives back where it was kept, if it was.
    fn save_versioned(
        &self,
        path: &Path,
        text: &str,
        kind: &Saved<'_>,
    ) -> Result<Option<PathBuf>, String> {
        let (db, file) = self.place(path, kind.what, kind.is)?;
        if !file.is_file() {
            if !kind.made_when_missing {
                return Err("the file isn't there any more".to_owned());
            }
            folder_is_there(&file)?;
            return asset_db::write_atomically(&file, text.as_bytes())
                .map(|()| None)
                .map_err(|e| e.to_string());
        }
        crate::upgrades::save(
            db.project().root(),
            &file,
            text,
            kind.format,
            kind.read_back,
        )
    }

    /// Saves a sound event's file (`.solsound`, inside the assets). Only
    /// one that's there already: events are made by [`Self::new_sound_event`].
    /// Gives back where an older one was kept, if it was.
    pub fn save_sound_event(&self, path: &Path, text: &str) -> Result<Option<PathBuf>, String> {
        let kind = Saved {
            what: "a sound event file",
            is: |path| ends_in(path, &[asset_db::SOUND_EVENT_EXTENSION]),
            format: &sound::event::FORMAT,
            made_when_missing: false,
            read_back: |text| {
                sound::event::read(text)
                    .map(|read| sound::event::write(&read.event))
                    .map_err(|e| e.to_string())
            },
        };
        self.save_versioned(path, text, &kind)
    }

    /// Saves a sprite's clips file (`.solframes`, inside the assets),
    /// making it if it isn't there. Gives back where an older one was
    /// kept, if it was.
    pub fn save_frames(&self, path: &Path, text: &str) -> Result<Option<PathBuf>, String> {
        let kind = Saved {
            what: "a clips file",
            is: animation::frames_file::is_frames_file,
            format: &animation::frames_file::FORMAT,
            made_when_missing: true,
            read_back: |text| {
                animation::frames_file::read(text)
                    .map(|read| animation::frames_file::write(&read.file))
                    .map_err(|e| e.to_string())
            },
        };
        self.save_versioned(path, text, &kind)
    }

    /// Saves an animator file (`.solanim`, inside the assets, Phase 9, D3b),
    /// making it if it isn't there. Gives back where an older one was
    /// kept, if it was.
    pub fn save_animator(&self, path: &Path, text: &str) -> Result<Option<PathBuf>, String> {
        let kind = Saved {
            what: "an animator file",
            is: animation::animator_file::is_animator_file,
            format: &animation::animator_file::FORMAT,
            made_when_missing: true,
            read_back: |text| {
                animation::animator_file::read(text)
                    .map(|read| animation::animator_file::write(&read.file))
                    .map_err(|e| e.to_string())
            },
        };
        self.save_versioned(path, text, &kind)
    }

    /// Saves a new picture (a PNG, Phase 9, D5: a tileset template) at
    /// `path` inside the assets. Never replaces a file already there.
    pub fn save_picture(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        let (_, file) = self.place(path, "a picture", |path| ends_in(path, &["png"]))?;
        if file.exists() {
            return Err("a file is already there".to_owned());
        }
        folder_is_there(&file)?;
        asset_db::write_atomically(&file, bytes).map_err(|e| e.to_string())
    }

    /// Makes a new text file inside the assets (E1): a Lust code file
    /// (`.lst`) or an area (`.solarea`). Never replaces a file.
    pub fn save_new_text(&self, path: &Path, text: &str) -> Result<(), String> {
        let (_, file) = self.place(path, "a code file or an area", |path| {
            ends_in(path, &["lst", "solarea"])
        })?;
        if file.exists() {
            return Err("a file is already there".to_owned());
        }
        folder_is_there(&file)?;
        asset_db::write_atomically(&file, text.as_bytes()).map_err(|e| e.to_string())
    }

    /// Saves an atlas file (`.atlas.lst`, inside the assets, a picture's
    /// crops; decision 0089), making it if it isn't there. Version 1 is
    /// the only one, so there's no older one to keep.
    pub fn save_atlas(&self, path: &Path, text: &str) -> Result<(), String> {
        let (_, file) = self.place(path, "an atlas file", |path| {
            reflect::data_files::is(path, animation::atlas::KIND)
        })?;
        folder_is_there(&file)?;
        asset_db::write_atomically(&file, text.as_bytes()).map_err(|e| e.to_string())
    }

    /// Saves a screen's Lust file (inside the assets) after a change on
    /// the UI editor's canvas (decision 0089). Only a `.lst` file that's
    /// there already: the canvas changes screens, it never makes files.
    pub fn save_screen(&self, path: &Path, text: &str) -> Result<(), String> {
        let (_, file) = self.place(path, "a Lust file", |path| ends_in(path, &["lst"]))?;
        if !file.is_file() {
            return Err("it isn't there any more".to_owned());
        }
        if text.len() > ui::source::MAX_BYTES {
            return Err("it's too big".to_owned());
        }
        asset_db::write_atomically(&file, text.as_bytes()).map_err(|e| e.to_string())
    }

    /// Saves a tileset (`.tileset.lst`, inside the assets, Phase 9, D4),
    /// making it if it isn't there. Gives back where an older one was
    /// kept, if it was.
    pub fn save_tileset(&self, path: &Path, text: &str) -> Result<Option<PathBuf>, String> {
        let kind = Saved {
            what: "a tileset",
            is: tiles::tileset::is_tileset_file,
            format: &tiles::tileset::FORMAT,
            made_when_missing: true,
            read_back: |text| {
                tiles::tileset::read(text)
                    .map(|read| tiles::tileset::write(&read.tileset))
                    .map_err(|e| e.to_string())
            },
        };
        self.save_versioned(path, text, &kind)
    }

    /// Saves a tile map file (`.tilemap.lst`, inside the assets, Phase 9, D4),
    /// making it if it isn't there. Gives back where an older one was
    /// kept, if it was.
    pub fn save_tile_map(&self, path: &Path, text: &str) -> Result<Option<PathBuf>, String> {
        let kind = Saved {
            what: "a tile map",
            is: tiles::map_file::is_map_file,
            format: &tiles::map_file::FORMAT,
            made_when_missing: true,
            read_back: |text| {
                tiles::map_file::read(text)
                    .and_then(|read| tiles::map_file::write(&read.map))
                    .map_err(|e| e.to_string())
            },
        };
        self.save_versioned(path, text, &kind)
    }

    /// Moves a file or folder (inside the assets) to the Recycle Bin.
    pub fn delete(&self, path: &Path) -> Result<(), String> {
        let db = self.db.as_ref().ok_or("no project is open")?;
        if !plain_inside(path) {
            return Err(outside(path));
        }
        db.delete_to_trash(path).map_err(|e| e.to_string())
    }

    /// Copies a model from elsewhere into the project, and returns its
    /// new asset and where its file is now.
    pub fn bring_in(&mut self, source: &Path) -> std::io::Result<(AssetId, PathBuf)> {
        let db = self
            .db
            .as_mut()
            .ok_or_else(|| std::io::Error::other("no project is open"))?;
        // Already inside the assets folder: use it where it is.
        let target = if source.starts_with(db.project().assets()) {
            source.to_path_buf()
        } else {
            db.copy_in(source)?
        };
        // Made by Solis just now: taken at once, not left to settle.
        db.adopt(&target)?;
        db.rescan();
        let id = db
            .id_at(&target)
            .ok_or_else(|| std::io::Error::other("its sidecar couldn't be made"))?;
        Ok((id, target))
    }
}

/// A kind of file with a format version, as an editor saves it
/// (`Projects::save_versioned`).
struct Saved<'a> {
    /// What it's called when a path isn't one ("a clips file").
    what: &'a str,
    /// Whether a path's name says it's one.
    is: fn(&Path) -> bool,
    /// Its format, which tells an older file from a current one.
    format: &'a dyn reflect::upgrade::Versioned,
    /// Whether saving makes the file when it isn't there. Where it
    /// doesn't, a file that has gone is a mistake.
    made_when_missing: bool,
    /// Reads the text and writes it again, to check nothing is lost
    /// (`upgrades::save`).
    read_back: fn(&str) -> Result<String, String>,
}

/// Whether `path` is a plain path inside the assets: names only, with no
/// `..`, root or drive, so joined to the assets folder it stays inside it
/// on every system (`editor::plain_relative`). The one rule for a path a
/// file is saved at or handed to another program by; a path that isn't
/// text isn't one.
pub(crate) fn plain_inside(path: &Path) -> bool {
    path.to_str().is_some_and(editor::plain_relative)
}

/// Whether `path` is the assets folder itself (an empty path) or a plain
/// path inside it ([`plain_inside`]): the rule where a folder is named.
pub(crate) fn assets_or_inside(path: &Path) -> bool {
    path.as_os_str().is_empty() || plain_inside(path)
}

/// Whether `to` is somewhere inside the assets to move a file or folder
/// to: a name, in the assets folder itself or in a plain path inside it
/// ([`plain_inside`]). Whether the name can be a file's is `asset_db`'s
/// to say: its rule for a new name is stricter than this one (no `:` or
/// slash of either kind either), and explains itself to whoever typed
/// the name; a name that isn't new is `from`'s, which this rule passed.
fn named_inside(to: &Path) -> bool {
    let Some(to) = to.to_str() else {
        return false;
    };
    let (folder, name) = match to.rsplit_once(['/', '\\']) {
        Some((folder, name)) => (Some(folder), name),
        None => (None, to),
    };
    folder.is_none_or(editor::plain_relative) && !matches!(name, "" | "." | "..")
}

/// The mistake a path that fails the rule is, in `asset_db`'s words for
/// the same.
fn outside(path: &Path) -> String {
    format!(
        "{} isn't inside the project's assets folder",
        path.display()
    )
}

/// Whether `path`'s name ends in one of `extensions`, in capitals or not.
fn ends_in(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .is_some_and(|e| extensions.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// A mistake if the folder `file` is to be made in isn't there.
fn folder_is_there(file: &Path) -> Result<(), String> {
    match file.parent() {
        Some(folder) if !folder.is_dir() => Err("its folder isn't there any more".to_owned()),
        _ => Ok(()),
    }
}

/// The Library's kind for a file of `kind` (what its name's ending says it
/// is).
pub fn library_kind(kind: FileKind) -> LibraryKind {
    match kind {
        FileKind::Model => LibraryKind::Model,
        FileKind::Assembly => LibraryKind::Assembly,
        FileKind::Area => LibraryKind::Area,
        FileKind::Texture => LibraryKind::Texture,
        FileKind::Code => LibraryKind::Code,
        FileKind::Sound => LibraryKind::Sound,
        FileKind::SoundEvent => LibraryKind::SoundEvent,
        FileKind::SpriteSheet => LibraryKind::SpriteSheet,
        FileKind::SpriteClips => LibraryKind::SpriteClips,
        FileKind::Animator => LibraryKind::Animator,
        FileKind::Tileset => LibraryKind::Tileset,
        FileKind::TileMap => LibraryKind::TileMap,
        FileKind::Atlas => LibraryKind::Atlas,
        FileKind::Text => LibraryKind::Text,
        FileKind::Other => LibraryKind::Other,
    }
}

/// A path inside the assets with `/` between folders, as the Library shows
/// it on every system.
pub fn slashed(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
#[path = "project_saving_tests.rs"]
mod saving_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let f = std::env::temp_dir().join(format!("solis-app-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&f);
        f
    }

    #[test]
    fn a_new_project_is_made_opened_and_remembered() {
        let root = folder("projects");
        std::fs::create_dir_all(&root).unwrap();
        let mut projects = Projects::default();
        let opened = projects
            .open(&root, true, assets::Limits::default())
            .unwrap();
        assert_eq!(
            opened.info.name,
            format!("solis-app-projects-{}", std::process::id())
        );
        assert!(root.join("project.solis").exists());
        assert_eq!(projects.recent(), vec![root.clone()]);
        assert_eq!(asset_db::Project::summary(&root).unwrap().saved_by, None);
        // Opening it again doesn't make it again, and it stays first.
        projects
            .open(&root, false, assets::Limits::default())
            .unwrap();
        assert_eq!(projects.recent().len(), 1);
        assert!(
            projects
                .open(&folder("nowhere"), false, assets::Limits::default())
                .is_err()
        );
    }

    #[test]
    fn a_model_brought_in_is_copied_and_listed() {
        let root = folder("bring-in");
        std::fs::create_dir_all(&root).unwrap();
        let source = folder("bring-in-source");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("crate.glb"), assets::sample::sample_glb()).unwrap();
        let mut projects = Projects::default();
        projects
            .open(&root, true, assets::Limits::default())
            .unwrap();
        let (id, path) = projects.bring_in(&source.join("crate.glb")).unwrap();
        assert!(path.starts_with(root.join("assets")));
        let (library, _) = Projects::library(&projects.listing(), |_| None);
        assert_eq!(library.len(), 1);
        assert_eq!(library[0].id, Some(id));
        assert_eq!(library[0].name, "crate");
        // A file already in the assets folder isn't copied again.
        let (again, same) = projects.bring_in(&path).unwrap();
        assert_eq!((again, same), (id, path));
        assert_eq!(Projects::library(&projects.listing(), |_| None).0.len(), 1);
    }

    #[test]
    fn new_code_files_and_areas_are_made_but_never_replace_or_stray() {
        let root = folder("new-text");
        std::fs::create_dir_all(&root).unwrap();
        let mut projects = Projects::default();
        projects
            .open(&root, true, assets::Limits::default())
            .unwrap();
        projects
            .save_new_text(Path::new("main.lst"), "end\n")
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("assets/main.lst")).unwrap(),
            "end\n"
        );
        projects
            .save_new_text(Path::new("level.solarea"), "area version 3;\n")
            .unwrap();
        // Never over a file, never outside the assets, only code and areas.
        for (path, why) in [
            ("main.lst", "already"),
            ("../escape.lst", "isn't"),
            ("tool.exe", "isn't"),
            ("gone/x.lst", "folder"),
        ] {
            let err = projects.save_new_text(Path::new(path), "x").unwrap_err();
            assert!(err.contains(why), "{path}: {err}");
        }
        assert_eq!(
            std::fs::read_to_string(root.join("assets/main.lst")).unwrap(),
            "end\n"
        );
    }

    #[test]
    fn the_recent_list_is_kept_short() {
        let many: Vec<PathBuf> = (0..20).map(|i| PathBuf::from(format!("/p{i}"))).collect();
        let known = crate::known_projects::KnownProjects::from_saved(None, many);
        assert_eq!(Projects::with_known(known, None).recent().len(), MAX_RECENT);
    }

    #[test]
    fn a_numbered_build_notes_itself_in_each_project_it_opens() {
        let root = folder("noted-build");
        std::fs::create_dir_all(&root).unwrap();
        let mut projects = Projects::with_known(Default::default(), Some(61));
        projects
            .open(&root, true, assets::Limits::default())
            .unwrap();
        assert_eq!(
            asset_db::Project::summary(&root).unwrap().saved_by,
            Some(61)
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn sound_events_are_made_with_free_names_and_saved_only_as_events() {
        let root = folder("sound-events");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let mut projects = Projects::default();
        projects
            .open(&root, true, assets::Limits::default())
            .unwrap();
        let first = projects
            .new_sound_event(Path::new(""), "sound version 1;\n")
            .unwrap();
        let second = projects
            .new_sound_event(Path::new(""), "sound version 1;\n")
            .unwrap();
        assert_eq!(first, PathBuf::from("New sound event.solsound"));
        assert_eq!(second, PathBuf::from("New sound event 2.solsound"));
        let listing = projects.listing();
        let made = listing.files.iter().find(|f| f.path == first).unwrap();
        assert_eq!(made.kind, FileKind::SoundEvent);
        assert!(made.id.is_some(), "an ID at once, not after settling");
        assert!(projects.new_sound_event(Path::new("nowhere"), "x").is_err());

        projects
            .save_sound_event(&first, "sound version 1;\nloop = true;\n")
            .unwrap();
        let text = std::fs::read_to_string(root.join("assets").join(&first)).unwrap();
        assert!(text.contains("loop = true"));
        std::fs::write(root.join("assets/notes.txt"), "mine").unwrap();
        assert!(
            projects
                .save_sound_event(Path::new("notes.txt"), "x")
                .is_err()
        );
        assert!(
            projects
                .save_sound_event(Path::new("gone.solsound"), "x")
                .is_err()
        );
        assert_eq!(
            std::fs::read_to_string(root.join("assets/notes.txt")).unwrap(),
            "mine"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
