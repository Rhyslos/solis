//! What the window does for tags (stage 1): saving the project's
//! `tags.lst` (and writing game code's names again), reading it again
//! when it changes on disk, and counting where the project's saved levels
//! use each tag, on a worker, for Project settings › Tags.

use super::App;
use editor::{LogLevel, TagUse};
use level::tags::TagList;
use reflect::AssetId;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};

/// A count of where tags are used, on a worker, as it comes back.
#[derive(Default)]
pub(super) struct Counting {
    answer: Option<Receiver<TagUse>>,
}

/// What tells a `tags.lst` changed: when it was last changed and its
/// length; none when there's no file.
pub(super) type Stamp = Option<(std::time::SystemTime, u64)>;

/// `root`'s `tags.lst`'s stamp, from its metadata alone (nothing read).
pub(super) fn stamp(root: &Path) -> Stamp {
    let meta = std::fs::metadata(root.join(level::tags::FILE_NAME)).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// The most bytes of one level read to count its tags.
const MOST_READ: u64 = level::text::MAX_BYTES as u64;

impl App {
    /// Saves the project's tags, then writes game code's names again and
    /// checks the code with them.
    pub(super) fn save_tags(&mut self, tags: &TagList) {
        let Some(root) = self.projects.root().map(Path::to_path_buf) else {
            return;
        };
        let saved = crate::play_files::save_tags(&root, tags);
        self.tags_seen = stamp(&root);
        if let Some(assets) = self.projects.db().map(|db| db.project().files()) {
            self.game_code.check(assets);
        }
        match saved {
            Ok(Some(crate::play_files::Kept::Older(copy))) => self.workbench.log(
                LogLevel::Info,
                crate::upgrades::kept_note(&root, level::tags::FILE_NAME, &copy),
            ),
            Ok(Some(crate::play_files::Kept::Damaged(backup))) => self.workbench.log(
                LogLevel::Warning,
                format!(
                    "The tags file had a mistake, so it was kept as {backup} before being \
                     written again."
                ),
            ),
            Ok(None) => {}
            Err(why) => self
                .workbench
                .log(LogLevel::Error, format!("Couldn't save the tags: {why}.")),
        }
    }

    /// Reads `tags.lst` again if it changed on disk since it was last read
    /// or written (a pull, an edit by hand): only its metadata is looked
    /// at otherwise.
    pub(super) fn follow_tags_file(&mut self) {
        let Some(root) = self.projects.root().map(Path::to_path_buf) else {
            return;
        };
        let now = stamp(&root);
        if now == self.tags_seen {
            return;
        }
        self.tags_seen = now;
        let (tags, notes) = crate::play_files::load_tags(&root);
        if &tags != self.workbench.tags() {
            self.workbench.set_tags(tags);
            self.workbench.log(
                LogLevel::Info,
                format!(
                    "{} changed on disk: the project's tags are read again.",
                    level::tags::FILE_NAME
                ),
            );
        }
        for note in notes {
            self.workbench.log(LogLevel::Warning, note);
        }
    }

    /// Starts counting where the project's saved levels use each tag.
    pub(super) fn count_tag_use(&mut self) {
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
        let modules = self.workbench.modules().clone();
        let (sender, receiver) = channel();
        let spawned = std::thread::Builder::new()
            .name("count tags".into())
            .spawn(move || {
                let _ = sender.send(count_in(&files, &levels, &modules));
            });
        if spawned.is_ok() {
            self.tag_counts.answer = Some(receiver);
        }
    }

    /// Hands the editor the count that came back.
    pub(super) fn take_tag_use(&mut self) {
        let Some(answer) = &self.tag_counts.answer else {
            return;
        };
        if let Ok(counted) = answer.try_recv() {
            self.tag_counts.answer = None;
            self.workbench.set_tag_use(counted);
        }
    }
}

/// How many entities of each of `levels` (inside `files`) carry each tag,
/// each level read and its modules' copies made from `modules`, as the
/// editor opens it. A level that can't be read counts nothing.
pub(super) fn count_in(
    files: &Path,
    levels: &[PathBuf],
    modules: &BTreeMap<AssetId, Arc<level::text::LevelFile>>,
) -> TagUse {
    let mut counted = TagUse::default();
    let Ok(registry) = level::builtin::registry() else {
        return counted;
    };
    let lookup = |id: AssetId| modules.get(&id).cloned();
    for path in levels {
        let Ok(bytes) = asset_db::read_limited(&files.join(path), MOST_READ) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let Ok(file) = level::text::read_file(&text, &registry) else {
            continue;
        };
        let Ok(level) = level::module::Expander::new(&lookup).expand(&file) else {
            continue;
        };
        let mut here: BTreeMap<reflect::TagId, usize> = BTreeMap::new();
        for (_, entity) in level.iter() {
            if let Some(tags) = entity.get::<level::builtin::Tags>() {
                for id in tags.ids {
                    *here.entry(id).or_default() += 1;
                }
            }
        }
        let inside = path.to_string_lossy().replace('\\', "/");
        for (id, n) in here {
            counted
                .levels
                .entry(id)
                .or_default()
                .push((inside.clone(), n));
        }
    }
    counted
}

#[cfg(test)]
#[path = "window_tags_tests.rs"]
mod tests;
