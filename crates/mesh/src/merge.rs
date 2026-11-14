//! Deleting parts of a closed brush and closing the gap (Tools 1a, B4):
//! faces that touch merge into one, an edge goes and the faces on either
//! side merge, a point goes and the faces around it merge.
//!
//! Merging works on groups of faces: each group's outer border becomes
//! one face, which keeps the group's lowest id (and so its material); the
//! rest of the group goes, and so do points nothing uses any more. A
//! point left in the middle of a straight side, with only the merged face
//! and one other using it, goes too, from both. Nothing is changed, and
//! the reason is given in plain words, when a group's border isn't one
//! simple loop.

use crate::builder::{Builder, CornerFrom};
use crate::polygon::sides;
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Why a lone face can't be deleted.
pub const LONE_FACE: &str = "Deleting a face on its own would leave a hole: pick two or more \
                             faces that touch, to merge them into one";
/// Why a group whose border isn't one loop can't become one face.
pub const NOT_ONE_LOOP: &str = "Those faces go all the way round the brush, so they can't \
                                become one face";
/// Why a group with no border can't become one face.
pub const EVERYTHING: &str = "That would delete the whole brush";
/// Why a point stays.
pub const POINT_STAYS: &str = "That point can't go: the faces around it don't close round it";

/// Merges each group of touching `faces` (sharing an edge) into one face.
/// A face touching no other picked face is refused: deleting it alone
/// would leave a hole. Returns the merged faces.
pub fn merge_faces(mesh: &mut Mesh, faces: &[FaceId]) -> Result<Vec<FaceId>, &'static str> {
    let chosen: BTreeSet<FaceId> = faces
        .iter()
        .copied()
        .filter(|f| mesh.face(*f).is_some())
        .collect();
    let topology = mesh.topology();
    let mut groups = Groups::default();
    for face in &chosen {
        groups.add(*face);
        for edge in topology.face_edges(*face) {
            for other in topology.edge_faces(*edge) {
                if chosen.contains(&other) {
                    groups.join(*face, other);
                }
            }
        }
    }
    let groups = groups.finish();
    if groups.iter().any(|g| g.len() < 2) {
        return Err(LONE_FACE);
    }
    merge_groups(mesh, &groups, &[])
}

/// Deletes `edges`: the two faces on either side of each merge (several
/// edges in a row merge several faces into one). Returns the merged
/// faces.
pub fn dissolve_edges(mesh: &mut Mesh, edges: &[EdgeKey]) -> Result<Vec<FaceId>, &'static str> {
    let topology = mesh.topology();
    let mut groups = Groups::default();
    for edge in edges {
        let faces: Vec<FaceId> = topology.edge_faces(*edge).collect();
        for pair in faces.windows(2) {
            groups.add(pair[0]);
            groups.join(pair[0], pair[1]);
        }
    }
    let groups = groups.finish();
    merge_groups(mesh, &groups, &[])
}

/// Deletes `points`: the faces around each merge into one, without it.
/// Returns the merged faces.
pub fn dissolve_points(mesh: &mut Mesh, points: &[VertexId]) -> Result<Vec<FaceId>, &'static str> {
    let topology = mesh.topology();
    let mut groups = Groups::default();
    let mut gone = Vec::new();
    for point in points {
        let around = topology.vertex_faces(*point);
        let Some(first) = around.first() else {
            continue;
        };
        gone.push(*point);
        groups.add(*first);
        for face in &around[1..] {
            groups.join(*first, *face);
        }
    }
    let groups = groups.finish();
    merge_groups(mesh, &groups, &gone)
}

/// Faces joined into groups, each face once.
#[derive(Default)]
struct Groups {
    parent: HashMap<FaceId, FaceId>,
}

impl Groups {
    fn add(&mut self, face: FaceId) {
        self.parent.entry(face).or_insert(face);
    }

    fn root(&mut self, face: FaceId) -> FaceId {
        let mut at = face;
        while let Some(up) = self.parent.get(&at).copied().filter(|up| *up != at) {
            at = up;
        }
        // Shorten the path for next time.
        let mut walk = face;
        while walk != at {
            let up = self.parent.insert(walk, at).unwrap_or(at);
            walk = up;
        }
        at
    }

    fn join(&mut self, a: FaceId, b: FaceId) {
        self.add(a);
        self.add(b);
        let (ra, rb) = (self.root(a), self.root(b));
        if ra != rb {
            let (low, high) = if ra < rb { (ra, rb) } else { (rb, ra) };
            self.parent.insert(high, low);
        }
    }

    /// The groups, each sorted, in order of their lowest face.
    fn finish(mut self) -> Vec<Vec<FaceId>> {
        let faces: Vec<FaceId> = self.parent.keys().copied().collect();
        let mut by_root: BTreeMap<FaceId, Vec<FaceId>> = BTreeMap::new();
        for face in faces {
            let root = self.root(face);
            by_root.entry(root).or_default().push(face);
        }
        by_root
            .into_values()
            .map(|mut g| {
                g.sort_unstable();
                g
            })
            .collect()
    }
}

/// The border of a group of faces as one loop of corners, each with the
/// face whose side starts there.
fn border_loop(mesh: &Mesh, group: &[FaceId]) -> Result<Vec<(VertexId, FaceId)>, &'static str> {
    let mut uses: HashMap<EdgeKey, usize> = HashMap::new();
    for face in group {
        for (a, b) in sides(mesh.face(*face).unwrap_or_default()) {
            *uses.entry(EdgeKey::new(a, b)).or_default() += 1;
        }
    }
    let mut next: BTreeMap<VertexId, (VertexId, FaceId)> = BTreeMap::new();
    for face in group {
        for (a, b) in sides(mesh.face(*face).unwrap_or_default()) {
            if uses.get(&EdgeKey::new(a, b)) == Some(&1) && next.insert(a, (b, *face)).is_some() {
                return Err(NOT_ONE_LOOP);
            }
        }
    }
    let Some((&start, _)) = next.first_key_value() else {
        return Err(EVERYTHING);
    };
    let mut ring = Vec::with_capacity(next.len());
    let mut at = start;
    while let Some(&(to, face)) = next.get(&at) {
        ring.push((at, face));
        at = to;
        if at == start || ring.len() > next.len() {
            break;
        }
    }
    if at != start || ring.len() != next.len() {
        return Err(NOT_ONE_LOOP);
    }
    Ok(ring)
}

/// Merges each group of faces into one face along its border; `gone` are
/// points that must not be left in use.
fn merge_groups(
    mesh: &mut Mesh,
    groups: &[Vec<FaceId>],
    gone: &[VertexId],
) -> Result<Vec<FaceId>, &'static str> {
    let mut rewrites: BTreeMap<FaceId, Vec<(VertexId, CornerFrom)>> = BTreeMap::new();
    let mut deleted = Vec::new();
    let mut loose = Vec::new();
    let mut kept = Vec::new();
    for group in groups.iter().filter(|g| g.len() >= 2) {
        let ring = border_loop(mesh, group)?;
        let keep = group[0];
        rewrites.insert(
            keep,
            ring.iter()
                .map(|(v, face)| (*v, CornerFrom::Copy(*face, *v)))
                .collect(),
        );
        deleted.extend_from_slice(&group[1..]);
        for face in group {
            loose.extend_from_slice(mesh.face(*face).unwrap_or_default());
        }
        kept.push(keep);
    }
    if kept.is_empty() {
        return Ok(kept);
    }
    let original = mesh.clone();
    let mut after = original.clone();
    Builder::new(&original, &mut after)
        .rewrite_faces(&rewrites)
        .map_err(|_| NOT_ONE_LOOP)?;
    after.delete_faces(&deleted);
    after.remove_unused_vertices(&loose);
    straighten(&mut after, &kept);
    if gone.iter().any(|v| after.position(*v).is_some()) {
        return Err(POINT_STAYS);
    }
    after.forget_unused_edge_values();
    *mesh = after;
    Ok(kept)
}

/// Takes out points left in the middle of a straight side of a merged
/// face (`faces`) that only it and one other face use, from both.
fn straighten(mesh: &mut Mesh, faces: &[FaceId]) {
    let topology = mesh.topology().clone();
    let mut drop: BTreeSet<VertexId> = BTreeSet::new();
    for face in faces {
        let corners = mesh.face(*face).unwrap_or_default();
        let n = corners.len();
        for i in 0..n {
            let (prev, at, next) = (corners[(i + n - 1) % n], corners[i], corners[(i + 1) % n]);
            if topology.vertex_faces(at).len() == 2 && straight(mesh, prev, at, next) {
                drop.insert(at);
            }
        }
    }
    let mut changes: BTreeMap<FaceId, Vec<VertexId>> = BTreeMap::new();
    for v in &drop {
        for face in topology.vertex_faces(*v) {
            let corners = changes
                .entry(*face)
                .or_insert_with(|| mesh.face(*face).unwrap_or_default().to_vec());
            if corners.len() > 3 {
                corners.retain(|c| c != v);
            }
        }
    }
    let used: BTreeSet<VertexId> = changes.values().flatten().copied().collect();
    // A point some face kept (it had only three corners) stays in all.
    let stays: BTreeSet<VertexId> = drop.iter().copied().filter(|v| used.contains(v)).collect();
    if !stays.is_empty() {
        for (face, corners) in &mut changes {
            *corners = mesh
                .face(*face)
                .unwrap_or_default()
                .iter()
                .copied()
                .filter(|c| !drop.contains(c) || stays.contains(c))
                .collect();
        }
    }
    if mesh.rewrite_faces(&changes).is_ok() {
        let gone: Vec<VertexId> = drop.into_iter().collect();
        mesh.remove_unused_vertices(&gone);
    }
}

/// Whether `at` lies on the straight line from `prev` to `next`, between
/// them.
fn straight(mesh: &Mesh, prev: VertexId, at: VertexId, next: VertexId) -> bool {
    let (Some(p), Some(a), Some(n)) = (mesh.position(prev), mesh.position(at), mesh.position(next))
    else {
        return false;
    };
    let (Some(into), Some(out)) = ((a - p).try_normalize(), (n - a).try_normalize()) else {
        return false;
    };
    into.dot(out) > 1.0 - 1e-5
}

#[cfg(test)]
#[path = "merge_tests.rs"]
mod tests;
