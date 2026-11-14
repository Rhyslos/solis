//! Whether a brush is still whole after an operation that adds, removes
//! or rewires faces (Tools 1a, B4): an extrude, an inset, a split or a
//! delete. A brush is whole when it is closed (every edge has a face on
//! each side), faces outwards (neighbours run opposite ways along the
//! edge they share, and the room inside is on the inside), has room
//! inside it at all, keeps at least four faces, and doesn't pass through
//! itself. A face left bent is fine, as for moves ([`crate::intact`]).
//!
//! Only the faces around what changed are tried edge by edge and against
//! the rest for passing through; the room inside is summed over every
//! face, which costs a few additions a face.

use crate::intact::{Broken, NEAR, check_face, passes_through};
use crate::{FaceId, Mesh, VertexId};
use glam::Vec3;
use std::collections::BTreeSet;

/// The fewest faces a brush keeps (a pyramid of three sides and a base).
pub const FEWEST_FACES: usize = 4;

/// A share of the box around the brush, cubed: less room inside than
/// this is flat.
const FLAT_SHARE: f32 = 1e-9;

/// Whether `after` is whole, looking closely at the faces that use any of
/// the points `around` (the points of the faces an operation made or
/// changed, and of any it removed). `Err` names what's wrong.
pub fn check_whole(after: &Mesh, around: &[VertexId]) -> Result<(), Broken> {
    if after.face_count() < FEWEST_FACES {
        return Err(Broken::TooFewFaces);
    }
    let topology = after.topology();
    let touched: BTreeSet<FaceId> = around
        .iter()
        .flat_map(|v| topology.vertex_faces(*v).iter().copied())
        .collect();
    for face in &touched {
        for edge in topology.face_edges(*face) {
            match topology.edge_uses(*edge) {
                [_] => return Err(Broken::Open),
                [a, b] if a.forward == b.forward => return Err(Broken::InsideOut),
                [_, _] => {}
                _ => return Err(Broken::Tangled),
            }
        }
    }
    let size = after
        .bounds()
        .map_or(1.0, |b| b.size().length())
        .max(f32::MIN_POSITIVE);
    let near = size * NEAR;
    for face in &touched {
        check_face(after, *face, after.face_normal(*face), near)?;
    }
    let room = room_inside(after);
    if !room.is_finite() || room.abs() <= size * size * size * FLAT_SHARE {
        return Err(Broken::Flat);
    }
    if room < 0.0 {
        return Err(Broken::InsideOut);
    }
    passes_through(after, &touched, near)
}

/// The room inside a closed mesh whose faces run counter-clockwise seen
/// from outside: positive when it faces out, negative inside out. Each
/// face is fanned from its first corner; any cut of a bent face gives the
/// same sign.
pub fn room_inside(mesh: &Mesh) -> f32 {
    let mut sum = 0.0_f64;
    for (_, corners) in mesh.faces() {
        let at = |i: usize| mesh.position(corners[i]).unwrap_or(Vec3::ZERO);
        let first = at(0);
        for i in 1..corners.len().saturating_sub(1) {
            sum += f64::from(first.dot(at(i).cross(at(i + 1))));
        }
    }
    (sum / 6.0) as f32
}

#[cfg(test)]
#[path = "whole_tests.rs"]
mod tests;
