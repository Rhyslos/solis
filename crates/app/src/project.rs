//! The open project, as the window holds it: its asset database, the
//! watcher on its folder, and the list of projects Solis knows
//! ([decision 0021](../../../docs/decisions/0021-asset-database-and-models.md),
//! [decision 0086](../../../docs/decisions/0086-the-launcher.md)).
//!
//! It also saves the files the editors make and change. Every saver is
//! told where by a path in the project, and finds the file through one
//! function (`Projects::place`), which takes only a plain path there, of
//! the saver's own kind: a path can't make one write outside the project
//! folder, or over another kind of file. The Library's file changes (a
//! move, a delete, a new folder or sound event) keep to the same rule
//! (`plain_inside`). Only [`Projects::open`] and [`Projects::bring_in`]
//! take a path from anywhere on the computer: the user chose it there, in
//! the system's own window or by dropping a file.

use asset_db::{AssetDb, Change, FileKind, Listing, Project, Scanned, Watcher};
use editor::{LibraryItem, LibraryKind, ProjectInfo};
use reflect::AssetId;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};

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
    /// Whether the project's files changed since last asked (any file,
    /// not only assets: the Library lists them all).
    changed: bool,
    /// A look at the project's files running on a worker (it can take
    /// seconds in a big project: CONTRIBUTING rule 10).
    scanning: Option<Receiver<Scanned>>,
    /// Whether the folder changed again while that look ran.
    again: bool,
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
        // Older files Solis made move to their new names first (Lust,
        // decision 0081; areas to levels and assemblies to prefabs,
        // decision 0118), so the asset
        // database finds them there; then the project file says so.
        let mut moved = crate::older_files::move_older_files(root, &project.files());
        moved.extend(upgrade_project_file(root, &project));
        // The folder its files are in (decision 0118: its own), Solis's
        // own folders passed over. None if the system won't watch.
        let watcher = Watcher::new_project(&project.files()).ok();
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
            assets: db.project().files(),
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
        // A look at the last project's folder is no longer wanted.
        self.scanning = None;
        self.again = false;
        Ok(Opened {
            info,
            notes,
            watching,
        })
    }

    /// Closes the open project (File › Close project): its database and
    /// watcher go, and nothing of it is looked at again.
    pub fn close(&mut self) {
        self.db = None;
        self.watcher = None;
        self.scanning = None;
        self.again = false;
        self.changed = false;
    }

    /// Gives the open project the kind `kind` (Project settings), in its
    /// project file.
    pub fn set_kind(&mut self, kind: asset_db::ProjectKind) -> Result<(), String> {
        let db = self.db.as_mut().ok_or("no project is open")?;
        db.set_project_kind(kind)
            .map(drop)
            .map_err(|e| e.to_string())
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

    /// What changed in the project, found by the last look at it
    /// that has finished, with notes in plain words. Never waits: the
    /// folder is looked at on a worker when the watcher reports a change,
    /// and what it found is handed back by a later call.
    pub fn poll(&mut self) -> (Vec<Change>, Vec<String>) {
        let mut found = (Vec::new(), Vec::new());
        if let Some(scanning) = &self.scanning {
            match scanning.try_recv() {
                Ok(scanned) => {
                    self.scanning = None;
                    self.changed = true;
                    if let Some(taken) = self.db.as_mut().and_then(|db| db.take(scanned)) {
                        found = taken;
                    }
                }
                Err(TryRecvError::Empty) => {}
                // The look ended without an answer: the next change
                // starts another.
                Err(TryRecvError::Disconnected) => self.scanning = None,
            }
        }
        let heard = self.watcher.as_ref().is_some_and(Watcher::changed);
        self.again |= heard;
        // A new file left waiting for its sidecar is looked at again once
        // it has settled, even if nothing else happens.
        self.again |= self.scanning.is_none()
            && self
                .db
                .as_ref()
                .and_then(AssetDb::look_again_at)
                .is_some_and(|at| std::time::Instant::now() >= at);
        if self.again && self.scanning.is_none() {
            self.start_scan();
        }
        found
    }

    /// Starts a look at the project's files on a worker.
    fn start_scan(&mut self) {
        let Some(db) = self.db.as_mut() else {
            return;
        };
        let scan = db.scan();
        let (sender, receiver) = std::sync::mpsc::channel();
        let started = std::thread::Builder::new()
            .name("assets scan".into())
            .spawn(move || {
                let _ = sender.send(scan.run());
            });
        if started.is_ok() {
            self.scanning = Some(receiver);
            self.again = false;
        }
    }

    /// Whether the project's folder is being looked at, or a new file is
    /// waiting for its sidecar, to be looked at again shortly.
    pub fn waiting(&self) -> bool {
        self.scanning.is_some()
            || self
                .db
                .as_ref()
                .is_some_and(|db| db.look_again_at().is_some())
    }

    /// Whether anything in the project changed since last asked.
    pub fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    /// Gives a file Solis just wrote into the assets its sidecar now (see
    /// [`AssetDb::adopt`]), and returns its ID.
    pub fn adopt(&self, path: &Path) -> std::io::Result<AssetId> {
        self.db
            .as_ref()
            .ok_or_else(|| std::io::Error::other("no project is open"))?
            .adopt(path)
    }

    /// Looks at the whole project again, now (after Solis changed
    /// it), unless a look on a worker is walking it: then nothing now,
    /// never waiting for that look (the second review pass's N-13), and
    /// another look follows it on the worker, whose findings come from
    /// [`Self::poll`]. A look on a worker begun before one made now is out
    /// of date, and dropped.
    pub fn rescan(&mut self) -> (Vec<Change>, Vec<String>) {
        let Some(db) = self.db.as_mut() else {
            return (Vec::new(), Vec::new());
        };
        db.rescan_if_free().unwrap_or_else(|| {
            self.again = true;
            (Vec::new(), Vec::new())
        })
    }

    /// Every file and folder in the project (nothing without a
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
                // The project's own tags.lst, at the top of its files, is
                // the project's tags, never game code (stage 1).
                kind: if f.path == Path::new(level::tags::FILE_NAME) {
                    LibraryKind::Tags
                } else {
                    library_kind(f.kind)
                },
                // Files Solis doesn't know keep their whole name, so
                // "brick.png.exe" never passes for "brick.png".
                // A data file's name is its stem: `chicken`, not
                // `chicken.prefab`.
                name: if matches!(f.kind, FileKind::Other | FileKind::Text) {
                    f.path.file_name().map(|n| n.to_string_lossy().into_owned())
                } else {
                    reflect::data_files::stem(&f.path)
                }
                .unwrap_or_else(|| slashed(&f.path)),
                path: slashed(&f.path),
                thumbnail: thumbnail(&f.path),
            })
            .collect();
        items.sort_by_cached_key(|item| item.name.to_lowercase());
        let folders = listing.folders.iter().map(|f| slashed(f)).collect();
        (items, folders)
    }

    /// Renames or moves a file or folder in the project, with its
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

    /// Makes a new folder inside `parent` (in the project; empty for
    /// its own folder).
    pub fn new_folder(&self, parent: &Path) -> Result<PathBuf, String> {
        let db = self.db.as_ref().ok_or("no project is open")?;
        if !assets_or_inside(parent) {
            return Err(outside(parent));
        }
        db.new_folder(parent).map_err(|e| e.to_string())
    }

    /// Makes a new sound event in `folder` (in the project; empty for
    /// its own folder) holding `text`, named "New sound event"
    /// (with a number if that's taken), and gives it its ID at once.
    /// Returns where it is, in the project.
    pub fn new_sound_event(&mut self, folder: &Path, text: &str) -> Result<PathBuf, String> {
        let db = self.db.as_mut().ok_or("no project is open")?;
        let assets = db.project().files();
        let dir = assets.join(folder);
        // The path is checked before the disk is asked about it.
        if !assets_or_inside(folder) || !dir.is_dir() {
            return Err(format!(
                "{} isn't a folder in the project",
                folder.display()
            ));
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
        self.rescan();
        Ok(folder.join(name))
    }

    /// Where the file at `path` (in the project) is on disk, for saving
    /// `what` ("a clips file"). Every saver finds its file here, so none can
    /// write outside the project's folder or over a file of another kind:
    /// `path` must be a plain path in the project ([`plain_inside`])
    /// whose name `is_kind` takes for that kind.
    fn place(
        &self,
        path: &Path,
        what: &str,
        is_kind: impl FnOnce(&Path) -> bool,
    ) -> Result<(&AssetDb, PathBuf), String> {
        let db = self.db.as_ref().ok_or("no project is open")?;
        if !plain_inside(path) || !is_kind(path) {
            return Err(format!("it isn't {what} in the project"));
        }
        Ok((db, db.project().files().join(path)))
    }

    /// Saves `text` as the file of `kind` at `path` (in the project). A
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

    /// Saves a sound event's file (`.solsound`, in the project). Only
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

    /// Saves a file of one of the kinds the editor changes itself
    /// (`WorkbenchAction::SaveFile`), as `text`, with its kind's saver.
    /// Gives back where an older one was kept, if it was.
    pub fn save_file(
        &self,
        path: &Path,
        file: &editor::OwnFile,
        text: &str,
    ) -> Result<Option<PathBuf>, String> {
        use editor::OwnFile;
        match file {
            OwnFile::Frames(_) => self.save_frames(path, text),
            OwnFile::Animator(_) => self.save_animator(path, text),
            OwnFile::Tileset(_) => self.save_tileset(path, text),
            OwnFile::Atlas(_) => self.save_atlas(path, text).map(|()| None),
            OwnFile::TileMap(_) => self.save_tile_map(path, text),
            OwnFile::Table(_) => self.save_table(path, text),
        }
    }

    /// Saves a data table (`.table.lst`, stage 1), making it if it isn't
    /// there. Gives back where an older one was kept, if it was.
    pub fn save_table(&self, path: &Path, text: &str) -> Result<Option<PathBuf>, String> {
        let kind = Saved {
            what: "a data table",
            is: level::table::file::is_table_file,
            format: &level::table::file::FORMAT,
            made_when_missing: true,
            read_back: |text| {
                level::table::file::read(text, "Row")
                    .map(|read| level::table::file::write(&read.table))
                    .map_err(|e| e.to_string())
            },
        };
        self.save_versioned(path, text, &kind)
    }

    /// Saves a sprite's clips file (`.solframes`, in the project),
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

    /// Saves an animator file (`.solanim`, in the project, Phase 9, D3b),
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
    /// `path` in the project. Never replaces a file already there.
    pub fn save_picture(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        let (_, file) = self.place(path, "a picture", |path| ends_in(path, &["png"]))?;
        if file.exists() {
            return Err("a file is already there".to_owned());
        }
        folder_is_there(&file)?;
        asset_db::write_atomically(&file, bytes).map_err(|e| e.to_string())
    }

    /// Writes a data table's CSV (`items.csv`, in the project), replacing
    /// one there (stage 1, data tables).
    pub fn write_csv(&self, path: &Path, text: &str) -> Result<(), String> {
        let (_, file) = self.place(path, "a CSV file", |path| ends_in(path, &["csv"]))?;
        folder_is_there(&file)?;
        asset_db::write_atomically(&file, text.as_bytes()).map_err(|e| e.to_string())
    }

    /// Reads a CSV file in the project, within the tables' limit, to take
    /// into a data table.
    pub fn read_csv(&self, path: &Path) -> Result<String, String> {
        let (_, file) = self.place(path, "a CSV file", |path| ends_in(path, &["csv"]))?;
        if !file.is_file() {
            return Err(format!(
                "there's no {} beside the table: export one, or put a CSV of that name there",
                path.display()
            ));
        }
        let bytes = asset_db::read_limited(&file, level::table::text::MAX_BYTES as u64)
            .map_err(|e| e.to_string())?;
        String::from_utf8(bytes).map_err(|_| "it isn't text (save it as UTF-8 CSV)".to_owned())
    }

    /// Makes a new text file in the project (E1): a Lust code file
    /// (`.lst`) or a level (`.sollevel`). Never replaces a file.
    pub fn save_new_text(&self, path: &Path, text: &str) -> Result<(), String> {
        let (_, file) = self.place(path, "a code file or a level", |path| {
            ends_in(path, &["lst", asset_db::LEVEL_EXTENSION])
        })?;
        if file.exists() {
            return Err("a file is already there".to_owned());
        }
        folder_is_there(&file)?;
        asset_db::write_atomically(&file, text.as_bytes()).map_err(|e| e.to_string())
    }

    /// Saves an atlas file (`.atlas.lst`, in the project, a picture's
    /// crops; decision 0089), making it if it isn't there. Version 1 is
    /// the only one, so there's no older one to keep.
    pub fn save_atlas(&self, path: &Path, text: &str) -> Result<(), String> {
        let (_, file) = self.place(path, "an atlas file", |path| {
            reflect::data_files::is(path, animation::atlas::KIND)
        })?;
        folder_is_there(&file)?;
        asset_db::write_atomically(&file, text.as_bytes()).map_err(|e| e.to_string())
    }

    /// Saves a screen's Lust file (in the project) after a change on
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

    /// Saves a tileset (`.tileset.lst`, in the project, Phase 9, D4),
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

    /// Saves a tile map file (`.tilemap.lst`, in the project, Phase 9, D4),
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

    /// Copies the file at `path` (in the project) beside it, as "hay
    /// copy.png" (or "hay copy 2.png" …), and gives the copy an ID of its
    /// own at once. Returns where the copy is, in the project.
    pub fn duplicate(&mut self, path: &Path) -> Result<PathBuf, String> {
        let db = self.db.as_mut().ok_or("no project is open")?;
        if !plain_inside(path) {
            return Err(outside(path));
        }
        let copy = db.duplicate(path).map_err(|e| e.to_string())?;
        let full = db.project().files().join(&copy);
        // An asset gets its ID now; any other file needs none.
        if asset_db::AssetKind::of(&full).is_some() {
            db.adopt(&full).map_err(|e| e.to_string())?;
        }
        self.rescan();
        Ok(copy)
    }

    /// Moves a file or folder (in the project) to the Recycle Bin.
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
        // Already in the project: used where it is.
        let target = if source.starts_with(db.project().files())
            && source
                .strip_prefix(db.project().files())
                .is_ok_and(|inside| !asset_db::is_own(inside))
        {
            source.to_path_buf()
        } else {
            db.copy_in(source)?
        };
        // Made by Solis just now: taken at once, not left to settle. Its
        // ID is known now, whether or not the look below can be made.
        let id = db.adopt(&target)?;
        self.rescan();
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

/// Brings the project file up to this build's format, keeping a copy of
/// the older one in the backups (`Project::upgrade_file`): format 2 once
/// its levels are `.sollevel` files (decision 0118), so a build from
/// before refuses the project as newer instead of opening it without
/// them. A note for the Console when it changed or couldn't be.
fn upgrade_project_file(root: &Path, project: &Project) -> Option<String> {
    let file = asset_db::PROJECT_FILE;
    match project.upgrade_file() {
        Ok(Some(copy)) => Some(crate::upgrades::kept_note(root, file, &copy)),
        Ok(None) => None,
        Err(why) => Some(format!(
            "{file} couldn't be saved in the current format ({why}); it's read as it is."
        )),
    }
}

/// Whether `path` is a plain path inside the project: names only, with no
/// `..`, root or drive, so joined to the project's folder it stays inside
/// it on every system (`editor::plain_relative`), and not in Solis's own
/// folders or the project file (`asset_db::is_own`). The one rule for a
/// path a file is saved at or handed to another program by; a path that
/// isn't text isn't one.
pub(crate) fn plain_inside(path: &Path) -> bool {
    path.to_str().is_some_and(editor::plain_relative) && !asset_db::is_own(path)
}

/// Whether `path` is the project's own folder (an empty path) or a plain
/// path inside it ([`plain_inside`]): the rule where a folder is named.
pub(crate) fn assets_or_inside(path: &Path) -> bool {
    path.as_os_str().is_empty() || plain_inside(path)
}

/// Whether `to` is somewhere in the project to move a file or folder
/// to: a name, in the project's own folder or in a plain path inside it
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
    folder.is_none_or(editor::plain_relative)
        && !matches!(name, "" | "." | "..")
        && !asset_db::is_own(Path::new(to))
}

/// The mistake a path that fails the rule is, in `asset_db`'s words for
/// the same.
fn outside(path: &Path) -> String {
    format!("{} isn't inside the project's folder", path.display())
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
        FileKind::Prefab => LibraryKind::Prefab,
        FileKind::Level => LibraryKind::Level,
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
        FileKind::Table => LibraryKind::Table,
        FileKind::Text => LibraryKind::Text,
        FileKind::Font => LibraryKind::Font,
        FileKind::Other => LibraryKind::Other,
    }
}

/// A path in the project with `/` between folders, as the Library shows
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
#[path = "project_levels_tests.rs"]
mod levels_tests;

#[cfg(test)]
#[path = "project_prefabs_tests.rs"]
mod prefabs_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> reflect::test_support::TempFolder {
        reflect::test_support::TempFolder::new(&format!("app-{name}")).unwrap()
    }

    /// Polls `projects` frame by frame until a look finds something (at
    /// most ten seconds): what it found, the longest a poll took, and
    /// whether a poll came back empty while the folder was being looked
    /// at.
    fn poll_until_found(projects: &mut Projects) -> (Vec<Change>, f64, bool) {
        let start = std::time::Instant::now();
        let (mut longest, mut looked_on_a_worker) = (0.0_f64, false);
        while start.elapsed().as_secs() < 10 {
            let frame = std::time::Instant::now();
            let (changes, _) = projects.poll();
            longest = longest.max(frame.elapsed().as_secs_f64() * 1000.0);
            if !changes.is_empty() {
                return (changes, longest, looked_on_a_worker);
            }
            looked_on_a_worker |= projects.scanning.is_some();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        (Vec::new(), longest, looked_on_a_worker)
    }

    /// The review, BUG-32: when the watcher said the assets changed, the
    /// frame looked at the whole folder itself (walking it, reading every
    /// sidecar, hashing new files). The look runs on a worker now, and
    /// what it found comes back with a later frame.
    #[test]
    fn a_change_in_the_assets_is_looked_at_on_a_worker() {
        let root = folder("scan-worker");
        std::fs::create_dir_all(&root).unwrap();
        let mut projects = Projects::default();
        projects
            .open(&root, true, assets::Limits::default())
            .unwrap();
        let file = root.join("crate.glb");
        std::fs::write(&file, assets::sample::sample_glb()).unwrap();
        projects.adopt(&file).unwrap();
        let (changes, _, looked_on_a_worker) = poll_until_found(&mut projects);
        let id = projects.db().and_then(|db| db.id_at(&file));
        assert_eq!(
            changes,
            id.map(Change::Added).into_iter().collect::<Vec<_>>()
        );
        assert!(looked_on_a_worker, "the frame never looked itself");
        assert!(
            projects.take_changed(),
            "the Library lists the folder again"
        );
    }

    /// The project's own folder holds its files (decision 0118, R4): no
    /// assets folder is made, what's put beside the project file is seen,
    /// and Solis's own folders are passed over. A project from before
    /// that keeps its files in an assets folder is left as it was.
    #[test]
    fn a_project_s_own_folder_holds_its_files() {
        let root = folder("own-folder");
        std::fs::create_dir_all(&root).unwrap();
        let mut projects = Projects::default();
        let opened = projects
            .open(&root, true, assets::Limits::default())
            .unwrap();
        assert!(opened.watching, "the project's own folder is watched");
        assert_eq!(
            opened.info.assets,
            root.to_path_buf(),
            "the Library shows the project's folder"
        );
        std::fs::create_dir_all(root.join("farm")).unwrap();
        let file = root.join("farm/crate.glb");
        std::fs::write(&file, assets::sample::sample_glb()).unwrap();
        let (changes, _, _) = poll_until_found(&mut projects);
        assert!(
            matches!(changes.as_slice(), [Change::Added(_)]),
            "{changes:?}"
        );
        let listing = projects.listing();
        assert_eq!(listing.folders, [PathBuf::from("farm")], "not the cache");
        assert!(
            listing
                .files
                .iter()
                .all(|f| f.path != Path::new("project.solis"))
        );
        // A file Solis saves goes where it's asked, with no folder made
        // for it; nothing is saved into Solis's own folders.
        projects
            .save_new_text(Path::new("main.lst"), "end\n")
            .unwrap();
        assert!(root.join("main.lst").is_file());
        assert!(!root.join("assets").exists());
        assert!(
            projects
                .save_new_text(Path::new("builds/x.lst"), "end\n")
                .is_err()
        );
        assert!(
            projects
                .save_new_text(Path::new("project.solis"), "x")
                .is_err()
        );
        projects.close();
        // An older project: its files stay in its assets folder.
        let older = folder("own-folder-older");
        asset_db::Project::create(&older, "Older").unwrap();
        std::fs::create_dir_all(older.join("assets/props")).unwrap();
        std::fs::write(older.join("assets/main.lst"), "end\n").unwrap();
        let opened = projects
            .open(&older, false, assets::Limits::default())
            .unwrap();
        assert_eq!(opened.info.assets, older.join("assets"));
        assert_eq!(projects.listing().folders, [PathBuf::from("props")]);
        projects.close();
        assert!(
            projects.db().is_none() && projects.root().is_none(),
            "closed"
        );
    }

    /// The review, BUG-32: a new 256 MB sound settled in the project. Its
    /// first look hashes it whole; that was the frame's wait before.
    #[test]
    #[ignore = "scale check: run by tools/scale/scale-check.sh"]
    fn scale_check_a_big_new_file_in_the_assets() {
        const TARGET_MS: f64 = 1.0;
        let root = folder("scale-scan");
        std::fs::create_dir_all(&root).unwrap();
        let mut projects = Projects::default();
        projects
            .open(&root, true, assets::Limits::default())
            .unwrap();
        let file = root.join("music.wav");
        let big = std::fs::File::create(&file).unwrap();
        big.set_len(256 << 20).unwrap(); // sparse: nothing is written
        let long_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
        big.set_modified(long_ago).unwrap();
        drop(big);
        let (changes, ms, _) = poll_until_found(&mut projects);
        assert_eq!(changes.len(), 1, "{changes:?}");
        // The same look made here and now, as the frame used to.
        std::fs::remove_file(root.join("music.wav.solmeta")).unwrap();
        let start = std::time::Instant::now();
        projects.rescan();
        let look_ms = start.elapsed().as_secs_f64() * 1000.0;
        let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
        println!(
            "Scale check: the longest frame's part while a new 256 MB file is looked at {ms:.3} ms \
             (target {TARGET_MS} ms; the look itself, now on a worker, {look_ms:.0} ms) {verdict}"
        );
        assert!(ms <= TARGET_MS, "{ms:.3} ms");
    }

    #[test]
    fn a_new_project_is_made_opened_and_remembered() {
        let root = folder("projects");
        std::fs::create_dir_all(&root).unwrap();
        let mut projects = Projects::default();
        let opened = projects
            .open(&root, true, assets::Limits::default())
            .unwrap();
        // Named after its folder.
        assert_eq!(
            Some(opened.info.name.as_str()),
            root.file_name().and_then(|n| n.to_str())
        );
        assert!(root.join("project.solis").exists());
        assert_eq!(projects.recent(), vec![root.to_path_buf()]);
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
        assert_eq!(path, root.join("crate.glb"), "beside the project file");
        let (library, _) = Projects::library(&projects.listing(), |_| None);
        assert_eq!(library.len(), 1);
        assert_eq!(library[0].id, Some(id));
        assert_eq!(library[0].name, "crate");
        // A file already in the project isn't copied again.
        let (again, same) = projects.bring_in(&path).unwrap();
        assert_eq!((again, same), (id, path));
        assert_eq!(Projects::library(&projects.listing(), |_| None).0.len(), 1);
    }

    #[test]
    fn new_code_files_and_levels_are_made_but_never_replace_or_stray() {
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
            std::fs::read_to_string(root.join("main.lst")).unwrap(),
            "end\n"
        );
        projects
            .save_new_text(Path::new("level.sollevel"), "level version 4;\n")
            .unwrap();
        // Never over a file, never outside the project, only code and levels.
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
            std::fs::read_to_string(root.join("main.lst")).unwrap(),
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
    }

    #[test]
    fn sound_events_are_made_with_free_names_and_saved_only_as_events() {
        let root = folder("sound-events");
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

        // Saved as the editor writes it: the current version, which reads
        // back the same (the file made a moment ago is version 1, so a
        // copy of it is kept first).
        let looping = sound::event::write(&sound::SoundEvent {
            looping: true,
            ..sound::SoundEvent::default()
        });
        projects.save_sound_event(&first, &looping).unwrap();
        let text = std::fs::read_to_string(root.join(&first)).unwrap();
        assert!(text.contains("loop = true"));
        std::fs::write(root.join("notes.txt"), "mine").unwrap();
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
            std::fs::read_to_string(root.join("notes.txt")).unwrap(),
            "mine"
        );
    }
}
