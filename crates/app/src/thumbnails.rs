//! The Library's thumbnails: pictures shrunk on a worker thread, and
//! models read on a worker and drawn once on the GPU from a three-quarter
//! view. A few at a time, so they never hold up the editor or the
//! project's real imports; made again when the file changes.

use crate::gpu::GpuViewport;
use asset_db::{FileKind, Jobs, Listing};
use eframe::egui;
use reflect::{AssetId, Id};
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

/// Thumbnails are this many pixels across.
pub const SIDE: u32 = 128;
/// The most thumbnails being made at once.
const AT_ONCE: usize = 2;
/// Pictures bigger than this aren't read for a thumbnail.
const MAX_PICTURE_BYTES: u64 = 64 * 1024 * 1024;

/// What a thumbnail job gave.
enum Made {
    Picture(Result<egui::ColorImage, String>),
    Model(Result<assets::Model, String>),
}

/// One file waiting for its thumbnail.
#[derive(Debug, Clone)]
struct Wanted {
    path: PathBuf,
    full: PathBuf,
    kind: FileKind,
}

/// Every thumbnail made, and those still to make.
pub struct Thumbnails {
    made: HashMap<PathBuf, egui::TextureId>,
    /// Pictures' textures, kept so egui keeps them.
    pictures: HashMap<PathBuf, egui::TextureHandle>,
    queued: VecDeque<Wanted>,
    /// Files started or failed, so they aren't tried every frame.
    tried: HashSet<PathBuf>,
    jobs: Jobs<(PathBuf, Made)>,
}

impl Default for Thumbnails {
    fn default() -> Self {
        Self {
            made: HashMap::new(),
            pictures: HashMap::new(),
            queued: VecDeque::new(),
            tried: HashSet::new(),
            jobs: Jobs::new(1),
        }
    }
}

/// A job number for `path` (the pool tells jobs apart by asset ID).
fn job_id(path: &Path) -> AssetId {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    AssetId(Id::from_raw(hasher.finish() | 1).unwrap_or_else(Id::random))
}

impl Thumbnails {
    /// The thumbnail of the file at `path` (inside the assets), if made.
    pub fn get(&self, path: &Path) -> Option<egui::TextureId> {
        self.made.get(path).copied()
    }

    /// Queues a thumbnail for every model and picture in `listing` that
    /// doesn't have one yet.
    pub fn want(&mut self, listing: &Listing, assets: &Path) {
        for file in &listing.files {
            if !matches!(file.kind, FileKind::Model | FileKind::Texture)
                || self.tried.contains(&file.path)
            {
                continue;
            }
            self.tried.insert(file.path.clone());
            self.queued.push_back(Wanted {
                path: file.path.clone(),
                full: assets.join(&file.path),
                kind: file.kind,
            });
        }
    }

    /// Forgets the thumbnail of the file at `path`, to make it again (it
    /// changed, or moved).
    pub fn forget(&mut self, path: &Path, gpu: Option<&GpuViewport>) {
        self.tried.remove(path);
        self.pictures.remove(path);
        if let Some(id) = self.made.remove(path)
            && let Some(gpu) = gpu
        {
            gpu.free_thumbnail(id);
        }
    }

    /// Forgets every thumbnail (another project was opened).
    pub fn clear(&mut self, gpu: Option<&GpuViewport>) {
        let paths: Vec<PathBuf> = self.made.keys().cloned().collect();
        for path in paths {
            self.forget(&path, gpu);
        }
        self.tried.clear();
        self.queued.clear();
    }

    /// Starts the next jobs and takes in finished ones. Returns whether a
    /// new thumbnail is ready (so the Library should be refreshed).
    pub fn poll(&mut self, ctx: &egui::Context, mut gpu: Option<&mut GpuViewport>) -> bool {
        while self.jobs.running() < AT_ONCE {
            let Some(next) = self.queued.pop_front() else {
                break;
            };
            let path = next.path.clone();
            self.jobs.start(job_id(&next.path), move || {
                let made = std::panic::catch_unwind(|| make(&next))
                    .unwrap_or_else(|_| Made::Picture(Err("making it failed unexpectedly".into())));
                (path, made)
            });
        }
        let mut new = false;
        for (_, (path, made)) in self.jobs.finished() {
            let texture = match made {
                Made::Picture(Ok(image)) => {
                    let handle = ctx.load_texture(
                        format!("thumbnail {}", path.display()),
                        image,
                        egui::TextureOptions::LINEAR,
                    );
                    let id = handle.id();
                    self.pictures.insert(path.clone(), handle);
                    Some(id)
                }
                Made::Model(Ok(model)) => gpu.as_deref_mut().and_then(|g| g.thumbnail(&model)),
                Made::Picture(Err(_)) | Made::Model(Err(_)) => None,
            };
            if let Some(id) = texture {
                self.made.insert(path, id);
                new = true;
            }
        }
        new
    }
}

/// Reads the file for its thumbnail (on a worker thread).
fn make(wanted: &Wanted) -> Made {
    match wanted.kind {
        FileKind::Texture => Made::Picture(picture(&wanted.full)),
        _ => {
            // Small textures are plenty for a thumbnail, and keep memory
            // down; the import cache is left alone.
            let limits = assets::Limits {
                max_texture_side: 256,
                ..assets::Limits::default()
            };
            Made::Model(assets::load(&wanted.full, &limits).map_err(|e| e.to_string()))
        }
    }
}

/// A picture, shrunk to fit a thumbnail.
fn picture(path: &Path) -> Result<egui::ColorImage, String> {
    let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > MAX_PICTURE_BYTES {
        return Err("too big for a thumbnail".into());
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let limits = assets::Limits {
        max_texture_side: SIDE,
        ..assets::Limits::default()
    };
    let texture =
        assets::texture::decode(&bytes, assets::Sampler::default(), true, &limits, u64::MAX)?;
    let pixels = texture.levels.first().ok_or("an empty picture")?;
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [texture.width as usize, texture.height as usize],
        pixels,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_is_shrunk_to_fit() {
        let folder = std::env::temp_dir().join(format!("solis-thumb-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let file = folder.join("big.png");
        image::RgbaImage::from_pixel(512, 256, image::Rgba([200, 30, 30, 255]))
            .save(&file)
            .unwrap();
        let image = picture(&file).unwrap();
        assert_eq!(image.size, [128, 64]);
        assert_eq!(image.pixels[0].r(), 200);
        std::fs::write(folder.join("bad.png"), b"not a picture").unwrap();
        assert!(picture(&folder.join("bad.png")).is_err());
        assert!(picture(&folder.join("missing.png")).is_err());
    }

    #[test]
    fn models_and_pictures_are_wanted_once() {
        let mut t = Thumbnails::default();
        let listing = Listing {
            folders: Vec::new(),
            files: vec![
                asset_db::FileEntry {
                    path: "a.glb".into(),
                    kind: FileKind::Model,
                    id: None,
                },
                asset_db::FileEntry {
                    path: "b.png".into(),
                    kind: FileKind::Texture,
                    id: None,
                },
                asset_db::FileEntry {
                    path: "c.solarea".into(),
                    kind: FileKind::Area,
                    id: None,
                },
            ],
        };
        t.want(&listing, Path::new("/project/assets"));
        t.want(&listing, Path::new("/project/assets"));
        assert_eq!(t.queued.len(), 2);
        t.forget(Path::new("a.glb"), None);
        t.want(&listing, Path::new("/project/assets"));
        assert_eq!(t.queued.len(), 3, "made again after it changed");
        assert_ne!(job_id(Path::new("a")), job_id(Path::new("b")));
    }
}
