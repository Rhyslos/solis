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

fn describe_gltf_error(error: &gltf::Error) -> String {
    match error {
        gltf::Error::Io(e) => format!("reading failed ({e})"),
        gltf::Error::Deserialize(e) => format!("its text isn't valid glTF ({e})"),
        gltf::Error::Validation(_) => "it refers to parts that don't exist".into(),
        gltf::Error::Binary(_) => "its binary layout is broken".into(),
    }
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
    let meshes = geometry::read_meshes(document, &buffers, limits, &mut warnings)?;
    let bounds = Bounds::around(
        meshes
            .iter()
            .flat_map(|m| m.vertices.iter().map(|v| glam::Vec3::from(v.position))),
    )
    .ok_or(ImportError::Empty)?;
    Ok(Model {
        name: name.to_string(),
        meshes,
        materials,
        textures,
        bounds,
        warnings,
    })
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
    Ok(Address::File(folder.join(decoded.as_ref())))
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
                    Address::File(path) => {
                        read_limited(&path, limits.max_file_bytes).map_err(|e| match e {
                            ImportError::Unreadable(_) => ImportError::Damaged(format!(
                                "it needs the file {}, which isn't there",
                                path.file_name().map_or_else(
                                    || uri.to_string(),
                                    |n| n.to_string_lossy().into_owned()
                                )
                            )),
                            other => other,
                        })?
                    }
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

/// The materials, and the textures they use. A texture that can't be read
/// is left out (the material keeps its colour) with a warning, rather than
/// refusing the whole model.
fn read_materials(
    document: &gltf::Document,
    buffers: &[Vec<u8>],
    folder: Option<&Path>,
    limits: &Limits,
    warnings: &mut Vec<String>,
) -> (Vec<Material>, Vec<Texture>) {
    let mut textures = Vec::new();
    // One texture per (image, sampler) pair the file uses.
    let mut made: HashMap<(usize, Option<usize>), Option<usize>> = HashMap::new();
    let materials = document
        .materials()
        .map(|material| {
            let pbr = material.pbr_metallic_roughness();
            let base_color_texture = pbr.base_color_texture().and_then(|info| {
                if info.tex_coord() != 0 {
                    warnings.push(format!(
                        "{}: only the first set of texture coordinates is used",
                        material_name(&material)
                    ));
                }
                let gltf_texture = info.texture();
                let image = gltf_texture.source();
                let key = (image.index(), gltf_texture.sampler().index());
                *made.entry(key).or_insert_with(|| {
                    let sampler = sampler(&gltf_texture.sampler());
                    match read_image(&image, buffers, folder, limits)
                        .and_then(|bytes| texture::decode(&bytes, sampler, limits))
                    {
                        Ok(texture) => {
                            textures.push(texture);
                            Some(textures.len() - 1)
                        }
                        Err(why) => {
                            warnings.push(format!(
                                "{}: its texture was left out ({why})",
                                image_name(&image)
                            ));
                            None
                        }
                    }
                })
            });
            Material {
                name: material_name(&material),
                base_color: pbr.base_color_factor().map(|c| {
                    if c.is_finite() {
                        c.clamp(0.0, 1.0)
                    } else {
                        1.0
                    }
                }),
                base_color_texture,
                alpha: match material.alpha_mode() {
                    gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
                    gltf::material::AlphaMode::Mask => {
                        let cutoff = material.alpha_cutoff().unwrap_or(0.5);
                        AlphaMode::Mask(if cutoff.is_finite() {
                            cutoff.clamp(0.0, 1.0)
                        } else {
                            0.5
                        })
                    }
                    gltf::material::AlphaMode::Blend => AlphaMode::Blend,
                },
                double_sided: material.double_sided(),
            }
        })
        .collect();
    (materials, textures)
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

/// The encoded bytes of an image (still PNG or JPEG).
fn read_image(
    image: &gltf::Image<'_>,
    buffers: &[Vec<u8>],
    folder: Option<&Path>,
    limits: &Limits,
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
                Address::File(path) => read_limited(&path, limits.max_file_bytes).map_err(|_| {
                    format!(
                        "the file {} isn't there",
                        path.file_name()
                            .map_or_else(|| uri.to_string(), |n| n.to_string_lossy().into_owned())
                    )
                }),
                Address::Refused(uri) => Err(format!("it is at {uri}, which Solis doesn't fetch")),
            }
        }
    }
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
