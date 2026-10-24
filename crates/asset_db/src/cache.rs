//! The import cache: imported models saved in `.solis/cache/` as `.solbin`
//! files, keyed by a hash of their content and the import settings, so
//! renaming a file or switching branches never imports it again
//! ([decision 0021](../../../docs/decisions/0021-asset-database-and-models.md)).
//!
//! A cache file is read as hostile (CONTRIBUTING.md, rule 9): every count is
//! checked against what's left of the file and against the import limits
//! before anything is allocated, every index is checked to point at
//! something that exists, and anything wrong is refused (the model is then
//! simply imported again).

use crate::hash::ContentHash;
use crate::project::write_atomically;
use assets::{
    AlphaMode, Bounds, Lamp, LampKind, Limits, Material, Mesh, Model, Node, Sampler, Texture,
    Vertex, Wrap,
};
use glam::{Mat4, Vec3};
use std::path::{Path, PathBuf};

/// What a cache file starts with.
const MAGIC: &[u8; 8] = b"SOLBIN\0\0";
/// The layout's version: bump it whenever the layout or the importer's
/// output changes, and old entries are simply ignored.
pub const VERSION: u32 = 3;
/// The cache files' extension.
pub const EXTENSION: &str = "solbin";

/// The key of a model's cache entry: its bytes (and those of the files it
/// reads), the import limits that shape the result, and the layout version.
pub fn key(path: &Path, limits: &Limits) -> std::io::Result<u64> {
    let mut hash = ContentHash::default();
    hash.update_file(path)?;
    if let Ok(files) = assets::referenced_files(path, limits) {
        for file in files {
            // A missing companion file makes the import fail anyway.
            let _ = hash.update_file(&file);
        }
    }
    for n in [
        u64::from(VERSION),
        u64::from(limits.max_texture_side),
        limits.max_mesh_vertices as u64,
        limits.max_vertices as u64,
        limits.max_triangles as u64,
        limits.max_meshes as u64,
        limits.max_file_bytes,
    ] {
        hash.update(&n.to_le_bytes());
    }
    Ok(hash.finish())
}

/// Where the entry for `key` lives in `folder`.
pub fn entry_path(folder: &Path, key: u64) -> PathBuf {
    folder.join(format!("{key:016x}.{EXTENSION}"))
}

/// Reads `path` through the cache in `folder`: a hit is read back, a miss
/// imported and stored. Returns the model and whether it came from the
/// cache. Without a folder (no project open), it's simply imported.
pub fn load(
    path: &Path,
    folder: Option<&Path>,
    limits: &Limits,
) -> Result<(Model, bool), assets::ImportError> {
    let Some(folder) = folder else {
        return assets::load(path, limits).map(|m| (m, false));
    };
    let key = key(path, limits).ok();
    if let Some(key) = key {
        let entry = entry_path(folder, key);
        if let Ok(bytes) = read_entry(&entry, limits)
            && let Ok(mut model) = decode(&bytes, limits)
        {
            // The name follows the file, which may have been renamed.
            model.name = assets_name(path);
            // Used now: the last to go when the cache is trimmed.
            let _ = std::fs::File::options()
                .write(true)
                .open(&entry)
                .and_then(|f| f.set_modified(std::time::SystemTime::now()));
            return Ok((model, true));
        }
    }
    let model = assets::load(path, limits)?;
    if let Some(key) = key {
        // A cache that can't be written only costs time next start.
        let _ = std::fs::create_dir_all(folder)
            .and_then(|()| write_atomically(&entry_path(folder, key), &encode(&model)));
    }
    Ok((model, false))
}

/// The most the cache keeps on disk. Past it, the entries used least
/// lately are deleted (they're only made again if needed).
pub const MAX_BYTES: u64 = 4 << 30;

/// Deletes the cache entries in `folder` used least lately until they
/// take at most `max_bytes` together. Returns how many bytes were freed.
/// Only cache files are touched; anything else in the folder is left.
pub fn trim(folder: &Path, max_bytes: u64) -> u64 {
    let Ok(listing) = std::fs::read_dir(folder) else {
        return 0;
    };
    let mut entries: Vec<(std::time::SystemTime, u64, PathBuf)> = listing
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == EXTENSION))
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            meta.is_file().then(|| {
                let used = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
                (used, meta.len(), e.path())
            })
        })
        .collect();
    let mut total: u64 = entries.iter().map(|(_, size, _)| size).sum();
    // Oldest first.
    entries.sort();
    let mut freed = 0;
    for (_, size, path) in entries {
        if total <= max_bytes {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total -= size;
            freed += size;
        }
    }
    freed
}

fn assets_name(path: &Path) -> String {
    path.file_stem()
        .map_or_else(|| "Model".to_string(), |s| s.to_string_lossy().into_owned())
}

/// Reads an entry, refusing one bigger than any model the limits allow.
fn read_entry(path: &Path, limits: &Limits) -> std::io::Result<Vec<u8>> {
    let size = std::fs::metadata(path)?.len();
    // Decoded textures and mip levels can outgrow the file they came
    // from, so allow a generous multiple.
    if size > limits.max_file_bytes.saturating_mul(4) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "too big",
        ));
    }
    std::fs::read(path)
}

// ---------------------------------------------------------------- write ---

struct Writer(Vec<u8>);

impl Writer {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn len(&mut self, n: usize) {
        self.u32(u32::try_from(n).unwrap_or(u32::MAX));
    }
    fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f32s(&mut self, v: &[f32]) {
        for &x in v {
            self.f32(x);
        }
    }
    fn index(&mut self, v: Option<usize>) {
        self.u32(v.map_or(u32::MAX, |i| u32::try_from(i).unwrap_or(u32::MAX - 1)));
    }
    fn text(&mut self, s: &str) {
        self.len(s.len());
        self.0.extend_from_slice(s.as_bytes());
    }
    fn bytes(&mut self, b: &[u8]) {
        self.len(b.len());
        self.0.extend_from_slice(b);
    }
    /// Corners, then three indices into them per triangle.
    fn corners(&mut self, vertices: &[Vertex], indices: &[u32]) {
        self.len(vertices.len());
        for v in vertices {
            self.f32s(&v.position);
            self.f32s(&v.normal);
            self.f32s(&v.uv);
        }
        self.len(indices.len());
        for &i in indices {
            self.u32(i);
        }
    }
    fn mat4(&mut self, m: &Mat4) {
        self.f32s(&m.to_cols_array());
    }
    fn vec3(&mut self, v: Vec3) {
        self.f32s(&v.to_array());
    }
}

/// A model as a cache entry.
pub fn encode(model: &Model) -> Vec<u8> {
    let mut w = Writer(Vec::new());
    w.0.extend_from_slice(MAGIC);
    w.u32(VERSION);
    w.text(&model.name);
    w.len(model.meshes.len());
    for mesh in &model.meshes {
        w.corners(&mesh.vertices, &mesh.indices);
        w.index(mesh.material);
        w.len(mesh.lods.len());
        for lod in &mesh.lods {
            w.corners(&lod.vertices, &lod.indices);
            w.f32(lod.error);
        }
    }
    w.len(model.nodes.len());
    for node in &model.nodes {
        w.text(&node.name);
        w.index(node.parent);
        w.mat4(&node.local);
        w.mat4(&node.world);
        w.len(node.meshes.len());
        for &m in &node.meshes {
            w.index(Some(m));
        }
    }
    w.len(model.materials.len());
    for m in &model.materials {
        w.text(&m.name);
        w.f32s(&m.base_color);
        w.index(m.base_color_texture);
        w.f32(m.metallic);
        w.f32(m.roughness);
        w.index(m.metallic_roughness_texture);
        w.index(m.normal_texture);
        w.f32(m.normal_scale);
        w.index(m.occlusion_texture);
        w.f32(m.occlusion_strength);
        w.f32s(&m.emissive);
        w.index(m.emissive_texture);
        match m.alpha {
            AlphaMode::Opaque => w.u8(0),
            AlphaMode::Mask(cutoff) => {
                w.u8(1);
                w.f32(cutoff);
            }
            AlphaMode::Blend => w.u8(2),
        }
        w.u8(u8::from(m.double_sided));
    }
    w.len(model.textures.len());
    for t in &model.textures {
        w.u32(t.width);
        w.u32(t.height);
        w.len(t.levels.len());
        for level in &t.levels {
            w.bytes(level);
        }
        for wrap in [t.sampler.wrap_u, t.sampler.wrap_v] {
            w.u8(match wrap {
                Wrap::Repeat => 0,
                Wrap::Mirror => 1,
                Wrap::Clamp => 2,
            });
        }
        w.u8(u8::from(t.sampler.nearest));
        w.u8(u8::from(t.srgb));
    }
    w.len(model.lamps.len());
    for l in &model.lamps {
        w.text(&l.name);
        match l.kind {
            LampKind::Sun => w.u8(0),
            LampKind::Point => w.u8(1),
            LampKind::Spot { inner, outer } => {
                w.u8(2);
                w.f32(inner);
                w.f32(outer);
            }
        }
        w.vec3(l.position);
        w.vec3(l.direction);
        w.f32s(&l.color);
        w.f32(l.strength);
        match l.range {
            Some(r) => {
                w.u8(1);
                w.f32(r);
            }
            None => w.u8(0),
        }
    }
    w.vec3(model.bounds.min);
    w.vec3(model.bounds.max);
    w.len(model.warnings.len());
    for warning in &model.warnings {
        w.text(warning);
    }
    w.0
}

// ----------------------------------------------------------------- read ---

/// Why an entry was refused. The details don't matter: it's imported again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused(&'static str);

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Refused> {
        let end = self.at.checked_add(n).ok_or(Refused("overflow"))?;
        let slice = self.data.get(self.at..end).ok_or(Refused("cut short"))?;
        self.at = end;
        Ok(slice)
    }
    fn u8(&mut self) -> Result<u8, Refused> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, Refused> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn f32(&mut self) -> Result<f32, Refused> {
        Ok(f32::from_bits(self.u32()?))
    }
    fn f32s<const N: usize>(&mut self) -> Result<[f32; N], Refused> {
        let mut out = [0.0; N];
        for v in &mut out {
            *v = self.f32()?;
        }
        Ok(out)
    }
    fn bool(&mut self) -> Result<bool, Refused> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Refused("bad yes or no")),
        }
    }
    /// A count of things at least `min_size` bytes each, checked against
    /// what's left and against `limit`.
    fn count(&mut self, min_size: usize, limit: usize) -> Result<usize, Refused> {
        let n = self.u32()? as usize;
        let left = self.data.len() - self.at;
        if n > limit || n.saturating_mul(min_size.max(1)) > left {
            return Err(Refused("count too big"));
        }
        Ok(n)
    }
    fn index(&mut self) -> Result<Option<usize>, Refused> {
        let v = self.u32()?;
        Ok((v != u32::MAX).then_some(v as usize))
    }
    fn text(&mut self) -> Result<String, Refused> {
        let n = self.count(1, 1 << 20)?;
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| Refused("not text"))
    }
    fn mat4(&mut self) -> Result<Mat4, Refused> {
        Ok(Mat4::from_cols_array(&self.f32s::<16>()?))
    }
    fn vec3(&mut self) -> Result<Vec3, Refused> {
        Ok(Vec3::from_array(self.f32s::<3>()?))
    }
}

/// Reads a cache entry back, within `limits`. Anything wrong is refused.
pub fn decode(data: &[u8], limits: &Limits) -> Result<Model, Refused> {
    let mut r = Reader { data, at: 0 };
    if r.take(MAGIC.len())? != MAGIC {
        return Err(Refused("not a cache entry"));
    }
    if r.u32()? != VERSION {
        return Err(Refused("an older layout"));
    }
    let name = r.text()?;
    let mut total_vertices = 0usize;
    let mesh_count = r.count(12, limits.max_meshes)?;
    let mut meshes = Vec::with_capacity(mesh_count);
    // Corners (counted against the limit), then indices into them, or
    // into `shared` when there are none of their own.
    let mut corners =
        |r: &mut Reader<'_>, shared: Option<usize>| -> Result<(Vec<Vertex>, Vec<u32>), Refused> {
            let n = r.count(32, limits.max_vertices)?;
            total_vertices = total_vertices.saturating_add(n);
            if total_vertices > limits.max_vertices {
                return Err(Refused("too many corners"));
            }
            let mut vertices = Vec::with_capacity(n);
            for _ in 0..n {
                vertices.push(Vertex {
                    position: r.f32s()?,
                    normal: r.f32s()?,
                    uv: r.f32s()?,
                });
            }
            let reach = match shared {
                Some(shared) if n == 0 => shared,
                _ => n,
            };
            let k = r.count(4, limits.max_triangles.saturating_mul(3))?;
            let mut indices = Vec::with_capacity(k);
            for _ in 0..k {
                let i = r.u32()?;
                if i as usize >= reach {
                    return Err(Refused("a corner that doesn't exist"));
                }
                indices.push(i);
            }
            if k % 3 != 0 {
                return Err(Refused("half a triangle"));
            }
            Ok((vertices, indices))
        };
    for _ in 0..mesh_count {
        let (vertices, indices) = corners(&mut r, None)?;
        let material = r.index()?;
        let lod_count = r.count(12, assets::lods::MAX_LODS)?;
        let mut lods = Vec::with_capacity(lod_count);
        for _ in 0..lod_count {
            let (own, lod_indices) = corners(&mut r, Some(vertices.len()))?;
            let error = r.f32()?;
            if !error.is_finite() || error < 0.0 {
                return Err(Refused("a level of detail's error isn't a number"));
            }
            lods.push(assets::Lod {
                vertices: own,
                indices: lod_indices,
                error,
            });
        }
        meshes.push(Mesh {
            vertices,
            indices,
            material,
            lods,
        });
    }
    let node_count = r.count(140, limits.max_meshes.max(1 << 20))?;
    let mut nodes = Vec::with_capacity(node_count);
    for index in 0..node_count {
        let name = r.text()?;
        let parent = r.index()?;
        if parent.is_some_and(|p| p >= index) {
            return Err(Refused("a parent after its child"));
        }
        let local = r.mat4()?;
        let world = r.mat4()?;
        let n = r.count(4, mesh_count)?;
        let mut node_meshes = Vec::with_capacity(n);
        for _ in 0..n {
            match r.index()? {
                Some(m) if m < mesh_count => node_meshes.push(m),
                _ => return Err(Refused("a mesh that doesn't exist")),
            }
        }
        nodes.push(Node {
            name,
            parent,
            local,
            world,
            meshes: node_meshes,
        });
    }
    let material_count = r.count(60, 1 << 16)?;
    let mut materials = Vec::with_capacity(material_count);
    for _ in 0..material_count {
        materials.push(Material {
            name: r.text()?,
            base_color: r.f32s()?,
            base_color_texture: r.index()?,
            metallic: r.f32()?,
            roughness: r.f32()?,
            metallic_roughness_texture: r.index()?,
            normal_texture: r.index()?,
            normal_scale: r.f32()?,
            occlusion_texture: r.index()?,
            occlusion_strength: r.f32()?,
            emissive: r.f32s()?,
            emissive_texture: r.index()?,
            alpha: match r.u8()? {
                0 => AlphaMode::Opaque,
                1 => AlphaMode::Mask(r.f32()?),
                2 => AlphaMode::Blend,
                _ => return Err(Refused("an unknown see-through mode")),
            },
            double_sided: r.bool()?,
        });
    }
    let texture_count = r.count(16, 1 << 16)?;
    let mut textures = Vec::with_capacity(texture_count);
    let mut texture_bytes = 0u64;
    for _ in 0..texture_count {
        let width = r.u32()?;
        let height = r.u32()?;
        if width == 0 || height == 0 || width.max(height) > limits.max_texture_side {
            return Err(Refused("a texture of an impossible size"));
        }
        let level_count = r.count(4, 32)?;
        if level_count != assets::texture::level_count(width, height) as usize {
            return Err(Refused("the wrong number of mip levels"));
        }
        let mut levels = Vec::with_capacity(level_count);
        for level in 0..level_count {
            let (w, h) = ((width >> level).max(1), (height >> level).max(1));
            let expected = (w as usize) * (h as usize) * 4;
            let n = r.count(1, expected)?;
            if n != expected {
                return Err(Refused("a mip level of the wrong size"));
            }
            texture_bytes += n as u64;
            if texture_bytes > limits.max_file_bytes.saturating_mul(4) {
                return Err(Refused("too much texture"));
            }
            levels.push(r.take(n)?.to_vec());
        }
        let wrap = |v| match v {
            0 => Ok(Wrap::Repeat),
            1 => Ok(Wrap::Mirror),
            2 => Ok(Wrap::Clamp),
            _ => Err(Refused("an unknown wrap")),
        };
        let sampler = Sampler {
            wrap_u: wrap(r.u8()?)?,
            wrap_v: wrap(r.u8()?)?,
            nearest: r.bool()?,
        };
        textures.push(Texture {
            width,
            height,
            levels,
            sampler,
            srgb: r.bool()?,
        });
    }
    let lamp_count = r.count(45, assets::lights::MAX_LAMPS)?;
    let mut lamps = Vec::with_capacity(lamp_count);
    for _ in 0..lamp_count {
        let name = r.text()?;
        let kind = match r.u8()? {
            0 => LampKind::Sun,
            1 => LampKind::Point,
            2 => LampKind::Spot {
                inner: r.f32()?,
                outer: r.f32()?,
            },
            _ => return Err(Refused("an unknown light")),
        };
        lamps.push(Lamp {
            name,
            kind,
            position: r.vec3()?,
            direction: r.vec3()?,
            color: r.f32s()?,
            strength: r.f32()?,
            range: if r.bool()? { Some(r.f32()?) } else { None },
        });
    }
    let bounds = Bounds {
        min: r.vec3()?,
        max: r.vec3()?,
    };
    let warning_count = r.count(4, 1 << 12)?;
    let mut warnings = Vec::with_capacity(warning_count);
    for _ in 0..warning_count {
        warnings.push(r.text()?);
    }
    if r.at != data.len() {
        return Err(Refused("something after the end"));
    }
    // Every index into the model's lists must point somewhere.
    let texture_ok = |t: Option<usize>| t.is_none_or(|t| t < textures.len());
    let materials_ok = materials.iter().all(|m| {
        [
            m.base_color_texture,
            m.metallic_roughness_texture,
            m.normal_texture,
            m.occlusion_texture,
            m.emissive_texture,
        ]
        .into_iter()
        .all(texture_ok)
    });
    let meshes_ok = meshes
        .iter()
        .all(|m| m.material.is_none_or(|i| i < materials.len()));
    if !materials_ok || !meshes_ok {
        return Err(Refused("an index that points nowhere"));
    }
    Ok(Model {
        name,
        meshes,
        nodes,
        materials,
        textures,
        lamps,
        bounds,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_folder;

    fn showcase() -> Model {
        assets::load_bytes(
            &assets::sample::showcase_glb(),
            None,
            "Showcase",
            &Limits::default(),
        )
        .unwrap()
    }

    /// Gives the first mesh of `model` a level of detail sharing its
    /// corners, and one with corners of its own.
    fn with_lods(model: &mut Model) {
        let mesh = &mut model.meshes[0];
        let shared = assets::Lod {
            vertices: Vec::new(),
            indices: mesh.indices[..mesh.indices.len() / 6 * 3].to_vec(),
            error: 0.01,
        };
        let own = assets::Lod {
            vertices: mesh.vertices[..3].to_vec(),
            indices: vec![0, 1, 2],
            error: 0.05,
        };
        mesh.lods = vec![shared, own];
    }

    #[test]
    fn a_model_round_trips_through_the_cache_layout() {
        let mut model = showcase();
        let bytes = encode(&model);
        assert_eq!(decode(&bytes, &Limits::default()).unwrap(), model);
        with_lods(&mut model);
        let bytes = encode(&model);
        assert_eq!(decode(&bytes, &Limits::default()).unwrap(), model);
    }

    #[test]
    fn levels_of_detail_are_made_once_and_kept_in_the_cache() {
        let folder = test_folder("cache-lods");
        std::fs::create_dir_all(&folder).unwrap();
        let file = folder.join("scene.glb");
        std::fs::write(&file, assets::sample::showcase_glb()).unwrap();
        let cache = folder.join("cache");
        let (made, hit) = load(&file, Some(&cache), &Limits::default()).unwrap();
        assert!(!hit);
        assert!(
            made.meshes.iter().any(|m| !m.lods.is_empty()),
            "the scene's detailed meshes got levels"
        );
        let (kept, hit) = load(&file, Some(&cache), &Limits::default()).unwrap();
        assert!(hit, "read back, not made again");
        assert_eq!(kept.meshes, made.meshes);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_level_pointing_past_its_corners_is_refused() {
        let mut model = showcase();
        with_lods(&mut model);
        model.meshes[0].lods[0].indices[0] = model.meshes[0].vertices.len() as u32;
        assert!(decode(&encode(&model), &Limits::default()).is_err());
        let mut model = showcase();
        with_lods(&mut model);
        model.meshes[0].lods[1].indices[0] = 3;
        assert!(decode(&encode(&model), &Limits::default()).is_err());
        let mut model = showcase();
        with_lods(&mut model);
        model.meshes[0].lods[1].error = f32::NAN;
        assert!(decode(&encode(&model), &Limits::default()).is_err());
    }

    #[test]
    fn trimming_deletes_the_entries_used_least_lately() {
        let folder = test_folder("cache-trim");
        std::fs::create_dir_all(&folder).unwrap();
        let day = std::time::Duration::from_secs(86_400);
        let now = std::time::SystemTime::now();
        // Three entries of 100 bytes, used 3, 2 and 1 days ago.
        for (i, age) in [3u32, 2, 1].into_iter().enumerate() {
            let path = entry_path(&folder, i as u64);
            std::fs::write(&path, [0u8; 100]).unwrap();
            std::fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(now - day * age)
                .unwrap();
        }
        std::fs::write(folder.join("notes.txt"), [0u8; 500]).unwrap();
        assert_eq!(trim(&folder, 1_000), 0, "within its size");
        assert_eq!(trim(&folder, 150), 200, "the two oldest go");
        assert!(entry_path(&folder, 2).exists());
        assert!(!entry_path(&folder, 0).exists() && !entry_path(&folder, 1).exists());
        assert!(folder.join("notes.txt").exists(), "only cache files");
        assert_eq!(trim(&folder.join("missing"), 0), 0);
    }

    #[test]
    fn a_renamed_file_is_a_cache_hit() {
        let root = test_folder("cache-rename");
        let cache = root.join("cache");
        std::fs::create_dir_all(&root).unwrap();
        let first = root.join("crate.glb");
        std::fs::write(&first, assets::sample::sample_glb()).unwrap();
        let limits = Limits::default();
        let (model, hit) = load(&first, Some(&cache), &limits).unwrap();
        assert!(!hit, "imported the first time");
        let renamed = root.join("box.glb");
        std::fs::rename(&first, &renamed).unwrap();
        let (again, hit) = load(&renamed, Some(&cache), &limits).unwrap();
        assert!(hit, "read back after the rename");
        assert_eq!(again.name, "box", "named after the file now");
        assert_eq!(again.meshes, model.meshes);
        // Other limits give another entry.
        let small = Limits {
            max_texture_side: 16,
            ..limits
        };
        assert!(!load(&renamed, Some(&cache), &small).unwrap().1);
        // A damaged entry is imported again, not trusted.
        let entry = entry_path(&cache, key(&renamed, &limits).unwrap());
        std::fs::write(&entry, b"SOLBIN\0\0rubbish").unwrap();
        let (fixed, hit) = load(&renamed, Some(&cache), &limits).unwrap();
        assert!(!hit);
        assert_eq!(fixed.meshes, model.meshes);
    }

    #[test]
    fn damaged_entries_never_crash_the_reader() {
        // Fuzzing: every prefix, and thousands of single-byte corruptions,
        // of a real entry. Each must be refused or read safely.
        let limits = Limits::default();
        // The small sample: the same layout, quick enough to try often,
        // with a level of detail of each kind added.
        let mut sample =
            assets::load_bytes(&assets::sample::sample_glb(), None, "S", &limits).unwrap();
        with_lods(&mut sample);
        let bytes = encode(&sample);
        for end in (0..bytes.len()).step_by(7) {
            assert!(decode(&bytes[..end], &limits).is_err());
        }
        let mut seed = 0x1234_5678u64;
        for _ in 0..3000 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let mut broken = bytes.clone();
            let at = (seed as usize) % broken.len();
            broken[at] ^= (seed >> 40) as u8 | 1;
            if let Ok(model) = decode(&broken, &limits) {
                // Read safely: everything still points somewhere.
                for mesh in &model.meshes {
                    assert!(
                        mesh.indices
                            .iter()
                            .all(|&i| (i as usize) < mesh.vertices.len())
                    );
                    assert!(mesh.lods.len() <= assets::lods::MAX_LODS);
                    for lod in &mesh.lods {
                        let reach = if lod.vertices.is_empty() {
                            mesh.vertices.len()
                        } else {
                            lod.vertices.len()
                        };
                        assert!(lod.indices.iter().all(|&i| (i as usize) < reach));
                        assert!(lod.error.is_finite() && lod.error >= 0.0);
                    }
                }
                assert!(model.placements().all(|(m, _)| m < model.meshes.len()));
            }
        }
        // Huge counts are refused before anything is allocated.
        let mut huge = MAGIC.to_vec();
        huge.extend_from_slice(&VERSION.to_le_bytes());
        huge.extend_from_slice(&0u32.to_le_bytes());
        huge.extend_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode(&huge, &limits).is_err());
    }
}
