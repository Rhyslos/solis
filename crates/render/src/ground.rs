//! The ground's chunks on the GPU (Tools 1b, G1): drawn as models, one
//! per chunk, so each is culled on its own and shadowed and lit as models
//! are, but sharing what models each make for themselves:
//!
//! - **One set of materials** for every chunk ([`GroundMaterials`]): a
//!   ground kind's colour, made once per palette, never per chunk.
//! - **A pool of buffers** ([`BufferPool`]): a chunk's corners and
//!   triangles go into a buffer the pool already has, of the next size up
//!   in powers of two, written in place; a chunk made again gives its
//!   buffers back ([`BufferPool::recycle`]). So chunks arriving while the
//!   camera moves don't each ask the card for new memory.

use crate::gpu_model::{GpuMaterial, GpuMesh, GpuModel, assemble};
use crate::renderer::ViewportRenderer;
use assets::{Material, Model};
use std::collections::HashMap;

/// The smallest buffer the pool hands out, in bytes.
const SMALLEST: u64 = 4096;

/// A ground's materials on the GPU: one per ground kind, shared by every
/// chunk.
#[derive(Debug, Clone)]
pub struct GroundMaterials {
    materials: Vec<GpuMaterial>,
}

impl GroundMaterials {
    /// How many kinds it has.
    pub fn len(&self) -> usize {
        self.materials.len()
    }

    /// Whether it has none.
    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }
}

/// Buffers given back by chunks, by kind (corners or triangles) and size.
#[derive(Debug, Default)]
pub struct BufferPool {
    free: HashMap<(bool, u64), Vec<wgpu::Buffer>>,
    made: usize,
    reused: usize,
}

impl BufferPool {
    /// A buffer of at least `bytes` for corners (`index` false) or
    /// triangles, from the pool when it has one, holding `contents`.
    fn take(
        &mut self,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        index: bool,
        contents: &[u8],
    ) -> wgpu::Buffer {
        let size = (contents.len() as u64).max(SMALLEST).next_power_of_two();
        let buffer = match self.free.get_mut(&(index, size)).and_then(Vec::pop) {
            Some(buffer) => {
                self.reused += 1;
                buffer
            }
            None => {
                self.made += 1;
                let usage = if index {
                    wgpu::BufferUsages::INDEX
                } else {
                    wgpu::BufferUsages::VERTEX
                };
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(if index {
                        "ground triangles"
                    } else {
                        "ground corners"
                    }),
                    size,
                    usage: usage | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            }
        };
        queue.write_buffer(&buffer, 0, contents);
        buffer
    }

    /// Gives a chunk's buffers back, for the next chunk to use.
    pub fn recycle(&mut self, model: GpuModel) {
        for mesh in model.into_meshes().into_iter().flatten() {
            let (vertices, indices) = mesh.into_buffers();
            for (index, buffer) in [(false, vertices), (true, indices)] {
                self.free
                    .entry((index, buffer.size()))
                    .or_default()
                    .push(buffer);
            }
        }
    }

    /// How many buffers it has made, and how many times it handed one
    /// back out instead.
    pub fn counts(&self) -> (usize, usize) {
        (self.made, self.reused)
    }
}

impl ViewportRenderer {
    /// A ground's materials: one plain one per colour (its kinds'), shared
    /// by every chunk.
    pub fn upload_ground_materials(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        colours: &[[u8; 3]],
    ) -> GroundMaterials {
        let model = Model {
            name: String::new(),
            meshes: Vec::new(),
            nodes: Vec::new(),
            textures: Vec::new(),
            lamps: Vec::new(),
            bounds: assets::Bounds {
                min: glam::Vec3::ZERO,
                max: glam::Vec3::ZERO,
            },
            warnings: Vec::new(),
            materials: colours
                .iter()
                .map(|c| Material {
                    base_color: [
                        srgb_to_linear(c[0]),
                        srgb_to_linear(c[1]),
                        srgb_to_linear(c[2]),
                        1.0,
                    ],
                    roughness: 0.95,
                    metallic: 0.0,
                    ..Material::default()
                })
                .collect(),
        };
        let mut materials = self.models.upload_materials(device, queue, &model);
        // The plain default the uploader adds last isn't a kind.
        materials.pop();
        for m in &mut materials {
            m.outlined = false;
        }
        GroundMaterials { materials }
    }

    /// One chunk, its meshes' corners and triangles in buffers from `pool`,
    /// its materials `materials`' (a mesh's material is its kind).
    pub fn upload_ground_chunk(
        &self,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        pool: &mut BufferPool,
        materials: &GroundMaterials,
        model: &Model,
    ) -> GpuModel {
        let last = materials.materials.len().saturating_sub(1);
        let meshes = model
            .meshes
            .iter()
            .map(|mesh| {
                if mesh.indices.is_empty() || mesh.vertices.is_empty() {
                    return None;
                }
                let vertices =
                    pool.take((device, queue), false, bytemuck::cast_slice(&mesh.vertices));
                let indices = pool.take((device, queue), true, bytemuck::cast_slice(&mesh.indices));
                Some(GpuMesh::pooled(
                    vertices,
                    indices,
                    mesh.indices.len() as u32,
                    mesh.material.unwrap_or(0).min(last),
                ))
            })
            .collect();
        assemble(model, meshes, materials.materials.clone())
    }
}

fn srgb_to_linear(c: u8) -> f32 {
    let c = f32::from(c) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}
