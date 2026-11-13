//! Copies of a model: mirrored, placed twice, one part alone.

use super::*;

/// Renders `model` once for each of `transforms`, in the standard look,
/// 64 pixels square, looking north from `eye`.
fn render_placed(model: &assets::Model, transforms: &[Mat4], eye: Vec3) -> Option<Vec<u8>> {
    let whole: Vec<(Option<usize>, Mat4)> = transforms.iter().map(|&t| (None, t)).collect();
    render_parts(model, &whole, eye)
}

/// As [`render_placed`], each placement the whole model (`None`) or one of
/// its nodes.
fn render_parts(
    model: &assets::Model,
    placements: &[(Option<usize>, Mat4)],
    eye: Vec3,
) -> Option<Vec<u8>> {
    let gpu = gpu()?;
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let size = 64;
    let mut target = renderer.create_target(&gpu.device, [size, size]);
    let uploaded = renderer.upload_model(&gpu.device, &gpu.queue, model);
    let placed: Vec<Placement<'_>> = placements
        .iter()
        .map(|&(node, transform)| Placement {
            model: &uploaded,
            node,
            transform,
            pick: 0,
        })
        .collect();
    renderer.render(
        &gpu.device,
        &gpu.queue,
        &mut target,
        &FrameParams {
            view_projection: looking_north(eye),
            camera_position: eye,
            focus: Vec3::new(1.0e7, 1.0e7, 0.0),
            sky_top: srgb(SKY_TOP),
            sky_horizon: srgb(SKY_HORIZON),
            ground: srgb(GROUND),
            shading: Shading::default(),
            size_px: [size, size],
            sun: Sun::default(),
            environment: unmapped(),
            editor_lines: true,
            grid_plane: GridPlane::Ground,
            grid_step: 1.0,
        },
        &placed,
        &[],
    );
    Some(read_pixels(&gpu.device, &gpu.queue, &target).unwrap())
}

#[test]
fn a_mirrored_instance_looks_like_the_old_mirrored_bake() {
    // The sample mirrored left to right: once as an instance with a
    // negative scale (drawn with the flipped pipelines), once baked into
    // its corners with the triangles turned, as Phase 1 did.
    let eye = Vec3::new(1.5, -7.0, 1.4);
    let mirror = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
    let model = sample();
    let Some(instanced) = render_placed(&model, &[mirror], eye) else {
        return;
    };
    let mut baked = model.clone();
    baked.meshes = model.placed_meshes();
    baked.show_every_mesh();
    let mut baked_mirror = baked.clone();
    for mesh in &mut baked_mirror.meshes {
        for v in &mut mesh.vertices {
            v.position[0] = -v.position[0];
            v.normal[0] = -v.normal[0];
        }
        mesh.indices = mesh
            .indices
            .chunks_exact(3)
            .flat_map(|t| [t[0], t[2], t[1]])
            .collect();
    }
    let reference = render_placed(&baked_mirror, &[Mat4::IDENTITY], eye).unwrap();
    let differing = instanced
        .chunks_exact(4)
        .zip(reference.chunks_exact(4))
        .filter(|(a, b)| a.iter().zip(*b).any(|(x, y)| x.abs_diff(*y) > 6))
        .count();
    assert!(differing < 40, "{differing} pixels differ");
    // And it isn't just empty: the mirrored cube is drawn (not culled
    // inside out), so the view differs from the backdrop alone.
    let empty = render_placed(&model, &[], eye).unwrap();
    assert_ne!(instanced, empty);
}

#[test]
fn one_model_placed_twice_is_drawn_twice() {
    let eye = Vec3::new(0.0, -14.0, 1.4);
    let model = sample();
    let Some(once) = render_placed(&model, &[Mat4::IDENTITY], eye) else {
        return;
    };
    let twice = render_placed(
        &model,
        &[
            Mat4::from_translation(Vec3::new(-4.0, 0.0, 0.0)),
            Mat4::from_translation(Vec3::new(4.0, 0.0, 0.0)),
        ],
        eye,
    )
    .unwrap();
    // Compared picture to picture rather than by colour, because GPUs
    // (Windows' WARP especially) differ in shading and at edges: the two
    // copies change the sides of the picture a lot, and leave the middle,
    // where the single copy stood.
    let changed_in = |xs: std::ops::Range<u32>| {
        let mut n = 0;
        for y in 0..64u32 {
            for x in xs.clone() {
                let i = ((y * 64 + x) * 4) as usize;
                let diff = (0..3)
                    .map(|c| once[i + c].abs_diff(twice[i + c]))
                    .max()
                    .unwrap_or(0);
                if diff > 30 {
                    n += 1;
                }
            }
        }
        n
    };
    let (left, middle, right) = (changed_in(0..16), changed_in(26..38), changed_in(48..64));
    assert!(
        left > 40 && right > 40 && middle > 40,
        "the picture changes at both sides and in the middle: {left}, {right} and {middle} pixels"
    );
}

#[test]
fn one_part_of_a_model_can_be_drawn_on_its_own() {
    // The sample's cube without its red base: what an unpacked part draws.
    let eye = Vec3::new(0.0, -7.0, 1.4);
    let model = sample();
    let cube = model
        .nodes
        .iter()
        .position(|n| {
            n.meshes
                .iter()
                .any(|&m| model.meshes[m].material == Some(0))
        })
        .unwrap();
    let Some(part) = render_parts(&model, &[(Some(cube), model.nodes[cube].world)], eye) else {
        return;
    };
    let whole = render_parts(&model, &[(None, Mat4::IDENTITY)], eye).unwrap();
    let red = |p: &[u8]| {
        p.chunks_exact(4)
            .filter(|c| c[0] > 120 && c[1] < 80 && c[2] < 80)
            .count()
    };
    // The red X axis on the ground shows either way; the base adds far more.
    let (whole_red, part_red) = (red(&whole), red(&part));
    assert!(
        whole_red > 4 * part_red,
        "{whole_red} vs {part_red}: the cube alone has no base"
    );
    assert_ne!(
        part,
        render_parts(&model, &[], eye).unwrap(),
        "but the cube is there"
    );
}
