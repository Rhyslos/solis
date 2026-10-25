//! Reading glTF 2.0 files (`.gltf` with its `.bin` and images, or a single
//! `.glb`) into a [`Model`].
//!
//! The file is treated as hostile (CONTRIBUTING.md, rule 9): its size is
//! checked before it is read, every buffer, view and accessor is checked to
//! fit before any of it is read, and the model's size is checked against
//! [`Limits`] before vertices are allocated. The `gltf` crate parses the
//! file's structure; Solis reads the buffers and images itself, so it can
//! apply those checks.

use crate::geometry;
use crate::model::{
    AlphaMode, Bounds, ImportError, Limits, Material, Model, Sampler, Texture, Wrap,
};
use crate::texture;
use base64::Engine as _;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The file name extensions read as glTF.
pub const EXTENSIONS: [&str; 2] = ["gltf", "glb"];

/// Reads the glTF model at `path`, and any files it refers to beside it.
pub fn load(path: &Path, limits: &Limits) -> Result<Model, ImportError> {
    let bytes = read_limited(path, limits.max_file_bytes)?;
    let name = path
        .file_stem()
        .map_or_else(|| "Model".to_string(), |s| s.to_string_lossy().into_owned());
    load_bytes(&bytes, path.parent(), &name, limits)
}

/// Reads a glTF model from `bytes`. `folder` is where files it refers to
/// are looked for; with `None`, a model that refers to other files is
/// refused.
pub fn load_bytes(
    bytes: &[u8],
    folder: Option<&Path>,
    name: &str,
    limits: &Limits,
) -> Result<Model, ImportError> {
    if bytes.len() as u64 > limits.max_file_bytes {
        return Err(ImportError::TooBig {
            bytes: bytes.len() as u64,
            limit: limits.max_file_bytes,
        });
    }
    // The gltf crate checks the file's structure; any crash inside it on a
    // damaged file is caught and reported, as a last line of defence.
    let parsed = std::panic::catch_unwind(|| gltf::Gltf::from_slice(bytes))
        .map_err(|_| ImportError::Damaged("its structure couldn't be read".into()))?
        .map_err(|e| ImportError::NotGltf(describe_gltf_error(&e)))?;
    let reading = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        read_model(&parsed, folder, name, limits)
    }));
    reading.unwrap_or_else(|_| Err(ImportError::Damaged("part of it couldn't be read".into())))
}

/// The other files a model at `path` reads (a `.gltf`'s `.bin` and
/// images, beside it), so a project can copy them with it and notice when
/// they change. Files it would refuse to read aren't listed.
pub fn referenced_files(path: &Path, limits: &Limits) -> Result<Vec<PathBuf>, ImportError> {
    let bytes = read_limited(path, limits.max_file_bytes)?;
    let parsed = std::panic::catch_unwind(|| gltf::Gltf::from_slice(&bytes))
        .map_err(|_| ImportError::Damaged("its structure couldn't be read".into()))?
        .map_err(|e| ImportError::NotGltf(describe_gltf_error(&e)))?;
    let folder = path.parent();
    let buffers = parsed.document.buffers().filter_map(|b| match b.source() {
        gltf::buffer::Source::Uri(uri) => Some(uri),
        gltf::buffer::Source::Bin => None,
    });
    let images = parsed.document.images().filter_map(|i| match i.source() {
        gltf::image::Source::Uri { uri, .. } => Some(uri),
        gltf::image::Source::View { .. } => None,
    });
    let mut files = Vec::new();
    for uri in buffers.chain(images) {
        if let Address::File(file) = resolve(uri, folder)?
            && !files.contains(&file)
        {
            files.push(file);
        }
    }
    Ok(files)
}

fn describe_gltf_error(error: &gltf::Error) -> String {
    match error {
        gltf::Error::Io(e) => format!("reading failed ({e})"),
        gltf::Error::Deserialize(e) => format!("its text isn't valid glTF ({e})"),
        gltf::Error::Validation(errors) => {
            // A file that needs an extension Solis doesn't read (Draco or
            // meshopt compression, say) fails validation too: say which.
            let needed: Vec<String> = errors
                .iter()
                .filter(|(_, e)| *e == gltf::json::validation::Error::Unsupported)
                .map(|(path, _)| {
                    // The path reads `extensionsRequired[0] = "NAME"`.
                    path.0
                        .split('"')
                        .nth(1)
                        .unwrap_or(path.0.as_str())
                        .to_string()
                })
                .collect();
            if needed.is_empty() {
                "it refers to parts that don't exist".into()
            } else {
                format!(
                    "it needs a glTF extension Solis doesn't read yet ({}); export it without compression",
                    needed.join(", ")
                )
            }
        }
        gltf::Error::Binary(_) => "its binary layout is broken".into(),
    }
}

fn limits_mb(bytes: u64) -> u64 {
    bytes / 1_000_000
}

fn read_model(
    parsed: &gltf::Gltf,
    folder: Option<&Path>,
    name: &str,
    limits: &Limits,
) -> Result<Model, ImportError> {
    let document = &parsed.document;
    let mut warnings = Vec::new();
    let buffers = read_buffers(document, parsed.blob.as_deref(), folder, limits)?;
    check_accessors(document, &buffers, limits)?;

    let (materials, textures) = read_materials(document, &buffers, folder, limits, &mut warnings);
    let (meshes, nodes) = geometry::read_meshes(document, &buffers, limits, &mut warnings)?;
    let lamps = crate::lights::read_lamps(&geometry::placed_nodes(document)?, &mut warnings)?;
    let bounds = Bounds::around(nodes.iter().flat_map(|node| {
        node.meshes.iter().flat_map(|&m| {
            meshes[m]
                .vertices
                .iter()
                .map(|v| node.world.transform_point3(glam::Vec3::from(v.position)))
        })
    }))
    .or_else(|| Bounds::around(lamps.iter().map(|l| l.position)))
    .ok_or(ImportError::Empty)?;
    let mut model = Model {
        name: name.to_string(),
        meshes,
        nodes,
        materials,
        textures,
        lamps,
        bounds,
        warnings,
    };
    crate::lods::add_lods(&mut model, limits);
    Ok(model)
}

/// Reads a file, refusing it first if it is bigger than `limit` bytes.
fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>, ImportError> {
    let size = std::fs::metadata(path)
        .map_err(|e| ImportError::Unreadable(format!("{} ({e})", path.display())))?
        .len();
    if size > limit {
        return Err(ImportError::TooBig { bytes: size, limit });
    }
    std::fs::read(path).map_err(|e| ImportError::Unreadable(format!("{} ({e})", path.display())))
}

/// Reads at most `length` bytes from the start of a file: a buffer only
/// ever takes what it declares, however big the file it points at is.
fn read_prefix(path: &Path, length: u64) -> Result<Vec<u8>, ImportError> {
    use std::io::Read;
    let unreadable =
        |e: std::io::Error| ImportError::Unreadable(format!("{} ({e})", path.display()));
    let file = std::fs::File::open(path).map_err(unreadable)?;
    let mut data = Vec::new();
    file.take(length)
        .read_to_end(&mut data)
        .map_err(unreadable)?;
    Ok(data)
}

/// Where a `uri` in the file points.
enum Address<'a> {
    /// Data inside the address itself.
    Data(Vec<u8>),
    /// A file beside the model.
    File(PathBuf),
    /// Something Solis won't read (another computer, an absolute path).
    Refused(&'a str),
}

fn resolve<'a>(uri: &'a str, folder: Option<&Path>) -> Result<Address<'a>, ImportError> {
    if let Some(rest) = uri.strip_prefix("data:") {
        let Some((header, payload)) = rest.split_once(',') else {
            return Err(ImportError::Damaged("a data address has no data".into()));
        };
        let data = if header.ends_with(";base64") {
            base64::engine::general_purpose::STANDARD
                .decode(payload.trim())
                .map_err(|_| ImportError::Damaged("a data address isn't valid base64".into()))?
        } else {
            percent_encoding::percent_decode_str(payload).collect()
        };
        return Ok(Address::Data(data));
    }
    if uri.contains("://") || uri.starts_with('/') || uri.starts_with('\\') || uri.contains(':') {
        return Ok(Address::Refused(uri));
    }
    let Some(folder) = folder else {
        return Ok(Address::Refused(uri));
    };
    let decoded = percent_encoding::percent_decode_str(uri).decode_utf8_lossy();
    // Only files in the model's folder or below it: never `..` out of it.
    let relative = Path::new(decoded.as_ref());
    if relative.components().any(|c| {
        !matches!(
            c,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        )
    }) {
        return Ok(Address::Refused(uri));
    }
    Ok(Address::File(folder.join(relative)))
}

fn read_buffers(
    document: &gltf::Document,
    blob: Option<&[u8]>,
    folder: Option<&Path>,
    limits: &Limits,
) -> Result<Vec<Vec<u8>>, ImportError> {
    let mut total: u64 = 0;
    document
        .buffers()
        .map(|buffer| {
            let declared = buffer.length() as u64;
            total = total.saturating_add(declared);
            if declared > limits.max_file_bytes || total > limits.max_file_bytes {
                return Err(ImportError::TooBig {
                    bytes: total.max(declared),
                    limit: limits.max_file_bytes,
                });
            }
            let data = match buffer.source() {
                gltf::buffer::Source::Bin => blob
                    .ok_or_else(|| ImportError::Damaged("its binary part is missing".into()))?
                    .to_vec(),
                gltf::buffer::Source::Uri(uri) => match resolve(uri, folder)? {
                    Address::Data(data) => data,
                    Address::File(path) => read_prefix(&path, declared).map_err(|e| match e {
                        ImportError::Unreadable(_) => ImportError::Damaged(format!(
                            "it needs the file {}, which isn't there",
                            path.file_name().map_or_else(
                                || uri.to_string(),
                                |n| n.to_string_lossy().into_owned()
                            )
                        )),
                        other => other,
                    })?,
                    Address::Refused(uri) => {
                        return Err(ImportError::Damaged(format!(
                            "it needs data from {uri}, which Solis doesn't fetch"
                        )));
                    }
                },
            };
            if (data.len() as u64) < declared {
                return Err(ImportError::Damaged(format!(
                    "a buffer holds {} bytes but should hold {declared}",
                    data.len()
                )));
            }
            let mut data = data;
            data.truncate(declared as usize);
            Ok(data)
        })
        .collect()
}

/// Checks that every accessor's data lies inside its buffer, so nothing
/// reads past the end of one. Done once, before any vertex is read.
fn check_accessors(
    document: &gltf::Document,
    buffers: &[Vec<u8>],
    limits: &Limits,
) -> Result<(), ImportError> {
    let damaged = || ImportError::Damaged("a list of numbers runs past the end of its data".into());
    let fits = |view: gltf::buffer::View<'_>, offset: usize, count: usize, size: usize| {
        let buffer = buffers.get(view.buffer().index())?;
        let view_end = view.offset().checked_add(view.length())?;
        if view_end > buffer.len() {
            return None;
        }
        if count == 0 {
            return Some(());
        }
        let stride = view.stride().unwrap_or(size);
        if stride < size {
            return None;
        }
        let end = stride
            .checked_mul(count - 1)?
            .checked_add(offset)?
            .checked_add(size)?;
        (end <= view.length()).then_some(())
    };
    for accessor in document.accessors() {
        if accessor.count()
            > limits
                .max_vertices
                .max(limits.max_triangles.saturating_mul(3))
        {
            return Err(ImportError::TooManyTriangles(format!(
                "a list holds {} entries",
                accessor.count()
            )));
        }
        if let Some(view) = accessor.view() {
            fits(view, accessor.offset(), accessor.count(), accessor.size()).ok_or_else(damaged)?;
        }
        if let Some(sparse) = accessor.sparse() {
            let index_size = sparse.indices().index_type().size();
            fits(
                sparse.indices().view(),
                sparse.indices().offset(),
                sparse.count(),
                index_size,
            )
            .ok_or_else(damaged)?;
            fits(
                sparse.values().view(),
                sparse.values().offset(),
                sparse.count(),
                accessor.size(),
            )
            .ok_or_else(damaged)?;
        }
    }
    Ok(())
}

/// Reads textures once each, however many materials use them.
struct TextureReader<'a> {
    buffers: &'a [Vec<u8>],
    folder: Option<&'a Path>,
    limits: &'a Limits,
    textures: Vec<Texture>,
    /// One texture per (image, sampler, colour or not) the file uses.
    made: HashMap<(usize, Option<usize>, bool), Option<usize>>,
    warnings: Vec<String>,
    /// Bytes still allowed to be read from image files beside the model.
    file_budget: u64,
    /// Bytes still allowed for decoded textures.
    texture_budget: u64,
}

impl TextureReader<'_> {
    /// The texture for `texture`, read if it hasn't been yet; `None` (with a
    /// warning) if it can't be read. `srgb` says whether its pixels are
    /// colours.
    fn get(
        &mut self,
        texture: &gltf::Texture<'_>,
        tex_coord: u32,
        srgb: bool,
        owner: &str,
    ) -> Option<usize> {
        if tex_coord != 0 {
            self.warnings.push(format!(
                "{owner}: only the first set of texture coordinates is used"
            ));
        }
        let image = texture.source();
        let key = (image.index(), texture.sampler().index(), srgb);
        if let Some(&made) = self.made.get(&key) {
            return made;
        }
        let sampler = sampler(&texture.sampler());
        let result = match read_image(&image, self.buffers, self.folder, &mut self.file_budget)
            .and_then(|bytes| {
                texture::decode(&bytes, sampler, srgb, self.limits, self.texture_budget)
            }) {
            Ok(texture) => {
                self.texture_budget = self.texture_budget.saturating_sub(texture.byte_size());
                self.textures.push(texture);
                Some(self.textures.len() - 1)
            }
            Err(why) => {
                self.warnings.push(format!(
                    "{}: its texture was left out ({why})",
                    image_name(&image)
                ));
                None
            }
        };
        self.made.insert(key, result);
        result
    }
}

/// A number from the file, kept within `low..=high`, or `fallback` if it
/// isn't a number at all.
fn tidy(value: f32, low: f32, high: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(low, high)
    } else {
        fallback
    }
}

/// The materials, and the textures they use. A texture that can't be read
/// is left out (the material keeps its plain values) with a warning, rather
/// than refusing the whole model.
fn read_materials(
    document: &gltf::Document,
    buffers: &[Vec<u8>],
    folder: Option<&Path>,
    limits: &Limits,
    warnings: &mut Vec<String>,
) -> (Vec<Material>, Vec<Texture>) {
    let buffer_bytes: u64 = buffers.iter().map(|b| b.len() as u64).sum();
    let mut reader = TextureReader {
        buffers,
        folder,
        limits,
        textures: Vec::new(),
        made: HashMap::new(),
        warnings: Vec::new(),
        file_budget: limits.max_file_bytes.saturating_sub(buffer_bytes),
        texture_budget: limits.max_texture_bytes,
    };
    let materials = document
        .materials()
        .map(|material| {
            let name = material_name(&material);
            let pbr = material.pbr_metallic_roughness();
            let base_color_texture = pbr
                .base_color_texture()
                .and_then(|info| reader.get(&info.texture(), info.tex_coord(), true, &name));
            let metallic_roughness_texture = pbr
                .metallic_roughness_texture()
                .and_then(|info| reader.get(&info.texture(), info.tex_coord(), false, &name));
            let normal = material.normal_texture();
            let normal_texture = normal
                .as_ref()
                .and_then(|n| reader.get(&n.texture(), n.tex_coord(), false, &name));
            let occlusion = material.occlusion_texture();
            let occlusion_texture = occlusion
                .as_ref()
                .and_then(|o| reader.get(&o.texture(), o.tex_coord(), false, &name));
            let emissive_texture = material
                .emissive_texture()
                .and_then(|info| reader.get(&info.texture(), info.tex_coord(), true, &name));
            Material {
                base_color: pbr.base_color_factor().map(|c| tidy(c, 0.0, 1.0, 1.0)),
                base_color_texture,
                metallic: tidy(pbr.metallic_factor(), 0.0, 1.0, 1.0),
                roughness: tidy(pbr.roughness_factor(), 0.0, 1.0, 1.0),
                metallic_roughness_texture,
                normal_texture,
                normal_scale: normal.map_or(1.0, |n| tidy(n.scale(), -10.0, 10.0, 1.0)),
                occlusion_texture,
                occlusion_strength: occlusion.map_or(1.0, |o| tidy(o.strength(), 0.0, 1.0, 1.0)),
                // Brighter than 1 only through the strength extension,
                // which isn't read yet.
                emissive: material.emissive_factor().map(|c| tidy(c, 0.0, 1.0, 0.0)),
                emissive_texture,
                alpha: match material.alpha_mode() {
                    gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
                    gltf::material::AlphaMode::Mask => {
                        AlphaMode::Mask(tidy(material.alpha_cutoff().unwrap_or(0.5), 0.0, 1.0, 0.5))
                    }
                    gltf::material::AlphaMode::Blend => AlphaMode::Blend,
                },
                double_sided: material.double_sided(),
                name,
            }
        })
        .collect();
    warnings.append(&mut reader.warnings);
    (materials, reader.textures)
}

fn material_name(material: &gltf::Material<'_>) -> String {
    material.name().map_or_else(
        || format!("Material {}", material.index().map_or(0, |i| i + 1)),
        str::to_string,
    )
}

fn image_name(image: &gltf::Image<'_>) -> String {
    match (image.name(), image.source()) {
        (Some(name), _) => name.to_string(),
        (None, gltf::image::Source::Uri { uri, .. }) if !uri.starts_with("data:") => {
            uri.to_string()
        }
        _ => format!("Image {}", image.index() + 1),
    }
}

fn sampler(sampler: &gltf::texture::Sampler<'_>) -> Sampler {
    let wrap = |mode| match mode {
        gltf::texture::WrappingMode::Repeat => Wrap::Repeat,
        gltf::texture::WrappingMode::MirroredRepeat => Wrap::Mirror,
        gltf::texture::WrappingMode::ClampToEdge => Wrap::Clamp,
    };
    Sampler {
        wrap_u: wrap(sampler.wrap_s()),
        wrap_v: wrap(sampler.wrap_t()),
        nearest: sampler.mag_filter() == Some(gltf::texture::MagFilter::Nearest),
    }
}

/// The encoded bytes of an image (still PNG or JPEG). Image files beside
/// the model share `file_budget` bytes with each other and the buffers.
fn read_image(
    image: &gltf::Image<'_>,
    buffers: &[Vec<u8>],
    folder: Option<&Path>,
    file_budget: &mut u64,
) -> Result<Vec<u8>, String> {
    match image.source() {
        gltf::image::Source::View { view, .. } => {
            let buffer = buffers
                .get(view.buffer().index())
                .ok_or("its data is missing")?;
            let end = view
                .offset()
                .checked_add(view.length())
                .ok_or("its data is damaged")?;
            buffer
                .get(view.offset()..end)
                .map(<[u8]>::to_vec)
                .ok_or_else(|| "its data runs past the end of the file".to_string())
        }
        gltf::image::Source::Uri { uri, .. } => {
            match resolve(uri, folder).map_err(|e| e.to_string())? {
                Address::Data(data) => Ok(data),
                Address::File(path) => {
                    let name = path
                        .file_name()
                        .map_or_else(|| uri.to_string(), |n| n.to_string_lossy().into_owned());
                    let data = read_limited(&path, *file_budget).map_err(|e| match e {
                        ImportError::TooBig { .. } => format!(
                            "{name} would take the model past {} MB of files",
                            limits_mb(*file_budget)
                        ),
                        _ => format!("the file {name} isn't there"),
                    })?;
                    *file_budget = file_budget.saturating_sub(data.len() as u64);
                    Ok(data)
                }
                Address::Refused(uri) => Err(format!("it is at {uri}, which Solis doesn't fetch")),
            }
        }
    }
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
