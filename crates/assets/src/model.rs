//! A model as plain data, ready for the renderer: meshes, a tree of nodes
//! placing them, materials and textures, in Solis's own units (metres,
//! Z up).

use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec3};
use std::fmt;

/// One corner of a triangle.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Vertex {
    /// Where it is, in metres, Z up.
    pub position: [f32; 3],
    /// Which way the surface faces here (length 1).
    pub normal: [f32; 3],
    /// Where on the texture it takes its colour from (0 to 1 across).
    pub uv: [f32; 2],
}

/// Triangles that share one material.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mesh {
    /// The corners.
    pub vertices: Vec<Vertex>,
    /// Three indices into `vertices` per triangle.
    pub indices: Vec<u32>,
    /// Which of the model's materials it uses; `None` for the plain default.
    pub material: Option<usize>,
    /// Simpler versions, each simpler than the one before, drawn when a
    /// copy is small on screen (see `lods`).
    pub lods: Vec<Lod>,
}

/// A simpler version of a mesh (a level of detail).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Lod {
    /// Its own corners, or none to use the mesh's (a level made by
    /// simplifying the mesh shares them).
    pub vertices: Vec<Vertex>,
    /// Three indices into the corners per triangle.
    pub indices: Vec<u32>,
    /// How far it strays from the full mesh, as a share of the mesh's
    /// size (0.01 is 1%): it's drawn once that's too small to see.
    pub error: f32,
}

impl Lod {
    /// How many triangles it has.
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }
}

impl Mesh {
    /// How many triangles it has.
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }
}

/// How a material treats see-through parts of its colour.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AlphaMode {
    /// Fully solid; the colour's alpha is ignored.
    #[default]
    Opaque,
    /// Parts with alpha below the cutoff are cut away; the rest is solid
    /// (leaves, fences).
    Mask(f32),
    /// Blended with what's behind (glass, smoke).
    Blend,
}

/// How a surface looks: glTF's metal-and-roughness material, the standard
/// most programs export. Each texture is an index into the model's
/// textures, and multiplies the matching factor.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// The material's name in the file, if it had one.
    pub name: String,
    /// The colour, in linear RGBA (multiplied with the texture).
    pub base_color: [f32; 4],
    /// The colour texture (sRGB).
    pub base_color_texture: Option<usize>,
    /// How metal the surface is: 0 paint, plastic, wood; 1 bare metal.
    pub metallic: f32,
    /// How rough: 0 mirror-smooth, 1 chalky.
    pub roughness: f32,
    /// Metal in its blue channel, roughness in its green (linear).
    pub metallic_roughness_texture: Option<usize>,
    /// Small surface detail (bumps, panel lines) as directions (linear).
    pub normal_texture: Option<usize>,
    /// How strongly the normal texture bends the surface (1 as made).
    pub normal_scale: f32,
    /// Darkening in creases and corners, in its red channel (linear).
    pub occlusion_texture: Option<usize>,
    /// How much of the occlusion texture applies (0 to 1).
    pub occlusion_strength: f32,
    /// Light the surface gives off itself, linear RGB (lamps, screens).
    pub emissive: [f32; 3],
    /// The glowing parts' colour (sRGB), multiplied with `emissive`.
    pub emissive_texture: Option<usize>,
    /// How see-through parts are handled.
    pub alpha: AlphaMode,
    /// Whether both sides of each triangle are drawn.
    pub double_sided: bool,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            name: String::new(),
            base_color: [1.0; 4],
            base_color_texture: None,
            // A plain, slightly glossy surface for meshes without a
            // material. (glTF's own default is rough bare metal, which
            // looks dark without a sky to reflect.)
            metallic: 0.0,
            roughness: 0.5,
            metallic_roughness_texture: None,
            normal_texture: None,
            normal_scale: 1.0,
            occlusion_texture: None,
            occlusion_strength: 1.0,
            emissive: [0.0; 3],
            emissive_texture: None,
            alpha: AlphaMode::Opaque,
            double_sided: false,
        }
    }
}

/// What a texture does past its edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Wrap {
    /// Repeats.
    #[default]
    Repeat,
    /// Repeats, mirrored every other time.
    Mirror,
    /// Keeps the edge colour.
    Clamp,
}

/// How a texture is sampled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sampler {
    /// Across.
    pub wrap_u: Wrap,
    /// Down.
    pub wrap_v: Wrap,
    /// Sharp pixels (pixel art) rather than smooth.
    pub nearest: bool,
}

/// A texture: RGBA pixels in sRGB, with every smaller mip level worked
/// out (each half the size of the one before, down to 1×1), so distant
/// surfaces don't shimmer.
#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    /// Width of the full-size level, in pixels.
    pub width: u32,
    /// Height of the full-size level, in pixels.
    pub height: u32,
    /// The levels, largest first: 4 bytes per pixel, rows top to bottom.
    pub levels: Vec<Vec<u8>>,
    /// How it is sampled.
    pub sampler: Sampler,
    /// Whether the pixels are colours in sRGB (colour and glow textures),
    /// rather than plain numbers (metal, roughness, normals, occlusion).
    pub srgb: bool,
}

impl Texture {
    /// How many bytes its pixels take, every mip level included.
    pub fn byte_size(&self) -> u64 {
        self.levels.iter().map(|l| l.len() as u64).sum()
    }
}

/// The box a model fits in, in metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// The lowest corner.
    pub min: Vec3,
    /// The highest corner.
    pub max: Vec3,
}

impl Bounds {
    /// The box around `points`, or `None` when there are none.
    pub fn around(points: impl IntoIterator<Item = Vec3>) -> Option<Self> {
        points.into_iter().fold(None, |bounds, p| {
            Some(match bounds {
                None => Self { min: p, max: p },
                Some(Self { min, max }) => Self {
                    min: min.min(p),
                    max: max.max(p),
                },
            })
        })
    }

    /// The middle.
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// The size along each axis.
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }
}

/// One node of a model's tree: a named place that can carry meshes, as in
/// Blender's outliner.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Its name in the file, or "Node 3".
    pub name: String,
    /// The node it belongs to; parents always come before their children.
    pub parent: Option<usize>,
    /// Where it is relative to its parent.
    pub local: Mat4,
    /// Where it is in the model: its parents' placements and its own.
    pub world: Mat4,
    /// The meshes it shows (indices into [`Model::meshes`]).
    pub meshes: Vec<usize>,
}

/// A whole model, read from a file.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    /// A name to show, such as the file's name without its extension.
    pub name: String,
    /// The meshes, each once, in their own space: nodes place them, and a
    /// mesh used by several nodes is stored once.
    pub meshes: Vec<Mesh>,
    /// The nodes, parents first.
    pub nodes: Vec<Node>,
    /// The materials meshes refer to.
    pub materials: Vec<Material>,
    /// The textures materials refer to.
    pub textures: Vec<Texture>,
    /// The lights the file carries, placed.
    pub lamps: Vec<crate::lights::Lamp>,
    /// The box around every mesh (or, for a file of lights only, around
    /// the lights).
    pub bounds: Bounds,
    /// Things in the file that were left out or replaced, in plain words.
    pub warnings: Vec<String>,
}

impl Model {
    /// Every mesh as drawn: which mesh, and where in the model.
    pub fn placements(&self) -> impl Iterator<Item = (usize, Mat4)> + '_ {
        self.nodes
            .iter()
            .flat_map(|node| node.meshes.iter().map(move |&m| (m, node.world)))
            .filter(|(m, _)| *m < self.meshes.len())
    }

    /// How many triangles each level of detail draws, counting each
    /// placement: the full model first, then each simpler level (a mesh
    /// with fewer levels draws its simplest). Just the full model when no
    /// mesh has any.
    pub fn lod_triangles(&self) -> Vec<usize> {
        let levels = self.meshes.iter().map(|m| m.lods.len()).max().unwrap_or(0);
        (0..=levels)
            .map(|level| {
                self.placements()
                    .map(|(m, _)| {
                        let mesh = &self.meshes[m];
                        match level.min(mesh.lods.len()) {
                            0 => mesh.triangles(),
                            l => mesh.lods[l - 1].triangles(),
                        }
                    })
                    .sum()
            })
            .collect()
    }

    /// How many triangles are drawn, counting each placement.
    pub fn triangles(&self) -> usize {
        self.placements()
            .map(|(m, _)| self.meshes[m].triangles())
            .sum()
    }

    /// How many vertices are drawn, counting each placement.
    pub fn vertices(&self) -> usize {
        self.placements()
            .map(|(m, _)| self.meshes[m].vertices.len())
            .sum()
    }

    /// Makes one node showing every mesh where it is, for a model built by
    /// hand (in tests and samples).
    pub fn show_every_mesh(&mut self) {
        self.nodes = vec![Node {
            name: self.name.clone(),
            parent: None,
            local: Mat4::IDENTITY,
            world: Mat4::IDENTITY,
            meshes: (0..self.meshes.len()).collect(),
        }];
    }

    /// Every placement baked into a mesh of its own, in the model's space,
    /// as models were held before node trees: a mirrored placement has
    /// its triangles turned so their fronts stay outside.
    pub fn placed_meshes(&self) -> Vec<Mesh> {
        self.placements()
            .map(|(m, world)| bake(&self.meshes[m], world))
            .collect()
    }
}

/// `mesh` moved by `world`.
fn bake(mesh: &Mesh, world: Mat4) -> Mesh {
    let turn = Mat3::from_mat4(world);
    let normal_matrix = if turn.determinant().abs() > 1e-12 {
        turn.inverse().transpose()
    } else {
        Mat3::IDENTITY
    };
    let vertices = mesh
        .vertices
        .iter()
        .map(|v| Vertex {
            position: world.transform_point3(Vec3::from(v.position)).to_array(),
            normal: (normal_matrix * Vec3::from(v.normal))
                .try_normalize()
                .unwrap_or(Vec3::Z)
                .to_array(),
            uv: v.uv,
        })
        .collect();
    let indices = if turn.determinant() < 0.0 {
        mesh.indices
            .chunks_exact(3)
            .flat_map(|t| [t[0], t[2], t[1]])
            .collect()
    } else {
        mesh.indices.clone()
    };
    Mesh {
        vertices,
        indices,
        material: mesh.material,
        lods: Vec::new(),
    }
}

/// The most a model may hold, checked before anything big is allocated,
/// so a damaged or hostile file can't exhaust memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The largest file read, in bytes (also each separate buffer file).
    pub max_file_bytes: u64,
    /// The most vertices in the whole model.
    pub max_vertices: usize,
    /// The most triangles in the whole model.
    pub max_triangles: usize,
    /// The widest or tallest texture kept; bigger ones are scaled down.
    /// Set it to what the graphics card can hold.
    pub max_texture_side: u32,
    /// The widest or tallest image decoded at all; bigger ones are refused.
    pub max_image_side: u32,
    /// The most bytes all of a model's textures may take once decoded,
    /// with their mip levels; textures past it are left out.
    pub max_texture_bytes: u64,
    /// The most meshes in the whole model, counting each placement of a
    /// mesh the file reuses.
    pub max_meshes: usize,
    /// The most vertices in one mesh; bigger meshes are split into pieces
    /// this size. Set it to what one GPU buffer can hold (32 bytes a
    /// vertex).
    pub max_mesh_vertices: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_file_bytes: 1 << 30,
            max_vertices: 30_000_000,
            max_triangles: 30_000_000,
            max_texture_side: 8192,
            max_image_side: 16_384,
            max_texture_bytes: 2 << 30,
            max_meshes: 100_000,
            // 256 MiB, the smallest buffer limit a GPU may have, at 32
            // bytes a vertex.
            max_mesh_vertices: (256 << 20) / 32,
        }
    }
}

/// Why a model couldn't be read, in plain words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    /// The file couldn't be read at all.
    Unreadable(String),
    /// The file is bigger than the limit.
    TooBig {
        /// Its size in bytes.
        bytes: u64,
        /// The limit in bytes.
        limit: u64,
    },
    /// The file isn't glTF, or its structure is broken.
    NotGltf(String),
    /// The file is glTF, but parts of it are damaged or missing.
    Damaged(String),
    /// The model is bigger than the limits allow.
    TooManyTriangles(String),
    /// The file holds nothing that can be drawn.
    Empty,
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable(why) => write!(f, "the file couldn't be read: {why}"),
            Self::TooBig { bytes, limit } => write!(
                f,
                "the file is {} MB, more than the {} MB limit",
                bytes / 1_000_000,
                limit / 1_000_000
            ),
            Self::NotGltf(why) => write!(f, "this isn't a glTF file Solis can read: {why}"),
            Self::Damaged(why) => write!(f, "the file is damaged: {why}"),
            Self::TooManyTriangles(why) => write!(f, "the model is too big: {why}"),
            Self::Empty => write!(f, "the file holds no triangles to draw"),
        }
    }
}

impl std::error::Error for ImportError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertices_are_tightly_packed() {
        assert_eq!(std::mem::size_of::<Vertex>(), 32);
    }

    #[test]
    fn bounds_surround_their_points() {
        let b = Bounds::around([Vec3::new(1.0, -2.0, 3.0), Vec3::new(-1.0, 4.0, 0.0)]).unwrap();
        assert_eq!(b.min, Vec3::new(-1.0, -2.0, 0.0));
        assert_eq!(b.max, Vec3::new(1.0, 4.0, 3.0));
        assert_eq!(b.center(), Vec3::new(0.0, 1.0, 1.5));
        assert!(Bounds::around([]).is_none());
    }

    #[test]
    fn errors_read_as_plain_sentences() {
        let e = ImportError::TooBig {
            bytes: 3_000_000_000,
            limit: 1_000_000_000,
        };
        assert_eq!(
            e.to_string(),
            "the file is 3000 MB, more than the 1000 MB limit"
        );
        assert!(ImportError::Empty.to_string().contains("no triangles"));
    }
}
