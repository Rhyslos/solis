//! Every project Solis knows, for the launcher ([decision
//! 0086](../../../docs/decisions/0086-the-launcher.md)): where each is,
//! whether it's pinned, when it was last opened, and where on this
//! computer it was last exported for playtesting. Kept between runs;
//! the list as it was before the launcher (eight recent folders) is read
//! once into it. Nothing here touches a project's files: removing one from
//! the list only forgets it.

use editor::KnownProject;
use editor::export_dialog::local_folder;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The most projects kept in the list: the oldest unpinned go first.
pub const MAX_KNOWN: usize = 200;

/// One project in the list, as saved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Known {
    /// Its folder.
    pub root: PathBuf,
    /// Kept at the top of the list.
    #[serde(default)]
    pub pinned: bool,
    /// When it was last opened (seconds since 1970), if known: projects
    /// from the list before the launcher have none.
    #[serde(default)]
    pub last_opened: Option<u64>,
    /// Its name when last read, shown while its folder can't be found.
    #[serde(default)]
    pub name: Option<String>,
    /// The folder it was last exported to for playtesting, if it was.
    /// Kept here, in the program's own settings, and never in the
    /// project: a project from someone else mustn't say where on this
    /// computer (or on which other computer) its export goes.
    #[serde(default)]
    pub exported_to: Option<PathBuf>,
}

/// The list, most recently opened first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KnownProjects {
    list: Vec<Known>,
}

/// Seconds since 1970, now.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl KnownProjects {
    /// The list as saved, or, the first time, the recent projects saved
    /// before the launcher (newest first). Empty folders and repeats are
    /// dropped, and so is an export folder that isn't a whole path on
    /// this computer's own drives (saved settings can be changed too).
    pub fn from_saved(saved: Option<Vec<Known>>, older_recent: Vec<PathBuf>) -> Self {
        let list = saved.unwrap_or_else(|| {
            older_recent
                .into_iter()
                .map(|root| Known {
                    root,
                    pinned: false,
                    last_opened: None,
                    name: None,
                    exported_to: None,
                })
                .collect()
        });
        let mut known = Self::default();
        for mut entry in list {
            if entry.root.as_os_str().is_empty() || known.find(&entry.root).is_some() {
                continue;
            }
            entry.exported_to = entry.exported_to.filter(|f| local_folder(f));
            known.list.push(entry);
        }
        known.trim();
        known
    }

    /// The list, for saving.
    pub fn saved(&self) -> &[Known] {
        &self.list
    }

    fn find(&self, root: &Path) -> Option<usize> {
        self.list.iter().position(|k| k.root == root)
    }

    /// Notes that the project in `root` (called `name`) was opened at
    /// `when`: it goes to the top, keeping its pin and where it was last
    /// exported.
    pub fn opened(&mut self, root: &Path, name: &str, when: u64) {
        let before = self.find(root).map(|at| self.list.remove(at));
        self.list.insert(
            0,
            Known {
                root: root.to_path_buf(),
                pinned: before.as_ref().is_some_and(|k| k.pinned),
                last_opened: Some(when),
                name: Some(name.to_string()),
                exported_to: before.and_then(|k| k.exported_to),
            },
        );
        self.trim();
    }

    /// Notes that the project in `root` was exported for playtesting to
    /// `folder`, so the next export is suggested beside it. Only a whole
    /// path on this computer's own drives is kept: an export to a folder
    /// on another computer works, but isn't suggested again.
    pub fn exported(&mut self, root: &Path, folder: &Path) {
        if let Some(at) = self.find(root) {
            self.list[at].exported_to = local_folder(folder).then(|| folder.to_path_buf());
        }
    }

    /// Where the project in `root` was last exported for playtesting on
    /// this computer, if it was.
    pub fn exported_to(&self, root: &Path) -> Option<&Path> {
        let known = &self.list[self.find(root)?];
        known.exported_to.as_deref()
    }

    /// Pins or unpins the project in `root`.
    pub fn pin(&mut self, root: &Path, pinned: bool) {
        if let Some(at) = self.find(root) {
            self.list[at].pinned = pinned;
        }
    }

    /// Adds the project in `root` if it's new (found by a scan), at the
    /// end, never opened. Returns whether it was new.
    pub fn add(&mut self, root: &Path) -> bool {
        if self.find(root).is_some() || self.list.len() >= MAX_KNOWN {
            return false;
        }
        self.list.push(Known {
            root: root.to_path_buf(),
            pinned: false,
            last_opened: None,
            name: None,
            exported_to: None,
        });
        true
    }

    /// The project known in `old` is now in `new` (Locate…): it keeps its
    /// pin and when it was opened. A project already known in `new` is
    /// kept instead.
    pub fn relocate(&mut self, old: &Path, new: &Path) {
        let Some(at) = self.find(old) else {
            return;
        };
        if self.find(new).is_some() {
            self.list.remove(at);
            return;
        }
        self.list[at].root = new.to_path_buf();
        self.list[at].name = None;
    }

    /// Forgets the project in `root`. Its files stay where they are.
    pub fn forget(&mut self, root: &Path) {
        if let Some(at) = self.find(root) {
            self.list.remove(at);
        }
    }

    /// The most recently opened, for File > Recent projects.
    pub fn recent(&self, how_many: usize) -> Vec<PathBuf> {
        self.list
            .iter()
            .take(how_many)
            .map(|k| k.root.clone())
            .collect()
    }

    /// Keeps the list within [`MAX_KNOWN`], dropping the oldest unpinned
    /// projects first.
    fn trim(&mut self) {
        while self.list.len() > MAX_KNOWN {
            match self.list.iter().rposition(|k| !k.pinned) {
                Some(at) => {
                    self.list.remove(at);
                }
                None => {
                    self.list.pop();
                }
            }
        }
    }

    /// The list as the launcher shows it: each project's name and the
    /// build that last opened it, read from its project file now (a
    /// folder that's gone, or a file that can't be read, says so). Names
    /// read are remembered for when a folder goes missing.
    pub fn rows(&mut self) -> Vec<KnownProject> {
        self.list
            .iter_mut()
            .map(|known| {
                let (name, saved_by, problem) = match asset_db::Project::summary(&known.root) {
                    Ok(summary) => {
                        known.name = Some(summary.name.clone());
                        (summary.name, summary.saved_by, None)
                    }
                    Err(why) => {
                        let name = known.name.clone().unwrap_or_else(|| {
                            known.root.file_name().map_or_else(
                                || known.root.display().to_string(),
                                |n| n.to_string_lossy().into_owned(),
                            )
                        });
                        let problem = match why {
                            asset_db::ProjectError::NotAProject(_) if !known.root.is_dir() => {
                                "Folder not found (moved, deleted, or on a drive that's \
                                 unplugged?)"
                                    .to_string()
                            }
                            asset_db::ProjectError::NotAProject(_) => {
                                format!("{} is gone from its folder", asset_db::PROJECT_FILE)
                            }
                            asset_db::ProjectError::TooNew(_) => {
                                format!(
                                    "Made by a newer {}: update to open it",
                                    crate::branding::APP_NAME
                                )
                            }
                            other => format!("Can't be read: {other}"),
                        };
                        (name, None, Some(problem))
                    }
                };
                KnownProject {
                    root: known.root.clone(),
                    name,
                    problem,
                    pinned: known.pinned,
                    last_opened: known.last_opened,
                    saved_by,
                }
            })
            .collect()
    }
}

/// Slow tidying done on a worker: copying a project's folder, or looking
/// through a folder for projects.
#[derive(Default)]
pub struct Tidying {
    job: Option<std::sync::mpsc::Receiver<Tidied>>,
}

/// What a worker finished.
#[derive(Debug)]
pub enum Tidied {
    /// A project copied to `to` (or why not).
    Copied {
        /// The copy's folder.
        to: PathBuf,
        /// What was copied, or why it wasn't.
        result: Result<asset_db::tidy::Copied, String>,
    },
    /// Projects found in a folder, and whether the scan stopped early.
    Scanned {
        /// The folder looked through.
        folder: PathBuf,
        /// The projects found.
        found: Vec<PathBuf>,
        /// Whether it stopped at its limit.
        stopped: bool,
    },
}

impl Tidying {
    /// Whether a job is running (another waits for it).
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    /// Copies the project in `from` to `to`, named `name`, on a worker.
    pub fn copy(&mut self, from: PathBuf, to: PathBuf, name: String) {
        self.start(move || {
            let result = asset_db::tidy::copy_project(&from, &to, &name);
            Tidied::Copied { to, result }
        });
    }

    /// Looks through `folder` for projects, on a worker.
    pub fn scan(&mut self, folder: PathBuf) {
        self.start(move || {
            let (found, stopped) = asset_db::tidy::find_projects(&folder);
            Tidied::Scanned {
                folder,
                found,
                stopped,
            }
        });
    }

    fn start(&mut self, work: impl FnOnce() -> Tidied + Send + 'static) {
        let (send, receive) = std::sync::mpsc::channel();
        let started = std::thread::Builder::new()
            .name("tidying projects".into())
            .spawn(move || {
                let _ = send.send(work());
            });
        if started.is_ok() {
            self.job = Some(receive);
        }
    }

    /// The job's result, once it's done (never waits).
    pub fn done(&mut self) -> Option<Tidied> {
        let result = self.job.as_ref()?.try_recv();
        match result {
            Ok(done) => {
                self.job = None;
                Some(done)
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.job = None;
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known(root: &str) -> Known {
        Known {
            root: root.into(),
            pinned: false,
            last_opened: Some(1),
            name: None,
            exported_to: None,
        }
    }

    #[test]
    fn the_recent_list_from_before_is_read_once() {
        let older = vec!["/a".into(), "/b".into(), "/a".into(), "".into()];
        let list = KnownProjects::from_saved(None, older.clone());
        assert_eq!(list.recent(8), vec![PathBuf::from("/a"), "/b".into()]);
        assert!(list.saved().iter().all(|k| k.last_opened.is_none()));
        // Once saved, the old list is ignored.
        let saved = KnownProjects::from_saved(Some(vec![known("/c")]), older);
        assert_eq!(saved.recent(8), vec![PathBuf::from("/c")]);
        assert_eq!(
            KnownProjects::from_saved(Some(Vec::new()), vec!["/a".into()]).recent(8),
            Vec::<PathBuf>::new(),
            "an emptied list stays empty"
        );
    }

    #[test]
    fn opening_moves_a_project_to_the_top_and_keeps_its_pin() {
        let mut list = KnownProjects::from_saved(Some(vec![known("/a"), known("/b")]), vec![]);
        list.pin(Path::new("/b"), true);
        list.opened(Path::new("/b"), "Bee", 500);
        let top = &list.saved()[0];
        assert_eq!(top.root, PathBuf::from("/b"));
        assert!(top.pinned);
        assert_eq!(top.last_opened, Some(500));
        assert_eq!(top.name.as_deref(), Some("Bee"));
        list.opened(Path::new("/new"), "New", 600);
        assert_eq!(list.recent(1), vec![PathBuf::from("/new")]);
        assert_eq!(list.saved().len(), 3);
        list.forget(Path::new("/a"));
        list.forget(Path::new("/not-there"));
        assert_eq!(list.recent(8), vec![PathBuf::from("/new"), "/b".into()]);
        list.pin(Path::new("/b"), false);
        assert!(!list.saved()[1].pinned);
    }

    #[test]
    fn the_list_keeps_within_its_size_and_never_drops_a_pin_first() {
        let many: Vec<Known> = (0..MAX_KNOWN + 50)
            .map(|i| known(&format!("/p{i}")))
            .collect();
        let mut list = KnownProjects::from_saved(Some(many), vec![]);
        assert_eq!(list.saved().len(), MAX_KNOWN);
        let last = list.saved()[MAX_KNOWN - 1].root.clone();
        list.pin(&last, true);
        list.opened(Path::new("/fresh"), "Fresh", 9);
        assert_eq!(list.saved().len(), MAX_KNOWN);
        assert!(
            list.saved().iter().any(|k| k.root == last),
            "the pinned one stays"
        );
    }

    #[test]
    fn scans_add_and_locating_moves_keeping_the_pin() {
        let mut list = KnownProjects::from_saved(Some(vec![known("/a")]), vec![]);
        assert!(list.add(Path::new("/b")));
        assert!(!list.add(Path::new("/a")), "known already");
        assert_eq!(list.saved()[1].last_opened, None);
        list.pin(Path::new("/a"), true);
        list.relocate(Path::new("/a"), Path::new("/moved"));
        assert_eq!(list.saved()[0].root, PathBuf::from("/moved"));
        assert!(list.saved()[0].pinned);
        // Located where another known project is: one entry stays.
        list.relocate(Path::new("/moved"), Path::new("/b"));
        assert_eq!(list.recent(8), vec![PathBuf::from("/b")]);
        list.relocate(Path::new("/nowhere"), Path::new("/x"));
        assert_eq!(list.saved().len(), 1);
    }

    #[test]
    fn rows_read_each_project_and_say_what_went_wrong() {
        let base = std::env::temp_dir().join(format!("solis-known-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let farm = base.join("farm");
        let project = asset_db::Project::create(&farm, "Farm").unwrap();
        project.record_build(57).unwrap();
        let emptied = base.join("emptied");
        std::fs::create_dir_all(&emptied).unwrap();
        let gone = base.join("gone");
        let mut list = KnownProjects::from_saved(
            Some(vec![
                known(farm.to_str().unwrap()),
                known(emptied.to_str().unwrap()),
                known(gone.to_str().unwrap()),
            ]),
            vec![],
        );
        let rows = list.rows();
        assert_eq!(rows[0].name, "Farm");
        assert_eq!(rows[0].saved_by, Some(57));
        assert!(rows[0].problem.is_none());
        assert!(rows[1].problem.as_deref().unwrap().contains("gone from"));
        assert_eq!(rows[2].name, "gone", "the folder's name when none was read");
        assert!(
            rows[2]
                .problem
                .as_deref()
                .unwrap()
                .starts_with("Folder not found")
        );
        // A name read once is kept for when the folder goes.
        std::fs::remove_dir_all(&farm).unwrap();
        let rows = list.rows();
        assert_eq!(rows[0].name, "Farm");
        assert!(rows[0].problem.is_some());
        let _ = std::fs::remove_dir_all(&base);
    }
}
