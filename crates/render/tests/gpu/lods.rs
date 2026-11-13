//! Levels of detail: simpler levels far away, and a model's own.

use super::*;

/// A bumpy grid 4 m square, `side` squares across, just above the
/// ground: bumpy enough that its simpler levels show up close.
fn grid(side: u32) -> assets::Mesh {
    let mut vertices = Vec::new();
    for y in 0..=side {
        for x in 0..=side {
            let (u, v) = (x as f32 / side as f32, y as f32 / side as f32);
            vertices.push(assets::Vertex {
                position: [
                    u * 4.0 - 2.0,
                    v * 4.0 - 2.0,
                    0.3 + 0.2 * (u * 40.0).sin() * (v * 35.0).cos(),
                ],
                normal: [0.0, 0.0, 1.0],
                uv: [u, v],
            });
        }
    }
    let row = side + 1;
    let mut indices = Vec::new();
    for y in 0..side {
        for x in 0..side {
            let a = y * row + x;
            indices.extend_from_slice(&[a, a + 1, a + row + 1, a, a + row + 1, a + row]);
        }
    }
    assets::Mesh {
        vertices,
        indices,
        material: None,
        lods: Vec::new(),
    }
}

/// A model of `meshes`, each shown by a node named as given.
fn named_model(meshes: Vec<assets::Mesh>, names: &[&str]) -> assets::Model {
    let mut model = floor(false);
    model.meshes = meshes;
    model.nodes = names
        .iter()
        .enumerate()
        .map(|(i, name)| assets::Node {
            name: (*name).to_string(),
            parent: None,
            local: Mat4::IDENTITY,
            world: Mat4::IDENTITY,
            meshes: vec![i],
        })
        .collect();
    model.bounds = assets::Bounds {
        min: Vec3::new(-2.0, -2.0, 0.0),
        max: Vec3::new(2.0, 2.0, 0.6),
    };
    assets::lods::add_lods(&mut model, &assets::Limits::default());
    model
}

#[test]
fn a_far_copy_draws_a_simpler_level() {
    let model = named_model(vec![grid(48)], &["Hill"]);
    assert_eq!(model.meshes[0].lods.len(), 3, "made on import");
    let near = Vec3::new(0.0, -2.5, 1.2);
    let Some((_, close)) = render_scene(
        &[(&model, Mat4::IDENTITY)],
        looking_north(near),
        near,
        Sun::default(),
    ) else {
        return;
    };
    assert_eq!(close.lods[0], 1, "close: in full");
    assert_eq!(close.triangles, 48 * 48 * 2);
    let far_eye = Vec3::new(0.0, -300.0, 1.2);
    let (_, far) = render_scene(
        &[(&model, Mat4::IDENTITY)],
        looking_north(far_eye),
        far_eye,
        Sun::default(),
    )
    .unwrap();
    assert_eq!(far.instances, 1, "still drawn");
    assert_eq!(far.lods[0], 0, "far: not in full");
    assert!(
        far.triangles * 4 < close.triangles,
        "{} vs {}",
        far.triangles,
        close.triangles
    );
}

#[test]
fn a_model_s_own_level_of_detail_is_drawn_far_away() {
    // Its full version a fine grid; its own LOD1 a single square.
    let mut plain = floor(false);
    plain.meshes = vec![square(0.0, 0.3, 2.0)];
    let model = named_model(
        vec![grid(32), plain.meshes.remove(0)],
        &["Hill", "Hill_LOD1"],
    );
    assert_eq!(model.meshes[0].lods.len(), 1, "its own, not made ones");
    let near = Vec3::new(0.0, -1.6, 0.9);
    let Some((_, close)) = render_scene(
        &[(&model, Mat4::IDENTITY)],
        looking_north(near),
        near,
        Sun::default(),
    ) else {
        return;
    };
    assert_eq!(
        close.triangles,
        32 * 32 * 2,
        "close: in full, and the LOD isn't drawn too"
    );
    let far_eye = Vec3::new(0.0, -300.0, 1.2);
    let (_, far) = render_scene(
        &[(&model, Mat4::IDENTITY)],
        looking_north(far_eye),
        far_eye,
        Sun::default(),
    )
    .unwrap();
    assert_eq!(far.lods[1], 1);
    assert_eq!(far.triangles, 2, "its own square");
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_10000_units_most_far_away() {
    // 10,000 detailed units (4,608 triangles each) on a field 300 m
    // square, the camera low at its near edge looking across it: most are
    // far away and drawn at simpler levels.
    let unit = named_model(vec![grid(48)], &["Unit"]);
    let Some((ms, stats)) =
        gathering_ms(&unit, 10_000, 100, Vec3::new(0.0, -10.0, 4.0), true, true)
    else {
        return;
    };
    let full = u64::from(stats.lods.iter().sum::<u32>()) * 48 * 48 * 2;
    gathering_verdict(
        &format!(
            "10,000 moving detailed units, most far away: {} triangles drawn instead of {} \
             (copies at each level {:?})",
            stats.triangles, full, stats.lods
        ),
        ms,
        8.0,
        true,
    );
    assert!(
        stats.triangles * 3 < full,
        "levels of detail cut the triangles"
    );
}
