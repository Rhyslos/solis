//! Mending a brush read from a file (Tools 1a, B4), when that is simple
//! and safe: faces facing the wrong way are turned round, and a hole
//! whose border is one flat loop is filled with one face. Anything else
//! (three faces at an edge, a hole whose border isn't flat, faces that
//! can't all agree on a way to face) is left as it is, with the reason.
//!
//! The points never move and every face keeps its id, so a pick, the
//! file's list of points and the editor's checks stay as they were; a
//! filled hole is a new face at the end.

use crate::builder::{Builder, CornerFrom};
use crate::whole::room_inside;
use crate::{FaceId, Mesh, VertexId};
use glam::Vec3;
use std::collections::{BTreeMap, VecDeque};

/// What mending a brush did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mended {
    /// How many faces were turned round.
    pub turned: usize,
    /// How many holes were filled.
    pub filled: usize,
}

/// Why a brush is open or inside out and couldn't be mended.
pub const TANGLED: &str = "three or more of its faces meet at one edge";
/// Why a brush's faces couldn't be turned to agree.
pub const TWISTED: &str = "its faces can't all be turned to face the same way";
/// Why a hole couldn't be filled.
pub const UNEVEN_HOLE: &str = "it has a hole whose border isn't one flat loop";

/// How far off flat a hole's border may be, as a share of its size.
const FLAT_ENOUGH: f32 = 1e-3;

/// `mesh` mended, if it needed it: `Ok(None)` when it is closed and faces
/// outwards already (or has no faces), `Ok(Some(..))` with the mended
/// mesh and what was done, and `Err` with why when it can't be mended.
pub fn repair(mesh: &Mesh) -> Result<Option<(Mesh, Mended)>, &'static str> {
    if mesh.face_count() == 0 || is_whole(mesh) {
        return Ok(None);
    }
    if !mesh.topology().non_manifold_edges().is_empty() {
        return Err(TANGLED);
    }
    let mut fixed = mesh.clone();
    let mut done = Mended::default();
    let turn = agreeing(&fixed)?;
    done.turned += turn.len();
    turn_round(&mut fixed, &turn);
    done.filled = fill_holes(&mut fixed)?;
    if room_inside(&fixed) < 0.0 {
        let every: Vec<FaceId> = fixed.faces().map(|(f, _)| f).collect();
        turn_round(&mut fixed, &every);
        // The file's own faces turned are the ones the first pass left.
        done.turned = mesh.face_count() - done.turned;
    }
    if !is_whole(&fixed) {
        return Err(TWISTED);
    }
    Ok(Some((fixed, done)))
}

/// Why `mesh` isn't closed and facing outwards, or `None` when it is (or
/// has no faces): for the Problems list.
pub fn why_not_whole(mesh: &Mesh) -> Option<&'static str> {
    if mesh.face_count() == 0 || is_whole(mesh) {
        return None;
    }
    let topology = mesh.topology();
    Some(if !topology.non_manifold_edges().is_empty() {
        TANGLED
    } else if !topology.border_edges().is_empty() {
        "it has a hole"
    } else {
        "it is inside out"
    })
}

/// Closed, every neighbour running the other way along their edge, and
/// facing out.
fn is_whole(mesh: &Mesh) -> bool {
    let topology = mesh.topology();
    topology
        .edges()
        .all(|e| matches!(topology.edge_uses(e), [a, b] if a.forward != b.forward))
        && room_inside(mesh) > 0.0
}

/// The faces to turn round so every pair of neighbours agree, each piece
/// following its lowest face.
fn agreeing(mesh: &Mesh) -> Result<Vec<FaceId>, &'static str> {
    let topology = mesh.topology();
    let (starts, _) = mesh.face_layout();
    let mut turned: Vec<Option<bool>> = vec![None; starts.len().saturating_sub(1)];
    for (start, _) in mesh.faces() {
        if turned[start.get() as usize].is_some() {
            continue;
        }
        turned[start.get() as usize] = Some(false);
        let mut queue = VecDeque::from([start]);
        while let Some(face) = queue.pop_front() {
            let mine = turned[face.get() as usize].unwrap_or(false);
            for edge in topology.face_edges(face) {
                let uses = topology.edge_uses(*edge);
                let Some(me) = uses.iter().find(|u| u.face == face) else {
                    continue;
                };
                for other in uses.iter().filter(|u| u.face != face) {
                    // Agreeing neighbours run opposite ways along the edge.
                    let theirs = mine ^ (me.forward == other.forward);
                    match turned[other.face.get() as usize] {
                        None => {
                            turned[other.face.get() as usize] = Some(theirs);
                            queue.push_back(other.face);
                        }
                        Some(t) if t != theirs => return Err(TWISTED),
                        Some(_) => {}
                    }
                }
            }
        }
    }
    Ok(turned
        .iter()
        .enumerate()
        .filter(|(_, t)| **t == Some(true))
        .map(|(i, _)| FaceId::from_raw(i as u32))
        .collect())
}

/// Turns `faces` round: their corners the other way.
fn turn_round(mesh: &mut Mesh, faces: &[FaceId]) {
    for face in faces {
        let mut corners = mesh.face(*face).unwrap_or_default().to_vec();
        corners.reverse();
        // Can't fail: the same corners.
        let _ = mesh.replace_corners(*face, &corners);
    }
}

/// Fills each hole whose border is one flat loop with one face, running
/// the other way to the faces around it. Returns how many.
fn fill_holes(mesh: &mut Mesh) -> Result<usize, &'static str> {
    let topology = mesh.topology();
    // Each border side as its one face runs along it; the fill runs back.
    let mut back: BTreeMap<VertexId, (VertexId, FaceId)> = BTreeMap::new();
    for edge in topology.border_edges() {
        let [only] = topology.edge_uses(edge) else {
            continue;
        };
        let (low, high) = edge.ends();
        let (a, b) = if only.forward {
            (low, high)
        } else {
            (high, low)
        };
        if back.insert(b, (a, only.face)).is_some() {
            return Err(UNEVEN_HOLE);
        }
    }
    let mut loops = Vec::new();
    while let Some((&start, _)) = back.first_key_value() {
        let mut ring = Vec::new();
        let mut at = start;
        while let Some((to, face)) = back.remove(&at) {
            ring.push((at, face));
            at = to;
        }
        if at != start || ring.len() < 3 || !flat(mesh, &ring) {
            return Err(UNEVEN_HOLE);
        }
        loops.push(ring);
    }
    let original = mesh.clone();
    let mut build = Builder::new(&original, mesh);
    for ring in &loops {
        let corners: Vec<(VertexId, CornerFrom)> = ring
            .iter()
            .map(|(v, face)| (*v, CornerFrom::Copy(*face, *v)))
            .collect();
        build
            .face(&corners, ring.first().map(|(_, f)| *f))
            .ok_or(UNEVEN_HOLE)?;
    }
    Ok(loops.len())
}

/// Whether the points of `ring` lie on one plane.
fn flat(mesh: &Mesh, ring: &[(VertexId, FaceId)]) -> bool {
    let points: Vec<Vec3> = ring.iter().filter_map(|(v, _)| mesh.position(*v)).collect();
    let middle = points.iter().copied().sum::<Vec3>() / points.len().max(1) as f32;
    let mut normal = Vec3::ZERO;
    for (p, q) in points.iter().zip(points.iter().cycle().skip(1)) {
        normal += (*p - middle).cross(*q - middle);
    }
    let Some(normal) = normal.try_normalize() else {
        return false;
    };
    let reach = points
        .iter()
        .map(|p| p.distance(middle))
        .fold(0.0_f32, f32::max);
    points
        .iter()
        .all(|p| (*p - middle).dot(normal).abs() <= reach * FLAT_ENOUGH)
}

#[cfg(test)]
#[path = "repair_tests.rs"]
mod tests;
