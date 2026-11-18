//! Flat outlines: a ring of points on a plane, drawn point by point, such
//! as a 2D trigger zone's freeform shape (stage 1, triggers).
//!
//! An outline may bend inward (concave), but never cross itself: [`check`]
//! says why one can't be used, in plain words. A usable one is split into
//! convex pieces once ([`convex_pieces`]): cut into triangles ear by ear,
//! then triangles that share an edge are joined again wherever the joined
//! piece stays convex (Hertel and Mehlhorn's way), so a shape of 256 points
//! makes a few dozen pieces, each tested quickly. Together the pieces
//! cover exactly the outline's inside, no more and no less.

use glam::Vec2;

/// The most points an outline may have.
pub const MAX_OUTLINE_POINTS: usize = 256;

/// The fewest points an outline may have: a triangle.
pub const MIN_OUTLINE_POINTS: usize = 3;

/// The furthest from the middle a point may be, in metres.
pub const MAX_OUTLINE_REACH: f32 = 100_000.0;

/// Why an outline can't be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlineError {
    /// Fewer than [`MIN_OUTLINE_POINTS`] points.
    TooFew,
    /// More than [`MAX_OUTLINE_POINTS`] points.
    TooMany,
    /// A point that isn't a number, or is further than
    /// [`MAX_OUTLINE_REACH`].
    OutOfRange,
    /// Two points one after the other in the same place.
    SamePlace(usize),
    /// Two edges cross or touch: the edges starting at these points.
    Crosses(usize, usize),
    /// The points all lie on one line.
    NoArea,
}

impl std::fmt::Display for OutlineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooFew => write!(f, "a shape needs at least {MIN_OUTLINE_POINTS} points"),
            Self::TooMany => write!(f, "a shape has at most {MAX_OUTLINE_POINTS} points"),
            Self::OutOfRange => write!(
                f,
                "a point is further than {} km from the zone's middle",
                MAX_OUTLINE_REACH / 1000.0
            ),
            Self::SamePlace(i) => write!(f, "points {} and {} are in the same place", i + 1, i + 2),
            Self::Crosses(..) => write!(
                f,
                "its outline would cross itself, so its inside and outside couldn't be told apart"
            ),
            Self::NoArea => write!(f, "its points all lie on one line, so it has no inside"),
        }
    }
}

/// Twice the outline's area, positive when its points run
/// counter-clockwise.
pub fn twice_area(points: &[Vec2]) -> f32 {
    let n = points.len();
    (0..n)
        .map(|i| points[i].perp_dot(points[(i + 1) % n]))
        .sum()
}

/// Whether `points` make a usable outline, or why not: three to
/// [`MAX_OUTLINE_POINTS`] points, every one a number within reach, no two
/// in a row in the same place, some area, and no edge crossing or
/// touching another (edges side by side meet only at their shared point).
pub fn check(points: &[Vec2]) -> Result<(), OutlineError> {
    let n = points.len();
    if n < MIN_OUTLINE_POINTS {
        return Err(OutlineError::TooFew);
    }
    if n > MAX_OUTLINE_POINTS {
        return Err(OutlineError::TooMany);
    }
    if points
        .iter()
        .any(|p| !p.is_finite() || p.abs().max_element() > MAX_OUTLINE_REACH)
    {
        return Err(OutlineError::OutOfRange);
    }
    if let Some(i) = (0..n).find(|&i| points[i] == points[(i + 1) % n]) {
        return Err(OutlineError::SamePlace(i));
    }
    for i in 0..n {
        let (a, b) = (points[i], points[(i + 1) % n]);
        for j in i + 1..n {
            let (c, d) = (points[j], points[(j + 1) % n]);
            let next = j == i + 1;
            let wraps = i == 0 && j == n - 1;
            let meet = if next {
                // Side by side: they share b; they're wrong only when the
                // second turns straight back along the first.
                folds_back(a, b, d)
            } else if wraps {
                folds_back(c, a, b)
            } else {
                segments_meet(a, b, c, d)
            };
            if meet {
                return Err(OutlineError::Crosses(i, j));
            }
        }
    }
    if twice_area(points).abs() <= f32::EPSILON * scale_of(points).powi(2) {
        return Err(OutlineError::NoArea);
    }
    Ok(())
}

/// The outline's size: its biggest distance from 0 along an axis (at
/// least 1), for tolerances that grow with it.
fn scale_of(points: &[Vec2]) -> f32 {
    points
        .iter()
        .map(|p| p.abs().max_element())
        .fold(1.0, f32::max)
}

/// Whether going a → b → c turns straight back: c lies on the line
/// through a and b, back toward a.
fn folds_back(a: Vec2, b: Vec2, c: Vec2) -> bool {
    let (ab, bc) = (b - a, c - b);
    ab.perp_dot(bc) == 0.0 && ab.dot(bc) < 0.0
}

/// Which side of the line a → b the point p is on: positive left, negative
/// right, zero on it. Worked out in f64, so it's exact for the f32 inputs'
/// sizes an outline has.
fn side(a: Vec2, b: Vec2, p: Vec2) -> f64 {
    let (ax, ay) = (f64::from(a.x), f64::from(a.y));
    let (bx, by) = (f64::from(b.x), f64::from(b.y));
    let (px, py) = (f64::from(p.x), f64::from(p.y));
    (bx - ax) * (py - ay) - (by - ay) * (px - ax)
}

/// Whether p, known to be on the line through a and b, lies between them.
fn within(a: Vec2, b: Vec2, p: Vec2) -> bool {
    p.x >= a.x.min(b.x) && p.x <= a.x.max(b.x) && p.y >= a.y.min(b.y) && p.y <= a.y.max(b.y)
}

/// Whether the segments a–b and c–d cross or touch.
fn segments_meet(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let (d1, d2) = (side(c, d, a), side(c, d, b));
    let (d3, d4) = (side(a, b, c), side(a, b, d));
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }
    (d1 == 0.0 && within(c, d, a))
        || (d2 == 0.0 && within(c, d, b))
        || (d3 == 0.0 && within(a, b, c))
        || (d4 == 0.0 && within(a, b, d))
}

/// Whether `p` is inside the outline (on an edge counts as inside), by
/// counting the edges a line from it to the right crosses.
pub fn contains(points: &[Vec2], p: Vec2) -> bool {
    let n = points.len();
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (points[i], points[(i + 1) % n]);
        if side(a, b, p) == 0.0 && within(a, b, p) {
            return true;
        }
        if (a.y > p.y) != (b.y > p.y) {
            let x = a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x);
            if p.x < x {
                inside = !inside;
            }
        }
    }
    inside
}

/// The outline cut into convex pieces that together cover exactly its
/// inside, each counter-clockwise. Nothing for an outline [`check`]
/// refuses. Worked out once when the shape changes, never each frame: up
/// to [`MAX_OUTLINE_POINTS`] points it takes well under a millisecond.
pub fn convex_pieces(points: &[Vec2]) -> Vec<Vec<Vec2>> {
    if check(points).is_err() {
        return Vec::new();
    }
    let mut ring: Vec<Vec2> = points.to_vec();
    if twice_area(&ring) < 0.0 {
        ring.reverse();
    }
    let triangles = ears(&ring);
    let mut pieces: Vec<Vec<usize>> = triangles.into_iter().map(|t| t.to_vec()).collect();
    join_convex(&ring, &mut pieces);
    pieces
        .into_iter()
        .map(|piece| piece.into_iter().map(|i| ring[i]).collect())
        .collect()
}

/// Cuts a counter-clockwise outline into triangles, ear by ear: each time,
/// the first corner (in order) that turns left with no other corner in or
/// on its triangle is cut off. A corner in a straight line with its
/// neighbours is dropped with no triangle.
fn ears(ring: &[Vec2]) -> Vec<[usize; 3]> {
    let mut left: Vec<usize> = (0..ring.len()).collect();
    let mut out = Vec::with_capacity(ring.len().saturating_sub(2));
    let mut i = 0;
    let mut since_cut = 0;
    while left.len() > 3 {
        let n = left.len();
        let (pa, pb, pc) = (left[(i + n - 1) % n], left[i % n], left[(i + 1) % n]);
        let (a, b, c) = (ring[pa], ring[pb], ring[pc]);
        let turn = side(a, b, c);
        let straight = turn == 0.0;
        let ear = turn > 0.0
            && left
                .iter()
                .all(|&k| k == pa || k == pb || k == pc || !in_triangle(a, b, c, ring[k]));
        if straight || ear {
            if ear {
                out.push([pa, pb, pc]);
            }
            left.remove(i % n);
            since_cut = 0;
            i %= left.len();
            continue;
        }
        i = (i + 1) % n;
        since_cut += 1;
        if since_cut > n {
            // Only for an outline `check` would refuse: never loop.
            break;
        }
    }
    if left.len() == 3 && side(ring[left[0]], ring[left[1]], ring[left[2]]) > 0.0 {
        out.push([left[0], left[1], left[2]]);
    }
    out
}

/// Whether p is in the counter-clockwise triangle a b c or on its edges.
fn in_triangle(a: Vec2, b: Vec2, c: Vec2, p: Vec2) -> bool {
    side(a, b, p) >= 0.0 && side(b, c, p) >= 0.0 && side(c, a, p) >= 0.0
}

/// Joins pieces that share an edge wherever the joined piece stays
/// convex, until no more can be: in a fixed order, so the same outline
/// always gives the same pieces.
fn join_convex(ring: &[Vec2], pieces: &mut Vec<Vec<usize>>) {
    let mut joined = true;
    while joined {
        joined = false;
        'search: for i in 0..pieces.len() {
            for j in i + 1..pieces.len() {
                if let Some(both) = joined_piece(&pieces[i], &pieces[j])
                    && is_convex(ring, &both)
                {
                    pieces[i] = both;
                    pieces.remove(j);
                    joined = true;
                    break 'search;
                }
            }
        }
    }
}

/// The two pieces joined along an edge they share (a → b in `p`, b → a in
/// `q`), or none if they share none.
fn joined_piece(p: &[usize], q: &[usize]) -> Option<Vec<usize>> {
    let (np, nq) = (p.len(), q.len());
    for s in 0..np {
        let (a, b) = (p[s], p[(s + 1) % np]);
        let Some(t) = (0..nq).find(|&t| q[t] == b && q[(t + 1) % nq] == a) else {
            continue;
        };
        // p from b round to a, then q's corners between a and b.
        let mut both: Vec<usize> = (0..np).map(|k| p[(s + 1 + k) % np]).collect();
        both.extend((2..nq).map(|k| q[(t + k) % nq]));
        return Some(both);
    }
    None
}

/// Whether the counter-clockwise piece turns left (or runs straight) at
/// every corner.
fn is_convex(ring: &[Vec2], piece: &[usize]) -> bool {
    let n = piece.len();
    (0..n).all(|k| {
        let (a, b, c) = (
            ring[piece[k]],
            ring[piece[(k + 1) % n]],
            ring[piece[(k + 2) % n]],
        );
        side(a, b, c) >= 0.0
    })
}

#[cfg(test)]
#[path = "outline_tests.rs"]
mod tests;
