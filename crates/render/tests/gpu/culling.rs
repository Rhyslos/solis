//! Leaving out what's out of view, shadows from outside it, and frames that send nothing new.

use super::*;

/// Renders each (model, place) seen by `view_projection` from `eye`, lit
/// by `sun`, 128 pixels square; the pixels and what was drawn.
pub(crate) fn render_scene(
    scene: &[(&assets::Model, Mat4)],
    view_projection: Mat4,
    eye: Vec3,
    sun: Sun,
) -> Option<(Vec<u8>, render::FrameStats)> {
    let gpu = gpu()?;
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let size = 128;
    let mut target = renderer.create_target(&gpu.device, [size, size]);
    let uploaded: Vec<_> = scene
        .iter()
        .map(|(model, _)| renderer.upload_model(&gpu.device, &gpu.queue, model))
        .collect();
    let placed: Vec<Placement<'_>> = scene
        .iter()
        .zip(&uploaded)
        .map(|((_, transform), model)| Placement {
            model,
            node: None,
            transform: *transform,
            pick: 0,
        })
        .collect();
    let stats = renderer.render(
        &gpu.device,
        &gpu.queue,
        &mut target,
        &FrameParams {
            view_projection,
            camera_position: eye,
            focus: Vec3::new(1.0e7, 1.0e7, 0.0),
            sky_top: srgb(SKY_TOP),
            sky_horizon: srgb(SKY_HORIZON),
            ground: srgb(GROUND),
            shading: Shading::default(),
            size_px: [size, size],
            sun,
            environment: unmapped(),
            editor_lines: true,
            grid_plane: GridPlane::Ground,
            grid_step: 1.0,
        },
        &placed,
        &[],
    );
    Some((
        read_pixels(&gpu.device, &gpu.queue, &target).unwrap(),
        stats,
    ))
}

#[test]
fn copies_out_of_view_are_not_drawn_and_change_nothing() {
    let eye = Vec3::new(0.0, -6.0, 1.1);
    let model = sample();
    let mut scene = vec![(&model, Mat4::IDENTITY)];
    let Some((_, one)) = render_scene(&scene, looking_north(eye), eye, Sun::default()) else {
        return;
    };
    // A thousand more behind the camera, and far off to the sides.
    for i in 0..1000 {
        let x = if i % 2 == 0 { -400.0 } else { 400.0 };
        scene.push((
            &model,
            Mat4::from_translation(Vec3::new(x, -50.0 - i as f32, 0.0)),
        ));
    }
    let (_, all) = render_scene(&scene, looking_north(eye), eye, Sun::default()).unwrap();
    assert_eq!(
        all.instances, one.instances,
        "only the one in view is drawn"
    );
    assert_eq!(all.triangles, one.triangles);
    assert_eq!(
        all.casting, one.casting,
        "their shadows can't reach the view"
    );
    // The same picture (without shadows, whose map covers more of the
    // world now, so is a little less sharp).
    let unshadowed = Sun {
        shadows: false,
        ..Sun::default()
    };
    let (alone, _) = render_scene(&scene[..1], looking_north(eye), eye, unshadowed).unwrap();
    let (crowded, _) = render_scene(&scene, looking_north(eye), eye, unshadowed).unwrap();
    assert!(alone == crowded, "the same picture");
}

#[test]
fn a_copy_at_the_edge_of_the_view_is_still_drawn() {
    // A big square mostly off the left edge of a view looking down from
    // 10 m: only a sliver shows, and it must.
    let eye = Vec3::new(0.0, 0.0, 10.0);
    let mut edge = floor(false);
    edge.meshes = vec![square(-8.0, 1.0, 3.0)];
    edge.show_every_mesh();
    let Some((with, stats)) = render_scene(
        &[(&edge, Mat4::IDENTITY)],
        looking_down(eye),
        eye,
        Sun::default(),
    ) else {
        return;
    };
    assert_eq!(stats.instances, 1);
    let (without, _) = render_scene(&[], looking_down(eye), eye, Sun::default()).unwrap();
    let size = 128;
    let column_differs = (0..size).any(|y| pixel(&with, size, 0, y) != pixel(&without, size, 0, y));
    assert!(column_differs, "the sliver shows at the left edge");
}

#[test]
fn a_shadow_from_outside_the_view_falls_into_it() {
    // Looking down from 10 m (about 5.8 m each way on the ground), the sun
    // slanting towards +X: a square 6 m up at x = -8, out of view, throws
    // its shadow onto the floor at x = -2, in view.
    let eye = Vec3::new(0.0, 0.0, 10.0);
    let ground = floor(false);
    let mut roof = floor(false);
    roof.meshes = vec![square(-8.0, 6.0, 0.8)];
    roof.show_every_mesh();
    let scene = [(&ground, Mat4::IDENTITY), (&roof, Mat4::IDENTITY)];
    let Some((with, stats)) = render_scene(&scene, looking_down(eye), eye, slanted_sun(true))
    else {
        return;
    };
    assert_eq!(stats.instances, 1, "the roof itself isn't in view");
    assert_eq!(stats.casting, 2, "but it casts into it");
    let size = 128;
    let shadow = ground_pixel(&with, size, -2.0, 0.0);
    let open = ground_pixel(&with, size, 2.5, 0.0);
    assert!(
        brightness(shadow) + 60 < brightness(open),
        "shadow {shadow:?} not darker than open ground {open:?}"
    );
}

#[test]
fn a_frame_that_did_not_change_sends_nothing() {
    let Some(gpu) = gpu() else {
        return;
    };
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let mut target = renderer.create_target(&gpu.device, [32, 32]);
    let model = sample();
    let uploaded = renderer.upload_model(&gpu.device, &gpu.queue, &model);
    let eye = Vec3::new(0.0, -10.0, 8.0);
    let mut copies: Vec<Placement<'_>> = (0..2000)
        .map(|i| Placement {
            model: &uploaded,
            node: None,
            transform: Mat4::from_translation(Vec3::new(
                (i % 40) as f32 * 3.0 - 60.0,
                (i / 40) as f32 * 3.0,
                0.0,
            )),
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
        size_px: [32, 32],
        sun: Sun::default(),
        environment: unmapped(),
        editor_lines: false,
        grid_plane: GridPlane::Ground,
        grid_step: 1.0,
    };
    let first = renderer.render(&gpu.device, &gpu.queue, &mut target, &params, &copies, &[]);
    assert!(first.sent_bytes > 0);
    let first_pixels = read_pixels(&gpu.device, &gpu.queue, &target).unwrap();
    let again = renderer.render(&gpu.device, &gpu.queue, &mut target, &params, &copies, &[]);
    assert_eq!(again.sent_bytes, 0, "nothing moved");
    // One copy in view moves a little, staying in view: one run of copies
    // is sent for each of its meshes, not all of them. (A copy coming
    // into view or leaving it moves the ones after it in the buffer, and
    // those are sent too.)
    let home = copies[420].transform;
    copies[420].transform = Mat4::from_translation(Vec3::new(0.5, 30.0, 0.0));
    let moved = renderer.render(&gpu.device, &gpu.queue, &mut target, &params, &copies, &[]);
    assert_eq!(moved.instances, first.instances);
    assert!(
        moved.sent_bytes > 0 && moved.sent_bytes * 3 < first.sent_bytes,
        "{moved:?} vs {first:?}"
    );
    // And back: the picture is as it was.
    copies[420].transform = home;
    renderer.render(&gpu.device, &gpu.queue, &mut target, &params, &copies, &[]);
    assert!(read_pixels(&gpu.device, &gpu.queue, &target).unwrap() == first_pixels);
}
