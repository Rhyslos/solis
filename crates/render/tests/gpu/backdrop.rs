//! The backdrop, the ground axes and grid, target sizes, and one viewport frame's time.

use super::*;

#[test]
fn the_ground_axes_show_in_red_and_green_unless_editor_lines_are_off() {
    let Some(gpu) = gpu() else { return };
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let size = 64;
    let mut target = renderer.create_target(&gpu.device, [size, size]);
    // Standing on the Y axis, so it runs straight ahead into the middle of
    // the lower half; the X axis crosses in front.
    let eye = Vec3::new(0.0, -10.0, 2.0);
    let mut colours = |editor_lines| {
        renderer.render(
            &gpu.device,
            &gpu.queue,
            &mut target,
            &FrameParams {
                view_projection: looking_north(eye),
                camera_position: eye,
                focus: Vec3::ZERO,
                sky_top: srgb(SKY_TOP),
                sky_horizon: srgb(SKY_HORIZON),
                ground: srgb(GROUND),
                shading: Shading::default(),
                size_px: [64, 64],
                sun: Sun::default(),
                environment: Environment::default(),
                editor_lines,
                grid_plane: GridPlane::Ground,
                grid_step: 1.0,
            },
            &[],
            &[],
        );
        let pixels = read_pixels(&gpu.device, &gpu.queue, &target).unwrap();
        let greenish = (size / 2..size)
            .map(|y| pixel(&pixels, size, size / 2, y))
            .any(|[r, g, b]| g > r + 20 && g > b + 20);
        let reddish = (0..size)
            .flat_map(|y| (0..size).map(move |x| (x, y)))
            .map(|(x, y)| pixel(&pixels, size, x, y))
            .any(|[r, g, b]| r > g + 20 && r > b + 20);
        (greenish, reddish)
    };
    let (greenish, reddish) = colours(true);
    assert!(greenish, "no green Y axis down the middle");
    assert!(reddish, "no red X axis");
    // The game's own view: no grid and no axes.
    assert_eq!(colours(false), (false, false));
}

#[test]
fn a_front_views_grid_stands_upright() {
    let Some(gpu) = gpu() else { return };
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let size = 64;
    let mut target = renderer.create_target(&gpu.device, [size, size]);
    // A flat view from the front: 20 m tall, looking north from 100 m
    // south, depth reversed (1 at the eye, 0 at 200 m).
    let eye = Vec3::new(0.0, -100.0, 2.0);
    let view = Mat4::from_cols(
        Vec4::new(1.0, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, -1.0, 0.0),
        Vec4::new(0.0, 1.0, 0.0, 0.0),
        Vec4::new(-eye.x, -eye.z, eye.y, 1.0),
    );
    let far = 200.0;
    let flat = Mat4::from_cols(
        Vec4::new(0.1, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 0.1, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0 / far, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 1.0),
    );
    let mut lines = |grid_plane| {
        renderer.render(
            &gpu.device,
            &gpu.queue,
            &mut target,
            &FrameParams {
                view_projection: flat * view,
                camera_position: eye,
                focus: Vec3::new(0.0, 0.0, 2.0),
                sky_top: srgb(SKY_TOP),
                sky_horizon: srgb(SKY_HORIZON),
                ground: srgb(GROUND),
                shading: Shading::default(),
                size_px: [size, size],
                sun: Sun::default(),
                environment: Environment::default(),
                editor_lines: true,
                grid_plane,
                grid_step: 1.0,
            },
            &[],
            &[],
        );
        let pixels = read_pixels(&gpu.device, &gpu.queue, &target).unwrap();
        // Grid lines are lighter than the backdrop behind them.
        let corner = pixel(&pixels, size, 1, 1);
        (0..size)
            .flat_map(|y| (0..size).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let p = pixel(&pixels, size, x, y);
                p.iter().zip(corner).any(|(a, b)| a.abs_diff(b) > 12)
            })
            .count()
    };
    let upright = lines(GridPlane::Front);
    let edge_on = lines(GridPlane::Ground);
    assert!(
        upright > edge_on + 100,
        "upright {upright}, edge on {edge_on}"
    );
}

#[test]
fn tiny_and_huge_sizes_are_clamped() {
    let Some(gpu) = gpu() else { return };
    let renderer = ViewportRenderer::new(&gpu.device, 1);
    assert_eq!(renderer.create_target(&gpu.device, [0, 0]).size(), [1, 1]);
    let max = gpu.device.limits().max_texture_dimension_2d;
    assert_eq!(
        renderer.create_target(&gpu.device, [u32::MAX, 3]).size(),
        [max, 3]
    );
}

/// Part of the scale check (`tools/scale/scale-check.sh`, decision 0011):
/// how long one viewport frame takes at 1920×1080. The number depends on
/// the graphics card, so it is reported, not judged. Not run with the
/// normal tests.
#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
#[allow(clippy::print_stdout)]
fn scale_check_viewport_frame() {
    let Some(gpu) = gpu() else { return };
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let mut target = renderer.create_target(&gpu.device, [1920, 1080]);
    let eye = Vec3::new(-16.0, -24.0, 10.0);
    let params = FrameParams {
        view_projection: looking_north(eye),
        camera_position: eye,
        focus: Vec3::ZERO,
        sky_top: srgb(SKY_TOP),
        sky_horizon: srgb(SKY_HORIZON),
        ground: srgb(GROUND),
        shading: Shading::default(),
        size_px: [1920, 1080],
        sun: Sun::default(),
        environment: Environment::default(),
        editor_lines: true,
        grid_plane: GridPlane::Ground,
        grid_step: 1.0,
    };
    let frames = 60;
    let start = std::time::Instant::now();
    for _ in 0..frames {
        renderer.render(&gpu.device, &gpu.queue, &mut target, &params, &[], &[]);
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .unwrap();
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    println!(
        "Scale check: viewport frame at 1920x1080 {ms:.2} ms on {}",
        gpu.adapter.get_info().name
    );
}
