//! Where several planes come closest to meeting: the point dual
//! contouring puts in a cube, from the places and facings where the
//! surface crosses its edges (a small least-squares problem).

use glam::{Mat3, Vec3};

/// How strongly the answer is pulled towards the planes' average place,
/// so planes that are nearly parallel still give a sensible point.
const PULL: f32 = 0.05;

/// The point nearest every plane through `points[i]` facing
/// `normals[i]`, pulled a little towards `middle`.
pub(crate) fn solve(points: &[Vec3], normals: &[Vec3], middle: Vec3) -> Vec3 {
    let mut ata = Mat3::from_diagonal(Vec3::splat(PULL));
    let mut atb = middle * PULL;
    for (p, n) in points.iter().zip(normals) {
        ata += Mat3::from_cols(*n * n.x, *n * n.y, *n * n.z);
        atb += *n * n.dot(*p);
    }
    let det = ata.determinant();
    if det.abs() < 1.0e-9 || !det.is_finite() {
        return middle;
    }
    let x = ata.inverse() * atb;
    if x.is_finite() { x } else { middle }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_planes_meet_at_their_corner() {
        let points = [
            Vec3::new(1.0, 0.3, 0.2),
            Vec3::new(0.1, 1.0, 0.7),
            Vec3::new(0.4, 0.5, 1.0),
        ];
        let normals = [Vec3::X, Vec3::Y, Vec3::Z];
        let x = solve(&points, &normals, Vec3::splat(0.5));
        assert!((x - Vec3::ONE).length() < 0.1, "{x}");
    }

    #[test]
    fn one_plane_keeps_the_middle_on_it() {
        let x = solve(
            &[Vec3::new(0.2, 0.5, 0.5)],
            &[Vec3::X],
            Vec3::new(0.2, 0.5, 0.5),
        );
        assert!((x.x - 0.2).abs() < 0.01 && (x.y - 0.5).abs() < 0.01, "{x}");
    }
}
