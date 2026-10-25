//! Levels of detail (LODs): simpler versions of each mesh, drawn when a
//! copy is small on screen ([decision 0032](../../../docs/decisions/0032-lods-made-on-import-or-the-model-s-own.md)).
//!
//! A model can bring its own: a node named `Ship_LOD1` (`_LOD2` …) holds
//! the simpler meshes of the node named `Ship` (or `Ship_LOD0`), matched
//! in order. Every other mesh gets up to three made by simplifying it with
//! meshoptimizer: about a half, a quarter and an eighth of its triangles,
//! sharing its corners. A mesh that can't be simplified well keeps only
//! its full version.

use crate::model::{Lod, Mesh, Model};
use glam::{Mat4, Vec3};

/// How many triangles each level made on import aims for, as a share of
/// the full mesh's.
pub const LOD_SHARES: [f32; 3] = [0.5, 0.25, 0.125];

/// Meshes with fewer triangles than this aren't simplified: they cost
/// next to nothing already.
pub const LOD_MIN_TRIANGLES: usize = 64;

/// Meshes with more triangles than this aren't simplified on import, to
/// keep importing quick (a few seconds at most for one mesh).
pub const LOD_MAX_TRIANGLES: usize = 4_000_000;

/// The most levels a mesh keeps, its own or made.
pub const MAX_LODS: usize = 4;

/// How far a made level may stray from the full mesh, as a share of the
/// mesh's size. Past this the shape suffers too much to be worth it.
const MOST_ERROR: f32 = 0.2;

/// A level is kept only if it has at most this share of the triangles of
/// the level before; otherwise the mesh didn't simplify well, and
/// simplifying stops.
const MUST_SHRINK_TO: f32 = 0.8;

/// Adds levels of detail to every mesh of `model`: the model's own (named
/// `_LOD1` …) first, then made ones for meshes without. Warnings for the
/// model's own LODs that couldn't be used go into `warnings`.
pub fn add_lods(model: &mut Model) {
    let mut warnings = Vec::new();
    own_lods(model, &mut warnings);
    model.warnings.extend(warnings);
    for mesh in &mut model.meshes {
        if mesh.lods.is_empty() {
            mesh.lods = simplified(mesh);
        }
    }
}

/// Simpler versions of `mesh`, each sharing its corners: up to three, each
/// with roughly the share of triangles in [`LOD_SHARES`] and how far it
/// strays from the full mesh, as a share of the mesh's size.
pub fn simplified(mesh: &Mesh) -> Vec<Lod> {
    let triangles = mesh.triangles();
    if !(LOD_MIN_TRIANGLES..=LOD_MAX_TRIANGLES).contains(&triangles)
        || !mesh.indices.len().is_multiple_of(3)
        || mesh
            .indices
            .iter()
            .any(|&i| i as usize >= mesh.vertices.len())
    {
        return Vec::new();
    }
    let positions: &[u8] = bytemuck::cast_slice(&mesh.vertices);
    let Ok(adapter) =
        meshopt::VertexDataAdapter::new(positions, std::mem::size_of::<crate::model::Vertex>(), 0)
    else {
        return Vec::new();
    };
    let mut lods = Vec::new();
    let mut previous = mesh.indices.len();
    for share in LOD_SHARES {
        let target = ((mesh.indices.len() as f32 * share) as usize / 3 * 3).max(3);
        let mut error = 0.0;
        let indices = meshopt::simplify(
            &mesh.indices,
            &adapter,
            target,
            MOST_ERROR,
            meshopt::SimplifyOptions::None,
            Some(&mut error),
        );
        if indices.is_empty() || indices.len() as f32 > previous as f32 * MUST_SHRINK_TO {
            break;
        }
        previous = indices.len();
        lods.push(Lod {
            vertices: Vec::new(),
            indices,
            error,
        });
    }
    lods
}

/// The level named in a node's name (`Ship_LOD2` → `Ship`, 2), if it
/// names one.
pub fn lod_name(name: &str) -> Option<(&str, usize)> {
    // The last "_", then "LOD" in any case and one or two digits.
    let underscore = name.rfind('_')?;
    let (base, tail) = (name.get(..underscore)?, name.get(underscore + 1..)?);
    let (lod, digits) = (tail.get(..3)?, tail.get(3..)?);
    let fits = lod.eq_ignore_ascii_case("lod")
        && (1..=2).contains(&digits.len())
        && digits.bytes().all(|b| b.is_ascii_digit())
        && !base.is_empty();
    fits.then(|| digits.parse().ok().map(|level| (base, level)))
        .flatten()
}

/// Errors given to a model's own levels, which don't say how far they
/// stray: level 1 is used once the model is about a quarter of the screen
/// tall, each next one at about half that.
fn own_error(level: usize) -> f32 {
    0.004 * 2.0_f32.powi(level as i32 - 1)
}

/// Moves the model's own levels (nodes named `Name_LOD1` …) onto the
/// meshes of the node they belong to, and stops them being drawn as parts
/// of their own.
fn own_lods(model: &mut Model, warnings: &mut Vec<String>) {
    // Each LOD node, by its base name and level.
    let mut found: Vec<(usize, usize, usize)> = Vec::new();
    for (i, node) in model.nodes.iter().enumerate() {
        let Some((base, level)) = lod_name(&node.name) else {
            continue;
        };
        if level == 0 || node.meshes.is_empty() {
            continue;
        }
        let owner = model.nodes.iter().position(|n| {
            !n.meshes.is_empty()
                && (n.name == base || lod_name(&n.name).is_some_and(|(b, l)| b == base && l == 0))
        });
        match owner {
            Some(owner) if level <= MAX_LODS => found.push((owner, level, i)),
            Some(_) => warnings.push(format!(
                "{} is level {level} of detail, but at most {MAX_LODS} are used: it's drawn as a \
                 part of its own",
                node.name
            )),
            None => warnings.push(format!(
                "{} looks like a level of detail, but there's no part called {base}: it's \
                 drawn as a part of its own",
                node.name
            )),
        }
    }
    // Lowest levels first, so each mesh's list is in order.
    found.sort_by_key(|&(owner, level, _)| (owner, level));
    let mut used = Vec::new();
    for (owner, level, lod_node) in found {
        let into_owner = model.nodes[owner].world.inverse() * model.nodes[lod_node].world;
        if !into_owner.is_finite() {
            continue;
        }
        let owner_meshes = model.nodes[owner].meshes.clone();
        let lod_meshes = model.nodes[lod_node].meshes.clone();
        for (&mesh, &lod_mesh) in owner_meshes.iter().zip(&lod_meshes) {
            if mesh == lod_mesh || mesh >= model.meshes.len() || lod_mesh >= model.meshes.len() {
                continue;
            }
            // Levels skipped in the file (LOD1, then LOD3) just follow on.
            if model.meshes[mesh].lods.len() >= MAX_LODS {
                continue;
            }
            let lod = &model.meshes[lod_mesh];
            let vertices = lod.vertices.iter().map(|v| moved(v, into_owner)).collect();
            let lod = Lod {
                vertices,
                indices: lod.indices.clone(),
                error: own_error(level),
            };
            model.meshes[mesh].lods.push(lod);
        }
        used.push(lod_node);
    }
    // The LOD nodes no longer draw anything themselves; their meshes are
    // emptied when nothing else draws them.
    let mut emptied: Vec<usize> = Vec::new();
    for &node in &used {
        emptied.extend(std::mem::take(&mut model.nodes[node].meshes));
    }
    emptied.sort_unstable();
    emptied.dedup();
    for mesh in emptied {
        let still_drawn = model.nodes.iter().any(|n| n.meshes.contains(&mesh));
        if !still_drawn && let Some(m) = model.meshes.get_mut(mesh) {
            *m = Mesh::default();
        }
    }
}

/// A corner moved by `by` (its normal turned with it).
fn moved(v: &crate::model::Vertex, by: Mat4) -> crate::model::Vertex {
    let normal = by
        .transform_vector3(Vec3::from(v.normal))
        .try_normalize()
        .unwrap_or(Vec3::from(v.normal));
    crate::model::Vertex {
        position: by.transform_point3(Vec3::from(v.position)).to_array(),
        normal: normal.to_array(),
        uv: v.uv,
    }
}

#[cfg(test)]
#[path = "lods_tests.rs"]
mod tests;
