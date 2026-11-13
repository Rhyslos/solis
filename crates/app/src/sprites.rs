//! The project's sprite sheets (Phase 9, D2): every Aseprite file read on
//! a worker and packed into an atlas, again whenever it changes (so saving
//! in Aseprite shows at once), for the Library, Properties and drawing.
//! Tile maps and the pictures read whole (tilesets', atlases' and the
//! screens') go to the same workers: each can be 64 MB, too much to read
//! and decode on the window's thread (CONTRIBUTING rule 10).

use asset_db::{FileKind, Jobs, Listing};
use editor::sprite_sheets::{
    AnimatorState, ArtState, FramesState, MapFileState, SheetState, SpriteSheets, TilesetState,
};
use reflect::ordinary_file::Refused;
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

/// What a worker reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    /// A sprite sheet ([`read_sheet`]).
    Sheet,
    /// A tile map ([`read_tile_map`]).
    TileMap,
    /// A picture read whole ([`read_art`]).
    Art,
}

/// A worker's read of a file of each [`Kind`].
enum Read {
    Sheet(SheetState),
    TileMap(MapFileState),
    Art(ArtState),
}

/// A file waiting for a worker.
struct Queued {
    kind: Kind,
    /// Inside the assets, as the sheets know it.
    path: String,
    /// On disk.
    full: PathBuf,
    /// How it was when it was asked for: a read of it is taken only while
    /// that's still what's wanted (not changed, saved by the editor or
    /// forgotten since).
    stamp: Stamp,
}

/// What a worker hands back: the file, the project it was read for and
/// what was read.
type Done = (Queued, u64, Read);

/// A job number for `path` read as `kind` (the pool tells jobs apart by
/// asset ID, and drops an older one's result).
fn job_id(kind: Kind, path: &Path) -> AssetId {
    let mut hasher = DefaultHasher::new();
    (kind, path).hash(&mut hasher);
    AssetId(Id::from_raw(hasher.finish() | 1).unwrap_or_else(Id::random))
}

/// Reads and packs one Aseprite file, or a PNG or JPEG as a sheet of one
/// frame (on a worker).
pub fn read_sheet(path: &Path) -> SheetState {
    let limits = assets::aseprite::AseLimits::default();
    if !assets::aseprite::is_aseprite_file(path) {
        return match asset_db::read_limited(path, limits.max_file_bytes as u64) {
            Ok(bytes) => match assets::sprite_sheet::picture_sheet(&bytes) {
                Ok(sheet) => SheetState::Read(Arc::new(sheet)),
                Err(why) => SheetState::Failed(why.0),
            },
            Err(e) if e.kind() == std::io::ErrorKind::FileTooLarge => SheetState::Failed(format!(
                "it's bigger than {} MB, too big for a sprite",
                limits.max_file_bytes >> 20
            )),
            Err(e) => SheetState::Failed(format!("it couldn't be read ({e})")),
        };
    }
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

/// A small text file's text, read within `max` bytes, or why not.
fn text_within(path: &Path, max: usize, what: &str) -> Result<String, String> {
    let bytes = match asset_db::read_limited(path, max as u64) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::FileTooLarge => {
            return Err(format!(
                "it's bigger than {} KB, too big for {what}",
                max >> 10
            ));
        }
        Err(e) => return Err(format!("it couldn't be read ({e})")),
    };
    String::from_utf8(bytes).map_err(|_| "it isn't text".to_owned())
}

/// Reads a tileset (`.tileset.lst`, Phase 9, D4).
pub fn read_tileset(path: &Path) -> TilesetState {
    match text_within(path, tiles::tileset::MAX_BYTES, "a tileset")
        .and_then(|t| tiles::tileset::read(&t).map_err(|e| e.to_string()))
    {
        Ok(read) => TilesetState::Read(Arc::new(read.tileset), read.notes),
        Err(why) => TilesetState::Failed(why),
    }
}

/// Reads an atlas file (`.atlas.lst`, Atlas Crop).
pub fn read_atlas(path: &Path) -> editor::sprite_sheets::AtlasState {
    use editor::sprite_sheets::AtlasState;
    match text_within(path, animation::atlas::MAX_BYTES, "an atlas file")
        .and_then(|t| animation::atlas::read(&t).map_err(|e| e.to_string()))
    {
        Ok(read) => AtlasState::Read(Arc::new(read.atlas), read.notes),
        Err(why) => AtlasState::Failed(why),
    }
}

/// Reads a tile map file (`.tilemap.lst`, Phase 9, D4).
pub fn read_tile_map(path: &Path) -> MapFileState {
    match text_within(path, tiles::map_file::MAX_BYTES, "a tile map")
        .and_then(|t| tiles::map_file::read(&t).map_err(|e| e.to_string()))
    {
        Ok(read) => MapFileState::Read(Arc::new(read.map), read.notes),
        Err(why) => MapFileState::Failed(why),
    }
}

/// Reads a tileset's picture (Phase 9, D5): a PNG, JPEG or Aseprite file,
/// whole.
pub fn read_art(path: &Path) -> ArtState {
    let most = assets::tile_art::MAX_FILE_BYTES as u64;
    let read = reflect::ordinary_file::read_limited(path, most)
        .map_err(|why| match why {
            Refused::TooBig { bytes, limit } => format!(
                "it's {} MB; pictures up to {} MB are read",
                bytes >> 20,
                limit >> 20
            ),
            why => format!("couldn't read it: {why}"),
        })
        .and_then(|bytes| {
            assets::tile_art::read_tile_art(&bytes, assets::aseprite::is_aseprite_file(path))
        });
    match read {
        Ok(art) => ArtState::Read(Arc::new(art)),
        Err(why) => ArtState::Failed(why),
    }
}

/// Reads `full` as `kind` (on a worker). A reader that panics is a
/// failed read, said in plain words.
fn read_on_a_worker(kind: Kind, full: &Path) -> Read {
    let unexpected = || "reading it failed unexpectedly".to_owned();
    match kind {
        Kind::Sheet => Read::Sheet(
            std::panic::catch_unwind(|| read_sheet(full))
                .unwrap_or_else(|_| SheetState::Failed(unexpected())),
        ),
        Kind::TileMap => Read::TileMap(
            std::panic::catch_unwind(|| read_tile_map(full))
                .unwrap_or_else(|_| MapFileState::Failed(unexpected())),
        ),
        Kind::Art => Read::Art(
            std::panic::catch_unwind(|| read_art(full))
                .unwrap_or_else(|_| ArtState::Failed(unexpected())),
        ),
    }
}

/// Every sprite sheet read, and those still to read.
pub struct Sprites {
    sheets: SpriteSheets,
    queued: VecDeque<Queued>,
    /// Files read or failed, with how they were then.
    tried: HashMap<String, Stamp>,
    /// Clips files read or failed, with how they were then (read at once:
    /// they're small text).
    frames_tried: HashMap<String, Stamp>,
    /// Animator files read or failed, with how they were then (read at
    /// once: small text too).
    animators_tried: HashMap<String, Stamp>,
    /// Tilesets and tile maps read or failed, with how they were then.
    tiles_tried: HashMap<String, Stamp>,
    /// Tilesets' pictures read or failed, with how they were then.
    art_tried: HashMap<String, Stamp>,
    /// The pictures the game's screens show (decision 0089), read whole
    /// like tilesets' pictures.
    ui_wanted: HashSet<String>,
    /// Goes up when another project opens: results for the last one are
    /// thrown away.
    generation: u64,
    jobs: Jobs<Done>,
    /// Whether anything changed since the editor was last told.
    dirty: bool,
    /// The assets folder of the last listing, for pictures a sprite asks
    /// for between listings.
    assets: Option<PathBuf>,
}

impl Default for Sprites {
    fn default() -> Self {
        Self {
            sheets: SpriteSheets::default(),
            queued: VecDeque::new(),
            tried: HashMap::new(),
            frames_tried: HashMap::new(),
            animators_tried: HashMap::new(),
            tiles_tried: HashMap::new(),
            art_tried: HashMap::new(),
            ui_wanted: HashSet::new(),
            generation: 0,
            jobs: Jobs::new(1),
            dirty: false,
            assets: None,
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

    /// Queues every Aseprite file and tile map in `listing` not read yet,
    /// or changed since, and reads every clips file, animator, tileset and
    /// atlas file (small text) not read yet or changed since; forgets those
    /// no longer there.
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
        let mut tiles_present = HashSet::new();
        for file in listing.files.iter().filter(|f| {
            matches!(
                f.kind,
                FileKind::Tileset | FileKind::TileMap | FileKind::Atlas
            )
        }) {
            let path = crate::project::slashed(&file.path);
            if let Some(id) = file.id {
                paths.insert(id, path.clone());
            }
            tiles_present.insert(path.clone());
            let full = assets.join(&file.path);
            let now = stamp(&full);
            if self.tiles_tried.get(&path) == Some(&now) {
                continue;
            }
            self.tiles_tried.insert(path.clone(), now);
            if file.kind == FileKind::Tileset {
                self.sheets.tilesets.insert(path, read_tileset(&full));
            } else if file.kind == FileKind::Atlas {
                self.sheets.atlases.insert(path, read_atlas(&full));
            } else {
                // The last one read stays until this one's read.
                self.queue(Kind::TileMap, path, full, now);
                continue;
            }
            self.dirty = true;
        }
        let before =
            self.sheets.tilesets.len() + self.sheets.tile_maps.len() + self.sheets.atlases.len();
        self.sheets.atlases.retain(|p, _| tiles_present.contains(p));
        self.sheets
            .tilesets
            .retain(|p, _| tiles_present.contains(p));
        self.sheets
            .tile_maps
            .retain(|p, _| tiles_present.contains(p));
        self.tiles_tried.retain(|p, _| tiles_present.contains(p));
        self.dirty |= before
            != self.sheets.tilesets.len() + self.sheets.tile_maps.len() + self.sheets.atlases.len();
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
            self.queue(Kind::Sheet, path, full, now);
        }
        // Pictures have IDs too (D5), for tilesets to take art from, and
        // for sprites: one a sprite has asked for is read as a sheet of
        // one frame, again when it changes.
        for file in listing.files.iter().filter(|f| f.kind == FileKind::Texture) {
            let path = crate::project::slashed(&file.path);
            if let Some(id) = file.id {
                paths.insert(id, path.clone());
            }
            if self.tried.contains_key(&path) {
                present.insert(path.clone());
                let full = assets.join(&file.path);
                let now = stamp(&full);
                if self.tried.get(&path) != Some(&now) {
                    self.tried.insert(path.clone(), now);
                    self.queue(Kind::Sheet, path, full, now);
                }
            }
        }
        self.assets = Some(assets.to_path_buf());
        self.want_art(&paths, assets);
        if paths != self.sheets.paths {
            self.sheets.paths = paths;
            self.dirty = true;
        }
        let before = self.sheets.by_path.len();
        self.sheets.by_path.retain(|p, _| present.contains(p));
        self.tried.retain(|p, _| present.contains(p));
        self.dirty |= before != self.sheets.by_path.len();
    }

    /// Queues the pictures in `wanted` (paths inside the assets) for the
    /// game's screens, as well as tilesets' and atlas files' pictures.
    pub fn want_pictures(&mut self, wanted: impl IntoIterator<Item = String>, assets: &Path) {
        self.ui_wanted = wanted.into_iter().collect();
        let paths = self.sheets.paths.clone();
        self.want_art(&paths, assets);
    }

    /// Queues every picture a read tileset takes its art from, an atlas
    /// file crops, or the game's screens show, not read yet or changed
    /// since; forgets those nothing uses.
    fn want_art(&mut self, paths: &HashMap<AssetId, String>, assets: &Path) {
        use editor::sprite_sheets::AtlasState;
        let wanted: HashSet<String> = self
            .sheets
            .tilesets
            .values()
            .filter_map(|t| match t {
                TilesetState::Read(set, _) => set.picture.and_then(|p| paths.get(&p)).cloned(),
                TilesetState::Failed(_) => None,
            })
            .chain(self.sheets.atlases.iter().filter_map(|(path, a)| match a {
                AtlasState::Read(atlas, _) => Some(animation::atlas::picture_path(path, atlas)),
                AtlasState::Failed(_) => None,
            }))
            .chain(self.ui_wanted.iter().cloned())
            .collect();
        for path in &wanted {
            let full = assets.join(path);
            let now = stamp(&full);
            if self.art_tried.get(path) == Some(&now) {
                continue;
            }
            self.art_tried.insert(path.clone(), now);
            // The last one read stays until this one's read.
            self.queue(Kind::Art, path.clone(), full, now);
        }
        let before = self.sheets.tile_art.len();
        self.sheets.tile_art.retain(|p, _| wanted.contains(p));
        self.art_tried.retain(|p, _| wanted.contains(p));
        self.dirty |= before != self.sheets.tile_art.len();
    }

    /// Takes in a file the editor changed and Solis just saved at `path`
    /// (inside the assets), as the file from now on: nothing read later
    /// hands the editor an older copy. The file isn't read again for this
    /// save, only when it next changes on disk.
    pub fn saved(&mut self, path: &str, file: editor::OwnFile) {
        use editor::OwnFile;
        let tried = match &file {
            OwnFile::Frames(_) => &mut self.frames_tried,
            OwnFile::Animator(_) => &mut self.animators_tried,
            OwnFile::Tileset(_) | OwnFile::Atlas(_) | OwnFile::TileMap(_) => &mut self.tiles_tried,
        };
        if let Some(assets) = &self.assets {
            tried.insert(path.to_owned(), stamp(&assets.join(path)));
        }
        // A read of the file before this save is no longer wanted.
        self.queued
            .retain(|q| q.kind != Kind::TileMap || q.path != path);
        self.sheets.put(path, file);
        self.dirty = true;
    }

    /// Forgets every sheet (another project was opened).
    pub fn clear(&mut self) {
        self.sheets = SpriteSheets::default();
        self.queued.clear();
        self.tried.clear();
        self.frames_tried.clear();
        self.animators_tried.clear();
        self.tiles_tried.clear();
        self.art_tried.clear();
        self.ui_wanted.clear();
        self.generation += 1;
        self.dirty = true;
    }

    /// Queues `full` (`path` inside the assets, as it is: `stamp`) to be
    /// read as `kind` on a worker, in place of any read of it queued.
    fn queue(&mut self, kind: Kind, path: String, full: PathBuf, stamp: Stamp) {
        self.queued.retain(|q| q.kind != kind || q.path != path);
        self.queued.push_back(Queued {
            kind,
            path,
            full,
            stamp,
        });
    }

    /// Whether `file`'s read is still what's wanted of that file: it's
    /// still asked for as it was then.
    fn still_wanted(&self, file: &Queued) -> bool {
        let tried = match file.kind {
            Kind::Sheet => &self.tried,
            Kind::TileMap => &self.tiles_tried,
            Kind::Art => &self.art_tried,
        };
        tried.get(&file.path) == Some(&file.stamp)
    }

    /// Starts the next reads and takes in finished ones. Returns the sprite
    /// sheets read since the last call (the GPU uploads them), and whether
    /// the editor should be told what's changed.
    pub fn poll(&mut self) -> (Vec<String>, bool) {
        // Pictures a sprite asked for since (they're read as sheets of one
        // frame from now on).
        for path in self.sheets.pictures_wanted.take() {
            let Some(assets) = self.assets.as_ref() else {
                break;
            };
            if self.tried.contains_key(&path) || !self.sheets.paths.values().any(|p| *p == path) {
                continue;
            }
            let full = assets.join(&path);
            let now = stamp(&full);
            self.tried.insert(path.clone(), now);
            self.sheets
                .by_path
                .insert(path.clone(), SheetState::Reading);
            self.queue(Kind::Sheet, path, full, now);
            self.dirty = true;
        }
        while self.jobs.running() < AT_ONCE {
            let Some(file) = self.queued.pop_front() else {
                break;
            };
            let generation = self.generation;
            self.jobs.start(job_id(file.kind, &file.full), move || {
                let read = read_on_a_worker(file.kind, &file.full);
                (file, generation, read)
            });
        }
        let mut read = Vec::new();
        for (_, (file, generation, state)) in self.jobs.finished() {
            if generation != self.generation || !self.still_wanted(&file) {
                continue;
            }
            let path = file.path;
            match state {
                Read::Sheet(state) => {
                    // A failed read keeps the last good sheet, so a
                    // half-saved file never makes sprites vanish.
                    let keep_last = matches!(state, SheetState::Failed(_))
                        && matches!(self.sheets.by_path.get(&path), Some(SheetState::Read(_)));
                    if !keep_last {
                        self.sheets.by_path.insert(path.clone(), state);
                        read.push(path);
                    }
                }
                Read::TileMap(state) => {
                    self.sheets.tile_maps.insert(path, state);
                }
                Read::Art(state) => {
                    self.sheets.tile_art.insert(path, state);
                }
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

    /// Polls until every read has finished: up to 30 s, then fails
    /// saying so (it gave up after 2.5 s without a word, so a slow
    /// machine failed a later check about sheets: the review, TEST-04).
    fn settle(s: &mut Sprites) -> Vec<String> {
        let mut read = Vec::new();
        for _ in 0..6000 {
            read.extend(s.poll().0);
            if !s.waiting() {
                return read;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("reading never finished");
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
    fn a_picture_is_read_as_a_sheet_only_once_a_sprite_asks_for_it() {
        let dir = folder("picture");
        let id = AssetId::random();
        let file = dir.join("sprites/hill.png");
        let png = assets::tile_art::png_of([4, 2], &[200; 4 * 2 * 4]).unwrap();
        std::fs::write(&file, &png).unwrap();
        let mut s = Sprites::default();
        let list = listing(&[("sprites/hill.png", id)]);
        s.want(&list, &dir);
        settle(&mut s);
        // Nothing asked: a project's pictures aren't all read as sheets.
        assert!(s.sheets().by_path.is_empty());
        // A sprite draws it: asked, read, a sheet of one frame.
        assert!(s.sheets().get(id).is_none());
        assert_eq!(settle(&mut s), ["sprites/hill.png"]);
        let sheet = s.sheets().get(id).expect("read").clone();
        assert_eq!((sheet.size, sheet.frames.len()), ([4, 2], 1));
        // Kept with the next listing; read again when it changes.
        s.want(&list, &dir);
        assert!(!s.waiting());
        assert!(s.sheets().get(id).is_some());
        std::thread::sleep(std::time::Duration::from_millis(20));
        let png = assets::tile_art::png_of([2, 2], &[90; 2 * 2 * 4]).unwrap();
        std::fs::write(&file, &png).unwrap();
        s.want(&list, &dir);
        settle(&mut s);
        assert_eq!(s.sheets().get(id).expect("read").size, [2, 2]);
        // A broken picture says why.
        std::fs::write(&file, b"not a png").unwrap();
        assert!(matches!(read_sheet(&file), SheetState::Failed(_)));
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

    #[test]
    fn a_picture_the_screens_show_is_read_and_forgotten_once_they_don_t() {
        let dir = folder("ui-pictures");
        let png_id = AssetId::random();
        std::fs::write(
            dir.join("sprites/icon.png"),
            assets::tile_art::png_of([4, 2], &[7; 4 * 2 * 4]).unwrap(),
        )
        .unwrap();
        let list = listing(&[("sprites/icon.png", png_id)]);
        let mut s = Sprites::default();
        s.want(&list, &dir);
        assert!(s.sheets().tile_art.is_empty(), "nothing uses it yet");
        s.want_pictures(["sprites/icon.png".to_string()], &dir);
        settle(&mut s);
        assert!(matches!(
            s.sheets().tile_art.get("sprites/icon.png"),
            Some(ArtState::Read(art)) if art.size == [4, 2]
        ));
        // Listing again keeps it while the screens show it.
        s.want(&list, &dir);
        assert!(s.sheets().tile_art.contains_key("sprites/icon.png"));
        s.want_pictures([], &dir);
        assert!(s.sheets().tile_art.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The review, BUG-09: what the editor saved is the file from then on,
    /// and isn't read again until it changes on disk.
    #[test]
    fn a_file_the_editor_saved_is_kept_and_read_again_only_when_it_changes() {
        use editor::OwnFile;
        let dir = folder("saved");
        let id = AssetId::random();
        let file = dir.join("sprites/hero.atlas.lst");
        let atlas = |grid: u32| {
            let mut atlas = animation::atlas::Atlas::new("hero.png");
            atlas.grid = [grid, grid];
            Arc::new(atlas)
        };
        let held = |s: &Sprites| match s.sheets().atlases.get("sprites/hero.atlas.lst") {
            Some(editor::sprite_sheets::AtlasState::Read(atlas, _)) => Some(Arc::clone(atlas)),
            _ => None,
        };
        std::fs::write(&file, animation::atlas::write(&atlas(8))).unwrap();
        let list = listing(&[("sprites/hero.atlas.lst", id)]);
        let mut s = Sprites::default();
        s.want(&list, &dir);
        assert_eq!(s.poll(), (Vec::new(), true), "read: the editor is told");
        assert_eq!(held(&s).unwrap().grid, [8, 8]);
        // Saved by the editor: written, then kept as the file at once.
        let saved = atlas(16);
        std::fs::write(&file, animation::atlas::write(&saved)).unwrap();
        s.saved("sprites/hero.atlas.lst", OwnFile::Atlas(Arc::clone(&saved)));
        assert!(Arc::ptr_eq(&held(&s).unwrap(), &saved));
        assert_eq!(s.poll(), (Vec::new(), true), "the editor is told");
        // The watcher's listing: Solis's own save isn't read again.
        s.want(&list, &dir);
        assert!(Arc::ptr_eq(&held(&s).unwrap(), &saved), "not read again");
        assert_eq!(s.poll(), (Vec::new(), false), "nothing changed");
        // Changed on disk since (a longer file): read.
        std::fs::write(&file, animation::atlas::write(&atlas(128))).unwrap();
        s.want(&list, &dir);
        assert_eq!(held(&s).unwrap().grid, [128, 128]);
        // Gone from the project: forgotten, saved or not.
        s.saved("sprites/hero.atlas.lst", OwnFile::Atlas(saved));
        s.want(&listing(&[]), &dir);
        assert!(held(&s).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A project can hold a folder, or a link to a device or a pipe, named
    /// like any file read here: each is refused at once, saying why.
    #[test]
    fn only_ordinary_files_are_read_as_pictures_sheets_and_data() {
        let dir = folder("ordinary");
        let unread = |why: &str| why.contains("it isn't an ordinary file");
        for odd in crate::odd_files(&dir.join("art"), "tiles.png") {
            assert_eq!(
                read_art(&odd),
                ArtState::Failed("couldn't read it: it isn't an ordinary file".into()),
                "{odd:?}"
            );
            assert!(matches!(read_sheet(&odd), SheetState::Failed(why) if unread(&why)));
        }
        for odd in crate::odd_files(&dir.join("sheets"), "hero.aseprite") {
            assert!(matches!(read_art(&odd), ArtState::Failed(why) if unread(&why)));
            assert!(matches!(read_sheet(&odd), SheetState::Failed(why) if unread(&why)));
        }
        for odd in crate::odd_files(&dir.join("data"), "farm.lst") {
            use editor::sprite_sheets::AtlasState;
            assert!(matches!(read_frames(&odd), FramesState::Failed(why) if unread(&why)));
            assert!(matches!(read_animator(&odd), AnimatorState::Failed(why) if unread(&why)));
            assert!(matches!(read_tileset(&odd), TilesetState::Failed(why) if unread(&why)));
            assert!(matches!(read_atlas(&odd), AtlasState::Failed(why) if unread(&why)));
            assert!(matches!(read_tile_map(&odd), MapFileState::Failed(why) if unread(&why)));
        }
        // A picture past the limit is refused by its size, before any of
        // it is read; one that isn't there says so.
        let big = dir.join("big.png");
        let most = assets::tile_art::MAX_FILE_BYTES as u64;
        let file = std::fs::File::create(&big).unwrap();
        file.set_len(most + (1 << 20)).unwrap(); // sparse: nothing is written
        assert_eq!(
            read_art(&big),
            ArtState::Failed("it's 65 MB; pictures up to 64 MB are read".into())
        );
        assert!(matches!(
            read_art(&dir.join("none.png")),
            ArtState::Failed(why) if why.starts_with("couldn't read it: ")
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_tileset_s_picture_is_read_whole_and_again_when_it_changes() {
        let dir = folder("tile-art");
        let (set_id, png_id) = (AssetId::random(), AssetId::random());
        let png = dir.join("sprites/tiles.png");
        std::fs::write(
            &png,
            assets::tile_art::png_of([16, 8], &[9; 16 * 8 * 4]).unwrap(),
        )
        .unwrap();
        let set = tiles::Tileset {
            picture: Some(png_id),
            ..tiles::Tileset::default()
        };
        std::fs::write(
            dir.join("sprites/farm.soltileset"),
            tiles::tileset::write(&set),
        )
        .unwrap();
        let list = listing(&[
            ("sprites/farm.soltileset", set_id),
            ("sprites/tiles.png", png_id),
        ]);
        let mut s = Sprites::default();
        s.want(&list, &dir);
        settle(&mut s);
        assert_eq!(
            s.sheets().paths.get(&png_id).map(String::as_str),
            Some("sprites/tiles.png")
        );
        assert_eq!(s.sheets().tile_art(png_id).unwrap().size, [16, 8]);
        // Changed: read again; damaged: says why.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&png, b"not a picture any more").unwrap();
        s.want(&list, &dir);
        settle(&mut s);
        assert!(matches!(
            s.sheets().tile_art.get("sprites/tiles.png"),
            Some(ArtState::Failed(why)) if why.contains("PNG")
        ));
        // No tileset uses it: forgotten.
        std::fs::write(
            dir.join("sprites/farm.soltileset"),
            tiles::tileset::write(&tiles::Tileset::default()),
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        s.want(&list, &dir);
        assert!(s.sheets().tile_art.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A tile map's file (`farm.tilemap.lst`) of `size` cells.
    fn map_text(size: u32) -> String {
        tiles::map_file::write(&tiles::TileMap::new([size, size], None, Some(b'.'))).unwrap()
    }

    fn held_map(s: &Sprites, path: &str) -> Option<Arc<tiles::TileMap>> {
        match s.sheets().tile_maps.get(path) {
            Some(MapFileState::Read(map, _)) => Some(Arc::clone(map)),
            _ => None,
        }
    }

    /// The review, BUG-31: tile maps and the pictures read whole (each up
    /// to 64 MB) were read and decoded inside `want` and `want_pictures`,
    /// on the window's thread. They're queued for a worker now, like
    /// sprite sheets: asking returns at once, and the files arrive with a
    /// later `poll`.
    #[test]
    fn tile_maps_and_whole_pictures_are_read_on_a_worker_not_where_they_re_asked_for() {
        let dir = folder("on-a-worker");
        let (map_id, png_id) = (AssetId::random(), AssetId::random());
        std::fs::write(dir.join("sprites/farm.tilemap.lst"), map_text(4)).unwrap();
        std::fs::write(
            dir.join("sprites/icon.png"),
            assets::tile_art::png_of([4, 2], &[7; 4 * 2 * 4]).unwrap(),
        )
        .unwrap();
        let list = listing(&[
            ("sprites/farm.tilemap.lst", map_id),
            ("sprites/icon.png", png_id),
        ]);
        let mut s = Sprites::default();
        s.want(&list, &dir);
        s.want_pictures(["sprites/icon.png".to_string()], &dir);
        assert!(s.sheets().tile_maps.is_empty(), "not read where asked for");
        assert!(s.sheets().tile_art.is_empty(), "not read where asked for");
        assert!(s.waiting(), "queued for a worker");
        settle(&mut s);
        assert_eq!(
            held_map(&s, "sprites/farm.tilemap.lst").unwrap().size(),
            [4, 4]
        );
        assert!(matches!(
            s.sheets().tile_art.get("sprites/icon.png"),
            Some(ArtState::Read(art)) if art.size == [4, 2]
        ));
        // Changed: the last one read stays until the new one is.
        std::fs::write(dir.join("sprites/farm.tilemap.lst"), map_text(8)).unwrap();
        s.want(&list, &dir);
        assert_eq!(
            held_map(&s, "sprites/farm.tilemap.lst").unwrap().size(),
            [4, 4]
        );
        settle(&mut s);
        assert_eq!(
            held_map(&s, "sprites/farm.tilemap.lst").unwrap().size(),
            [8, 8]
        );
        // Gone: forgotten at once, and a read on its way isn't taken.
        std::fs::write(dir.join("sprites/farm.tilemap.lst"), map_text(2)).unwrap();
        s.want(&list, &dir);
        s.want(&listing(&[]), &dir);
        s.want_pictures([], &dir);
        settle(&mut s);
        assert!(s.sheets().tile_maps.is_empty());
        assert!(s.sheets().tile_art.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A read of a tile map asked for before the editor saved it never
    /// replaces what was saved (BUG-09 still holds with reads on a worker).
    #[test]
    fn a_tile_map_read_asked_for_before_a_save_never_replaces_the_save() {
        use editor::OwnFile;
        let dir = folder("read-then-saved");
        let id = AssetId::random();
        let file = dir.join("sprites/farm.tilemap.lst");
        std::fs::write(&file, map_text(4)).unwrap();
        let list = listing(&[("sprites/farm.tilemap.lst", id)]);
        let mut s = Sprites::default();
        s.want(&list, &dir);
        // Queued and started, but not yet taken in, when the save lands.
        s.poll();
        let saved = Arc::new(tiles::TileMap::new([16, 16], None, Some(b'.')));
        std::fs::write(&file, tiles::map_file::write(&saved).unwrap()).unwrap();
        s.saved(
            "sprites/farm.tilemap.lst",
            OwnFile::TileMap(Arc::clone(&saved)),
        );
        settle(&mut s);
        let held = held_map(&s, "sprites/farm.tilemap.lst").unwrap();
        assert!(Arc::ptr_eq(&held, &saved), "the save, not the earlier read");
        // And asking again doesn't read Solis's own save.
        s.want(&list, &dir);
        assert!(!s.waiting());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The review, BUG-31: asking for a big tile map and a big picture
    /// used to read and decode both on the window's thread. The reads
    /// themselves are timed too: that was the frame's wait before.
    #[test]
    #[ignore = "scale check: run by tools/scale/scale-check.sh"]
    fn scale_check_asking_for_a_big_tile_map_and_picture() {
        const TARGET_MS: f64 = 1.0;
        let dir = folder("scale-asking");
        let (map_id, png_id) = (AssetId::random(), AssetId::random());
        let map = dir.join("sprites/world.tilemap.lst");
        std::fs::write(&map, map_text(1024)).unwrap();
        let png = dir.join("sprites/sky.png");
        let pixels: Vec<u8> = (0..4096u32 * 4096 * 4).map(|i| (i % 251) as u8).collect();
        std::fs::write(
            &png,
            assets::tile_art::png_of([4096, 4096], &pixels).unwrap(),
        )
        .unwrap();
        let list = listing(&[
            ("sprites/world.tilemap.lst", map_id),
            ("sprites/sky.png", png_id),
        ]);
        let start = std::time::Instant::now();
        assert!(matches!(read_tile_map(&map), MapFileState::Read(..)));
        assert!(matches!(read_art(&png), ArtState::Read(_)));
        let reads_ms = start.elapsed().as_secs_f64() * 1000.0;
        let mut s = Sprites::default();
        let start = std::time::Instant::now();
        s.want(&list, &dir);
        s.want_pictures(["sprites/sky.png".to_string()], &dir);
        s.poll();
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        settle(&mut s);
        assert!(held_map(&s, "sprites/world.tilemap.lst").is_some());
        let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
        println!(
            "Scale check: asking for a 1,024 x 1,024 tile map and a 4,096 x 4,096 picture {ms:.3} ms \
             (target {TARGET_MS} ms; reading them, now on a worker, {reads_ms:.0} ms) {verdict}"
        );
        let _ = std::fs::remove_dir_all(&dir);
        assert!(ms <= TARGET_MS, "{ms:.3} ms");
    }
}
