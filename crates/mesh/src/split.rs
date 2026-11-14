//! Splitting (Tools 1a, B4): cutting a face in two between two of its
//! points, putting a new point in the middle of edges, and cutting faces
//! in two across their middle.
//!
//! Every face keeps its id: a face cut in two keeps it for one half, and
//! the other half is new, like it (its material, say). A new point on an
//! edge goes into every face along that edge, so the brush stays closed.
//! Each function changes nothing and says why in plain words when the
//! cut can't be made.

use crate::builder::{Builder, CornerFrom, PointFrom};
use crate::polygon::sides;
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use glam::Vec3;
use std::collections::BTreeMap;

/// Why two points can't cut a face.
pub const NOT_ON_ONE_FACE: &str = "Pick two points on the same face";
/// Why two points next to each other can't cut a face.
pub const ALREADY_JOINED: &str = "Those two points are joined by an edge already: pick two \
                                  points across a face";

/// Cuts the face that has both `a` and `b` in two along a new edge
/// between them. The face keeps its id for the half running from `a` to
/// `b`; the other half is new, and returned with it.
pub fn split_face_between(
    mesh: &mut Mesh,
    a: VertexId,
    b: VertexId,
) -> Result<(FaceId, FaceId), &'static str> {
    if a == b {
        return Err(NOT_ON_ONE_FACE);
    }
    let shared: Vec<FaceId> = mesh
        .topology()
        .vertex_faces(a)
        .iter()
        .copied()
        .filter(|f| mesh.face(*f).is_some_and(|c| c.contains(&b)))
        .collect();
    let face = shared
        .iter()
        .copied()
        .find(|f| !next_to(mesh.face(*f).unwrap_or(&[]), a, b))
        .ok_or(if shared.is_empty() {
            NOT_ON_ONE_FACE
        } else {
            ALREADY_JOINED
        })?;
    split_face_at(mesh, face, (a, b))
}

/// Cuts `face` in two between its corners `a` and `b` (not next to each
/// other): it keeps its id for the half from `a` to `b`.
fn split_face_at(
    mesh: &mut Mesh,
    face: FaceId,
    (a, b): (VertexId, VertexId),
) -> Result<(FaceId, FaceId), &'static str> {
    let corners = mesh.face(face).unwrap_or_default().to_vec();
    let n = corners.len();
    let (Some(i), Some(j)) = (
        corners.iter().position(|v| *v == a),
        corners.iter().position(|v| *v == b),
    ) else {
        return Err(NOT_ON_ONE_FACE);
    };
    if a == b || next_to(&corners, a, b) {
        return Err(ALREADY_JOINED);
    }
    let run = |from: usize, to: usize| -> Vec<(VertexId, CornerFrom)> {
        let count = (to + n - from) % n + 1;
        (0..count)
            .map(|k| corners[(from + k) % n])
            .map(|v| (v, CornerFrom::Copy(face, v)))
            .collect()
    };
    let original = mesh.clone();
    let mut build = Builder::new(&original, mesh);
    build
        .rewrite_faces(&BTreeMap::from([(face, run(i, j))]))
        .map_err(|_| NOT_ON_ONE_FACE)?;
    let other = build.face(&run(j, i), Some(face)).ok_or(NOT_ON_ONE_FACE)?;
    Ok((face, other))
}

/// Whether `a` and `b` are corners next to each other in `corners`.
fn next_to(corners: &[VertexId], a: VertexId, b: VertexId) -> bool {
    sides(corners).any(|(p, q)| (p, q) == (a, b) || (p, q) == (b, a))
}

/// Puts a new point on each of `edges`, `t` of the way from its lower id
/// to its higher, and into every face along it. Returns the new points,
/// one for each edge the mesh has (in `edges`' order, each once).
pub fn split_edges_at(mesh: &mut Mesh, edges: &[(EdgeKey, f32)]) -> Vec<VertexId> {
    let original = mesh.clone();
    let topology = original.topology();
    let mut build = Builder::new(&original, mesh);
    let mut middles: BTreeMap<EdgeKey, VertexId> = BTreeMap::new();
    let mut made = Vec::new();
    for (edge, t) in edges {
        if middles.contains_key(edge) || !topology.has_edge(*edge) {
            continue;
        }
        let (a, b) = edge.ends();
        let (Some(p), Some(q)) = (original.position(a), original.position(b)) else {
            continue;
        };
        let t = t.clamp(0.0, 1.0);
        let middle = build.point(p.lerp(q, t), PointFrom::Between(a, b, t));
        build.copy_edge(*edge, a, middle);
        build.copy_edge(*edge, middle, b);
        build.clear_edge(*edge);
        middles.insert(*edge, middle);
        made.push(middle);
    }
    let mut changes: BTreeMap<FaceId, Vec<(VertexId, CornerFrom)>> = BTreeMap::new();
    for edge in middles.keys() {
        for face in topology.edge_faces(*edge) {
            changes.entry(face).or_insert_with(|| {
                let corners = original.face(face).unwrap_or_default();
                let mut out = Vec::with_capacity(corners.len() + 2);
                for (p, q) in sides(corners) {
                    out.push((p, CornerFrom::Copy(face, p)));
                    if let Some(m) = middles.get(&EdgeKey::new(p, q)) {
                        out.push((*m, CornerFrom::Between(face, p, face, q, 0.5)));
                    }
                }
                out
            });
        }
    }
    // Can't fail: the same faces, each with more distinct points.
    let _ = build.rewrite_faces(&changes);
    made
}

/// Puts a new point in the middle of each of `edges`. Returns the new
/// points.
pub fn split_edges(mesh: &mut Mesh, edges: &[EdgeKey]) -> Vec<VertexId> {
    let halves: Vec<(EdgeKey, f32)> = edges.iter().map(|e| (*e, 0.5)).collect();
    split_edges_at(mesh, &halves)
}

/// Why a face can't be cut across its middle.
pub const NOT_ACROSS: &str = "That face is too uneven to cut across its middle";

/// Cuts each of `faces` in two across its middle: square to its longest
/// side, through the average of its corners, so a long face becomes two
/// shorter ones. Each face keeps its id for one half; the other halves
/// are returned. Refused, changing nothing, if a face's cut doesn't cross
/// it exactly twice.
pub fn split_faces_across(mesh: &mut Mesh, faces: &[FaceId]) -> Result<Vec<FaceId>, &'static str> {
    let mut after = mesh.clone();
    let mut made = Vec::new();
    for face in faces {
        if after.face(*face).is_none() {
            continue;
        }
        made.push(split_one_across(&mut after, *face)?);
    }
    *mesh = after;
    Ok(made)
}

/// Where a face's cut crosses its border: at a corner, or along a side.
#[derive(Clone, Copy)]
enum Crossing {
    Corner(VertexId),
    Side(EdgeKey, f32),
}

/// Cuts one face across its middle.
fn split_one_across(mesh: &mut Mesh, face: FaceId) -> Result<FaceId, &'static str> {
    let corners = mesh.face(face).unwrap_or_default().to_vec();
    let points: Vec<Vec3> = corners
        .iter()
        .map(|v| mesh.position(*v).unwrap_or(Vec3::ZERO))
        .collect();
    let n = points.len();
    let longest = (0..n)
        .max_by(|i, j| {
            let length = |k: usize| points[k].distance_squared(points[(k + 1) % n]);
            length(*i).total_cmp(&length(*j))
        })
        .unwrap_or(0);
    let Some(way) = (points[(longest + 1) % n] - points[longest]).try_normalize() else {
        return Err(NOT_ACROSS);
    };
    let middle = points.iter().copied().sum::<Vec3>() / n.max(1) as f32;
    let reach = points
        .iter()
        .map(|p| p.distance(middle))
        .fold(0.0_f32, f32::max);
    let near = reach.max(f32::MIN_POSITIVE) * 1e-4;
    let side_of: Vec<f32> = points.iter().map(|p| (*p - middle).dot(way)).collect();
    let mut crossings = Vec::new();
    for k in 0..n {
        let (s, t) = (side_of[k], side_of[(k + 1) % n]);
        if s.abs() <= near {
            crossings.push(Crossing::Corner(corners[k]));
        } else if t.abs() > near && (s < 0.0) != (t < 0.0) {
            let (a, b) = (corners[k], corners[(k + 1) % n]);
            let share = s / (s - t);
            // As the edge runs from its lower id to its higher.
            let along = if a < b { share } else { 1.0 - share };
            crossings.push(Crossing::Side(EdgeKey::new(a, b), along));
        }
    }
    let [first, second] = crossings[..] else {
        return Err(NOT_ACROSS);
    };
    let mut cuts = Vec::new();
    let mut ends = [VertexId::from_raw(0); 2];
    for (end, crossing) in ends.iter_mut().zip([first, second]) {
        match crossing {
            Crossing::Corner(v) => *end = v,
            Crossing::Side(edge, t) => cuts.push((edge, t)),
        }
    }
    let made = split_edges_at(mesh, &cuts);
    let mut fresh = made.into_iter();
    for (end, crossing) in ends.iter_mut().zip([first, second]) {
        if let Crossing::Side(..) = crossing {
            *end = fresh.next().ok_or(NOT_ACROSS)?;
        }
    }
    let (_, other) = split_face_at(mesh, face, (ends[0], ends[1])).map_err(|_| NOT_ACROSS)?;
    Ok(other)
}

#[cfg(test)]
#[path = "split_tests.rs"]
mod tests;
