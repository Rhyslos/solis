//! Models on the GPU: an [`assets::Model`] uploaded once into vertex and
//! index buffers, textures and per-material settings, then drawn every
//! frame in the chosen look (standard, stylized or unlit; see
//! `viewport.wgsl`), lit by a fixed sun and the sky until real lights
//! arrive in milestone 4.

use crate::renderer::{DEPTH_FORMAT, HDR_FORMAT};
use assets::{AlphaMode, Material, Model, Texture, Wrap};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// A material's settings exactly as the shader's `Material` struct
/// expects them.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct MaterialUniform {
    /// Linear RGBA, multiplied with the colour texture.
    base_color: [f32; 4],
    /// Glow, linear RGB; w unused.
    emissive: [f32; 4],
    /// Metallic, roughness, normal strength, occlusion strength.
    surface: [f32; 4],
    /// x: alpha cutoff; y: 0 solid, 1 cut out, 2 blended; zw unused.
    alpha: [f32; 4],
}

impl MaterialUniform {
    pub(crate) fn new(material: &Material) -> Self {
        let (cutoff, mode) = match material.alpha {
            AlphaMode::Opaque => (0.0, 0.0),
            AlphaMode::Mask(cutoff) => (cutoff, 1.0),
            AlphaMode::Blend => (0.0, 2.0),
        };
        let [r, g, b] = material.emissive;
        Self {
            base_color: material.base_color,
            emissive: [r, g, b, 0.0],
            surface: [
                material.metallic,
                material.roughness,
                material.normal_scale,
                material.occlusion_strength,
            ],
            alpha: [cutoff, mode, 0.0, 0.0],
        }
    }
}

/// How a mesh is drawn, which picks its pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pass {
    /// Solid, back faces hidden.
    Solid,
    /// Solid, both sides drawn.
    SolidDoubleSided,
    /// Blended with what's behind, drawn last, both sides.
    Blended,
}

#[derive(Debug)]
struct GpuMaterial {
    bind_group: wgpu::BindGroup,
    pass: Pass,
    /// Whether the stylized look draws an outline around it: solid,
    /// single-sided surfaces without cut-out parts.
    outlined: bool,
}

#[derive(Debug)]
struct GpuMesh {
    vertices: wgpu::Buffer,
    /// The same corners with smoothed normals, for outlines (only for
    /// outlined materials).
    outline_vertices: Option<wgpu::Buffer>,
    indices: wgpu::Buffer,
    index_count: u32,
    material: usize,
    /// Its simpler levels, each simpler than the one before.
    lods: Vec<GpuLod>,
}

/// A mesh's simpler level on the GPU: its own triangles, over the mesh's
/// corners or its own.
#[derive(Debug)]
struct GpuLod {
    /// Its own corners (and outline corners), or `None` for the mesh's.
    vertices: Option<(wgpu::Buffer, Option<wgpu::Buffer>)>,
    indices: wgpu::Buffer,
    index_count: u32,
}

/// What to draw for one level of one mesh.
struct Shape<'a> {
    vertices: &'a wgpu::Buffer,
    outline: Option<&'a wgpu::Buffer>,
    indices: &'a wgpu::Buffer,
    index_count: u32,
    material: &'a GpuMaterial,
}

impl Shape<'_> {
    fn draw(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        vertices: &wgpu::Buffer,
        instances: std::ops::Range<u32>,
    ) {
        render_pass.set_vertex_buffer(0, vertices.slice(..));
        render_pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.index_count, 0, instances);
    }
}

/// A model uploaded to the GPU, ready to draw with
/// [`ViewportRenderer::render`](crate::ViewportRenderer::render): each mesh
/// once, and where the model places each (decision 0021).
#[derive(Debug)]
pub struct GpuModel {
    /// The meshes, by their index in the model; `None` for one that can't
    /// be drawn (empty, or too big for the card).
    meshes: Vec<Option<GpuMesh>>,
    /// Every mesh as drawn: which, where in the model, and the ball
    /// around it there.
    placements: Vec<Drawn>,
    /// Each node's own meshes, for drawing one part on its own.
    nodes: Vec<Vec<usize>>,
    materials: Vec<GpuMaterial>,
    triangles: usize,
    bounds: (glam::Vec3, glam::Vec3),
    /// A ball around each mesh, centre and radius, in the mesh's own
    /// space, for culling.
    spheres: Vec<(glam::Vec3, f32)>,
    /// For each level of detail (the full model first, at 0), how far it
    /// strays from the full model at most, as a share of the model's
    /// size.
    lod_errors: Vec<f32>,
    /// The ball around the whole model, centre and radius, for choosing
    /// its level of detail.
    sphere: (glam::Vec3, f32),
}

impl GpuModel {
    /// How many triangles it draws.
    pub fn triangles(&self) -> usize {
        self.triangles
    }

    /// The box the model fits in, lowest and highest corners, in metres.
    pub fn bounds(&self) -> (glam::Vec3, glam::Vec3) {
        self.bounds
    }

    /// A ball around mesh `mesh` in its own space: centre and radius
    /// (a point for one that isn't there).
    pub(crate) fn sphere_of(&self, mesh: usize) -> (glam::Vec3, f32) {
        self.spheres
            .get(mesh)
            .copied()
            .unwrap_or((glam::Vec3::ZERO, 0.0))
    }

    /// Every mesh as drawn: which, where in the model, and the ball
    /// around it there.
    pub(crate) fn placements(&self) -> &[Drawn] {
        &self.placements
    }

    /// Node `node`'s own meshes (none for a node that isn't there).
    pub(crate) fn node_meshes(&self, node: usize) -> &[usize] {
        self.nodes.get(node).map_or(&[], Vec::as_slice)
    }

    /// Level `lod` of mesh `mesh` (0 for the full mesh, past its last
    /// for its simplest) and its material, if it can be drawn.
    fn shape(&self, mesh: usize, lod: usize) -> Option<Shape<'_>> {
        let mesh = self.meshes.get(mesh)?.as_ref()?;
        let material = self.materials.get(mesh.material)?;
        let full = Shape {
            vertices: &mesh.vertices,
            outline: mesh.outline_vertices.as_ref(),
            indices: &mesh.indices,
            index_count: mesh.index_count,
            material,
        };
        let shape = match lod.min(mesh.lods.len()).checked_sub(1) {
            None => full,
            Some(i) => {
                let level = &mesh.lods[i];
                let (vertices, outline) = match &level.vertices {
                    Some((own, outline)) => (own, outline.as_ref()),
                    None => (&mesh.vertices, mesh.outline_vertices.as_ref()),
                };
                Shape {
                    vertices,
                    outline,
                    indices: &level.indices,
                    index_count: level.index_count,
                    material,
                }
            }
        };
        (shape.index_count > 0).then_some(shape)
    }

    /// How many simpler levels mesh `mesh` has.
    pub(crate) fn lods_of(&self, mesh: usize) -> usize {
        self.meshes
            .get(mesh)
            .and_then(Option::as_ref)
            .map_or(0, |m| m.lods.len())
    }

    /// For each level of detail (the full model at 0), how far it strays
    /// at most, as a share of the model's size.
    pub(crate) fn lod_errors(&self) -> &[f32] {
        &self.lod_errors
    }

    /// The ball around the whole model: centre and radius.
    pub(crate) fn sphere(&self) -> (glam::Vec3, f32) {
        self.sphere
    }

    /// How many triangles level `lod` of a mesh has (0 if it can't be
    /// drawn).
    pub(crate) fn triangles_of(&self, mesh: usize, lod: usize) -> u32 {
        self.shape(mesh, lod).map_or(0, |s| s.index_count / 3)
    }

    /// Which pass a mesh is drawn in.
    pub(crate) fn pass_of(&self, mesh: usize) -> Option<Pass> {
        self.shape(mesh, 0).map(|s| s.material.pass)
    }

    /// Draws `instances` of level `lod` of a mesh's shape only, for the
    /// sun's shadow map (blended ones, like glass, cast no shadow).
    pub(crate) fn draw_depth(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh: usize,
        lod: usize,
        instances: std::ops::Range<u32>,
    ) {
        let Some(shape) = self.shape(mesh, lod) else {
            return;
        };
        if shape.material.pass == Pass::Blended {
            return;
        }
        shape.draw(render_pass, shape.vertices, instances);
    }

    /// Draws `instances` of a mesh's full shape only, whatever its
    /// material (glass too), for picking.
    pub(crate) fn draw_shape(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh: usize,
        instances: std::ops::Range<u32>,
    ) {
        let Some(shape) = self.shape(mesh, 0) else {
            return;
        };
        shape.draw(render_pass, shape.vertices, instances);
    }

    /// Draws `instances` of level `lod` of a mesh with its material, with
    /// the pipeline for its pass already set.
    pub(crate) fn draw(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh: usize,
        lod: usize,
        instances: std::ops::Range<u32>,
    ) {
        let Some(shape) = self.shape(mesh, lod) else {
            return;
        };
        render_pass.set_bind_group(1, &shape.material.bind_group, &[]);
        shape.draw(render_pass, shape.vertices, instances);
    }

    /// Draws the outline of `instances` of level `lod` of a mesh, if it
    /// gets one, with the outline pipeline already set.
    pub(crate) fn draw_outline(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        mesh: usize,
        lod: usize,
        instances: std::ops::Range<u32>,
    ) {
        let Some(shape) = self.shape(mesh, lod) else {
            return;
        };
        if let Some(outline) = shape.outline {
            shape.draw(render_pass, outline, instances);
        }
    }
}

/// The textures a material reads, in binding order: colour, metal and
/// roughness, normals, occlusion, glow.
const TEXTURE_SLOTS: usize = 5;

/// The pipelines for one way of winding triangles.
#[derive(Debug)]
pub(crate) struct Facing {
    pub(crate) solid: wgpu::RenderPipeline,
    pub(crate) solid_double_sided: wgpu::RenderPipeline,
    pub(crate) blended: wgpu::RenderPipeline,
    pub(crate) outline: wgpu::RenderPipeline,
}

impl Facing {
    /// The pipeline for meshes in `pass`.
    pub(crate) fn for_pass(&self, pass: Pass) -> &wgpu::RenderPipeline {
        match pass {
            Pass::Solid => &self.solid,
            Pass::SolidDoubleSided => &self.solid_double_sided,
            Pass::Blended => &self.blended,
        }
    }
}

/// The parts every model's materials share.
#[derive(Debug)]
pub(crate) struct ModelPipelines {
    pub(crate) material_layout: wgpu::BindGroupLayout,
    /// For copies as made.
    pub(crate) normal: Facing,
    /// For mirrored copies, whose triangles wind the other way.
    pub(crate) mirrored: Facing,
}

impl ModelPipelines {
    pub(crate) fn new(
        device: &wgpu::Device,
        frame_layout: &wgpu::BindGroupLayout,
        shader: &wgpu::ShaderModule,
        sample_count: u32,
    ) -> Self {
        let mut entries = vec![wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }];
        for slot in 0..TEXTURE_SLOTS as u32 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 1 + slot,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
        }
        for slot in 0..TEXTURE_SLOTS as u32 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 1 + TEXTURE_SLOTS as u32 + slot,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            });
        }
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material layout"),
            entries: &entries,
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("model layout"),
            bind_group_layouts: &[Some(frame_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let outline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("outline layout"),
            bind_group_layouts: &[Some(frame_layout)],
            immediate_size: 0,
        });
        let facing = |front| {
            let pipeline =
                |pass| create_pipeline(device, &layout, shader, sample_count, pass, front);
            Facing {
                solid: pipeline(Pass::Solid),
                solid_double_sided: pipeline(Pass::SolidDoubleSided),
                blended: pipeline(Pass::Blended),
                outline: create_outline_pipeline(
                    device,
                    &outline_layout,
                    shader,
                    sample_count,
                    front,
                ),
            }
        };
        Self {
            normal: facing(wgpu::FrontFace::Ccw),
            mirrored: facing(wgpu::FrontFace::Cw),
            material_layout,
        }
    }

    /// Uploads `model`. Textures larger than the GPU allows are replaced by
    /// plain ones rather than failing (the loader is normally told the
    /// GPU's limit, so this doesn't happen).
    pub(crate) fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        model: &Model,
    ) -> GpuModel {
        let max_side = device.limits().max_texture_dimension_2d;
        let textures: Vec<Option<(wgpu::TextureView, wgpu::Sampler)>> = model
            .textures
            .iter()
            .map(|t| {
                let usable =
                    t.width <= max_side && t.height <= max_side && t.width > 0 && t.height > 0;
                usable.then(|| upload_texture(device, queue, t))
            })
            .collect();
        // What a material without a given texture reads instead: white
        // (factors alone), or a flat normal.
        let white_colour = upload_texture(device, queue, &assets::texture::solid([255; 4], true));
        let white_data = upload_texture(device, queue, &assets::texture::solid([255; 4], false));
        let flat_normal = upload_texture(
            device,
            queue,
            &assets::texture::solid([128, 128, 255, 255], false),
        );
        // Each material, then the plain default last for meshes without one.
        let mut materials: Vec<&Material> = model.materials.iter().collect();
        let default = Material::default();
        materials.push(&default);
        let materials: Vec<GpuMaterial> = materials
            .into_iter()
            .map(|material| {
                let pick = |index: Option<usize>, fallback| {
                    index
                        .and_then(|i| textures.get(i))
                        .and_then(Option::as_ref)
                        .unwrap_or(fallback)
                };
                let slots = [
                    pick(material.base_color_texture, &white_colour),
                    pick(material.metallic_roughness_texture, &white_data),
                    pick(material.normal_texture, &flat_normal),
                    pick(material.occlusion_texture, &white_data),
                    pick(material.emissive_texture, &white_colour),
                ];
                let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("material"),
                    contents: bytemuck::bytes_of(&MaterialUniform::new(material)),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                let mut entries = vec![wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                }];
                for (slot, (view, _)) in slots.iter().enumerate() {
                    entries.push(wgpu::BindGroupEntry {
                        binding: 1 + slot as u32,
                        resource: wgpu::BindingResource::TextureView(view),
                    });
                }
                for (slot, (_, sampler)) in slots.iter().enumerate() {
                    entries.push(wgpu::BindGroupEntry {
                        binding: 1 + TEXTURE_SLOTS as u32 + slot as u32,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    });
                }
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("material bind group"),
                    layout: &self.material_layout,
                    entries: &entries,
                });
                let pass = match (material.alpha, material.double_sided) {
                    (AlphaMode::Blend, _) => Pass::Blended,
                    (_, true) => Pass::SolidDoubleSided,
                    (_, false) => Pass::Solid,
                };
                GpuMaterial {
                    bind_group,
                    pass,
                    outlined: pass == Pass::Solid && material.alpha == AlphaMode::Opaque,
                }
            })
            .collect();
        let default_material = materials.len() - 1;
        let meshes = model
            .meshes
            .iter()
            .map(|mesh| {
                // A mesh too big for one of the card's buffers would make
                // wgpu stop the program; the importer splits meshes to
                // fit, so this only guards against a model read with other
                // limits.
                let max = device.limits().max_buffer_size;
                let drawable = !mesh.indices.is_empty()
                    && !mesh.vertices.is_empty()
                    && (std::mem::size_of_val(mesh.vertices.as_slice()) as u64) <= max
                    && (std::mem::size_of_val(mesh.indices.as_slice()) as u64) <= max;
                drawable.then_some(mesh)
            })
            .map(|mesh| {
                let mesh = mesh?;
                let max = device.limits().max_buffer_size;
                let material = mesh
                    .material
                    .filter(|&m| m < default_material)
                    .unwrap_or(default_material);
                let outlined = materials.get(material).is_some_and(|m| m.outlined);
                Some(GpuMesh {
                    vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("mesh vertices"),
                        contents: bytemuck::cast_slice(&mesh.vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    }),
                    outline_vertices: outlined.then(|| {
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("outline vertices"),
                            contents: bytemuck::cast_slice(&outline_vertices(&mesh.vertices)),
                            usage: wgpu::BufferUsages::VERTEX,
                        })
                    }),
                    indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("mesh indices"),
                        contents: bytemuck::cast_slice(&mesh.indices),
                        usage: wgpu::BufferUsages::INDEX,
                    }),
                    index_count: mesh.indices.len() as u32,
                    material,
                    lods: mesh
                        .lods
                        .iter()
                        .filter(|lod| {
                            (std::mem::size_of_val(lod.vertices.as_slice()) as u64) <= max
                                && (std::mem::size_of_val(lod.indices.as_slice()) as u64) <= max
                                && !lod.indices.is_empty()
                        })
                        .map(|lod| GpuLod {
                            vertices: (!lod.vertices.is_empty()).then(|| {
                                (
                                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                        label: Some("level of detail vertices"),
                                        contents: bytemuck::cast_slice(&lod.vertices),
                                        usage: wgpu::BufferUsages::VERTEX,
                                    }),
                                    outlined.then(|| {
                                        device.create_buffer_init(
                                            &wgpu::util::BufferInitDescriptor {
                                                label: Some("level of detail outline vertices"),
                                                contents: bytemuck::cast_slice(&outline_vertices(
                                                    &lod.vertices,
                                                )),
                                                usage: wgpu::BufferUsages::VERTEX,
                                            },
                                        )
                                    }),
                                )
                            }),
                            indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("level of detail indices"),
                                contents: bytemuck::cast_slice(&lod.indices),
                                usage: wgpu::BufferUsages::INDEX,
                            }),
                            index_count: lod.indices.len() as u32,
                        })
                        .collect(),
                })
            })
            .collect();
        let spheres: Vec<(glam::Vec3, f32)> = model
            .meshes
            .iter()
            .map(|m| sphere_around(&m.vertices))
            .collect();
        let placements = model
            .placements()
            .map(|(mesh, local)| {
                Drawn::new(mesh, local, spheres[mesh], model.meshes[mesh].lods.len())
            })
            .collect();
        let lod_errors = model_lod_errors(model);
        let sphere = {
            let (min, max) = (model.bounds.min, model.bounds.max);
            if min.is_finite() && max.is_finite() {
                ((min + max) * 0.5, (max - min).length() * 0.5)
            } else {
                (glam::Vec3::ZERO, 0.0)
            }
        };
        GpuModel {
            lod_errors,
            sphere,
            meshes,
            placements,
            nodes: model.nodes.iter().map(|n| n.meshes.clone()).collect(),
            materials,
            triangles: model.triangles(),
            bounds: (model.bounds.min, model.bounds.max),
            spheres,
        }
    }
}

/// For each level of detail of `model` (the full model at 0), how far it
/// strays at most, as a share of the model's size: each mesh's error is
/// a share of its own size (its box's longest side, as meshoptimizer
/// measures), turned into a share of the model's diagonal. A mesh with
/// fewer levels draws its simplest at the levels past them.
pub(crate) fn model_lod_errors(model: &Model) -> Vec<f32> {
    let levels = model
        .meshes
        .iter()
        .map(|m| m.lods.len())
        .max()
        .unwrap_or(0)
        .min(assets::lods::MAX_LODS);
    let size = (model.bounds.max - model.bounds.min).length();
    if !(size.is_finite() && size > 0.0) {
        return vec![0.0];
    }
    let mut errors = vec![0.0_f32; levels + 1];
    for (mesh, local) in model.placements() {
        let mesh = &model.meshes[mesh];
        let Some((min, max)) = mesh.vertices.iter().fold(None, |acc, v| {
            let p = glam::Vec3::from(v.position);
            p.is_finite()
                .then(|| {
                    acc.map_or((p, p), |(lo, hi): (glam::Vec3, glam::Vec3)| {
                        (lo.min(p), hi.max(p))
                    })
                })
                .or(acc)
        }) else {
            continue;
        };
        let turn = glam::Mat3::from_mat4(local);
        let scale = turn
            .x_axis
            .length()
            .max(turn.y_axis.length())
            .max(turn.z_axis.length());
        let own = (max - min).max_element() * scale;
        for (level, error) in errors.iter_mut().enumerate().skip(1) {
            if let Some(lod) = mesh.lods.get(level.min(mesh.lods.len()).wrapping_sub(1)) {
                let share = lod.error * own / size;
                if share.is_finite() {
                    *error = error.max(share);
                }
            }
        }
    }
    errors
}

/// One mesh as a model draws it, worked out once so each copy of the
/// model does little: which mesh, where in the model, the ball around it
/// there, and whether it's mirrored there.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Drawn {
    pub(crate) mesh: usize,
    pub(crate) local: glam::Mat4,
    /// Whether `local` is the identity, so a copy's place is its own.
    pub(crate) at_origin: bool,
    pub(crate) centre: glam::Vec3,
    pub(crate) radius: f32,
    pub(crate) mirrored: bool,
    /// How many simpler levels the mesh has.
    pub(crate) lods: u8,
}

impl Drawn {
    /// Mesh `mesh` at `local` in its model, its ball `sphere` in its own
    /// space.
    pub(crate) fn new(
        mesh: usize,
        local: glam::Mat4,
        sphere: (glam::Vec3, f32),
        lods: usize,
    ) -> Self {
        let turn = glam::Mat3::from_mat4(local);
        let scale = turn
            .x_axis
            .length()
            .max(turn.y_axis.length())
            .max(turn.z_axis.length());
        Self {
            mesh,
            local,
            at_origin: local == glam::Mat4::IDENTITY,
            centre: local.transform_point3(sphere.0),
            radius: sphere.1 * scale,
            mirrored: turn.determinant() < 0.0,
            lods: lods.min(assets::lods::MAX_LODS) as u8,
        }
    }
}

/// A ball around `vertices`: the middle of their box, and the furthest
/// one from it. Corners that aren't numbers are left out.
pub(crate) fn sphere_around(vertices: &[assets::Vertex]) -> (glam::Vec3, f32) {
    let points = || {
        vertices
            .iter()
            .map(|v| glam::Vec3::from(v.position))
            .filter(|p| p.is_finite())
    };
    let Some((min, max)) = points().fold(None, |acc: Option<(glam::Vec3, glam::Vec3)>, p| {
        Some(acc.map_or((p, p), |(lo, hi)| (lo.min(p), hi.max(p))))
    }) else {
        return (glam::Vec3::ZERO, 0.0);
    };
    let centre = (min + max) * 0.5;
    let radius = points().map(|p| p.distance(centre)).fold(0.0, f32::max);
    (centre, radius)
}

/// The corners with each normal replaced by the average of all normals at
/// that same position, so a hard-edged shape (a box: three normals at each
/// corner) is pushed out evenly and its outline has no gaps.
pub(crate) fn outline_vertices(vertices: &[assets::Vertex]) -> Vec<assets::Vertex> {
    use std::collections::HashMap;
    let key = |v: &assets::Vertex| v.position.map(f32::to_bits);
    let mut sums: HashMap<[u32; 3], glam::Vec3> = HashMap::new();
    for v in vertices {
        *sums.entry(key(v)).or_default() += glam::Vec3::from(v.normal);
    }
    vertices
        .iter()
        .map(|v| {
            let smooth = sums
                .get(&key(v))
                .and_then(|n| n.try_normalize())
                .unwrap_or(glam::Vec3::from(v.normal));
            assets::Vertex {
                normal: smooth.to_array(),
                ..*v
            }
        })
        .collect()
}

fn upload_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &Texture,
) -> (wgpu::TextureView, wgpu::Sampler) {
    let size = wgpu::Extent3d {
        width: texture.width,
        height: texture.height,
        depth_or_array_layers: 1,
    };
    let gpu = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("model texture"),
        size,
        mip_level_count: texture.levels.len().max(1) as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // Colours are stored as sRGB, and sampling turns them into linear
        // light; data textures (metal, roughness, normals) are read as is.
        format: if texture.srgb {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        },
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let (mut width, mut height) = (texture.width, texture.height);
    for (level, pixels) in texture.levels.iter().enumerate() {
        if pixels.len() as u64 == u64::from(width) * u64::from(height) * 4 {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &gpu,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * width),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
    let address = |wrap| match wrap {
        Wrap::Repeat => wgpu::AddressMode::Repeat,
        Wrap::Mirror => wgpu::AddressMode::MirrorRepeat,
        Wrap::Clamp => wgpu::AddressMode::ClampToEdge,
    };
    let filter = if texture.sampler.nearest {
        wgpu::FilterMode::Nearest
    } else {
        wgpu::FilterMode::Linear
    };
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("model sampler"),
        address_mode_u: address(texture.sampler.wrap_u),
        address_mode_v: address(texture.sampler.wrap_v),
        mag_filter: filter,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..Default::default()
    });
    (
        gpu.create_view(&wgpu::TextureViewDescriptor::default()),
        sampler,
    )
}

/// How a mesh corner is laid out in the vertex buffer.
const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];

pub(crate) fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<assets::Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &VERTEX_ATTRIBUTES,
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    sample_count: u32,
    pass: Pass,
    front_face: wgpu::FrontFace,
) -> wgpu::RenderPipeline {
    let blended = pass == Pass::Blended;
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(match pass {
            Pass::Solid => "solid mesh pipeline",
            Pass::SolidDoubleSided => "double-sided mesh pipeline",
            Pass::Blended => "blended mesh pipeline",
        }),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("mesh_vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(vertex_layout()), Some(crate::instances::layout())],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face,
            cull_mode: (pass == Pass::Solid).then_some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(!blended),
            depth_compare: Some(wgpu::CompareFunction::Greater),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("mesh_fs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: HDR_FORMAT,
                blend: blended.then_some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// The outline pass: the back faces only (front faces culled), pushed out
/// by the vertex shader, behind the model itself.
fn create_outline_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    sample_count: u32,
    front_face: wgpu::FrontFace,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("outline pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("outline_vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(vertex_layout()), Some(crate::instances::layout())],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face,
            cull_mode: Some(wgpu::Face::Front),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Greater),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("outline_fs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: HDR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// The pipeline that draws models into the sun's shadow map: depth only,
/// both sides of every triangle, pushed back a little (depth bias) so
/// surfaces don't shadow themselves in stripes.
pub(crate) fn create_shadow_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("shadow pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("shadow_vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(vertex_layout()), Some(crate::instances::layout())],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: crate::lighting::SHADOW_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 2.0,
                clamp: 0.0,
            },
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: None,
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_normals_average_the_normals_at_a_corner() {
        let corner = |normal: [f32; 3]| assets::Vertex {
            position: [1.0, 1.0, 1.0],
            normal,
            uv: [0.0, 0.0],
        };
        let smoothed = outline_vertices(&[
            corner([1.0, 0.0, 0.0]),
            corner([0.0, 1.0, 0.0]),
            corner([0.0, 0.0, 1.0]),
        ]);
        let expected = glam::Vec3::ONE.normalize();
        for v in smoothed {
            assert!((glam::Vec3::from(v.normal) - expected).length() < 1e-5);
        }
    }

    #[test]
    fn a_mesh_s_ball_holds_every_corner() {
        let at = |position: [f32; 3]| assets::Vertex {
            position,
            normal: [0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
        };
        let corners = [
            at([0.0, 0.0, 0.0]),
            at([2.0, 0.0, 0.0]),
            at([1.0, 4.0, 0.0]),
        ];
        let (centre, radius) = sphere_around(&corners);
        for c in &corners {
            assert!(glam::Vec3::from(c.position).distance(centre) <= radius + 1e-5);
        }
        assert_eq!(sphere_around(&[]), (glam::Vec3::ZERO, 0.0));
        let broken = [at([f32::NAN, 0.0, 0.0]), at([1.0, 1.0, 1.0])];
        assert_eq!(sphere_around(&broken), (glam::Vec3::ONE, 0.0));
    }

    #[test]
    fn material_settings_match_the_shader_layout() {
        assert_eq!(std::mem::size_of::<MaterialUniform>(), 64);
        let m = MaterialUniform::new(&Material {
            alpha: AlphaMode::Mask(0.3),
            metallic: 1.0,
            roughness: 0.25,
            emissive: [1.0, 0.5, 0.0],
            ..Material::default()
        });
        assert_eq!(m.alpha[..2], [0.3, 1.0]);
        assert_eq!(m.surface, [1.0, 0.25, 1.0, 1.0]);
        assert_eq!(m.emissive, [1.0, 0.5, 0.0, 0.0]);
        assert_eq!(
            MaterialUniform::new(&Material::default()).base_color,
            [1.0; 4]
        );
    }
}
