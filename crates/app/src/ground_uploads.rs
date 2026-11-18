//! The ground's chunks on the GPU (Tools 1b, G1): each chunk the editor
//! made on a worker uploaded as it arrives, into buffers from one pool,
//! with one set of materials (its kinds' colours) shared by every chunk;
//! drawn in every view that shows the ground, each chunk culled on its
//! own by the renderer.

use editor::GroundMeshes;
use eframe::wgpu;
use glam::{IVec3, Mat4};
use render::{BufferPool, GpuModel, GroundMaterials, Placement};
use std::collections::BTreeMap;

/// The ground's chunks uploaded.
#[derive(Default)]
pub(crate) struct GroundUploads {
    revision: u64,
    materials: Option<GroundMaterials>,
    pool: BufferPool,
    chunks: BTreeMap<[i32; 3], (GpuModel, Mat4)>,
}

impl GroundUploads {
    /// Takes in what the editor made since last asked: a new ground lets
    /// every chunk go (their buffers back to the pool); each fresh chunk
    /// replaces the one it was.
    pub(crate) fn update(
        &mut self,
        renderer: &render::ViewportRenderer,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        meshes: GroundMeshes,
    ) {
        if meshes.revision != self.revision {
            self.revision = meshes.revision;
            for (_, (model, _)) in std::mem::take(&mut self.chunks) {
                self.pool.recycle(model);
            }
            self.materials = (!meshes.colours.is_empty())
                .then(|| renderer.upload_ground_materials(device, queue, &meshes.colours));
        }
        let Some(materials) = &self.materials else {
            return;
        };
        for mesh in meshes.fresh {
            if let Some((old, _)) = self.chunks.remove(&mesh.key.to_array()) {
                self.pool.recycle(old);
            }
            if mesh.is_empty() {
                continue;
            }
            let model = chunk_model(&mesh, materials.len());
            let gpu = renderer.upload_ground_chunk((device, queue), &mut self.pool, materials, &model);
            let at = Mat4::from_translation(mesh.origin.as_vec3());
            self.chunks.insert(mesh.key.to_array(), (gpu, at));
        }
    }

    /// Every chunk as a placement (not picked).
    pub(crate) fn placements(&self) -> impl Iterator<Item = Placement<'_>> + '_ {
        self.chunks.values().map(|(model, transform)| Placement {
            model,
            node: None,
            transform: *transform,
            pick: 0,
        })
    }

    /// How many chunks are uploaded.
    pub(crate) fn len(&self) -> usize {
        self.chunks.len()
    }
}

/// A chunk's mesh as a model: one mesh per ground kind it has (its
/// material), in its own space (the chunk's origin is its placement).
pub(crate) fn chunk_model(mesh: &ground::ChunkMesh, kinds: usize) -> assets::Model {
    let mut by_kind: BTreeMap<u8, (Vec<assets::Vertex>, Vec<u32>, BTreeMap<u32, u32>)> =
        BTreeMap::new();
    for t in mesh.indices.chunks_exact(3) {
        let kind = mesh.kinds.get(t[0] as usize).copied().unwrap_or(0);
        let kind = if usize::from(kind) < kinds { kind } else { 0 };
        let (vertices, indices, seen) = by_kind.entry(kind).or_default();
        for &i in t {
            let at = *seen.entry(i).or_insert_with(|| {
                let p = mesh.positions[i as usize];
                vertices.push(assets::Vertex {
                    position: p,
                    normal: mesh.normals[i as usize],
                    // Laid on from above, a metre a repeat (pictures per
                    // kind come with painting, G4).
                    uv: [p[0], p[1]],
                });
                vertices.len() as u32 - 1
            });
            indices.push(at);
        }
    }
    let meshes: Vec<assets::Mesh> = by_kind
        .into_iter()
        .map(|(kind, (vertices, indices, _))| assets::Mesh {
            vertices,
            indices,
            material: Some(usize::from(kind)),
            lods: Vec::new(),
        })
        .collect();
    let (mut min, mut max) = (glam::Vec3::splat(f32::MAX), glam::Vec3::splat(f32::MIN));
    for p in &mesh.positions {
        min = min.min(glam::Vec3::from_array(*p));
        max = max.max(glam::Vec3::from_array(*p));
    }
    assets::Model {
        name: format!("Ground chunk {}", IVec3::from(mesh.key)),
        nodes: vec![assets::Node {
            name: String::new(),
            parent: None,
            local: Mat4::IDENTITY,
            world: Mat4::IDENTITY,
            meshes: (0..meshes.len()).collect(),
        }],
        meshes,
        materials: Vec::new(),
        textures: Vec::new(),
        lamps: Vec::new(),
        bounds: assets::Bounds { min, max },
        warnings: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chunk_becomes_a_model_of_its_triangles_in_its_own_space() {
        let g = ground::Start::Flat { height: 40.0 }.ground(glam::UVec3::new(1, 1, 2));
        let mut s = ground::Samples::default();
        assert!(g.gather(IVec3::new(0, 0, 1), &mut s).known().is_some());
        let mesh = ground::smooth::mesh(&s);
        let model = chunk_model(&mesh, 4);
        assert_eq!(model.meshes.len(), 1);
        assert_eq!(model.meshes[0].indices.len(), mesh.indices.len());
        assert_eq!(model.meshes[0].material, Some(0));
        assert!((model.bounds.min.z - 8.0).abs() < 1e-3);
        assert_eq!(model.placements().count(), 1);
    }
}
