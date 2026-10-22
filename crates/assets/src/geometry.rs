//! Turning glTF meshes into Solis's triangles: placing each one where its
//! node puts it, turning glTF's Y-up into Solis's Z-up, and filling in
//! anything missing (normals, texture coordinates).

use crate::model::{ImportError, Limits, Mesh, Vertex};
use glam::{Mat3, Mat4, Vec3, Vec4};
use gltf::accessor::{DataType, Dimensions};

/// glTF's axes to Solis's: glTF's up (+Y) becomes +Z, and glTF's front
/// (+Z, towards the viewer) becomes -Y. Both are metres, so nothing is
/// scaled.
pub const Y_UP_TO_Z_UP: Mat4 = Mat4::from_cols(
    Vec4::new(1.0, 0.0, 0.0, 0.0),
    Vec4::new(0.0, 0.0, 1.0, 0.0),
    Vec4::new(0.0, -1.0, 0.0, 0.0),
    Vec4::new(0.0, 0.0, 0.0, 1.0),
);

/// Every node of the file's main scene (or its first), with where it is
/// placed in Solis's space. Empty for a file without scenes.
pub(crate) fn placed_nodes(
    document: &gltf::Document,
) -> Result<Vec<(gltf::Node<'_>, Mat4)>, ImportError> {
    let Some(scene) = document
        .default_scene()
        .or_else(|| document.scenes().next())
    else {
        return Ok(Vec::new());
    };
    // Walked with a list rather than recursion, so a deep or looping node
    // tree can't overflow the stack. glTF forbids loops; the step limit
    // stops a damaged file that has one. Kept in reverse, so nodes come
    // out in the file's order.
    let mut placed = Vec::new();
    let mut todo: Vec<(gltf::Node<'_>, Mat4)> = scene.nodes().map(|n| (n, Y_UP_TO_Z_UP)).collect();
    todo.reverse();
    let mut steps = 0usize;
    while let Some((node, parent)) = todo.pop() {
        steps += 1;
        if steps > 1_000_000 {
            return Err(ImportError::Damaged("its node tree loops".into()));
        }
        let local = Mat4::from_cols_array_2d(&node.transform().matrix());
        let world = if local.is_finite() {
            parent * local
        } else {
            parent
        };
        let children: Vec<_> = node.children().map(|child| (child, world)).collect();
        todo.extend(children.into_iter().rev());
        placed.push((node, world));
    }
    Ok(placed)
}

/// Every mesh the file shows, placed. A file without scenes shows each
/// mesh once, unplaced. May be empty (a file of lights only).
pub(crate) fn read_meshes(
    document: &gltf::Document,
    buffers: &[Vec<u8>],
    limits: &Limits,
    warnings: &mut Vec<String>,
) -> Result<Vec<Mesh>, ImportError> {
    let nodes = placed_nodes(document)?;
    let placed: Vec<(gltf::Mesh<'_>, Mat4)> = if document.scenes().next().is_some() {
        nodes
            .into_iter()
            .filter_map(|(node, world)| node.mesh().map(|m| (m, world)))
            .collect()
    } else {
        document.meshes().map(|m| (m, Y_UP_TO_Z_UP)).collect()
    };

    let mut meshes = Vec::new();
    let (mut vertices, mut triangles) = (0usize, 0usize);
    let too_big = |vertices: usize, triangles: usize| {
        ImportError::TooManyTriangles(format!(
            "more than {} triangles or {} corners; the limits are {} and {}",
            triangles, vertices, limits.max_triangles, limits.max_vertices
        ))
    };
    for (mesh, world) in placed {
        for primitive in mesh.primitives() {
            // Counted from the file's own numbers before anything is read,
            // so a small file can't ask for millions of corners (a list
            // with no data is all zeros, of any length).
            let corners = primitive
                .get(&gltf::Semantic::Positions)
                .map_or(0, |a| a.count());
            let listed = primitive.indices().map_or(corners, |a| a.count());
            if vertices.saturating_add(corners) > limits.max_vertices
                || triangles.saturating_add(listed / 3) > limits.max_triangles
            {
                return Err(too_big(
                    vertices.saturating_add(corners),
                    triangles.saturating_add(listed / 3),
                ));
            }
            let Some(read) = read_primitive(&primitive, buffers, world, warnings)? else {
                continue;
            };
            vertices = vertices.saturating_add(read.vertices.len());
            triangles = triangles.saturating_add(read.triangles());
            if vertices > limits.max_vertices || triangles > limits.max_triangles {
                return Err(too_big(vertices, triangles));
            }
            let pieces = split(read, limits.max_mesh_vertices);
            if meshes.len() + pieces.len() > limits.max_meshes {
                return Err(ImportError::TooManyTriangles(format!(
                    "more than {} meshes, counting each placement",
                    limits.max_meshes
                )));
            }
            meshes.extend(pieces);
        }
    }
    Ok(meshes)
}

/// `mesh` in pieces of at most `max_vertices` corners each (and as many
/// triangle corners as fit one GPU buffer), so a huge mesh still fits the
/// graphics card's buffers. A mesh that fits stays whole.
pub(crate) fn split(mesh: Mesh, max_vertices: usize) -> Vec<Mesh> {
    use std::collections::HashMap;
    let max_vertices = max_vertices.max(3);
    // Indices are 4 bytes, vertices 32: the same buffer holds 8 times as
    // many indices as vertices.
    let max_indices = max_vertices.saturating_mul(8) / 3 * 3;
    if mesh.vertices.len() <= max_vertices && mesh.indices.len() <= max_indices {
        return vec![mesh];
    }
    let mut pieces = Vec::new();
    let mut piece = Mesh {
        material: mesh.material,
        ..Mesh::default()
    };
    let mut remap: HashMap<u32, u32> = HashMap::new();
    for triangle in mesh.indices.chunks_exact(3) {
        let new_corners = triangle.iter().filter(|i| !remap.contains_key(i)).count();
        if piece.vertices.len() + new_corners > max_vertices
            || piece.indices.len() + 3 > max_indices
        {
            pieces.push(std::mem::replace(
                &mut piece,
                Mesh {
                    material: mesh.material,
                    ..Mesh::default()
                },
            ));
            remap.clear();
        }
        for &i in triangle {
            let next = piece.vertices.len() as u32;
            let local = *remap.entry(i).or_insert_with(|| {
                piece
                    .vertices
                    .push(mesh.vertices.get(i as usize).copied().unwrap_or(Vertex {
                        position: [0.0; 3],
                        normal: [0.0, 0.0, 1.0],
                        uv: [0.0; 2],
                    }));
                next
            });
            piece.indices.push(local);
        }
    }
    if !piece.indices.is_empty() {
        pieces.push(piece);
    }
    pieces
}

/// One glTF primitive as a mesh, or `None` (with a warning) for kinds
/// Solis doesn't draw (points and lines).
fn read_primitive(
    primitive: &gltf::Primitive<'_>,
    buffers: &[Vec<u8>],
    world: Mat4,
    warnings: &mut Vec<String>,
) -> Result<Option<Mesh>, ImportError> {
    use gltf::mesh::Mode;
    let mode = primitive.mode();
    if !matches!(
        mode,
        Mode::Triangles | Mode::TriangleStrip | Mode::TriangleFan
    ) {
        warnings.push("points and lines in the file were left out".into());
        return Ok(None);
    }
    let Some(positions) = primitive.get(&gltf::Semantic::Positions) else {
        return Ok(None);
    };
    let wrong = |what: &str| ImportError::Damaged(format!("its {what} are stored the wrong way"));
    if positions.data_type() != DataType::F32 || positions.dimensions() != Dimensions::Vec3 {
        return Err(wrong("positions"));
    }
    let normals_ok = primitive
        .get(&gltf::Semantic::Normals)
        .is_none_or(|a| a.data_type() == DataType::F32 && a.dimensions() == Dimensions::Vec3);
    let uvs_ok = primitive
        .get(&gltf::Semantic::TexCoords(0))
        .is_none_or(|a| {
            a.dimensions() == Dimensions::Vec2
                && matches!(a.data_type(), DataType::F32 | DataType::U8 | DataType::U16)
        });
    let indices_ok = primitive.indices().is_none_or(|a| {
        a.dimensions() == Dimensions::Scalar
            && matches!(a.data_type(), DataType::U8 | DataType::U16 | DataType::U32)
    });
    if !normals_ok {
        return Err(wrong("normals"));
    }
    if !uvs_ok {
        return Err(wrong("texture coordinates"));
    }
    if !indices_ok {
        return Err(wrong("triangle lists"));
    }

    let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(Vec::as_slice));
    let points: Vec<Vec3> = reader
        .read_positions()
        .map(|p| p.map(Vec3::from).collect())
        .unwrap_or_default();
    if points.is_empty() {
        return Ok(None);
    }
    let count = points.len();
    let normals: Option<Vec<Vec3>> = reader
        .read_normals()
        .map(|n| n.map(Vec3::from).collect::<Vec<_>>())
        .filter(|n| n.len() == count);
    let uvs: Vec<[f32; 2]> = reader
        .read_tex_coords(0)
        .map(|t| t.into_f32().collect::<Vec<_>>())
        .filter(|t| t.len() == count)
        .unwrap_or_else(|| vec![[0.0, 0.0]; count]);
    let raw: Vec<u32> = match reader.read_indices() {
        Some(indices) => indices.into_u32().collect(),
        None => (0..count as u32).collect(),
    };
    if raw.iter().any(|&i| i as usize >= count) {
        return Err(ImportError::Damaged(
            "a triangle refers to a corner that doesn't exist".into(),
        ));
    }
    let indices = to_triangle_list(mode, &raw);

    let normal_matrix = {
        let m = Mat3::from_mat4(world);
        if m.determinant().abs() > 1e-12 {
            m.inverse().transpose()
        } else {
            // Squashed flat (a zero scale): keep at least the turn from
            // glTF's axes to Solis's.
            Mat3::from_mat4(Y_UP_TO_Z_UP)
        }
    };
    let positions: Vec<Vec3> = points
        .iter()
        .map(|&p| {
            let placed = world.transform_point3(p);
            if placed.is_finite() {
                placed
            } else {
                Vec3::ZERO
            }
        })
        .collect();
    // A mirrored placement turns triangles inside out; flip them back so
    // their front stays outside.
    let indices = if Mat3::from_mat4(world).determinant() < 0.0 {
        indices
            .chunks_exact(3)
            .flat_map(|t| [t[0], t[2], t[1]])
            .collect()
    } else {
        indices
    };
    // Normals worked out here come after the flip, so they face out too.
    let normals = match normals {
        Some(normals) => normals
            .iter()
            .map(|&n| (normal_matrix * n).try_normalize().unwrap_or(Vec3::Z))
            .collect(),
        None => smooth_normals(&positions, &indices),
    };
    let vertices = positions
        .iter()
        .zip(&normals)
        .zip(&uvs)
        .map(|((p, n), uv)| Vertex {
            position: p.to_array(),
            normal: n.to_array(),
            uv: uv.map(|v| if v.is_finite() { v } else { 0.0 }),
        })
        .collect();

    Ok(Some(Mesh {
        vertices,
        indices,
        material: primitive.material().index(),
    }))
}

/// Triangle strips and fans turned into a plain list of triangles.
pub(crate) fn to_triangle_list(mode: gltf::mesh::Mode, raw: &[u32]) -> Vec<u32> {
    use gltf::mesh::Mode;
    match mode {
        Mode::TriangleStrip => raw
            .windows(3)
            .enumerate()
            .flat_map(|(i, w)| {
                if i % 2 == 0 {
                    [w[0], w[1], w[2]]
                } else {
                    [w[1], w[0], w[2]]
                }
            })
            .collect(),
        Mode::TriangleFan => match raw.split_first() {
            Some((&centre, rest)) => rest.windows(2).flat_map(|w| [centre, w[0], w[1]]).collect(),
            None => Vec::new(),
        },
        _ => raw[..raw.len() - raw.len() % 3].to_vec(),
    }
}

/// Normals for a mesh that has none: each corner's is the average of the
/// triangles around it, weighted by their size, so curved surfaces look
/// smooth.
pub(crate) fn smooth_normals(positions: &[Vec3], indices: &[u32]) -> Vec<Vec3> {
    let mut sums = vec![Vec3::ZERO; positions.len()];
    for t in indices.chunks_exact(3) {
        let [a, b, c] = [t[0], t[1], t[2]].map(|i| i as usize);
        let (Some(&pa), Some(&pb), Some(&pc)) =
            (positions.get(a), positions.get(b), positions.get(c))
        else {
            continue;
        };
        let face = (pb - pa).cross(pc - pa);
        for i in [a, b, c] {
            sums[i] += face;
        }
    }
    sums.into_iter()
        .map(|n| n.try_normalize().unwrap_or(Vec3::Z))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gltf::mesh::Mode;

    #[test]
    fn y_up_becomes_z_up_and_front_becomes_minus_y() {
        assert_eq!(Y_UP_TO_Z_UP.transform_point3(Vec3::Y), Vec3::Z);
        assert_eq!(Y_UP_TO_Z_UP.transform_point3(Vec3::Z), -Vec3::Y);
        assert_eq!(Y_UP_TO_Z_UP.transform_point3(Vec3::X), Vec3::X);
        assert!((Mat3::from_mat4(Y_UP_TO_Z_UP).determinant() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn strips_and_fans_become_triangle_lists() {
        assert_eq!(
            to_triangle_list(Mode::TriangleStrip, &[0, 1, 2, 3]),
            [0, 1, 2, 2, 1, 3]
        );
        assert_eq!(
            to_triangle_list(Mode::TriangleFan, &[0, 1, 2, 3]),
            [0, 1, 2, 0, 2, 3]
        );
        assert_eq!(to_triangle_list(Mode::Triangles, &[0, 1, 2, 3]), [0, 1, 2]);
        assert!(to_triangle_list(Mode::TriangleFan, &[]).is_empty());
        assert!(to_triangle_list(Mode::TriangleStrip, &[0, 1]).is_empty());
    }

    #[test]
    fn missing_normals_point_out_of_the_surface() {
        let positions = [Vec3::ZERO, Vec3::X, Vec3::Y];
        let normals = smooth_normals(&positions, &[0, 1, 2]);
        assert!(normals.iter().all(|n| (*n - Vec3::Z).length() < 1e-6));
    }

    #[test]
    fn corners_of_no_triangle_still_get_a_normal() {
        let normals = smooth_normals(&[Vec3::ZERO, Vec3::ONE], &[]);
        assert_eq!(normals, [Vec3::Z, Vec3::Z]);
    }
}
