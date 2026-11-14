//! Whether a mesh is still whole after some of its points moved (Tools
//! 1a, B3): a move, turn or stretch of points, edges or faces is refused
//! when it would fold a face over itself, flatten one to nothing, or
//! make the mesh pass through itself. A face left bent (its corners no
//! longer on one plane) is allowed: it is cut into triangles for drawing
//! and for being solid.
//!
//! Only the faces that touch a moved point are looked at, so the check
//! costs the same in a small brush and a big one when a few parts move.
//! Passing through is found with a grid of boxes: each touched face is
//! tried only against the faces whose boxes it reaches.

use crate::polygon::sides;
use crate::{FaceId, Mesh, VertexId};
use glam::{IVec3, Mat3, Vec3};
use std::collections::{BTreeSet, HashMap, HashSet};

/// What a change would break, with the face it would break.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Broken {
    /// A face would turn over (face the other way) or cross over itself.
    Folded(FaceId),
    /// A face would have no area, or two of its corners would meet.
    Flattened(FaceId),
    /// The first face would pass through the second.
    PassesThrough(FaceId, FaceId),
    /// The brush would be left open: an edge with a face on one side only
    /// (B4's whole check, [`crate::check_whole`]).
    Open,
    /// The brush would face inwards: neighbouring faces running opposite
    /// ways, or the whole of it inside out.
    InsideOut,
    /// Three or more faces would meet at one edge.
    Tangled,
    /// The brush would have no room inside it.
    Flat,
    /// The brush would have fewer than [`crate::whole::FEWEST_FACES`]
    /// faces.
    TooFewFaces,
}

impl std::fmt::Display for Broken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Folded(_) => "it would fold a face over itself",
            Self::Flattened(_) => "it would flatten a face to nothing",
            Self::PassesThrough(..) => "it would make the brush pass through itself",
            Self::Open => "it would leave a hole in the brush",
            Self::InsideOut => "it would turn the brush inside out",
            Self::Tangled => "it would make three faces meet at one edge",
            Self::Flat => "it would flatten the brush",
            Self::TooFewFaces => "a brush needs at least 4 faces",
        })
    }
}

/// A share of the mesh's size: two corners closer than this have met, and
/// faces must overlap by more than this to pass through each other (so
/// faces that only touch, like two steps of a stair, are fine).
pub(crate) const NEAR: f32 = 1e-5;

/// Whether `after` (the same faces as `before`, with `moved` points
/// somewhere else) is still whole, looking only at the faces that use a
/// moved point. `change` is the turn and stretch the moved points were
/// given (identity for a move): a face all of whose corners moved must
/// face the way it turned to, so turning a whole brush over is fine but
/// mirroring one inside out isn't. `Err` names what would break and
/// where.
pub fn check_moved(
    before: &Mesh,
    after: &Mesh,
    moved: &[VertexId],
    change: Mat3,
) -> Result<(), Broken> {
    let topology = after.topology();
    let touched: BTreeSet<FaceId> = moved
        .iter()
        .flat_map(|v| topology.vertex_faces(*v).iter().copied())
        .collect();
    if touched.is_empty() {
        return Ok(());
    }
    let size = after
        .bounds()
        .map_or(1.0, |b| b.size().length())
        .max(f32::MIN_POSITIVE);
    let near = size * NEAR;
    let moved: HashSet<VertexId> = moved.iter().copied().collect();
    // Normals turn by the inverse transpose (the cofactors over the
    // determinant; only its sign matters). A mirror flips which way the
    // corners run, so a mirrored face then faces against it: inside out.
    let turned = Mat3::from_cols(
        change.y_axis.cross(change.z_axis),
        change.z_axis.cross(change.x_axis),
        change.x_axis.cross(change.y_axis),
    ) * change.determinant().signum();
    for &face in &touched {
        let whole = after
            .face(face)
            .is_some_and(|c| c.iter().all(|v| moved.contains(v)));
        let was = before.face_normal(face);
        let facing = if whole { turned * was } else { was };
        check_face(after, face, facing, near)?;
    }
    passes_through(after, &touched, near)
}

/// One face, on its own: its corners apart, its area kept, facing the way
/// it should (`facing`), and not crossing over itself.
pub(crate) fn check_face(
    after: &Mesh,
    face: FaceId,
    facing: Vec3,
    near: f32,
) -> Result<(), Broken> {
    let Some(corners) = after.face(face) else {
        return Ok(());
    };
    let points: Vec<Vec3> = corners
        .iter()
        .map(|v| after.position(*v).unwrap_or(Vec3::ZERO))
        .collect();
    if sides(corners).any(|(a, b)| {
        let (p, q) = (after.position(a), after.position(b));
        p.zip(q).is_some_and(|(p, q)| p.distance(q) <= near)
    }) {
        return Err(Broken::Flattened(face));
    }
    let area = newell(&points);
    if area.length() * 0.5 <= near * near {
        return Err(Broken::Flattened(face));
    }
    let normal = area.normalize();
    if normal.dot(facing) <= 0.0 {
        return Err(Broken::Folded(face));
    }
    // Each triangle it is cut into faces its way: a bent face folded back
    // on itself has one that doesn't.
    let mut folded = false;
    crate::triangulate(
        points.len(),
        |i| points[i],
        |[a, b, c]| {
            let n = (points[b] - points[a]).cross(points[c] - points[a]);
            folded |= n.dot(normal) < -(near * near);
        },
    );
    if folded || crosses_itself(&points, normal) {
        return Err(Broken::Folded(face));
    }
    Ok(())
}

/// Newell's sum for a polygon: its normal, twice its area long.
fn newell(points: &[Vec3]) -> Vec3 {
    let mut sum = Vec3::ZERO;
    for (i, p) in points.iter().enumerate() {
        let q = points[(i + 1) % points.len()];
        sum += Vec3::new(
            (p.y - q.y) * (p.z + q.z),
            (p.z - q.z) * (p.x + q.x),
            (p.x - q.x) * (p.y + q.y),
        );
    }
    sum
}

/// Whether two sides of the polygon, not next to each other, cross, seen
/// along `normal`.
fn crosses_itself(points: &[Vec3], normal: Vec3) -> bool {
    let n = points.len();
    if n < 4 {
        return false;
    }
    let (u, v) = normal.any_orthonormal_pair();
    let flat: Vec<glam::Vec2> = points
        .iter()
        .map(|p| glam::Vec2::new(p.dot(u), p.dot(v)))
        .collect();
    for i in 0..n {
        for j in i + 2..n {
            if i == 0 && j == n - 1 {
                continue;
            }
            let (a, b) = (flat[i], flat[(i + 1) % n]);
            let (c, d) = (flat[j], flat[(j + 1) % n]);
            if segments_cross(a, b, c, d) {
                return true;
            }
        }
    }
    false
}

/// Whether two flat segments cross at a point inside both.
fn segments_cross(a: glam::Vec2, b: glam::Vec2, c: glam::Vec2, d: glam::Vec2) -> bool {
    let side = |p: glam::Vec2, q: glam::Vec2, r: glam::Vec2| (q - p).perp_dot(r - p);
    let (d1, d2) = (side(c, d, a), side(c, d, b));
    let (d3, d4) = (side(a, b, c), side(a, b, d));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

/// A face's triangles, as corner positions.
fn triangles_of(mesh: &Mesh, face: FaceId) -> Vec<[Vec3; 3]> {
    let Some(corners) = mesh.face(face) else {
        return Vec::new();
    };
    let at = |i: usize| mesh.position(corners[i]).unwrap_or(Vec3::ZERO);
    let mut out = Vec::with_capacity(corners.len().saturating_sub(2));
    crate::triangulate(corners.len(), at, |[a, b, c]| {
        out.push([at(a), at(b), at(c)])
    });
    out
}

/// The box around a face's corners.
fn face_box(mesh: &Mesh, face: FaceId) -> Option<(Vec3, Vec3)> {
    let corners = mesh.face(face)?;
    let mut points = corners.iter().filter_map(|v| mesh.position(*v));
    let first = points.next()?;
    Some(points.fold((first, first), |(lo, hi), p| (lo.min(p), hi.max(p))))
}

/// Whether any `touched` face passes through a face it doesn't share a
/// point with. Faces sharing a point meet there by design.
pub(crate) fn passes_through(
    mesh: &Mesh,
    touched: &BTreeSet<FaceId>,
    near: f32,
) -> Result<(), Broken> {
    let grid = Grid::of(mesh);
    let mut tried = HashSet::new();
    for &face in touched {
        let Some((lo, hi)) = face_box(mesh, face) else {
            continue;
        };
        let mine: HashSet<VertexId> = mesh.face(face).unwrap_or(&[]).iter().copied().collect();
        let triangles = triangles_of(mesh, face);
        let my_slab = Slab::of(mesh, face);
        for other in grid.near(lo, hi) {
            if other == face || !tried.insert(ordered(face, other)) {
                continue;
            }
            let shares = mesh
                .face(other)
                .is_some_and(|c| c.iter().any(|v| mine.contains(v)));
            if shares || apart(mesh, (face, &my_slab), other, near) {
                continue;
            }
            let theirs = triangles_of(mesh, other);
            let through = triangles
                .iter()
                .any(|t| theirs.iter().any(|s| triangles_overlap(t, s, near)));
            if through {
                return Err(Broken::PassesThrough(face, other));
            }
        }
    }
    Ok(())
}

/// How far a face's corners reach along its facing: a flat face's lie
/// on one plane (`low` = `high`), a bent face's spread a little.
struct Slab {
    normal: Vec3,
    low: f32,
    high: f32,
}

impl Slab {
    fn of(mesh: &Mesh, face: FaceId) -> Self {
        let normal = mesh.face_normal(face);
        let (low, high) = corners_along(mesh, face, normal)
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), d| {
                (lo.min(d), hi.max(d))
            });
        Self { normal, low, high }
    }

    /// Whether some of `face`'s corners are clearly above it and some
    /// clearly below: only then can one of its sides cross this slab.
    fn straddled_by(&self, mesh: &Mesh, face: FaceId, near: f32) -> bool {
        let (mut above, mut below) = (false, false);
        for d in corners_along(mesh, face, self.normal) {
            above |= d > self.high + near;
            below |= d < self.low - near;
        }
        above && below
    }

    /// Whether every corner of `face` is clearly on one side of it.
    fn clear_of(&self, mesh: &Mesh, face: FaceId, near: f32) -> bool {
        let (mut above, mut below) = (true, true);
        for d in corners_along(mesh, face, self.normal) {
            above &= d > self.high + near;
            below &= d < self.low - near;
        }
        above || below
    }

    fn is_flat(&self, near: f32) -> bool {
        self.high - self.low <= near
    }
}

/// Each corner of `face`, how far along `normal`.
fn corners_along(mesh: &Mesh, face: FaceId, normal: Vec3) -> impl Iterator<Item = f32> + '_ {
    mesh.face(face)
        .unwrap_or(&[])
        .iter()
        .filter_map(move |v| mesh.position(*v).map(|p| p.dot(normal)))
}

/// Whether `face` (with its slab) and `other` surely don't pass through
/// each other, found from their corners alone: one wholly on one side of
/// the other, or both flat and neither's corners on both sides of the
/// other's plane. Saves cutting faces into triangles that can't meet,
/// such as the many beside a big merged face on the same plane.
fn apart(mesh: &Mesh, (face, slab): (FaceId, &Slab), other: FaceId, near: f32) -> bool {
    if slab.clear_of(mesh, other, near) {
        return true;
    }
    let theirs = Slab::of(mesh, other);
    if theirs.clear_of(mesh, face, near) {
        return true;
    }
    slab.is_flat(near)
        && theirs.is_flat(near)
        && !slab.straddled_by(mesh, other, near)
        && !theirs.straddled_by(mesh, face, near)
}

fn ordered(a: FaceId, b: FaceId) -> (FaceId, FaceId) {
    if a <= b { (a, b) } else { (b, a) }
}

/// Every face's box, in cells of a grid sized from the faces: a face is
/// listed in each cell its box reaches, or, if it reaches very many, in a
/// list tried against everything.
struct Grid {
    cell: f32,
    cells: HashMap<IVec3, Vec<FaceId>>,
    big: Vec<FaceId>,
}

/// A face reaching more cells than this is tried against everything.
const MOST_CELLS: i64 = 64;

impl Grid {
    fn of(mesh: &Mesh) -> Self {
        let boxes: Vec<(FaceId, Vec3, Vec3)> = mesh
            .faces()
            .filter_map(|(f, _)| face_box(mesh, f).map(|(lo, hi)| (f, lo, hi)))
            .collect();
        let average = boxes
            .iter()
            .map(|(_, lo, hi)| (*hi - *lo).max_element())
            .sum::<f32>()
            / boxes.len().max(1) as f32;
        let cell = if average.is_finite() && average > 0.0 {
            average
        } else {
            1.0
        };
        let mut grid = Self {
            cell,
            cells: HashMap::new(),
            big: Vec::new(),
        };
        for (face, lo, hi) in boxes {
            let (a, b) = (grid.cell_of(lo), grid.cell_of(hi));
            let reach = (b - a + IVec3::ONE).as_i64vec3();
            if reach.x * reach.y * reach.z > MOST_CELLS {
                grid.big.push(face);
                continue;
            }
            for x in a.x..=b.x {
                for y in a.y..=b.y {
                    for z in a.z..=b.z {
                        grid.cells
                            .entry(IVec3::new(x, y, z))
                            .or_default()
                            .push(face);
                    }
                }
            }
        }
        grid
    }

    fn cell_of(&self, p: Vec3) -> IVec3 {
        let c = (p / self.cell).floor();
        let limit = (i32::MAX / 2) as f32;
        c.clamp(Vec3::splat(-limit), Vec3::splat(limit)).as_ivec3()
    }

    /// The faces whose boxes may reach the box from `lo` to `hi`.
    fn near(&self, lo: Vec3, hi: Vec3) -> Vec<FaceId> {
        let (a, b) = (self.cell_of(lo), self.cell_of(hi));
        let reach = (b - a + IVec3::ONE).as_i64vec3();
        let mut found: BTreeSet<FaceId> = self.big.iter().copied().collect();
        if reach.x * reach.y * reach.z > MOST_CELLS {
            found.extend(self.cells.values().flatten().copied());
        } else {
            for x in a.x..=b.x {
                for y in a.y..=b.y {
                    for z in a.z..=b.z {
                        if let Some(faces) = self.cells.get(&IVec3::new(x, y, z)) {
                            found.extend(faces.iter().copied());
                        }
                    }
                }
            }
        }
        found.into_iter().collect()
    }
}

/// Whether two triangles pass through each other by more than `near`:
/// a side of one crosses the other's plane (its ends clearly on either
/// side) at a point clearly inside it. Two triangles that cut through
/// each other always have such a side; triangles that only touch, or lie
/// flat against each other, don't.
pub(crate) fn triangles_overlap(a: &[Vec3; 3], b: &[Vec3; 3], near: f32) -> bool {
    let through =
        |t: &[Vec3; 3], s: &[Vec3; 3]| (0..3).any(|i| side_through(s[i], s[(i + 1) % 3], t, near));
    through(a, b) || through(b, a)
}

/// Whether the side from `p` to `q` passes through `triangle`, by more
/// than `near` each way.
fn side_through(p: Vec3, q: Vec3, triangle: &[Vec3; 3], near: f32) -> bool {
    let [a, b, c] = *triangle;
    let Some(normal) = (b - a).cross(c - a).try_normalize() else {
        return false;
    };
    let (dp, dq) = ((p - a).dot(normal), (q - a).dot(normal));
    let crosses = (dp > near && dq < -near) || (dp < -near && dq > near);
    if !crosses {
        return false;
    }
    let x = p + (q - p) * (dp / (dp - dq));
    [(a, b), (b, c), (c, a)]
        .into_iter()
        .all(|(from, to)| (to - from).cross(x - from).dot(normal) > near * (to - from).length())
}

#[cfg(test)]
#[path = "intact_tests.rs"]
mod tests;
