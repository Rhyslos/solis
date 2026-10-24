//! Levels of detail: made by simplifying, or the model's own.

use super::*;
use crate::model::{Bounds, Node, Vertex};

/// A wavy sheet `side` squares across (two triangles each), 1 m wide,
/// raised by `lift`.
fn sheet(side: u32, lift: f32) -> Mesh {
    let mut vertices = Vec::new();
    for y in 0..=side {
        for x in 0..=side {
            let (u, v) = (x as f32 / side as f32, y as f32 / side as f32);
            vertices.push(Vertex {
                position: [u, v, lift + 0.05 * (u * 9.0).sin() * (v * 7.0).cos()],
                normal: [0.0, 0.0, 1.0],
                uv: [u, v],
            });
        }
    }
    let mut indices = Vec::new();
    let row = side + 1;
    for y in 0..side {
        for x in 0..side {
            let a = y * row + x;
            indices.extend_from_slice(&[a, a + 1, a + row + 1, a, a + row + 1, a + row]);
        }
    }
    Mesh {
        vertices,
        indices,
        material: None,
        lods: Vec::new(),
    }
}

fn node(name: &str, meshes: Vec<usize>, world: Mat4) -> Node {
    Node {
        name: name.into(),
        parent: None,
        local: world,
        world,
        meshes,
    }
}

fn model(meshes: Vec<Mesh>, nodes: Vec<Node>) -> Model {
    Model {
        name: "Test".into(),
        meshes,
        nodes,
        materials: Vec::new(),
        textures: Vec::new(),
        lamps: Vec::new(),
        bounds: Bounds {
            min: Vec3::ZERO,
            max: Vec3::ONE,
        },
        warnings: Vec::new(),
    }
}

#[test]
fn a_detailed_mesh_gets_simpler_levels_sharing_its_corners() {
    let mesh = sheet(64, 0.0);
    let full = mesh.triangles();
    let lods = simplified(&mesh);
    assert_eq!(lods.len(), 3, "a half, a quarter and an eighth");
    let mut before = full;
    let mut error = 0.0;
    for (lod, share) in lods.iter().zip(LOD_SHARES) {
        assert!(lod.vertices.is_empty(), "shares the mesh's corners");
        assert!(lod.triangles() < before, "each simpler than the last");
        assert!(
            lod.triangles() as f32 <= full as f32 * share * 1.2,
            "{} of {full}",
            lod.triangles()
        );
        assert!(
            lod.indices
                .iter()
                .all(|&i| (i as usize) < mesh.vertices.len())
        );
        assert_eq!(lod.indices.len() % 3, 0);
        assert!(lod.error >= error && lod.error <= 0.2, "{}", lod.error);
        before = lod.triangles();
        error = lod.error;
    }
}

#[test]
fn small_or_broken_meshes_get_no_levels() {
    assert!(
        simplified(&sheet(4, 0.0)).is_empty(),
        "32 triangles: too few"
    );
    let mut broken = sheet(64, 0.0);
    broken.indices[5] = 1_000_000;
    assert!(simplified(&broken).is_empty());
    let mut half = sheet(64, 0.0);
    half.indices.pop();
    assert!(simplified(&half).is_empty());
    assert!(simplified(&Mesh::default()).is_empty());
}

#[test]
fn a_mesh_that_will_not_simplify_keeps_only_its_full_version() {
    // Big separate triangles fanned around a point, each as big as the
    // whole: taking any corner away changes the shape by far more than
    // the most allowed, so no level comes out simpler.
    let mut mesh = Mesh::default();
    for i in 0..80 {
        let turn = i as f32 * std::f32::consts::TAU / 80.0;
        let base = mesh.vertices.len() as u32;
        for p in [
            [0.0, 0.0, 0.0],
            [turn.cos(), turn.sin(), 0.0],
            [0.0, 0.0, 1.0],
        ] {
            mesh.vertices.push(Vertex {
                position: p,
                normal: [-turn.sin(), turn.cos(), 0.0],
                uv: [0.0, 0.0],
            });
        }
        mesh.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }
    let lods = simplified(&mesh);
    assert!(
        lods.is_empty(),
        "{:?}",
        lods.iter()
            .map(|l| (l.triangles(), l.error))
            .collect::<Vec<_>>()
    );
}

#[test]
fn lod_names_are_read_and_odd_ones_ignored() {
    assert_eq!(lod_name("Ship_LOD1"), Some(("Ship", 1)));
    assert_eq!(lod_name("ship_lod02"), Some(("ship", 2)));
    assert_eq!(lod_name("Big_Ship_Lod0"), Some(("Big_Ship", 0)));
    assert_eq!(lod_name("Skip_LOD1é"), None);
    assert_eq!(lod_name("Ørn_LOD3"), Some(("Ørn", 3)));
    for odd in [
        "Ship",
        "Ship_LOD",
        "_LOD1",
        "Ship_LODx",
        "Ship_LOD123",
        "Ship LOD1",
        "",
        "_",
        "é_",
    ] {
        assert_eq!(lod_name(odd), None, "{odd}");
    }
}

#[test]
fn lod_names_never_panic() {
    // Every short string over a small alphabet with multi-byte letters.
    let letters = ['_', 'L', 'o', 'D', '1', '0', 'é', 'Ø', 'x'];
    let mut name = String::new();
    for a in letters {
        for b in letters {
            for c in letters {
                for d in letters {
                    for e in letters {
                        name.clear();
                        name.extend([a, b, c, d, e]);
                        let _ = lod_name(&name);
                    }
                }
            }
        }
    }
}

#[test]
fn a_model_s_own_levels_are_used_instead_and_not_drawn_as_parts() {
    // Ship's full mesh at the origin; its LOD1 made in the file half a
    // metre up (moved into Ship's space); a part of its own beside them.
    let lifted = Mat4::from_translation(Vec3::new(0.0, 0.0, 0.5));
    let mut m = model(
        vec![sheet(64, 0.0), sheet(8, 0.0), sheet(64, 0.0)],
        vec![
            node("Ship", vec![0], Mat4::IDENTITY),
            node("Ship_LOD1", vec![1], lifted),
            node("Turret", vec![2], Mat4::IDENTITY),
        ],
    );
    add_lods(&mut m, &crate::model::Limits::default());
    assert!(m.warnings.is_empty(), "{:?}", m.warnings);
    let ship = &m.meshes[0];
    assert_eq!(ship.lods.len(), 1, "its own, not made ones");
    assert_eq!(ship.lods[0].vertices.len(), sheet(8, 0.0).vertices.len());
    assert!(
        ship.lods[0].vertices.iter().all(|v| v.position[2] >= 0.45),
        "moved into Ship's space"
    );
    assert!(
        m.nodes[1].meshes.is_empty(),
        "the LOD isn't drawn on its own"
    );
    assert!(m.meshes[1].vertices.is_empty(), "and its mesh is let go");
    assert_eq!(m.meshes[2].lods.len(), 3, "other parts get made levels");
    // Drawn: Ship and Turret in full, then simpler.
    let triangles = m.lod_triangles();
    assert_eq!(triangles[0], 2 * 64 * 64 * 2);
    assert_eq!(triangles[1], 8 * 8 * 2 + m.meshes[2].lods[0].triangles());
    assert_eq!(triangles.len(), 4);
}

#[test]
fn a_lonely_or_too_deep_lod_is_said_and_left_alone() {
    let mut m = model(
        vec![sheet(8, 0.0), sheet(8, 0.0), sheet(8, 0.0)],
        vec![
            node("Rock_LOD1", vec![0], Mat4::IDENTITY),
            node("Ship", vec![1], Mat4::IDENTITY),
            node("Ship_LOD9", vec![2], Mat4::IDENTITY),
        ],
    );
    add_lods(&mut m, &crate::model::Limits::default());
    assert_eq!(m.warnings.len(), 2, "{:?}", m.warnings);
    assert!(m.warnings[0].contains("no part called Rock"));
    assert!(m.warnings[1].contains("at most 4"));
    assert_eq!(m.nodes[0].meshes, [0], "Rock_LOD1 is drawn as a part");
    assert_eq!(m.nodes[2].meshes, [2], "so is Ship_LOD9");
    assert!(
        m.meshes[1].lods.iter().all(|l| l.vertices.is_empty()),
        "Ship gets made levels, not those"
    );
}

#[test]
fn a_mesh_shown_by_another_node_too_is_kept() {
    // The LOD's mesh is also used by a part of its own: it stays.
    let mut m = model(
        vec![sheet(64, 0.0), sheet(8, 0.0)],
        vec![
            node("Ship", vec![0], Mat4::IDENTITY),
            node("Ship_LOD1", vec![1], Mat4::IDENTITY),
            node("Spare", vec![1], Mat4::IDENTITY),
        ],
    );
    add_lods(&mut m, &crate::model::Limits::default());
    assert_eq!(m.meshes[0].lods.len(), 1);
    assert!(!m.meshes[1].vertices.is_empty());
}

#[test]
fn one_mesh_named_the_level_of_many_parts_can_t_multiply_past_the_limits() {
    // A big mesh (64 x 64 squares) named as level 1 of 50 tiny parts: each
    // part would get its own copy.
    let mut meshes = vec![sheet(64, 0.0)];
    let mut nodes = Vec::new();
    for i in 0..50 {
        meshes.push(sheet(1, 0.0));
        nodes.push(node(&format!("Part{i}"), vec![i + 1], Mat4::IDENTITY));
        nodes.push(node(&format!("Part{i}_LOD1"), vec![0], Mat4::IDENTITY));
    }
    let mut m = model(meshes, nodes);
    let big = m.meshes[0].vertices.len();
    let limits = crate::model::Limits {
        max_vertices: big * 5,
        ..crate::model::Limits::default()
    };
    add_lods(&mut m, &limits);
    let total: usize = m
        .meshes
        .iter()
        .map(|mesh| mesh.vertices.len() + mesh.lods.iter().map(|l| l.vertices.len()).sum::<usize>())
        .sum();
    assert!(total <= limits.max_vertices, "{total} corners");
    assert!(
        m.warnings.iter().any(|w| w.contains("left out")),
        "{:?}",
        m.warnings
    );
}
