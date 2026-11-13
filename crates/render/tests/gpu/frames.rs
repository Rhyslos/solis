//! Picking, the axes close up, what a frame says it drew, and the same frame on one core or many.

use super::*;

/// Waits for a pick's answer (tests only: the editor never waits).
#[allow(clippy::panic)]
pub(crate) fn wait_for_pick(gpu: &Gpu, target: &mut render::ViewportTarget) -> render::PickResult {
    for _ in 0..100 {
        let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
        if let Some(result) = target.poll_pick(&gpu.device) {
            return result;
        }
    }
    panic!("the pick never came back");
}

#[test]
fn picking_finds_the_placement_under_a_pixel_and_where_it_is() {
    let Some(gpu) = gpu() else {
        return;
    };
    let model = sample();
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let size = 64;
    let mut target = renderer.create_target(&gpu.device, [size, size]);
    let uploaded = renderer.upload_model(&gpu.device, &gpu.queue, &model);
    let eye = Vec3::new(0.0, -14.0, 1.4);
    let view_projection = looking_north(eye);
    let placed = [
        Placement {
            model: &uploaded,
            node: None,
            transform: Mat4::from_translation(Vec3::new(-4.0, 0.0, 0.0)),
            pick: 7,
        },
        Placement {
            model: &uploaded,
            node: None,
            transform: Mat4::from_translation(Vec3::new(4.0, 0.0, 0.0)),
            pick: 9,
        },
    ];
    assert!(renderer.request_pick(
        &gpu.device,
        &gpu.queue,
        &mut target,
        view_projection,
        &placed,
        [0, 0, size, size],
    ));
    assert!(target.is_picking());
    assert!(
        !renderer.request_pick(
            &gpu.device,
            &gpu.queue,
            &mut target,
            view_projection,
            &placed,
            [0, 0, 1, 1],
        ),
        "one question at a time"
    );
    let result = wait_for_pick(&gpu, &mut target);
    assert!(!target.is_picking());
    assert_eq!(result.all(), std::collections::BTreeSet::from([7, 9]));
    // Both copies are found on their own side, with sky between them.
    let row = 36;
    let left = (0..32)
        .filter_map(|x| result.pick_at(x, row))
        .collect::<Vec<_>>();
    let right = (32..64)
        .filter_map(|x| result.pick_at(x, row))
        .collect::<Vec<_>>();
    assert!(!left.is_empty() && left.iter().all(|&p| p == 7), "{left:?}");
    assert!(
        !right.is_empty() && right.iter().all(|&p| p == 9),
        "{right:?}"
    );
    assert_eq!(result.pick_at(0, 0), None, "sky at the corner");
    // The point found is on the left copy, in front of the camera.
    let x = (0..32).find(|&x| result.pick_at(x, row).is_some()).unwrap();
    let point = result.point_at(x, row).unwrap();
    assert!(
        point.x < -1.0 && point.y > eye.y && point.y < 4.0,
        "{point}"
    );

    // A small rectangle is drawn alone: only what's inside it is found.
    assert!(renderer.request_pick(
        &gpu.device,
        &gpu.queue,
        &mut target,
        view_projection,
        &placed,
        [x, row, 1, 1],
    ));
    let one = wait_for_pick(&gpu, &mut target);
    assert_eq!(one.rect, [x, row, 1, 1]);
    assert_eq!(one.pick_at(x, row), Some(7));
    assert_eq!(one.all().len(), 1);
}

/// The camera at `eye` looking at `at`, Z up, as the editor's camera sees:
/// 50° tall, from 5 cm to 5 km.
fn looking_at(eye: Vec3, at: Vec3) -> Mat4 {
    let (near, far) = (0.05_f32, 5_000.0_f32);
    let scale = 1.0 / (25.0_f32.to_radians()).tan();
    let projection = Mat4::from_cols(
        Vec4::new(scale, 0.0, 0.0, 0.0),
        Vec4::new(0.0, scale, 0.0, 0.0),
        Vec4::new(0.0, 0.0, near / (far - near), -1.0),
        Vec4::new(0.0, 0.0, far * near / (far - near), 0.0),
    );
    let f = (at - eye).normalize();
    let r = f.cross(Vec3::Z).normalize();
    let u = r.cross(f);
    let view = Mat4::from_cols(
        Vec4::new(r.x, u.x, -f.x, 0.0),
        Vec4::new(r.y, u.y, -f.y, 0.0),
        Vec4::new(r.z, u.z, -f.z, 0.0),
        Vec4::new(-r.dot(eye), -u.dot(eye), f.dot(eye), 1.0),
    );
    projection * view
}

#[test]
fn the_ground_axes_stay_thin_lines_close_up() {
    // Regression: each axis was one line from end to end of the grid. Seen
    // from close by (as a Library thumbnail is), Mesa's Vulkan driver drew
    // the line passing behind the camera as a filled green wedge.
    let Some(gpu) = gpu() else {
        return;
    };
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let size = 128;
    let mut target = renderer.create_target(&gpu.device, [size, size]);
    let eye = Vec3::new(5.2, -7.8, 5.7);
    renderer.render(
        &gpu.device,
        &gpu.queue,
        &mut target,
        &FrameParams {
            view_projection: looking_at(eye, Vec3::Z),
            camera_position: eye,
            focus: Vec3::ZERO,
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
        &[],
        &[],
    );
    let pixels = read_pixels(&gpu.device, &gpu.queue, &target).unwrap();
    let green = pixels
        .chunks(4)
        .filter(|p| p[1] > p[0].saturating_add(40) && p[1] > p[2].saturating_add(40))
        .count();
    assert!(green > 0, "the Y axis is drawn");
    assert!(
        green < (size * size / 20) as usize,
        "{green} green pixels: the axis is a line, not a wedge"
    );
}

#[test]
fn a_frame_says_what_it_drew_and_whether_its_passes_are_timed() {
    let Some(gpu) = gpu() else {
        return;
    };
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let mut target = renderer.create_target(&gpu.device, [64, 64]);
    let model = sample();
    let uploaded = renderer.upload_model(&gpu.device, &gpu.queue, &model);
    let eye = Vec3::new(0.0, -6.0, 1.1);
    let copies: Vec<Placement<'_>> = (0..3)
        .map(|i| Placement {
            model: &uploaded,
            node: None,
            transform: Mat4::from_translation(Vec3::new(0.0, i as f32 * 3.0, 0.0)),
            pick: 0,
        })
        .collect();
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
    assert_eq!(target.can_time(), None, "not known before a frame");
    let one = renderer.render(
        &gpu.device,
        &gpu.queue,
        &mut target,
        &params,
        &copies[..1],
        &[],
    );
    assert_eq!(one.triangles, model.triangles() as u64);
    let stats = renderer.render(&gpu.device, &gpu.queue, &mut target, &params, &copies, &[]);
    assert_eq!(stats.instances, 3 * one.instances);
    assert_eq!(stats.triangles, 3 * one.triangles);
    // Without shadows (whose copies may split a mesh's draw in two), the
    // copies share their draw calls.
    let unshadowed = FrameParams {
        sun: Sun {
            shadows: false,
            ..Sun::default()
        },
        ..params
    };
    let alone = renderer.render(
        &gpu.device,
        &gpu.queue,
        &mut target,
        &unshadowed,
        &copies[..1],
        &[],
    );
    let all = renderer.render(
        &gpu.device,
        &gpu.queue,
        &mut target,
        &unshadowed,
        &copies,
        &[],
    );
    assert_eq!(all.batches, alone.batches, "copies share their draw calls");
    assert_eq!(all.casting, 0);
    assert!(stats.casting >= stats.instances, "what's seen casts here");
    let can_time = target.can_time().expect("known after a frame");
    println!("This card times its passes: {can_time}");
    assert_eq!(
        can_time,
        gpu.device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY)
    );
    // Times come back a frame or two later, never waited for; on a card
    // without timestamps, never.
    let _ = read_pixels(&gpu.device, &gpu.queue, &target);
    let mut times = None;
    for _ in 0..10 {
        renderer.render(&gpu.device, &gpu.queue, &mut target, &params, &copies, &[]);
        let _ = read_pixels(&gpu.device, &gpu.queue, &target);
        times = target.gpu_times(&gpu.device);
    }
    assert_eq!(times.is_some(), can_time);
    if let Some(t) = times {
        assert!(t.total() >= 0.0 && t.total() < 1000.0, "{t:?}");
    }
}

#[test]
fn a_frame_is_the_same_on_one_core_or_many() {
    // Decision 0031: gathering the copies in jobs never changes the
    // picture.
    let Some(gpu) = gpu() else {
        return;
    };
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let model = sample();
    let uploaded = renderer.upload_model(&gpu.device, &gpu.queue, &model);
    let copies: Vec<Placement<'_>> = (0..400)
        .map(|i| Placement {
            model: &uploaded,
            node: None,
            transform: Mat4::from_translation(Vec3::new(
                (i % 20) as f32 * 2.5 - 25.0,
                (i / 20) as f32 * 2.5 + 4.0,
                0.0,
            )),
            pick: 0,
        })
        .collect();
    let eye = Vec3::new(0.0, -10.0, 8.0);
    let params = FrameParams {
        view_projection: looking_north(eye),
        camera_position: eye,
        focus: Vec3::ZERO,
        sky_top: srgb(SKY_TOP),
        sky_horizon: srgb(SKY_HORIZON),
        ground: srgb(GROUND),
        shading: Shading::default(),
        size_px: [96, 96],
        sun: Sun::default(),
        environment: unmapped(),
        editor_lines: false,
        grid_plane: GridPlane::Ground,
        grid_step: 1.0,
    };
    let draw = |cores: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(cores)
            .build()
            .unwrap()
            .install(|| {
                let mut target = renderer.create_target(&gpu.device, [96, 96]);
                let stats =
                    renderer.render(&gpu.device, &gpu.queue, &mut target, &params, &copies, &[]);
                (
                    stats,
                    read_pixels(&gpu.device, &gpu.queue, &target).unwrap(),
                )
            })
    };
    let (one_stats, one) = draw(1);
    let (many_stats, many) = draw(4);
    assert_eq!(one_stats, many_stats);
    assert!(one == many, "the pictures differ");
}
