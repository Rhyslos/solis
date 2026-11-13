//! A mesh as plain triangles: drawn flat, each face facing its own way
//! ([`flat_triangles`]), or as one surface of shared points for physics
//! ([`solid_triangles`]). Both cut faces the same way
//! ([`crate::face_triangles`]), so what is drawn is what is solid.

use crate::material::slot_of;
use crate::{FaceId, Mesh, VertexId};
use glam::Vec3;
use std::collections::HashMap;

/// A mesh's faces as triangles to draw flat: every face's corners are
/// points of their own, all facing the face's way, so edges between faces
/// are crisp.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FlatTriangles {
    /// Each corner's place.
    pub positions: Vec<Vec3>,
    /// Each corner's facing: its face's (unit length).
    pub normals: Vec<Vec3>,
    /// Three corners per triangle, counter-clockwise seen from outside.
    pub indices: Vec<u32>,
    /// Each triangle's face.
    pub faces: Vec<FaceId>,
    /// Each triangle's material slot ([`crate::material`]).
    pub slots: Vec<u32>,
}

impl FlatTriangles {
    /// How many triangles there are.
    pub fn len(&self) -> usize {
        self.indices.len() / 3
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// Every face of `mesh` cut into triangles to draw flat, in face order.
pub fn flat_triangles(mesh: &Mesh) -> FlatTriangles {
    let mut out = FlatTriangles::default();
    for (face, corners) in mesh.faces() {
        let normal = mesh.face_normal(face);
        let slot = slot_of(mesh, face);
        let first = out.positions.len() as u32;
        for v in corners {
            out.positions.push(mesh.position(*v).unwrap_or(Vec3::ZERO));
            out.normals.push(normal);
        }
        crate::face_triangles(mesh, corners, |[a, b, c]| {
            out.indices
                .extend([first + a as u32, first + b as u32, first + c as u32]);
            out.faces.push(face);
            out.slots.push(slot);
        });
    }
    out
}

/// Every face of `mesh` cut into triangles over shared points, for
/// physics: the points used (in id order) and three of them per
/// triangle, counter-clockwise seen from outside.
pub fn solid_triangles(mesh: &Mesh) -> (Vec<Vec3>, Vec<[u32; 3]>) {
    let mut index: HashMap<VertexId, u32> = HashMap::new();
    let mut points = Vec::new();
    for (id, at) in mesh.vertices() {
        index.insert(id, points.len() as u32);
        points.push(at);
    }
    let mut triangles = Vec::new();
    for (_, corners) in mesh.faces() {
        let at = |i: usize| index.get(&corners[i]).copied().unwrap_or(0);
        crate::face_triangles(mesh, corners, |[a, b, c]| {
            triangles.push([at(a), at(b), at(c)]);
        });
    }
    (points, triangles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::Shape;

    #[test]
    fn a_box_is_twelve_triangles_each_facing_its_face_s_way() {
        let mesh = Shape::Box.build(&Shape::Box.standard());
        let flat = flat_triangles(&mesh);
        assert_eq!(flat.len(), 12);
        assert_eq!(flat.positions.len(), 24, "four corners per face");
        assert_eq!(flat.faces.len(), 12);
        assert!(flat.slots.iter().all(|s| *s == 0));
        for (t, face) in flat.indices.chunks(3).zip(&flat.faces) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| flat.positions[i as usize]);
            let facing = (b - a).cross(c - a).normalize();
            assert!(facing.dot(mesh.face_normal(*face)) > 0.99);
            assert_eq!(flat.normals[t[0] as usize], mesh.face_normal(*face));
        }
    }

    #[test]
    fn solid_triangles_share_points_and_match_the_drawn_ones() {
        let mesh = Shape::Arch.build(&Shape::Arch.standard());
        let (points, triangles) = solid_triangles(&mesh);
        assert_eq!(points.len(), mesh.vertex_count());
        assert_eq!(triangles.len(), flat_triangles(&mesh).len());
        assert!(
            triangles
                .iter()
                .flatten()
                .all(|i| (*i as usize) < points.len())
        );
    }

    #[test]
    fn an_empty_mesh_has_no_triangles() {
        assert!(flat_triangles(&Mesh::new()).is_empty());
        assert_eq!(solid_triangles(&Mesh::new()), (Vec::new(), Vec::new()));
    }
}
