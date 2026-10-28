//! The project's sprite sheets (Phase 9, D2): every Aseprite file read on
//! a worker and packed into an atlas, again whenever it changes (so saving
//! in Aseprite shows at once), for the Library, Properties and drawing.

use asset_db::{FileKind, Jobs, Listing};
use editor::sprite_sheets::{AnimatorState, FramesState, SheetState, SpriteSheets};
use reflect::{AssetId, Id};
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The most files read at once.
const AT_ONCE: usize = 2;

/// A file's size and time of last change: it's read again when they
/// change.
type Stamp = Option<(u64, Option<std::time::SystemTime>)>;

fn stamp(path: &Path) -> Stamp {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.len(), meta.modified().ok()))
}

/// A job number for `path` (the pool tells jobs apart by asset ID).
fn job_id(path: &Path) -> AssetId {
    let mut hasher = DefaultHasher::new();
    ("sprite sheet", path).hash(&mut hasher);
    AssetId(Id::from_raw(hasher.finish() | 1).unwrap_or_else(Id::random))
}

/// Reads and packs one Aseprite file (on a worker).
pub fn read_sheet(path: &Path) -> SheetState {
    let limits = assets::aseprite::AseLimits::default();
    let bytes = match asset_db::read_limited(path, limits.max_file_bytes as u64) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::FileTooLarge => {
            return SheetState::Failed(format!(
                "it's bigger than {} MB, too big for a sprite sheet",
                limits.max_file_bytes >> 20
            ));
        }
        Err(e) => return SheetState::Failed(format!("it couldn't be read ({e})")),
    };
    let file = match assets::aseprite::read_aseprite_within(&bytes, limits) {
        Ok(file) => file,
        Err(why) => return SheetState::Failed(why.0),
    };
    match assets::sprite_sheet::build_sheet(file) {
        Ok(sheet) => SheetState::Read(Arc::new(sheet)),
        Err(why) => SheetState::Failed(why.0),
    }
}

/// Reads a clips file (`.solframes`).
pub fn read_frames(path: &Path) -> FramesState {
    let bytes = match asset_db::read_limited(path, animation::frames_file::MAX_BYTES as u64) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::FileTooLarge => {
            return FramesState::Failed(format!(
                "it's bigger than {} KB, too big for a clips file",
                animation::frames_file::MAX_BYTES >> 10
            ));
        }
        Err(e) => return FramesState::Failed(format!("it couldn't be read ({e})")),
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return FramesState::Failed("it isn't text".to_owned());
    };
    match animation::frames_file::read(&text) {
        Ok(read) => FramesState::Read(Arc::new(read.file), read.notes),
        Err(why) => FramesState::Failed(why.to_string()),
    }
}

/// Reads an animator file (`.solanim`, Phase 9, D3b).
pub fn read_animator(path: &Path) -> AnimatorState {
    let bytes = match asset_db::read_limited(path, animation::animator_file::MAX_BYTES as u64) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::FileTooLarge => {
            return AnimatorState::Failed(format!(
                "it's bigger than {} KB, too big for an animator",
                animation::animator_file::MAX_BYTES >> 10
            ));
        }
        Err(e) => return AnimatorState::Failed(format!("it couldn't be read ({e})")),
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return AnimatorState::Failed("it isn't text".to_owned());
    };
    match animation::animator_file::read(&text) {
        Ok(read) => AnimatorState::Read(Arc::new(read.file), read.notes),
        Err(why) => AnimatorState::Failed(why.to_string()),
    }
}

/// Every sprite sheet read, and those still to read.
pub struct Sprites {
    sheets: SpriteSheets,
    queued: VecDeque<(String, PathBuf)>,
    /// Files read or failed, with how they were then.
    tried: HashMap<String, Stamp>,
    /// Clips files read or failed, with how they were then (read at once:
    /// they're small text).
    frames_tried: HashMap<String, Stamp>,
    /// Animator files read or failed, with how they were then (read at
    /// once: small text too).
    animators_tried: HashMap<String, Stamp>,
    /// Goes up when another project opens: results for the last one are
    /// thrown away.
    generation: u64,
    jobs: Jobs<(String, u64, SheetState)>,
    /// Whether anything changed since the editor was last told.
    dirty: bool,
}

impl Default for Sprites {
    fn default() -> Self {
        Self {
            sheets: SpriteSheets::default(),
            queued: VecDeque::new(),
            tried: HashMap::new(),
            frames_tried: HashMap::new(),
            animators_tried: HashMap::new(),
            generation: 0,
            jobs: Jobs::new(1),
            dirty: false,
        }
    }
}

impl std::fmt::Debug for Sprites {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sprites")
            .field("sheets", &self.sheets.by_path.len())
            .field("queued", &self.queued.len())
            .finish_non_exhaustive()
    }
}

impl Sprites {
    /// Whether files are still to be read or being read.
    pub fn waiting(&self) -> bool {
        !self.queued.is_empty() || self.jobs.running() > 0
    }

    /// The sheets as they are now.
    pub fn sheets(&self) -> &SpriteSheets {
        &self.sheets
    }

    /// Queues every Aseprite file in `listing` not read yet, or changed
    /// since, and reads every clips file (`.solframes`) not read yet or
    /// changed since; forgets those no longer there.
    pub fn want(&mut self, listing: &Listing, assets: &Path) {
        let mut present = HashSet::new();
        let mut paths = HashMap::new();
        let mut frames_present = HashSet::new();
        for file in listing
            .files
            .iter()
            .filter(|f| f.kind == FileKind::SpriteClips)
        {
            let path = crate::project::slashed(&file.path);
            if let Some(id) = file.id {
                paths.insert(id, path.clone());
            }
            frames_present.insert(path.clone());
            let full = assets.join(&file.path);
            let now = stamp(&full);
            if self.frames_tried.get(&path) == Some(&now) {
                continue;
            }
            self.frames_tried.insert(path.clone(), now);
            self.sheets.frames.insert(path, read_frames(&full));
            self.dirty = true;
        }
        let before = self.sheets.frames.len();
        self.sheets.frames.retain(|p, _| frames_present.contains(p));
        self.frames_tried.retain(|p, _| frames_present.contains(p));
        self.dirty |= before != self.sheets.frames.len();
        let mut animators_present = HashSet::new();
        for file in listing
            .files
            .iter()
            .filter(|f| f.kind == FileKind::Animator)
        {
            let path = crate::project::slashed(&file.path);
            if let Some(id) = file.id {
                paths.insert(id, path.clone());
            }
            animators_present.insert(path.clone());
            let full = assets.join(&file.path);
            let now = stamp(&full);
            if self.animators_tried.get(&path) == Some(&now) {
                continue;
            }
            self.animators_tried.insert(path.clone(), now);
            self.sheets.animators.insert(path, read_animator(&full));
            self.dirty = true;
        }
        let before = self.sheets.animators.len();
        self.sheets
            .animators
            .retain(|p, _| animators_present.contains(p));
        self.animators_tried
            .retain(|p, _| animators_present.contains(p));
        self.dirty |= before != self.sheets.animators.len();
        for file in listing
            .files
            .iter()
            .filter(|f| f.kind == FileKind::SpriteSheet)
        {
            let path = crate::project::slashed(&file.path);
            if let Some(id) = file.id {
                paths.insert(id, path.clone());
            }
            present.insert(path.clone());
            let full = assets.join(&file.path);
            let now = stamp(&full);
            if self.tried.get(&path) == Some(&now) {
                continue;
            }
            self.tried.insert(path.clone(), now);
            // The last good one stays until the new one's read.
            self.sheets
                .by_path
                .entry(path.clone())
                .or_insert(SheetState::Reading);
            self.queued.retain(|(p, _)| *p != path);
            self.queued.push_back((path, full));
        }
        if paths != self.sheets.paths {
            self.sheets.paths = paths;
            self.dirty = true;
        }
        let before = self.sheets.by_path.len();
        self.sheets.by_path.retain(|p, _| present.contains(p));
        self.tried.retain(|p, _| present.contains(p));
        self.dirty |= before != self.sheets.by_path.len();
    }

    /// Forgets every sheet (another project was opened).
    pub fn clear(&mut self) {
        self.sheets = SpriteSheets::default();
        self.queued.clear();
        self.tried.clear();
        self.frames_tried.clear();
        self.animators_tried.clear();
        self.generation += 1;
        self.dirty = true;
    }

    /// Starts the next reads and takes in finished ones. Returns the paths
    /// read since the last call (the GPU uploads them), and whether the
    /// editor should be told what's changed.
    pub fn poll(&mut self) -> (Vec<String>, bool) {
        while self.jobs.running() < AT_ONCE {
            let Some((path, full)) = self.queued.pop_front() else {
                break;
            };
            let generation = self.generation;
            self.jobs.start(job_id(&full), move || {
                let state = std::panic::catch_unwind(|| read_sheet(&full)).unwrap_or_else(|_| {
                    SheetState::Failed("reading it failed unexpectedly".to_owned())
                });
                (path, generation, state)
            });
        }
        let mut read = Vec::new();
        for (_, (path, generation, state)) in self.jobs.finished() {
            if generation != self.generation || !self.tried.contains_key(&path) {
                continue;
            }
            // A failed read keeps the last good sheet, so a half-saved file
            // never makes sprites vanish.
            let keep_last = matches!(state, SheetState::Failed(_))
                && matches!(self.sheets.by_path.get(&path), Some(SheetState::Read(_)));
            if !keep_last {
                self.sheets.by_path.insert(path.clone(), state);
                read.push(path);
            }
            self.dirty = true;
        }
        let dirty = std::mem::take(&mut self.dirty);
        (read, dirty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("solis-sprites-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sprites")).unwrap();
        dir
    }

    fn listing(paths: &[(&str, AssetId)]) -> Listing {
        Listing {
            folders: vec![PathBuf::from("sprites")],
            files: paths
                .iter()
                .map(|(p, id)| asset_db::FileEntry {
                    path: PathBuf::from(p),
                    kind: FileKind::of(Path::new(p)),
                    id: Some(*id),
                })
                .collect(),
        }
    }

    fn settle(s: &mut Sprites) -> Vec<String> {
        let mut read = Vec::new();
        for _ in 0..500 {
            read.extend(s.poll().0);
            if !s.waiting() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        read
    }

    /// A tiny valid Aseprite file: one 2 × 2 frame, all red.
    fn tiny() -> Vec<u8> {
        let mut cel = Vec::new();
        cel.extend_from_slice(&0u16.to_le_bytes());
        cel.extend_from_slice(&0i16.to_le_bytes());
        cel.extend_from_slice(&0i16.to_le_bytes());
        cel.push(255);
        cel.extend_from_slice(&0u16.to_le_bytes());
        cel.extend_from_slice(&0i16.to_le_bytes());
        cel.extend_from_slice(&[0; 5]);
        cel.extend_from_slice(&2u16.to_le_bytes());
        cel.extend_from_slice(&2u16.to_le_bytes());
        cel.extend_from_slice(&[255, 0, 0, 255].repeat(4));
        let mut layer = Vec::new();
        layer.extend_from_slice(&3u16.to_le_bytes());
        layer.extend_from_slice(&[0; 10]);
        layer.push(255);
        layer.extend_from_slice(&[0; 3]);
        layer.extend_from_slice(&4u16.to_le_bytes());
        layer.extend_from_slice(b"body");
        let chunk = |kind: u16, data: &[u8]| {
            let mut c = (data.len() as u32 + 6).to_le_bytes().to_vec();
            c.extend_from_slice(&kind.to_le_bytes());
            c.extend_from_slice(data);
            c
        };
        let mut body = chunk(0x2004, &layer);
        body.extend(chunk(0x2005, &cel));
        let mut frame = (body.len() as u32 + 16).to_le_bytes().to_vec();
        frame.extend_from_slice(&0xF1FAu16.to_le_bytes());
        frame.extend_from_slice(&2u16.to_le_bytes());
        frame.extend_from_slice(&100u16.to_le_bytes());
        frame.extend_from_slice(&[0; 2]);
        frame.extend_from_slice(&2u32.to_le_bytes());
        frame.extend(body);
        let mut out = ((128 + frame.len()) as u32).to_le_bytes().to_vec();
        out.extend_from_slice(&0xA5E0u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.resize(128, 0);
        out.extend(frame);
        out
    }

    #[test]
    fn sheets_are_read_and_read_again_when_they_change() {
        let dir = folder("read");
        let id = AssetId::random();
        let file = dir.join("sprites/walker.aseprite");
        std::fs::write(&file, tiny()).unwrap();
        let mut s = Sprites::default();
        let list = listing(&[("sprites/walker.aseprite", id)]);
        s.want(&list, &dir);
        assert!(matches!(
            s.sheets().by_path.get("sprites/walker.aseprite"),
            Some(SheetState::Reading)
        ));
        assert_eq!(settle(&mut s), ["sprites/walker.aseprite"]);
        let sheet = s.sheets().get(id).expect("read").clone();
        assert_eq!(sheet.size, [2, 2]);
        // Unchanged: not read again.
        s.want(&list, &dir);
        assert!(!s.waiting());
        // Changed (a frame added in Aseprite): read again.
        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut bigger = tiny();
        bigger.extend_from_slice(&[0; 4]);
        std::fs::write(&file, bigger).unwrap();
        s.want(&list, &dir);
        assert!(s.waiting());
        settle(&mut s);
        // Gone: forgotten.
        s.want(&listing(&[]), &dir);
        assert!(s.sheets().by_path.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_damaged_sheet_says_why_and_a_bad_save_keeps_the_last_good_one() {
        let dir = folder("damaged");
        let id = AssetId::random();
        let file = dir.join("sprites/walker.aseprite");
        std::fs::write(&file, b"not an aseprite file at all").unwrap();
        let mut s = Sprites::default();
        let list = listing(&[("sprites/walker.aseprite", id)]);
        s.want(&list, &dir);
        settle(&mut s);
        assert!(matches!(
            s.sheets().by_path.get("sprites/walker.aseprite"),
            Some(SheetState::Failed(why)) if why.contains("short") || why.contains("isn't")
        ));
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&file, tiny()).unwrap();
        s.want(&list, &dir);
        settle(&mut s);
        assert!(s.sheets().get(id).is_some());
        // Saved half-way (Aseprite still writing): the last good one stays.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&file, &tiny()[..60]).unwrap();
        s.want(&list, &dir);
        settle(&mut s);
        assert!(s.sheets().get(id).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
