//! What the window does for prefabs (decision 0118, board 07): Make
//! prefab…'s file and its Browse…, New prefab, a prefab's page opened and
//! saved, and counting where a prefab is placed, on a worker.

use super::{App, file_name};
use crate::import::FolderFor;
use editor::{LogLevel, MakePrefabPlan, Placed};
use reflect::AssetId;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

/// Counts of copies being made on a worker, as they come back.
#[derive(Default)]
pub(super) struct Counting {
    answers: Option<Receiver<(AssetId, Placed)>>,
}

/// The most bytes of one level read to count copies in it.
const MOST_READ: u64 = level::text::MAX_BYTES as u64;

impl App {
    /// Writes a new prefab from Make prefab…, then (if asked) swaps the
    /// picked entities for a copy of it.
    pub(super) fn make_prefab(&mut self, plan: MakePrefabPlan) {
        let Some((path, id)) = self.write_new_prefab(&plan.folder, &plan.name, &plan.file) else {
            return;
        };
        if let Some(swap) = &plan.swap {
            self.workbench.swap_for_copy(id, swap);
        }
        self.workbench.log(
            LogLevel::Info,
            format!("Made the prefab {}.", file_name(&path)),
        );
    }

    /// Makes a new, empty prefab in `folder` (New prefab) and opens its
    /// page.
    pub(super) fn new_prefab(&mut self, folder: &Path) {
        let file = level::prefab_file::empty("Prefab");
        if let Some((_, id)) = self.write_new_prefab(folder, "Prefab", &file) {
            self.workbench.open_prefab_page(id);
        }
    }

    /// Writes `file` as a new prefab named `name` in `folder` (inside the
    /// project), under a free name; makes it known to the editor and the
    /// Library (with a "New" badge). Its path and ID, or none (the Console
    /// says why).
    fn write_new_prefab(
        &mut self,
        folder: &Path,
        name: &str,
        file: &level::text::LevelFile,
    ) -> Option<(PathBuf, AssetId)> {
        let Some(db) = self.projects.db() else {
            self.workbench.log(
                LogLevel::Warning,
                "Prefabs live in a project: make or open one first.",
            );
            return None;
        };
        if !editor::plain_relative(&folder.to_string_lossy()) && !folder.as_os_str().is_empty() {
            self.workbench.log(
                LogLevel::Warning,
                "A prefab is saved in a folder of the project.",
            );
            return None;
        }
        let files = db.project().files();
        let target = files.join(folder);
        let path = crate::prefab_files::free_path(&target, name);
        let registry = &self.workbench.document().registry;
        let written = crate::prefab_files::text(file, registry)
            .and_then(|text| {
                std::fs::create_dir_all(&target)
                    .and_then(|()| asset_db::write_atomically(&path, text.as_bytes()))
                    .map_err(|e| e.to_string())
            })
            .and_then(|()| self.projects.adopt(&path).map_err(|e| e.to_string()));
        let id = match written {
            Ok(id) => id,
            Err(why) => {
                self.workbench
                    .log(LogLevel::Error, format!("Couldn't save the prefab: {why}."));
                return None;
            }
        };
        let (changes, notes) = self.projects.rescan();
        self.workbench
            .set_prefab(id, std::sync::Arc::new(file.clone()));
        self.follow_changes(changes, notes);
        self.refresh_library();
        if let Ok(inside) = path.strip_prefix(&files) {
            let inside = inside.to_string_lossy().replace('\\', "/");
            self.workbench.prefab_made(&inside);
        }
        Some((path, id))
    }

    /// Browse… in Make prefab…: the folder window, starting in `start`
    /// (inside the project).
    pub(super) fn choose_prefab_folder(&mut self, start: &str) {
        let files = self.projects.db().map(|db| db.project().files());
        let start = files.map(|f| f.join(start));
        self.importer.ask_folder(FolderFor::PrefabFolder(start));
    }

    /// The folder chosen for Make prefab…: inside the project, or refused.
    pub(super) fn prefab_folder_chosen(&mut self, folder: &Path) {
        let Some(files) = self.projects.db().map(|db| db.project().files()) else {
            return;
        };
        let canonical = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
        match canonical(folder).strip_prefix(canonical(&files)) {
            Ok(inside) => {
                let inside = inside.to_string_lossy().replace('\\', "/");
                self.workbench.set_prefab_folder(&inside);
            }
            Err(_) => self.workbench.log(
                LogLevel::Warning,
                "A prefab is saved inside the project: choose one of its folders.",
            ),
        }
    }

    /// Reads prefab `id` and opens its page.
    pub(super) fn open_prefab(&mut self, id: AssetId) {
        match self.load_prefab(id, false) {
            Ok(()) => self.workbench.open_prefab_page(id),
            Err(why) => self
                .workbench
                .log(LogLevel::Error, format!("Couldn't open the prefab: {why}.")),
        }
    }

    /// Saves a prefab's page: its file through the backups, then every
    /// copy in the open level follows it.
    pub(super) fn save_prefab(&mut self, asset: AssetId, file: level::text::LevelFile) {
        let Some(on_disk) = self.projects.db().and_then(|db| db.file(asset)) else {
            self.workbench.log(
                LogLevel::Error,
                "Couldn't save the prefab: its file isn't in the project any more.",
            );
            return;
        };
        let root = self.projects.root().map(Path::to_path_buf);
        let saved = crate::prefab_files::save(
            root.as_deref(),
            &on_disk,
            &file,
            &self.workbench.document().registry,
        );
        match saved {
            Ok(copy) => {
                if let (Some(copy), Some(root)) = (copy, &root) {
                    let note = crate::upgrades::kept_note(root, &file_name(&on_disk), &copy);
                    self.workbench.log(LogLevel::Info, note);
                }
                self.workbench
                    .prefab_saved(asset, std::sync::Arc::new(file), on_disk);
            }
            Err(why) => self.workbench.log(
                LogLevel::Error,
                format!("Couldn't save {}: {why}.", file_name(&on_disk)),
            ),
        }
    }

    /// Counts the copies of prefab `id` in the project's levels, on a
    /// worker: the answer comes back to [`Self::take_counts`].
    pub(super) fn count_placed(&mut self, id: AssetId) {
        let Some(db) = self.projects.db() else {
            return;
        };
        let files = db.project().files();
        let levels: Vec<PathBuf> = self
            .projects
            .listing()
            .files
            .into_iter()
            .filter(|f| f.kind == asset_db::FileKind::Level)
            .map(|f| f.path)
            .collect();
        let (sender, receiver) = channel();
        let spawned = std::thread::Builder::new()
            .name("count copies".into())
            .spawn(move || {
                let placed = count_in(&files, &levels, id);
                let _ = sender.send((id, placed));
            });
        if spawned.is_ok() {
            self.prefab_counts.answers = Some(receiver);
        }
    }

    /// Hands the editor the counts that came back.
    pub(super) fn take_counts(&mut self) {
        let Some(answers) = &self.prefab_counts.answers else {
            return;
        };
        while let Ok((id, placed)) = answers.try_recv() {
            self.workbench.set_placed(id, placed);
        }
    }
}

/// How many copies of `id` each of `levels` (inside `files`) places: the
/// blocks that say `prefab = "ID";` (or `assembly = "ID";`, as a level
/// from before prefabs does).
pub(super) fn count_in(files: &Path, levels: &[PathBuf], id: AssetId) -> Placed {
    let wanted = [
        format!("{} = \"{id}\";", level::text::PREFAB_WORD),
        format!("{} = \"{id}\";", level::text::OLD_PREFAB_WORD),
    ];
    let mut placed = Placed::default();
    for level in levels {
        let Ok(bytes) = asset_db::read_limited(&files.join(level), MOST_READ) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let n = text
            .lines()
            .filter(|line| {
                let line = line.trim_start();
                wanted.iter().any(|w| line.starts_with(w.as_str()))
            })
            .count();
        if n > 0 {
            let inside = level.to_string_lossy().replace('\\', "/");
            placed.levels.push((inside, n));
        }
    }
    placed
}

#[cfg(test)]
#[path = "window_prefabs_tests.rs"]
mod tests;
