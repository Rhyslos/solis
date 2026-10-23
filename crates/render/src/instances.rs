//! Instances: where each copy of a mesh is drawn. Every frame the
//! placements of every model are gathered into one buffer, grouped so each
//! mesh is drawn once for all its copies (decision 0021). Mirrored copies
//! (a negative scale) are grouped apart: their triangles wind the other
//! way, so they're drawn with pipelines that treat the other side as the
//! front.

use crate::gpu_model::GpuModel;
use bytemuck::{Pod, Zeroable};
use glam::{Mat3, Mat4, Vec3};
use std::ops::Range;

/// One instance exactly as the shader's `Instance` inputs expect it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct InstanceRaw {
    /// The placement, column by column.
    model: [[f32; 4]; 4],
    /// The normals' matrix (inverse transpose), column by column.
    normal: [[f32; 3]; 3],
}

impl InstanceRaw {
    pub(crate) fn new(model: Mat4) -> Self {
        let turn = Mat3::from_mat4(model);
        let normal = if turn.determinant().abs() > 1e-12 {
            turn.inverse().transpose()
        } else {
            // Squashed flat: keep normals as they are rather than break.
            Mat3::IDENTITY
        };
        Self {
            model: model.to_cols_array_2d(),
            normal: normal.to_cols_array_2d(),
        }
    }
}

/// How an instance is laid out in the instance buffer (vertex buffer 1).
const ATTRIBUTES: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
    3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4,
    7 => Float32x3, 8 => Float32x3, 9 => Float32x3,
];

pub(crate) fn layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &ATTRIBUTES,
    }
}

/// One mesh of one model, drawn for a run of instances.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Batch {
    /// Which placed model (index into the frame's placements' models).
    pub(crate) model: usize,
    /// Which of its meshes.
    pub(crate) mesh: usize,
    /// Whether these copies are mirrored.
    pub(crate) mirrored: bool,
    /// Their instances in the buffer.
    pub(crate) instances: Range<u32>,
}

/// A model placed in the world: one copy of it, all its meshes.
#[derive(Debug, Clone, Copy)]
pub struct Placement<'a> {
    /// The model.
    pub model: &'a GpuModel,
    /// Where it is.
    pub transform: Mat4,
}

/// Every mesh copy of `placements`, grouped into batches, and for each
/// batch model number the placement that holds that model. Models are told
/// apart by address, so the same model placed twice shares its batches.
pub(crate) fn gather(placements: &[Placement<'_>]) -> (Vec<InstanceRaw>, Vec<Batch>, Vec<usize>) {
    // Each distinct model once, in the order first seen, by the placement
    // that first showed it.
    let mut models: Vec<usize> = Vec::new();
    let mut seen: std::collections::HashMap<*const GpuModel, usize> = Default::default();
    let mut entries: Vec<(usize, usize, bool, Mat4)> = Vec::new();
    for (i, placement) in placements.iter().enumerate() {
        if !placement.transform.is_finite() {
            continue;
        }
        let model = *seen
            .entry(std::ptr::from_ref(placement.model))
            .or_insert_with(|| {
                models.push(i);
                models.len() - 1
            });
        for (mesh, local) in placement.model.placements() {
            let world = placement.transform * *local;
            let mirrored = Mat3::from_mat4(world).determinant() < 0.0;
            entries.push((model, *mesh, mirrored, world));
        }
    }
    entries.sort_by_key(|&(model, mesh, mirrored, _)| (model, mesh, mirrored));
    let mut raw = Vec::with_capacity(entries.len());
    let mut batches: Vec<Batch> = Vec::new();
    for (model, mesh, mirrored, world) in entries {
        let index = raw.len() as u32;
        raw.push(InstanceRaw::new(world));
        match batches.last_mut() {
            Some(b) if b.model == model && b.mesh == mesh && b.mirrored == mirrored => {
                b.instances.end = index + 1;
            }
            _ => batches.push(Batch {
                model,
                mesh,
                mirrored,
                instances: index..index + 1,
            }),
        }
    }
    (raw, batches, models)
}

/// The box around every placement, in the world, or `None` if there's
/// nothing.
pub(crate) fn world_bounds(placements: &[Placement<'_>]) -> Option<(Vec3, Vec3)> {
    placements
        .iter()
        .filter(|p| p.transform.is_finite())
        .filter_map(|p| {
            let (min, max) = p.model.bounds();
            if !(min.is_finite() && max.is_finite()) {
                return None;
            }
            let corners = (0..8).map(|i| {
                Vec3::new(
                    if i & 1 == 0 { min.x } else { max.x },
                    if i & 2 == 0 { min.y } else { max.y },
                    if i & 4 == 0 { min.z } else { max.z },
                )
            });
            corners.map(|c| p.transform.transform_point3(c)).fold(
                None,
                |acc: Option<(Vec3, Vec3)>, c| {
                    Some(acc.map_or((c, c), |(lo, hi)| (lo.min(c), hi.max(c))))
                },
            )
        })
        .reduce(|(a_min, a_max), (b_min, b_max)| (a_min.min(b_min), a_max.max(b_max)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instances_match_the_shader_layout() {
        // 4 columns of 4 floats, then 3 of 3: 25 floats.
        assert_eq!(std::mem::size_of::<InstanceRaw>(), 25 * 4);
        let last = ATTRIBUTES.last().unwrap();
        assert_eq!(last.offset + 12, std::mem::size_of::<InstanceRaw>() as u64);
    }

    #[test]
    fn normals_follow_stretching_and_mirroring() {
        let stretched = InstanceRaw::new(Mat4::from_scale(Vec3::new(2.0, 1.0, 1.0)));
        // A slanted surface's normal leans away from the stretch.
        let n = Mat3::from_cols_array_2d(&stretched.normal) * Vec3::new(1.0, 1.0, 0.0);
        assert!(n.x < n.y);
        let flat = InstanceRaw::new(Mat4::from_scale(Vec3::new(1.0, 1.0, 0.0)));
        assert_eq!(Mat3::from_cols_array_2d(&flat.normal), Mat3::IDENTITY);
    }
}
