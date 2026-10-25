//! The open project, as the window holds it: its asset database, the
//! watcher on its assets folder, and the list of recent projects
//! ([decision 0021](../../../docs/decisions/0021-asset-database-and-models.md)).

use asset_db::{AssetDb, Change, Project, Watcher};
use editor::{LibraryItem, ProjectInfo};
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
        let info = ProjectInfo {
            name: db.project().name().to_string(),
            root: root.to_path_buf(),
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
        if !changed {
            return (Vec::new(), Vec::new());
        }
        self.rescan()
    }

    /// Looks at the whole assets folder again.
    pub fn rescan(&mut self) -> (Vec<Change>, Vec<String>) {
        self.db
            .as_mut()
            .map_or_else(|| (Vec::new(), Vec::new()), AssetDb::rescan)
    }

    /// The Library's list: every asset, by name.
    pub fn library(&self) -> Vec<LibraryItem> {
        let Some(db) = &self.db else {
            return Vec::new();
        };
        let mut items: Vec<LibraryItem> = db
            .records()
            .values()
            .map(|r| LibraryItem {
                id: r.id,
                name: r.name(),
                path: r.path.display().to_string(),
            })
            .collect();
        items.sort_by_key(|item| item.name.to_lowercase());
        items
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
        db.rescan();
        let id = db
            .id_at(&target)
            .ok_or_else(|| std::io::Error::other("its sidecar couldn't be made"))?;
        Ok((id, target))
    }
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
        let library = projects.library();
        assert_eq!(library.len(), 1);
        assert_eq!(library[0].id, id);
        assert_eq!(library[0].name, "crate");
        // A file already in the assets folder isn't copied again.
        let (again, same) = projects.bring_in(&path).unwrap();
        assert_eq!((again, same), (id, path));
        assert_eq!(projects.library().len(), 1);
    }

    #[test]
    fn the_recent_list_is_kept_short() {
        let many: Vec<PathBuf> = (0..20).map(|i| PathBuf::from(format!("/p{i}"))).collect();
        assert_eq!(Projects::with_recent(many).recent().len(), MAX_RECENT);
    }
}
