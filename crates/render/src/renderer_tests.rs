//! The renderer's own tests: what a target makes only once it draws it,
//! sprites and model copies too many to draw, and the lines' layout.

use super::*;

/// A device to draw on; none skips the test, unless `REQUIRE_GPU` is
/// set (as in tests/gpu).
fn gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
    gpu_with(|limits| limits)
}

/// [`gpu`] with the adapter's limits changed by `limits`.
fn gpu_with(
    limits: impl FnOnce(wgpu::Limits) -> wgpu::Limits,
) -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle().with_env());
    let found =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .ok()
            .and_then(|adapter| {
                let descriptor = wgpu::DeviceDescriptor {
                    required_limits: limits(adapter.limits()),
                    ..wgpu::DeviceDescriptor::default()
                };
                pollster::block_on(adapter.request_device(&descriptor)).ok()
            });
    if found.is_none() {
        assert!(
            std::env::var_os("REQUIRE_GPU").is_none(),
            "REQUIRE_GPU is set but no GPU adapter was found"
        );
    }
    found
}

#[test]
fn a_target_makes_its_shadow_map_and_lamp_markers_only_once_it_draws_them() {
    // A layer's picture or the game's view never draws shadows or
    // marks lamps, so it never pays for the 16 MiB shadow map or the
    // markers' buffer.
    let Some((device, queue)) = gpu() else {
        return;
    };
    let renderer = ViewportRenderer::new(&device, 1);
    let mut target = renderer.create_target(&device, [32, 32]);
    assert!(target.frame.shadow_view.is_none());
    assert!(target.frame.markers.is_none());
    let model = assets::load_bytes(
        &assets::sample::sample_glb(),
        None,
        "Sample",
        &assets::Limits::default(),
    )
    .unwrap();
    let model = renderer.upload_model(&device, &queue, &model);
    let placed = [Placement {
        model: &model,
        node: None,
        transform: glam::Mat4::IDENTITY,
        pick: 0,
    }];
    let lamp = Light {
        kind: crate::lighting::LightKind::Point,
        position: Vec3::new(1.0, 1.0, 2.0),
        direction: -Vec3::Z,
        color: [1.0; 3],
        strength: 100.0,
        range: None,
    };
    // Looking north from 6 m back, 60° tall, depth reversed as every
    // pipeline has it (1 near, 0 far).
    let eye = Vec3::new(0.0, -6.0, 1.0);
    let (near, far, scale) = (0.1, 1000.0, 1.0 / 30.0_f32.to_radians().tan());
    let projection = glam::Mat4::from_cols(
        glam::Vec4::new(scale, 0.0, 0.0, 0.0),
        glam::Vec4::new(0.0, scale, 0.0, 0.0),
        glam::Vec4::new(0.0, 0.0, near / (far - near), -1.0),
        glam::Vec4::new(0.0, 0.0, far * near / (far - near), 0.0),
    );
    let view = glam::Mat4::from_cols(
        glam::Vec4::new(1.0, 0.0, 0.0, 0.0),
        glam::Vec4::new(0.0, 0.0, -1.0, 0.0),
        glam::Vec4::new(0.0, 1.0, 0.0, 0.0),
        glam::Vec4::new(-eye.x, -eye.z, eye.y, 1.0),
    );
    let mut params = FrameParams {
        view_projection: projection * view,
        camera_position: eye,
        focus: Vec3::ZERO,
        sky_top: [0.3; 3],
        sky_horizon: [0.3; 3],
        ground: [0.2; 3],
        shading: crate::frame::Shading {
            look: crate::frame::Look::Unlit,
            ..crate::frame::Shading::default()
        },
        size_px: [32, 32],
        sun: crate::lighting::Sun::default(),
        environment: crate::environment::Environment::default(),
        editor_lines: false,
        grid_plane: crate::frame::GridPlane::Ground,
        grid_step: 1.0,
    };
    target.set_layered(true);
    renderer.render(&device, &queue, &mut target, &params, &placed, &[lamp]);
    assert!(target.frame.shadow_view.is_none(), "unlit: no shadows");
    assert!(
        target.frame.markers.is_none(),
        "no editor lines: no markers"
    );
    // The editor's own view, lit: both, kept from then on.
    target.set_layered(false);
    params.shading.look = crate::frame::Look::Standard;
    params.editor_lines = true;
    renderer.render(&device, &queue, &mut target, &params, &placed, &[lamp]);
    assert!(target.frame.shadow_view.is_some(), "shadows drawn");
    assert!(target.frame.markers.is_some(), "a lamp marked");
    params.sun.shadows = false;
    renderer.render(&device, &queue, &mut target, &params, &placed, &[]);
    assert!(target.frame.shadow_view.is_some());
}

#[test]
fn sprites_too_many_to_draw_are_not_counted_and_warned_about_once() {
    // A card that holds 64 KiB in one buffer: 744 sprites at most.
    let Some((device, queue)) = gpu_with(|limits| wgpu::Limits {
        max_buffer_size: 1 << 16,
        ..limits
    }) else {
        return;
    };
    let renderer = ViewportRenderer::new(&device, 1);
    let mut target = renderer.create_target(&device, [16, 16]);
    let sheet = renderer.upload_sheet(&device, &queue, [1, 1], &[255; 4]);
    let sprite = crate::sprites::SpriteInstance {
        corner: [0.0; 3],
        flags: 0,
        axis_x: [0.01, 0.0],
        axis_y: [0.0, -0.01],
        rect: [0.0, 0.0, 1.0, 1.0],
        tint: [1.0; 4],
        outline: [0.0; 4],
        foot: [0.0; 2],
    };
    let size = std::mem::size_of::<crate::sprites::SpriteInstance>() as u64;
    let params = FrameParams {
        view_projection: glam::Mat4::IDENTITY,
        camera_position: Vec3::Z,
        focus: Vec3::ZERO,
        sky_top: [0.3; 3],
        sky_horizon: [0.3; 3],
        ground: [0.2; 3],
        shading: crate::frame::Shading::default(),
        size_px: [16, 16],
        sun: crate::lighting::Sun::default(),
        environment: crate::environment::Environment::default(),
        editor_lines: false,
        grid_plane: crate::frame::GridPlane::Ground,
        grid_step: 1.0,
    };
    let mut draw = |count: usize| {
        let instances = vec![sprite; count];
        let runs = [crate::sprites::SpriteRun {
            sheet: &sheet,
            instances: &instances,
        }];
        let scene = Scene {
            sprites: &runs,
            ..Scene::default()
        };
        let stats = renderer.render_scene(&device, &queue, &mut target, &params, &scene);
        (stats, target.frame.sprites_dropped)
    };
    let (stats, dropped) = draw(10);
    assert_eq!((stats.sprites, stats.sent_bytes), (10, 10 * size));
    assert!(!dropped);
    // Too many: none drawn, none counted, none sent; the warning is
    // logged on the first such frame only.
    for _ in 0..2 {
        let (stats, dropped) = draw(1000);
        assert_eq!((stats.sprites, stats.sent_bytes), (0, 0));
        assert!(dropped);
    }
    // Fitting again, and a warning again when they next don't.
    let (stats, dropped) = draw(700);
    assert_eq!((stats.sprites, stats.sent_bytes), (700, 700 * size));
    assert!(!dropped);
}

#[test]
fn model_copies_too_many_to_draw_are_warned_about_once() {
    // The second review pass: the warning was logged every frame. A card
    // that holds 64 KiB in one buffer: a few hundred copies at most.
    let Some((device, queue)) = gpu_with(|limits| wgpu::Limits {
        max_buffer_size: 1 << 16,
        ..limits
    }) else {
        return;
    };
    let renderer = ViewportRenderer::new(&device, 1);
    let mut target = renderer.create_target(&device, [16, 16]);
    let model = assets::load_bytes(
        &assets::sample::sample_glb(),
        None,
        "Sample",
        &assets::Limits::default(),
    )
    .unwrap();
    let model = renderer.upload_model(&device, &queue, &model);
    let params = FrameParams {
        view_projection: glam::Mat4::IDENTITY,
        camera_position: Vec3::Z,
        focus: Vec3::ZERO,
        sky_top: [0.3; 3],
        sky_horizon: [0.3; 3],
        ground: [0.2; 3],
        shading: crate::frame::Shading {
            look: crate::frame::Look::Unlit,
            ..crate::frame::Shading::default()
        },
        size_px: [16, 16],
        sun: crate::lighting::Sun::default(),
        environment: crate::environment::Environment::default(),
        editor_lines: false,
        grid_plane: crate::frame::GridPlane::Ground,
        grid_step: 1.0,
    };
    let mut draw = |count: usize| {
        let placed: Vec<Placement> = (0..count)
            .map(|i| Placement {
                model: &model,
                node: None,
                transform: glam::Mat4::from_translation(Vec3::new(i as f32 * 1e-4, 0.0, 0.0)),
                pick: 0,
            })
            .collect();
        renderer.render(&device, &queue, &mut target, &params, &placed, &[]);
        target.frame.copies_dropped
    };
    assert!(!draw(10));
    // Too many: dropped, and warned about on the first such frame only.
    assert!(draw(5000));
    assert!(draw(5000));
    // Fitting again, and a warning again when they next don't.
    assert!(!draw(10));
    assert!(draw(5000));
}

#[test]
fn line_vertices_match_the_shader_layout() {
    assert_eq!(std::mem::size_of::<LineVertex>(), 7 * 4);
    assert_eq!(LineVertex::LAYOUT.attributes.len(), 2);
}

#[test]
fn the_axes_lie_on_the_ground_through_the_origin() {
    let axes = ground_axes();
    assert!(axes.iter().all(|v| v.position[2] == 0.0));
    let (x, y): (Vec<&LineVertex>, Vec<&LineVertex>) =
        axes.iter().partition(|v| v.color == AXIS_X_COLOR);
    assert!(x.iter().all(|v| v.position[1] == 0.0), "X runs along y = 0");
    assert!(y.iter().all(|v| v.color == AXIS_Y_COLOR));
    assert!(y.iter().all(|v| v.position[0] == 0.0), "Y runs along x = 0");
    // From one end of the grid to the other, in short pieces.
    let ends = |line: &[&LineVertex], i: usize| {
        line.iter()
            .map(|v| v.position[i])
            .fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p), hi.max(p)))
    };
    assert_eq!(ends(&x, 0), (-GRID_HALF_EXTENT, GRID_HALF_EXTENT));
    assert_eq!(ends(&y, 1), (-GRID_HALF_EXTENT, GRID_HALF_EXTENT));
    for pair in axes.chunks(2) {
        let length = Vec3::from(pair[0].position).distance(Vec3::from(pair[1].position));
        assert!(length <= AXIS_PIECE + 1e-3);
    }
}
