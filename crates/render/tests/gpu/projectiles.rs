//! Projectiles: glowing, hidden behind models, in a layer's picture, and the scale check.

use super::*;

#[test]
fn projectiles_glow_in_their_colour_and_hide_behind_models() {
    let Some(gpu) = gpu() else {
        return;
    };
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let size = 64;
    let mut target = renderer.create_target(&gpu.device, [size, size]);
    // Looking north from 6 m back; a red streak flying left to right
    // across the middle, 2 m ahead.
    let eye = Vec3::new(0.0, -6.0, 1.0);
    let params = FrameParams {
        view_projection: looking_north(eye),
        camera_position: eye,
        focus: Vec3::ZERO,
        sky_top: srgb(SKY_TOP),
        sky_horizon: srgb(SKY_HORIZON),
        ground: srgb(GROUND),
        shading: Shading::default(),
        size_px: [size, size],
        sun: Sun::default(),
        environment: unmapped(),
        editor_lines: false,
        grid_plane: GridPlane::Ground,
        grid_step: 1.0,
    };
    let streak = render::Bolt {
        head: [1.0, -4.0, 1.0],
        trail: [-2.0, 0.0, 0.0],
        colour: [4.0, 0.1, 0.1],
        width: 0.3,
    };
    let stats = renderer.render_with_bolts(
        &gpu.device,
        &gpu.queue,
        &mut target,
        &params,
        &[],
        &[],
        &[streak],
    );
    assert_eq!(stats.bolts, 1);
    let lit = read_pixels(&gpu.device, &gpu.queue, &target).unwrap();
    let middle = pixel(&lit, size, size / 2, size / 2);
    assert!(middle[0] > 200 && middle[1] < 120, "red: {middle:?}");
    // Behind a model, it's hidden: the sample's cube stands in front.
    let model = renderer.upload_model(&gpu.device, &gpu.queue, &sample());
    let behind = render::Bolt {
        head: [1.0, 3.0, 1.0],
        ..streak
    };
    renderer.render_with_bolts(
        &gpu.device,
        &gpu.queue,
        &mut target,
        &params,
        &[Placement {
            model: &model,
            node: None,
            transform: Mat4::IDENTITY,
            pick: 0,
        }],
        &[],
        &[behind],
    );
    let hidden = read_pixels(&gpu.device, &gpu.queue, &target).unwrap();
    let middle = pixel(&hidden, size, size / 2, size / 2);
    assert!(!(middle[0] > 200 && middle[1] < 120), "hidden: {middle:?}");
}

#[test]
fn projectiles_and_unlit_models_show_in_a_layer_s_picture() {
    // A 2D game with a parallax layer behind draws its world into a
    // layer's picture (decision 0094), clear where nothing is. Projectiles
    // over empty ground, and models in the unlit look, must still cover
    // their pixels there, or the last pass leaves them out.
    let Some(gpu) = gpu() else {
        return;
    };
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let size = 64;
    let mut target = renderer.create_target(&gpu.device, [size, size]);
    target.set_layered(true);
    let eye = Vec3::new(0.0, -6.0, 1.0);
    let params = FrameParams {
        view_projection: looking_north(eye),
        camera_position: eye,
        focus: Vec3::ZERO,
        sky_top: srgb(SKY_TOP),
        sky_horizon: srgb(SKY_HORIZON),
        ground: srgb(GROUND),
        shading: Shading {
            look: Look::Unlit,
            ..Shading::default()
        },
        size_px: [size, size],
        sun: Sun::default(),
        environment: unmapped(),
        editor_lines: false,
        grid_plane: GridPlane::Ground,
        grid_step: 1.0,
    };
    let alpha = |pixels: &[u8], x: u32, y: u32| pixels[((y * size + x) * 4 + 3) as usize];
    // A red streak across the middle, nothing behind it.
    let streak = render::Bolt {
        head: [1.0, -4.0, 1.0],
        trail: [-2.0, 0.0, 0.0],
        colour: [4.0, 0.1, 0.1],
        width: 0.3,
    };
    renderer.render_with_bolts(
        &gpu.device,
        &gpu.queue,
        &mut target,
        &params,
        &[],
        &[],
        &[streak],
    );
    let pixels = read_pixels(&gpu.device, &gpu.queue, &target).unwrap();
    assert_eq!(alpha(&pixels, 0, 0), 0, "clear where nothing is");
    let middle = pixel(&pixels, size, size / 2, size / 2);
    assert_eq!(alpha(&pixels, size / 2, size / 2), 255, "the streak covers");
    assert!(middle[0] > 200 && middle[1] < 120, "red: {middle:?}");
    // A dimmer streak covers by how bright it is, its colour kept.
    let dim = render::Bolt {
        colour: [0.25, 0.0, 0.0],
        width: 2.0,
        ..streak
    };
    renderer.render_with_bolts(
        &gpu.device,
        &gpu.queue,
        &mut target,
        &params,
        &[],
        &[],
        &[dim],
    );
    let pixels = read_pixels(&gpu.device, &gpu.queue, &target).unwrap();
    let covered = alpha(&pixels, size / 2, size / 2);
    assert!(covered > 0 && covered < 255, "partly covered: {covered}");
    // The sample's cube in the unlit look, straight ahead.
    let model = renderer.upload_model(&gpu.device, &gpu.queue, &sample());
    renderer.render(
        &gpu.device,
        &gpu.queue,
        &mut target,
        &params,
        &[Placement {
            model: &model,
            node: None,
            transform: Mat4::IDENTITY,
            pick: 0,
        }],
        &[],
    );
    let pixels = read_pixels(&gpu.device, &gpu.queue, &target).unwrap();
    assert_eq!(alpha(&pixels, 0, 0), 0, "clear where nothing is");
    assert_eq!(alpha(&pixels, size / 2, size / 2), 255, "the model covers");
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_100000_projectiles() {
    // The CPU's side of drawing 100,000 lasers spread over a field:
    // handing them over in one go (the GPU's drawing isn't waited for).
    let Some(gpu) = gpu() else {
        return;
    };
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let mut target = renderer.create_target(&gpu.device, [64, 64]);
    let eye = Vec3::new(0.0, -10.0, 8.0);
    let params = FrameParams {
        view_projection: looking_north(eye),
        camera_position: eye,
        focus: Vec3::ZERO,
        sky_top: srgb(SKY_TOP),
        sky_horizon: srgb(SKY_HORIZON),
        ground: srgb(GROUND),
        shading: Shading::default(),
        size_px: [64, 64],
        sun: Sun::default(),
        environment: unmapped(),
        editor_lines: false,
        grid_plane: GridPlane::Ground,
        grid_step: 1.0,
    };
    let bolts: Vec<render::Bolt> = (0..100_000)
        .map(|i| render::Bolt {
            head: [(i % 300) as f32 - 150.0, (i / 300) as f32, 2.0],
            trail: [-2.0, 0.0, 0.0],
            colour: [4.0, 0.2, 0.1],
            width: 0.12,
        })
        .collect();
    let frames = 10;
    let mut spent = std::time::Duration::ZERO;
    for _ in 0..frames {
        let start = std::time::Instant::now();
        let stats = renderer.render_with_bolts(
            &gpu.device,
            &gpu.queue,
            &mut target,
            &params,
            &[],
            &[],
            &bolts,
        );
        spent += start.elapsed();
        assert_eq!(stats.bolts, 100_000);
        let _ = read_pixels(&gpu.device, &gpu.queue, &target);
    }
    let ms = spent.as_secs_f64() * 1000.0 / f64::from(frames);
    gathering_verdict("handing over 100,000 projectiles", ms, 2.0, true);
}
