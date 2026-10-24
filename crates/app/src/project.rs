//! The open project, as the window holds it: its asset database, the
//! watcher on its assets folder, and the list of recent projects
//! ([decision 0021](../../../docs/decisions/0021-asset-database-and-models.md)).

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
    recent: Vec<PathBuf>,
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
    /// Remembers `recent` (newest first).
    pub fn with_recent(recent: Vec<PathBuf>) -> Self {
        Self {
            recent: recent.into_iter().take(MAX_RECENT).collect(),
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
        let watcher = Watcher::new(&project.assets()).ok();
        let (db, _, notes) = AssetDb::open(project, limits);
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
        self.recent.retain(|p| p != root);
        self.recent.insert(0, root.to_path_buf());
        self.recent.truncate(MAX_RECENT);
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

    /// The projects opened lately, newest first.
    pub fn recent(&self) -> &[PathBuf] {
        &self.recent
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
                kind: match f.kind {
                    FileKind::Model => LibraryKind::Model,
                    FileKind::Assembly => LibraryKind::Assembly,
                    FileKind::Area => LibraryKind::Area,
                    FileKind::Texture => LibraryKind::Texture,
                    FileKind::Code => LibraryKind::Code,
                    FileKind::Other => LibraryKind::Other,
                },
                name: f
                    .path
                    .file_stem()
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
    /// sidecar.
    pub fn move_path(&self, from: &Path, to: &Path) -> Result<(), String> {
        let db = self.db.as_ref().ok_or("no project is open")?;
        db.move_path(from, to).map_err(|e| e.to_string())
    }

    /// Makes a new folder inside `parent` (inside the assets).
    pub fn new_folder(&self, parent: &Path) -> Result<PathBuf, String> {
        let db = self.db.as_ref().ok_or("no project is open")?;
        db.new_folder(parent).map_err(|e| e.to_string())
    }

    /// Moves a file or folder (inside the assets) to the Recycle Bin.
    pub fn delete(&self, path: &Path) -> Result<(), String> {
        let db = self.db.as_ref().ok_or("no project is open")?;
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

/// A path inside the assets with `/` between folders, as the Library shows
/// it on every system.
fn slashed(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

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
        assert_eq!(projects.recent(), std::slice::from_ref(&root));
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
    fn the_recent_list_is_kept_short() {
        let many: Vec<PathBuf> = (0..20).map(|i| PathBuf::from(format!("/p{i}"))).collect();
        assert_eq!(Projects::with_recent(many).recent().len(), MAX_RECENT);
    }
}
