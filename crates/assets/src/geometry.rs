//! Turning glTF meshes into Solis's triangles and a tree of nodes: each
//! mesh is read once, in its own space, and nodes say where it's placed
//! ([decision 0021](../../../docs/decisions/0021-asset-database-and-models.md)).
//! glTF's Y-up becomes Solis's Z-up: mesh corners turn by `C`, and a node's
//! placement `T` becomes `C·T·C⁻¹`, so the product puts every corner
//! exactly where baking `C·T` into it would. Missing normals and texture
//! coordinates are filled in.

use crate::model::{ImportError, Limits, Mesh, Node, Vertex};
use glam::{Mat3, Mat4, Vec3, Vec4};
use gltf::accessor::{DataType, Dimensions};
use std::collections::HashMap;

/// glTF's axes to Solis's: glTF's up (+Y) becomes +Z, and glTF's front
/// (+Z, towards the viewer) becomes -Y. Both are metres, so nothing is
/// scaled.
pub const Y_UP_TO_Z_UP: Mat4 = Mat4::from_cols(
    Vec4::new(1.0, 0.0, 0.0, 0.0),
    Vec4::new(0.0, 0.0, 1.0, 0.0),
    Vec4::new(0.0, -1.0, 0.0, 0.0),
    Vec4::new(0.0, 0.0, 0.0, 1.0),
);

/// The most nodes a model's tree may have. Real models have thousands; a
/// damaged or hostile file with millions would make every step after
/// reading it (part IDs, the Area panel, unpacking) crawl.
pub(crate) const MAX_NODES: usize = 200_000;

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
    too_many(0, 0, scene.nodes().len())?;
    let mut todo: Vec<(gltf::Node<'_>, Mat4)> = scene.nodes().map(|n| (n, Y_UP_TO_Z_UP)).collect();
    todo.reverse();
    let mut steps = 0usize;
    while let Some((node, parent)) = todo.pop() {
        steps += 1;
        if steps > MAX_NODES {
            return Err(ImportError::Damaged(format!(
                "its node tree loops, or has more than {MAX_NODES} nodes"
            )));
        }
        let local = Mat4::from_cols_array_2d(&node.transform().matrix());
        let world = if local.is_finite() {
            parent * local
        } else {
            parent
        };
        too_many(steps, todo.len(), node.children().len())?;
        let children: Vec<_> = node.children().map(|child| (child, world)).collect();
        todo.extend(children.into_iter().rev());
        placed.push((node, world));
    }
    Ok(placed)
}

/// Refuses a node tree before listing more nodes than it may have: one
/// node can name millions of children (the same one again and again) in a
/// small file, and listing them would take far more memory than the file.
fn too_many(steps: usize, waiting: usize, children: usize) -> Result<(), ImportError> {
    if steps.saturating_add(waiting).saturating_add(children) > MAX_NODES {
        return Err(ImportError::Damaged(format!(
            "its node tree loops, or has more than {MAX_NODES} nodes"
        )));
    }
    Ok(())
}

/// Every mesh the file uses, each read once in its own space (Z up), and
/// the nodes that place them. A file without scenes gets one node per
/// mesh. Both may be empty (a file of lights only).
pub(crate) fn read_meshes(
    document: &gltf::Document,
    buffers: &[Vec<u8>],
    limits: &Limits,
    warnings: &mut Vec<String>,
) -> Result<(Vec<Mesh>, Vec<Node>), ImportError> {
    let placed = scene_nodes(document)?;
    let mut meshes = Vec::new();
    // Each glTF mesh's pieces in `meshes`, read the first time it's used.
    let mut read: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut nodes: Vec<Node> = Vec::with_capacity(placed.len());
    let (mut vertices, mut triangles, mut placements) = (0usize, 0usize, 0usize);
    let too_big = |vertices: usize, triangles: usize| {
        ImportError::TooManyTriangles(format!(
            "more than {} triangles or {} corners; the limits are {} and {}",
            triangles, vertices, limits.max_triangles, limits.max_vertices
        ))
    };
    for PlacedNode {
        name,
        parent,
        local,
        world,
        mesh,
    } in placed
    {
        let mut node = Node {
            name,
            parent,
            local,
            world,
            meshes: Vec::new(),
        };
        if let Some(mesh) = mesh {
            // Counted per placement from the file's own numbers before
            // anything is read, so a small file can't ask for millions of
            // corners, read once or placed many times.
            for primitive in mesh.primitives() {
                let corners = primitive
                    .get(&gltf::Semantic::Positions)
                    .map_or(0, |a| a.count());
                let listed = primitive.indices().map_or(corners, |a| a.count());
                vertices = vertices.saturating_add(corners);
                triangles = triangles.saturating_add(listed / 3);
                if vertices > limits.max_vertices || triangles > limits.max_triangles {
                    return Err(too_big(vertices, triangles));
                }
            }
            if let std::collections::hash_map::Entry::Vacant(slot) = read.entry(mesh.index()) {
                let mut pieces = Vec::new();
                for primitive in mesh.primitives() {
                    let Some(local_mesh) = read_primitive(&primitive, buffers, warnings)? else {
                        continue;
                    };
                    for piece in split(local_mesh, limits.max_mesh_vertices) {
                        pieces.push(meshes.len());
                        meshes.push(piece);
                    }
                }
                slot.insert(pieces);
            }
            node.meshes = read.get(&mesh.index()).cloned().unwrap_or_default();
            placements += node.meshes.len();
            if placements > limits.max_meshes {
                return Err(ImportError::TooManyTriangles(format!(
                    "more than {} meshes, counting each placement",
                    limits.max_meshes
                )));
            }
        }
        nodes.push(node);
    }
    Ok((meshes, nodes))
}

/// A node of the file's main scene, in Solis's axes.
struct PlacedNode<'a> {
    name: String,
    parent: Option<usize>,
    local: Mat4,
    world: Mat4,
    mesh: Option<gltf::Mesh<'a>>,
}

/// The nodes of the file's main scene (or its first), parents before
/// children, with placements turned into Solis's axes (`C·T·C⁻¹`). A file
/// without scenes gets a node for each mesh, unplaced.
fn scene_nodes(document: &gltf::Document) -> Result<Vec<PlacedNode<'_>>, ImportError> {
    let Some(scene) = document
        .default_scene()
        .or_else(|| document.scenes().next())
    else {
        return Ok(document
            .meshes()
            .map(|mesh| PlacedNode {
                name: mesh.name().unwrap_or("Mesh").to_string(),
                parent: None,
                local: Mat4::IDENTITY,
                world: Mat4::IDENTITY,
                mesh: Some(mesh),
            })
            .collect());
    };
    let to_z_up = |m: Mat4| Y_UP_TO_Z_UP * m * Y_UP_TO_Z_UP.inverse();
    // Walked with a list rather than recursion, as `placed_nodes`.
    let mut out: Vec<PlacedNode<'_>> = Vec::new();
    too_many(0, 0, scene.nodes().len())?;
    let mut todo: Vec<(gltf::Node<'_>, Option<usize>)> = scene.nodes().map(|n| (n, None)).collect();
    todo.reverse();
    let mut steps = 0usize;
    while let Some((node, parent)) = todo.pop() {
        steps += 1;
        if steps > MAX_NODES {
            return Err(ImportError::Damaged(format!(
                "its node tree loops, or has more than {MAX_NODES} nodes"
            )));
        }
        let raw = Mat4::from_cols_array_2d(&node.transform().matrix());
        let local = if raw.is_finite() {
            to_z_up(raw)
        } else {
            Mat4::IDENTITY
        };
        let world = parent.map_or(local, |p| out[p].world * local);
        let index = out.len();
        too_many(steps, todo.len(), node.children().len())?;
        let children: Vec<_> = node.children().map(|c| (c, Some(index))).collect();
        todo.extend(children.into_iter().rev());
        out.push(PlacedNode {
            name: node
                .name()
                .map_or_else(|| format!("Node {}", node.index()), str::to_string),
            parent,
            local,
            world,
            mesh: node.mesh(),
        });
    }
    Ok(out)
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
    warnings: &mut Vec<String>,
) -> Result<Option<Mesh>, ImportError> {
    // Only turned into Solis's axes here; nodes place it.
    let world = Y_UP_TO_Z_UP;
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

    // A turn only, so normals turn the same way as positions.
    let normal_matrix = Mat3::from_mat4(world);
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
        lods: Vec::new(),
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
