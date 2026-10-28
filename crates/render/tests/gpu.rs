//! Renders real frames on a GPU and checks the pixels.
//!
//! Machines without a GPU skip these tests with a note, unless the
//! environment variable `REQUIRE_GPU` is set, in which case a missing GPU is
//! a failure. The CI window job sets it and installs a software driver, so
//! these tests always run somewhere.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stderr)]

use glam::{Mat4, Vec3, Vec4};
use render::{
    Environment, Fog, FrameParams, GridPlane, Light, LightKind, Look, PixelScaling, Placement,
    Shading, Sky, Sun, Tonemap, ViewportRenderer, best_sample_count, pixel_layout, read_pixels,
    read_texture,
};

/// The editor's backdrop, sRGB 0 to 255 (as in the editor theme).
const SKY_TOP: [u8; 3] = [0x16, 0x18, 0x1d];
const SKY_HORIZON: [u8; 3] = [0x1c, 0x1e, 0x23];
const GROUND: [u8; 3] = [0x1e, 0x21, 0x26];
/// The ground straight down: 15% lighter than at the horizon.
const GROUND_BELOW: [u8; 3] = [0x23, 0x26, 0x2c];

fn srgb(c: [u8; 3]) -> [f32; 3] {
    c.map(|v| f32::from(v) / 255.0)
}

struct Gpu {
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// Held while a test uses the GPU: tests take turns, since some
    /// software drivers (as on CI) can stall with several devices busy at
    /// once.
    _turn: std::sync::MutexGuard<'static, ()>,
}

/// Whose turn it is to use the GPU.
static GPU_TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn gpu() -> Option<Gpu> {
    // A test that failed while holding the turn still lets the others run.
    let turn = GPU_TURN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle().with_env());
    let found =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .ok()
            .and_then(|adapter| {
                // Timestamps too, when the card has them, as the editor
                // asks (for the Profiler).
                let descriptor = wgpu::DeviceDescriptor {
                    required_features: adapter.features() & wgpu::Features::TIMESTAMP_QUERY,
                    ..wgpu::DeviceDescriptor::default()
                };
                let (device, queue) =
                    pollster::block_on(adapter.request_device(&descriptor)).ok()?;
                Some(Gpu {
                    adapter,
                    device,
                    queue,
                    _turn: turn,
                })
            });
    if found.is_none() {
        assert!(
            std::env::var_os("REQUIRE_GPU").is_none(),
            "REQUIRE_GPU is set but no GPU adapter was found"
        );
        eprintln!("No GPU adapter available; skipping GPU rendering test.");
    }
    found
}

/// A 60° tall perspective for a square image, with depth reversed as the
/// editor's camera has it (1 near, 0 far).
fn projection() -> Mat4 {
    let (near, far) = (0.1, 1000.0);
    let scale = 1.0 / (30.0_f32.to_radians()).tan();
    Mat4::from_cols(
        Vec4::new(scale, 0.0, 0.0, 0.0),
        Vec4::new(0.0, scale, 0.0, 0.0),
        Vec4::new(0.0, 0.0, near / (far - near), -1.0),
        Vec4::new(0.0, 0.0, far * near / (far - near), 0.0),
    )
}

/// A camera at `eye` looking straight along +Y (level with the horizon),
/// 60° tall view, square image.
fn looking_north(eye: Vec3) -> Mat4 {
    // Right is +X, up is +Z, forward is +Y (camera looks down its -Z).
    let view = Mat4::from_cols(
        Vec4::new(1.0, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, -1.0, 0.0),
        Vec4::new(0.0, 1.0, 0.0, 0.0),
        Vec4::new(-eye.x, -eye.z, eye.y, 1.0),
    );
    projection() * view
}

fn pixel(pixels: &[u8], size: u32, x: u32, y: u32) -> [u8; 3] {
    let at = ((y * size + x) * 4) as usize;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

fn between(value: [u8; 3], low: [u8; 3], high: [u8; 3]) -> bool {
    (0..3).all(|i| value[i] + 3 >= low[i].min(high[i]) && value[i] <= low[i].max(high[i]) + 3)
}

#[test]
fn the_backdrop_shows_sky_above_and_ground_below_in_the_right_colours() {
    let Some(gpu) = gpu() else { return };
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let size = 64;
    let mut target = renderer.create_target(&gpu.device, [size, size]);
    let eye = Vec3::new(0.0, 0.0, 2.0);
    renderer.render(
        &gpu.device,
        &gpu.queue,
        &mut target,
        &FrameParams {
            view_projection: looking_north(eye),
            camera_position: eye,
            // Far away, so no grid lines land on the pixels checked.
            // The axes are still drawn.
            focus: Vec3::new(1.0e7, 1.0e7, 0.0),
            sky_top: srgb(SKY_TOP),
            sky_horizon: srgb(SKY_HORIZON),
            ground: srgb(GROUND),
            shading: Shading::default(),
            size_px: [64, 64],
            sun: Sun::default(),
            environment: Environment::default(),
            editor_lines: true,
            grid_plane: GridPlane::Ground,
            grid_step: 1.0,
        },
        &[],
        &[],
    );
    let pixels = read_pixels(&gpu.device, &gpu.queue, &target).unwrap();
    assert_eq!(pixels.len(), (size * size * 4) as usize);

    // The top row is sky, between the horizon and top colours. Written as
    // stored sRGB values: if the shader forgot to encode, these would come
    // out far darker.
    let top = pixel(&pixels, size, size / 2, 0);
    assert!(between(top, SKY_HORIZON, SKY_TOP), "top pixel {top:?}");
    // The bottom row is ground, lightening towards the feet. (Off
    // centre: the green Y axis runs straight ahead down the middle.)
    let bottom = pixel(&pixels, size, size / 4, size - 1);
    assert!(
        between(bottom, GROUND, GROUND_BELOW),
        "bottom pixel {bottom:?}"
    );
    assert!(
        top[2] < bottom[2],
        "sky {top:?} not darker than ground {bottom:?}"
    );
}

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

/// Renders a model in `shading` and returns the pixels of a `size`-pixel
/// square, looking from `eye` along +Y, level.
fn render_with(
    model: &assets::Model,
    eye: Vec3,
    shading: Shading,
    size: u32,
) -> Option<(Vec<u8>, u32)> {
    render_lit(
        model,
        looking_north(eye),
        eye,
        shading,
        size,
        Sun::default(),
        &[],
    )
}

/// Renders a model seen through `view_projection` from `eye`, lit by `sun`
/// and `lights`, and returns the pixels of a `size`-pixel square.
fn render_lit(
    model: &assets::Model,
    view_projection: Mat4,
    eye: Vec3,
    shading: Shading,
    size: u32,
    sun: Sun,
    lights: &[Light],
) -> Option<(Vec<u8>, u32)> {
    render_env(
        model,
        view_projection,
        eye,
        shading,
        size,
        sun,
        lights,
        unmapped(),
    )
}

/// No tone mapping: the lighting tests check light itself, so colours past
/// white are cut off as before (tone mapping has tests of its own).
fn unmapped() -> Environment {
    Environment {
        tonemap: Tonemap::Standard,
        ..Environment::default()
    }
}

/// As [`render_lit`], with the sky, fog and tone mapping in `environment`.
#[allow(clippy::too_many_arguments)]
fn render_env(
    model: &assets::Model,
    view_projection: Mat4,
    eye: Vec3,
    shading: Shading,
    size: u32,
    sun: Sun,
    lights: &[Light],
    environment: Environment,
) -> Option<(Vec<u8>, u32)> {
    let gpu = gpu()?;
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let mut target = renderer.create_target(&gpu.device, [size, size]);
    let uploaded = renderer.upload_model(&gpu.device, &gpu.queue, model);
    assert_eq!(uploaded.triangles(), model.triangles());
    renderer.render(
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
            shading,
            size_px: [size, size],
            sun,
            environment,
            editor_lines: true,
            grid_plane: GridPlane::Ground,
            grid_step: 1.0,
        },
        &[Placement {
            model: &uploaded,
            node: None,
            transform: glam::Mat4::IDENTITY,
            pick: 0,
        }],
        lights,
    );
    Some((read_pixels(&gpu.device, &gpu.queue, &target).unwrap(), size))
}

/// Renders a model in the standard look, 64 pixels square.
fn render_model(model: &assets::Model, eye: Vec3) -> Option<(Vec<u8>, u32)> {
    render_with(model, eye, Shading::default(), 64)
}

fn sample() -> assets::Model {
    assets::load_bytes(
        &assets::sample::sample_glb(),
        None,
        "Sample",
        &assets::Limits::default(),
    )
    .unwrap()
}

#[test]
fn an_imported_model_is_drawn_with_its_texture_and_light() {
    // 6 m in front of the sample's cube (which is 1.1 m up, Z up in
    // Solis), level with its middle.
    let Some((pixels, size)) = render_model(&sample(), Vec3::new(0.0, -6.0, 1.1)) else {
        return;
    };
    // The middle of the picture is the cube's front face: the checker's
    // light and blue squares, lit, far brighter than the dark backdrop.
    let mut light = 0;
    let mut blue = 0;
    for y in size / 2 - 6..size / 2 + 6 {
        for x in size / 2 - 6..size / 2 + 6 {
            let [r, g, b] = pixel(&pixels, size, x, y);
            if r > 100 && g > 100 && b > 100 {
                light += 1;
            }
            if b > r + 30 {
                blue += 1;
            }
        }
    }
    assert!(light > 10 && blue > 10, "light {light}, blue {blue}");
    // The top-left corner is still sky: the cube doesn't fill the view.
    let corner = pixel(&pixels, size, 1, 1);
    assert!(between(corner, SKY_HORIZON, SKY_TOP), "corner {corner:?}");
}

#[test]
fn the_red_base_shows_below_the_cube() {
    // From above the base's edge, looking level: the base is the band just
    // below the cube.
    let Some((pixels, size)) = render_model(&sample(), Vec3::new(0.0, -8.0, 0.6)) else {
        return;
    };
    let reddish = (size / 2..size)
        .map(|y| pixel(&pixels, size, size / 2, y))
        .any(|[r, g, b]| r > g + 40 && r > b + 40);
    assert!(reddish, "no red base");
}

#[test]
fn a_model_behind_the_camera_draws_nothing() {
    let Some((pixels, size)) = render_model(&sample(), Vec3::new(0.0, 10.0, 1.1)) else {
        return;
    };
    let middle = pixel(&pixels, size, size / 2, size / 4);
    assert!(between(middle, SKY_HORIZON, SKY_TOP), "{middle:?}");
}

fn showcase() -> assets::Model {
    assets::load_bytes(
        &assets::sample::showcase_glb(),
        None,
        "Showcase",
        &assets::Limits::default(),
    )
    .unwrap()
}

fn shading(look: Look) -> Shading {
    Shading {
        look,
        bands: 3,
        outline_px: 2.0,
    }
}

#[test]
fn the_unlit_look_shows_the_texture_as_it_is() {
    let Some((pixels, size)) = render_with(
        &sample(),
        Vec3::new(0.0, -3.0, 1.1),
        shading(Look::Unlit),
        128,
    ) else {
        return;
    };
    let near = |p: [u8; 3], want: [u8; 3]| (0..3).all(|i| p[i].abs_diff(want[i]) <= 12);
    let middle: Vec<[u8; 3]> = (size / 2 - 16..size / 2 + 16)
        .flat_map(|y| (size / 2 - 16..size / 2 + 16).map(move |x| (x, y)))
        .map(|(x, y)| pixel(&pixels, size, x, y))
        .collect();
    // The checker's two colours, exactly as painted.
    assert!(
        middle.iter().any(|&p| near(p, [235, 235, 235])),
        "no light squares"
    );
    assert!(
        middle.iter().any(|&p| near(p, [40, 110, 150])),
        "no blue squares"
    );
}

#[test]
fn the_stylized_look_draws_dark_outlines_and_the_standard_one_does_not() {
    // A little to the side and above, as a model is usually seen (seen
    // exactly head-on, a box's sides are edge-on and have no outline).
    let eye = Vec3::new(1.5, -6.0, 2.6);
    let dark = |pixels: &[u8], size: u32| {
        (0..size * size)
            .map(|i| pixel(pixels, size, i % size, i / size))
            .filter(|p| p.iter().all(|&c| c < 10))
            .count()
    };
    let Some((stylized, size)) = render_with(&sample(), eye, shading(Look::Stylized), 64) else {
        return;
    };
    let Some((standard, _)) = render_with(&sample(), eye, shading(Look::Standard), 64) else {
        return;
    };
    assert!(
        dark(&stylized, size) > 40,
        "outlines: {}",
        dark(&stylized, size)
    );
    assert_eq!(dark(&standard, size), 0);
    // Without outlines, the stylized look has none either.
    let plain = Shading {
        outline_px: 0.0,
        ..shading(Look::Stylized)
    };
    let Some((no_outline, _)) = render_with(&sample(), eye, plain, 64) else {
        return;
    };
    assert_eq!(dark(&no_outline, size), 0);
}

#[test]
fn glowing_parts_glow_in_every_look() {
    for look in [Look::Standard, Look::Stylized, Look::Unlit] {
        let Some((pixels, size)) =
            render_with(&showcase(), Vec3::new(0.0, -8.0, 2.0), shading(look), 128)
        else {
            return;
        };
        let orange = (0..size * size)
            .map(|i| pixel(&pixels, size, i % size, i / size))
            .filter(|&[r, g, b]| r > 200 && (110..235).contains(&g) && r > b.saturating_add(80))
            .count();
        assert!(orange > 20, "{look:?}: {orange} glowing pixels");
    }
}

#[test]
fn the_normal_texture_shapes_the_light_on_the_wall() {
    let eye = Vec3::new(0.0, -8.0, 1.5);
    let with = showcase();
    let mut without = showcase();
    for material in &mut without.materials {
        material.normal_texture = None;
    }
    let Some((a, size)) = render_with(&with, eye, Shading::default(), 128) else {
        return;
    };
    let Some((b, _)) = render_with(&without, eye, Shading::default(), 128) else {
        return;
    };
    let changed = (0..size * size)
        .filter(|&i| {
            let (pa, pb) = (
                pixel(&a, size, i % size, i / size),
                pixel(&b, size, i % size, i / size),
            );
            (0..3).any(|c| pa[c].abs_diff(pb[c]) > 6)
        })
        .count();
    assert!(changed > 200, "the grooves changed {changed} pixels");
}

#[test]
fn metal_reflects_the_sky_and_plastic_shows_its_colour() {
    // Looking at the two rows of spheres from the front: the plastic row
    // is blue, the metal row gold.
    let Some((pixels, size)) = render_with(
        &showcase(),
        Vec3::new(0.0, -9.5, 0.6),
        Shading::default(),
        128,
    ) else {
        return;
    };
    let all: Vec<[u8; 3]> = (0..size * size)
        .map(|i| pixel(&pixels, size, i % size, i / size))
        .collect();
    let wide = |p: &[u8; 3]| p.map(i32::from);
    let blue = all
        .iter()
        .map(wide)
        .filter(|&[r, g, b]| b > r + 40 && b > g + 10)
        .count();
    let gold = all
        .iter()
        .map(wide)
        .filter(|&[r, g, b]| r > b + 40 && g > b + 20)
        .count();
    assert!(blue > 50 && gold > 50, "blue {blue}, gold {gold}");
}

/// Not a check: when `DUMP_DIR` is set, writes a few renders there as PPM
/// images, for looking at the looks by eye while working on them.
#[test]
fn dump_renders_to_look_at() {
    let Some(dir) = std::env::var_os("DUMP_DIR") else {
        return;
    };
    for (name, model, eye, look) in [
        ("unlit", sample(), Vec3::new(0.0, -3.0, 1.1), Look::Unlit),
        (
            "stylized",
            sample(),
            Vec3::new(1.5, -6.0, 2.6),
            Look::Stylized,
        ),
        (
            "showcase",
            showcase(),
            Vec3::new(0.0, -8.0, 2.0),
            Look::Standard,
        ),
        (
            "spheres",
            showcase(),
            Vec3::new(0.0, -9.5, 0.6),
            Look::Standard,
        ),
    ] {
        let Some((pixels, size)) = render_with(&model, eye, shading(look), 128) else {
            return;
        };
        let mut ppm = format!("P6 {size} {size} 255\n").into_bytes();
        for c in pixels.chunks_exact(4) {
            ppm.extend_from_slice(&c[..3]);
        }
        std::fs::write(std::path::Path::new(&dir).join(format!("{name}.ppm")), ppm).unwrap();
    }
    // The showcase lit by a warm spotlight and a blue point light, with the
    // sun's shadows, under each sky.
    let eye = Vec3::new(1.5, -9.0, 2.6);
    let lights = [
        Light {
            kind: LightKind::Spot {
                inner: 0.3,
                outer: 0.45,
            },
            position: Vec3::new(-2.0, -2.0, 3.0),
            direction: Vec3::new(2.0, 2.0, -3.0),
            color: [1.0, 0.7, 0.4],
            strength: 3000.0,
            range: None,
        },
        Light {
            kind: LightKind::Point,
            position: Vec3::new(1.5, -1.0, 0.5),
            direction: -Vec3::Z,
            color: [0.3, 0.5, 1.0],
            strength: 400.0,
            range: None,
        },
    ];
    let day = Sky::Daylight { strength: 1.0 };
    let haze = Some(Fog {
        half_distance: 15.0,
        color: None,
    });
    for (name, look, environment) in [
        ("lit", Look::Standard, Environment::default()),
        ("lit-stylized", Look::Stylized, Environment::default()),
        ("day", Look::Standard, env(day, None, 0.0, Tonemap::Agx)),
        (
            "day-stylized",
            Look::Stylized,
            env(day, None, 0.0, Tonemap::Agx),
        ),
        (
            "day-aces",
            Look::Standard,
            env(day, None, 0.0, Tonemap::Aces),
        ),
        ("fog", Look::Standard, env(day, haze, 0.0, Tonemap::Agx)),
        (
            "backdrop-fog",
            Look::Standard,
            env(Sky::Backdrop, haze, 0.0, Tonemap::Agx),
        ),
    ] {
        let Some((pixels, size)) = render_env(
            &showcase_on_floor(),
            looking_north(eye),
            eye,
            shading(look),
            256,
            Sun::default(),
            &lights,
            environment,
        ) else {
            return;
        };
        let mut ppm = format!("P6 {size} {size} 255\n").into_bytes();
        for c in pixels.chunks_exact(4) {
            ppm.extend_from_slice(&c[..3]);
        }
        std::fs::write(std::path::Path::new(&dir).join(format!("{name}.ppm")), ppm).unwrap();
    }
    // A field of copies with the sun's shadows, from above at an angle:
    // shadows fitted to the view (Phase 5, M4).
    let Some(gpu) = gpu() else {
        return;
    };
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let size = 384;
    let mut target = renderer.create_target(&gpu.device, [size, size]);
    let ground = big_floor(200.0);
    let floor_model = renderer.upload_model(&gpu.device, &gpu.queue, &ground);
    let model = renderer.upload_model(&gpu.device, &gpu.queue, &sample());
    let mut placed = vec![Placement {
        model: &floor_model,
        node: None,
        transform: Mat4::IDENTITY,
        pick: 0,
    }];
    for i in 0..400 {
        placed.push(Placement {
            model: &model,
            node: None,
            transform: Mat4::from_translation(Vec3::new(
                (i % 20) as f32 * 4.0 - 40.0,
                (i / 20) as f32 * 4.0,
                0.0,
            )),
            pick: 0,
        });
    }
    let eye = Vec3::new(0.0, -14.0, 9.0);
    let tilt = Mat4::from_rotation_x(0.45);
    renderer.render(
        &gpu.device,
        &gpu.queue,
        &mut target,
        &FrameParams {
            view_projection: projection() * tilt * projection().inverse() * looking_north(eye),
            camera_position: eye,
            focus: Vec3::ZERO,
            sky_top: srgb(SKY_TOP),
            sky_horizon: srgb(SKY_HORIZON),
            ground: srgb(GROUND),
            shading: Shading::default(),
            size_px: [size, size],
            sun: Sun {
                direction: Vec3::new(0.6, 0.5, -0.7).normalize(),
                ..Sun::default()
            },
            environment: Environment::default(),
            editor_lines: true,
            grid_plane: GridPlane::Ground,
            grid_step: 1.0,
        },
        &placed,
        &[],
    );
    let pixels = read_pixels(&gpu.device, &gpu.queue, &target).unwrap();
    let mut ppm = format!("P6 {size} {size} 255\n").into_bytes();
    for c in pixels.chunks_exact(4) {
        ppm.extend_from_slice(&c[..3]);
    }
    std::fs::write(std::path::Path::new(&dir).join("field.ppm"), ppm).unwrap();
}

// ------------------------------------------------------------ lights ---

/// A camera at `eye` looking straight down (-Z), +X to the right and +Y
/// up the image, 60° tall view, square image.
fn looking_down(eye: Vec3) -> Mat4 {
    projection() * Mat4::from_translation(-eye)
}

/// The pixel showing ground point (`x`, `y`) to [`looking_down`] from 10 m
/// above the origin, in a `size`-pixel image.
fn ground_pixel(pixels: &[u8], size: u32, x: f32, y: f32) -> [u8; 3] {
    let half = 10.0 * 30.0_f32.to_radians().tan();
    let px = ((x / half + 1.0) * 0.5 * size as f32) as u32;
    let py = ((1.0 - y / half) * 0.5 * size as f32) as u32;
    pixel(pixels, size, px, py)
}

/// A flat square facing up at height `z`, `half` metres each way from
/// (`cx`, 0).
fn square(cx: f32, z: f32, half: f32) -> assets::Mesh {
    let corner = |x: f32, y: f32| assets::Vertex {
        position: [cx + x, y, z],
        normal: [0.0, 0.0, 1.0],
        uv: [0.0, 0.0],
    };
    assets::Mesh {
        vertices: vec![
            corner(-half, -half),
            corner(half, -half),
            corner(half, half),
            corner(-half, half),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        material: None,
        lods: Vec::new(),
    }
}

/// A grey floor 12 m across (raised a little, above the grid and axes),
/// and optionally a small square floating over its middle.
fn floor(with_roof: bool) -> assets::Model {
    let mut meshes = vec![square(0.0, 0.05, 6.0)];
    if with_roof {
        meshes.push(square(0.0, 1.5, 0.5));
    }
    let mut model = assets::Model {
        name: "Floor".into(),
        meshes,
        nodes: Vec::new(),
        materials: Vec::new(),
        textures: Vec::new(),
        lamps: Vec::new(),
        bounds: assets::Bounds {
            min: Vec3::new(-6.0, -6.0, 0.0),
            max: Vec3::new(6.0, 6.0, 1.5),
        },
        warnings: Vec::new(),
    };
    model.show_every_mesh();
    model
}

fn brightness(c: [u8; 3]) -> i32 {
    c.iter().map(|&v| i32::from(v)).sum()
}

/// A sun shining down at 45° towards +X.
fn slanted_sun(shadows: bool) -> Sun {
    Sun {
        direction: Vec3::new(1.0, 0.0, -1.0).normalize(),
        shadows,
        ..Sun::default()
    }
}

#[test]
fn the_sun_casts_a_shadow_where_it_should() {
    let eye = Vec3::new(0.0, 0.0, 10.0);
    for look in [Look::Standard, Look::Stylized] {
        let render = |shadows| {
            render_lit(
                &floor(true),
                looking_down(eye),
                eye,
                Shading {
                    look,
                    ..Shading::default()
                },
                128,
                slanted_sun(shadows),
                &[],
            )
        };
        let Some((with, size)) = render(true) else {
            return;
        };
        let (without, _) = render(false).unwrap();
        // The square 1.5 m up throws its shadow 1.5 m towards +X.
        let shadow = ground_pixel(&with, size, 1.5, 0.2);
        let open = ground_pixel(&with, size, -2.5, 0.2);
        assert!(
            brightness(shadow) + 60 < brightness(open),
            "{look:?}: shadow {shadow:?} not darker than open ground {open:?}"
        );
        // Without shadows, the same spot is lit like the rest.
        let unshadowed = ground_pixel(&without, size, 1.5, 0.2);
        assert!(
            (brightness(unshadowed) - brightness(open)).abs() < 12,
            "{look:?}: {unshadowed:?} vs {open:?}"
        );
        // Beyond the shadow, the ground is lit again.
        let past = ground_pixel(&with, size, 3.5, 0.2);
        assert!((brightness(past) - brightness(open)).abs() < 12, "{past:?}");
    }
}

#[test]
fn the_sun_shadow_falls_on_the_ground_grid_too() {
    let eye = Vec3::new(0.0, 0.0, 10.0);
    let mut floating = floor(true);
    floating.meshes.remove(0);
    floating.show_every_mesh();
    floating.bounds.min = Vec3::new(-0.5, -0.5, 1.5);
    let render = |shadows| {
        render_lit(
            &floating,
            looking_down(eye),
            eye,
            Shading::default(),
            128,
            slanted_sun(shadows),
            &[],
        )
    };
    let Some((with, size)) = render(true) else {
        return;
    };
    let (without, _) = render(false).unwrap();
    let shadow = ground_pixel(&with, size, 1.5, 0.2);
    let open = ground_pixel(&with, size, -2.5, 0.2);
    assert!(
        brightness(shadow) + 15 < brightness(open),
        "{shadow:?} vs {open:?}"
    );
    let plain = ground_pixel(&without, size, 1.5, 0.2);
    assert!(
        (brightness(plain) - brightness(open)).abs() < 6,
        "{plain:?}"
    );
}

fn no_sun() -> Sun {
    Sun {
        strength: 0.0,
        ..Sun::default()
    }
}

fn lamp(kind: LightKind, position: Vec3) -> Light {
    Light {
        kind,
        position,
        direction: -Vec3::Z,
        color: [1.0, 0.6, 0.3],
        strength: 60.0,
        range: None,
    }
}

#[test]
fn a_point_light_lights_the_ground_near_it_most() {
    let eye = Vec3::new(0.0, 0.0, 10.0);
    for look in [Look::Standard, Look::Stylized] {
        let shading = Shading {
            look,
            ..Shading::default()
        };
        let render = |lights: &[Light]| {
            render_lit(
                &floor(false),
                looking_down(eye),
                eye,
                shading,
                128,
                no_sun(),
                lights,
            )
        };
        let Some((dark, size)) = render(&[]) else {
            return;
        };
        let (lit, _) = render(&[lamp(LightKind::Point, Vec3::new(2.0, 0.0, 1.0))]).unwrap();
        let under = ground_pixel(&lit, size, 2.0, 0.3);
        let far = ground_pixel(&lit, size, -3.5, 0.3);
        let before = ground_pixel(&dark, size, 2.0, 0.3);
        assert!(
            brightness(under) > brightness(before) + 50,
            "{look:?}: {under:?} vs unlit {before:?}"
        );
        assert!(
            brightness(under) > brightness(far) + 60,
            "{look:?}: {under:?} vs far {far:?}"
        );
        // Warm light: it adds more red than blue.
        let gain = |c: usize| i32::from(under[c]) - i32::from(before[c]);
        assert!(gain(0) > gain(2) + 20, "{look:?}: {under:?} vs {before:?}");
    }
}

#[test]
fn a_spotlight_lights_only_inside_its_cone() {
    let eye = Vec3::new(0.0, 0.0, 10.0);
    let spot = Light {
        strength: 1000.0,
        ..lamp(
            LightKind::Spot {
                inner: 0.25,
                outer: 0.35,
            },
            Vec3::new(0.0, 0.0, 3.0),
        )
    };
    let render = |lights: &[Light]| {
        render_lit(
            &floor(false),
            looking_down(eye),
            eye,
            Shading::default(),
            128,
            no_sun(),
            lights,
        )
    };
    let Some((lit, size)) = render(&[spot]) else {
        return;
    };
    let (dark, _) = render(&[]).unwrap();
    // The cone reaches about 1.1 m out on the ground.
    let inside = ground_pixel(&lit, size, 0.4, 0.3);
    let outside = ground_pixel(&lit, size, 2.5, 0.3);
    let outside_dark = ground_pixel(&dark, size, 2.5, 0.3);
    assert!(
        brightness(inside) > brightness(outside) + 100,
        "{inside:?} {outside:?}"
    );
    assert!(
        (brightness(outside) - brightness(outside_dark)).abs() < 6,
        "outside the cone stays as it was: {outside:?} vs {outside_dark:?}"
    );
}

#[test]
fn a_lamp_with_a_range_stops_there() {
    let eye = Vec3::new(0.0, 0.0, 10.0);
    let mut short = lamp(LightKind::Point, Vec3::new(0.0, 0.0, 0.5));
    short.range = Some(2.0);
    let render = |lights: &[Light]| {
        render_lit(
            &floor(false),
            looking_down(eye),
            eye,
            Shading::default(),
            128,
            no_sun(),
            lights,
        )
    };
    let Some((lit, size)) = render(&[short]) else {
        return;
    };
    let (dark, _) = render(&[]).unwrap();
    let near = ground_pixel(&lit, size, 0.5, 0.3);
    let beyond = ground_pixel(&lit, size, 3.0, 0.3);
    let beyond_dark = ground_pixel(&dark, size, 3.0, 0.3);
    assert!(
        brightness(near) > brightness(beyond) + 60,
        "{near:?} {beyond:?}"
    );
    assert!((brightness(beyond) - brightness(beyond_dark)).abs() < 6);
}

#[test]
fn many_lamps_and_odd_values_draw_without_trouble() {
    let eye = Vec3::new(0.0, 0.0, 10.0);
    let mut lights: Vec<Light> = (0..40)
        .map(|i| lamp(LightKind::Point, Vec3::new(i as f32 * 0.2 - 4.0, 0.0, 1.0)))
        .collect();
    lights.push(Light {
        strength: f32::NAN,
        ..lamp(LightKind::Point, Vec3::ZERO)
    });
    lights.push(lamp(
        LightKind::Spot {
            inner: f32::INFINITY,
            outer: -1.0,
        },
        Vec3::new(0.0, 0.0, f32::NAN),
    ));
    let weird_sun = Sun {
        direction: Vec3::ZERO,
        strength: f32::INFINITY,
        ..Sun::default()
    };
    let Some((pixels, size)) = render_lit(
        &floor(true),
        looking_down(eye),
        eye,
        Shading::default(),
        64,
        weird_sun,
        &lights,
    ) else {
        return;
    };
    assert_eq!(pixels.len(), (size * size * 4) as usize);
}

// ---------------------------------------------- sky, fog and exposure ---

/// The showcase scene standing on a big floor, for looking at.
fn showcase_on_floor() -> assets::Model {
    let mut model = showcase();
    let floor = big_floor(60.0);
    model.meshes = model.placed_meshes();
    model.meshes.extend(floor.meshes);
    model.show_every_mesh();
    model.bounds.min = model.bounds.min.min(floor.bounds.min);
    model.bounds.max = model.bounds.max.max(floor.bounds.max);
    model
}

/// A grey floor `half` metres each way, a little above the grid.
fn big_floor(half: f32) -> assets::Model {
    let mut model = floor(false);
    model.meshes = vec![square(0.0, 0.05, half)];
    model.show_every_mesh();
    model.bounds = assets::Bounds {
        min: Vec3::new(-half, -half, 0.0),
        max: Vec3::new(half, half, 0.05),
    };
    model
}

fn env(sky: Sky, fog: Option<Fog>, exposure: f32, tonemap: Tonemap) -> Environment {
    Environment {
        sky,
        fog,
        exposure,
        tonemap,
    }
}

#[test]
fn tone_mapping_rolls_off_bright_light_and_exposure_scales_it() {
    let eye = Vec3::new(0.0, 0.0, 10.0);
    let bright = Sun {
        strength: 12.0,
        shadows: false,
        ..Sun::default()
    };
    let render = |environment| {
        render_env(
            &floor(false),
            looking_down(eye),
            eye,
            Shading::default(),
            64,
            bright,
            &[],
            environment,
        )
    };
    let Some((standard, size)) = render(env(Sky::Backdrop, None, 0.0, Tonemap::Standard)) else {
        return;
    };
    let (agx, _) = render(env(Sky::Backdrop, None, 0.0, Tonemap::Agx)).unwrap();
    let (aces, _) = render(env(Sky::Backdrop, None, 0.0, Tonemap::Aces)).unwrap();
    let (darker, _) = render(env(Sky::Backdrop, None, -3.0, Tonemap::Agx)).unwrap();
    let at = |p: &[u8]| ground_pixel(p, size, 1.0, 1.0);
    assert!(
        at(&standard).iter().all(|&c| c >= 250),
        "cut off at white: {:?}",
        at(&standard)
    );
    assert!(
        at(&agx).iter().all(|&c| c < 250),
        "AgX rolls off: {:?}",
        at(&agx)
    );
    assert!(brightness(at(&aces)) > brightness(at(&darker)));
    assert!(
        brightness(at(&darker)) + 100 < brightness(at(&agx)),
        "-3 EV: {:?} vs {:?}",
        at(&darker),
        at(&agx)
    );
}

#[test]
fn the_editor_backdrop_ignores_exposure_and_tone_mapping() {
    let eye = Vec3::new(0.0, 0.0, 2.0);
    let render = |environment| {
        render_env(
            &floor(false),
            looking_north(eye),
            eye,
            Shading::default(),
            64,
            Sun::default(),
            &[],
            environment,
        )
    };
    let Some((plain, size)) = render(env(Sky::Backdrop, None, 0.0, Tonemap::Standard)) else {
        return;
    };
    let (exposed, _) = render(env(Sky::Backdrop, None, 4.0, Tonemap::Aces)).unwrap();
    let top = |p: &[u8]| pixel(p, size, size / 2, 0);
    assert_eq!(top(&plain), top(&exposed));
}

#[test]
fn the_daylight_sky_follows_the_sun() {
    let eye = Vec3::new(0.0, 0.0, 2.0);
    let render = |sun: Sun| {
        render_env(
            &floor(false),
            looking_north(eye),
            eye,
            Shading::default(),
            64,
            sun,
            &[],
            env(Sky::Daylight { strength: 1.0 }, None, 0.0, Tonemap::Agx),
        )
    };
    // The sun behind the camera, high: a blue sky ahead.
    let noon = Sun {
        direction: Vec3::new(0.0, 0.5, -1.0).normalize(),
        ..Sun::default()
    };
    let Some((day, size)) = render(noon) else {
        return;
    };
    let up = pixel(&day, size, size / 2, 2);
    assert!(up[2] > up[0] + 30, "blue overhead: {up:?}");
    // The sun setting straight ahead: warm at the horizon, and its disc
    // shows bright in the middle of the view.
    let setting = Sun {
        direction: Vec3::new(0.0, -1.0, -0.02).normalize(),
        ..Sun::default()
    };
    let (dusk, _) = render(setting).unwrap();
    let beside = pixel(&dusk, size, size / 4, size / 2 - 3);
    assert!(beside[0] > beside[2], "warm horizon: {beside:?}");
    let disc = pixel(&dusk, size, size / 2, size / 2 - 1);
    assert!(brightness(disc) > 650, "the sun's disc: {disc:?}");
    // At night, the sky is dark.
    let night = Sun {
        direction: Vec3::new(0.0, 0.3, 1.0).normalize(),
        ..Sun::default()
    };
    let (dark, _) = render(night).unwrap();
    assert!(brightness(pixel(&dark, size, size / 2, 2)) < 40);
}

#[test]
fn fog_hides_the_distance_into_the_horizon() {
    let eye = Vec3::new(0.0, 0.0, 2.0);
    let render = |fog| {
        render_env(
            &big_floor(2000.0),
            looking_north(eye),
            eye,
            Shading::default(),
            128,
            Sun {
                shadows: false,
                ..Sun::default()
            },
            &[],
            env(Sky::Backdrop, fog, 0.0, Tonemap::Agx),
        )
    };
    let Some((clear, size)) = render(None) else {
        return;
    };
    let fog = Fog {
        half_distance: 20.0,
        color: None,
    };
    let (foggy, _) = render(Some(fog)).unwrap();
    // Just below the horizon the floor is a kilometre away: lost in
    // the fog, it shows the backdrop's horizon colour exactly. Near the
    // bottom of the view it's a few metres away and barely changes.
    let far = pixel(&foggy, size, size / 4, size / 2);
    let horizon = pixel(&clear, size, size / 4, size / 2 - 2);
    assert!(
        (brightness(far) - brightness(horizon)).abs() < 12,
        "{far:?} vs horizon {horizon:?}"
    );
    let near_clear = pixel(&clear, size, size / 4, size - 2);
    let near_foggy = pixel(&foggy, size, size / 4, size - 2);
    assert!(brightness(far) + 100 < brightness(near_foggy));
    assert!((brightness(near_clear) - brightness(near_foggy)).abs() < 80);
    // A fog colour of its own.
    let red = Fog {
        half_distance: 1.0,
        color: Some([1.0, 0.0, 0.0]),
    };
    let (reddened, _) = render(Some(red)).unwrap();
    let p = pixel(&reddened, size, size / 4, size / 2 + 4);
    assert!(i32::from(p[0]) > i32::from(p[1]) + 100, "{p:?}");
}

#[test]
fn the_unlit_look_ignores_exposure() {
    let eye = Vec3::new(0.0, 0.0, 10.0);
    let render = |exposure| {
        render_env(
            &floor(false),
            looking_down(eye),
            eye,
            Shading {
                look: Look::Unlit,
                ..Shading::default()
            },
            64,
            Sun::default(),
            &[],
            env(Sky::Backdrop, None, exposure, Tonemap::Agx),
        )
    };
    let Some((a, size)) = render(0.0) else {
        return;
    };
    let (b, _) = render(3.0).unwrap();
    assert_eq!(
        ground_pixel(&a, size, 1.0, 1.0),
        ground_pixel(&b, size, 1.0, 1.0)
    );
    assert_eq!(ground_pixel(&a, size, 1.0, 1.0), [255, 255, 255]);
}

#[test]
fn two_viewports_keep_their_own_frames() {
    let Some(gpu) = gpu() else { return };
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let floor = renderer.upload_model(&gpu.device, &gpu.queue, &floor(false));
    let params = |view_projection, eye, look| FrameParams {
        view_projection,
        camera_position: eye,
        focus: Vec3::new(1.0e7, 1.0e7, 0.0),
        sky_top: srgb(SKY_TOP),
        sky_horizon: srgb(SKY_HORIZON),
        ground: srgb(GROUND),
        shading: Shading {
            look,
            ..Shading::default()
        },
        size_px: [32, 32],
        sun: Sun::default(),
        environment: Environment::default(),
        editor_lines: true,
        grid_plane: GridPlane::Ground,
        grid_step: 1.0,
    };
    let down = Vec3::new(0.0, 0.0, 10.0);
    let away = Vec3::new(0.0, -100.0, 50.0);
    let mut a = renderer.create_target(&gpu.device, [32, 32]);
    let mut b = renderer.create_target(&gpu.device, [32, 32]);
    // Both drawn before either is read: each must keep its own camera and
    // look.
    renderer.render(
        &gpu.device,
        &gpu.queue,
        &mut a,
        &params(looking_down(down), down, Look::Unlit),
        &[Placement {
            model: &floor,
            node: None,
            transform: glam::Mat4::IDENTITY,
            pick: 0,
        }],
        &[],
    );
    renderer.render(
        &gpu.device,
        &gpu.queue,
        &mut b,
        &params(looking_north(away), away, Look::Standard),
        &[Placement {
            model: &floor,
            node: None,
            transform: glam::Mat4::IDENTITY,
            pick: 0,
        }],
        &[],
    );
    let pa = read_pixels(&gpu.device, &gpu.queue, &a).unwrap();
    let pb = read_pixels(&gpu.device, &gpu.queue, &b).unwrap();
    assert_eq!(
        pixel(&pa, 32, 16, 16),
        [255, 255, 255],
        "A: the unlit floor from above"
    );
    assert!(
        brightness(pixel(&pb, 32, 16, 2)) < 120,
        "B: sky above the far floor"
    );
}

// ------------------------------------------------------------ instances ---

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

/// Waits for a pick's answer (tests only: the editor never waits).
#[allow(clippy::panic)]
fn wait_for_pick(gpu: &Gpu, target: &mut render::ViewportTarget) -> render::PickResult {
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

/// The CPU's side of drawing `count` copies of `model` in rows of
/// `row` 3 m apart, seen from `eye` looking north, with or without the
/// sun's shadows: gathering, culling, sorting and handing them over (not
/// the GPU's drawing, which isn't waited for). With `moving`, every copy
/// moves every frame. The milliseconds a frame, and what the last drew.
fn gathering_ms(
    model: &assets::Model,
    count: usize,
    row: usize,
    eye: Vec3,
    shadows: bool,
    moving: bool,
) -> Option<(f64, render::FrameStats)> {
    let gpu = gpu()?;
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let uploaded = renderer.upload_model(&gpu.device, &gpu.queue, model);
    let place = |i: usize, frame: usize| {
        let nudge = if moving { frame as f32 * 0.01 } else { 0.0 };
        Mat4::from_translation(Vec3::new(
            (i % row) as f32 * 3.0 - (row as f32 * 1.5) + nudge,
            (i / row) as f32 * 3.0,
            0.0,
        ))
    };
    let mut copies: Vec<Placement<'_>> = (0..count)
        .map(|i| Placement {
            model: &uploaded,
            node: None,
            transform: place(i, 0),
            pick: 0,
        })
        .collect();
    let mut target = renderer.create_target(&gpu.device, [64, 64]);
    let params = FrameParams {
        view_projection: looking_north(eye),
        camera_position: eye,
        focus: Vec3::ZERO,
        sky_top: srgb(SKY_TOP),
        sky_horizon: srgb(SKY_HORIZON),
        ground: srgb(GROUND),
        shading: Shading::default(),
        size_px: [64, 64],
        sun: Sun {
            shadows,
            ..Sun::default()
        },
        environment: unmapped(),
        editor_lines: false,
        grid_plane: GridPlane::Ground,
        grid_step: 1.0,
    };
    renderer.render(&gpu.device, &gpu.queue, &mut target, &params, &copies, &[]);
    let _ = read_pixels(&gpu.device, &gpu.queue, &target);
    let frames = 10;
    let mut spent = std::time::Duration::ZERO;
    let mut stats = render::FrameStats::default();
    for frame in 1..=frames {
        if moving {
            for (i, copy) in copies.iter_mut().enumerate() {
                copy.transform = place(i, frame);
            }
        }
        let start = std::time::Instant::now();
        stats = renderer.render(&gpu.device, &gpu.queue, &mut target, &params, &copies, &[]);
        spent += start.elapsed();
        // The GPU catches up between frames, outside the timing.
        let _ = read_pixels(&gpu.device, &gpu.queue, &target);
    }
    Some((spent.as_secs_f64() * 1000.0 / frames as f64, stats))
}

/// The 100,000-copy cases are judged on machines with at least this many
/// cores (gathering runs on every core; the gate's PC has many); on fewer,
/// they're reported.
const GATHER_CORES: usize = 4;

/// Whether this machine has [`GATHER_CORES`].
fn enough_cores() -> bool {
    std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get) >= GATHER_CORES
}

#[allow(clippy::print_stdout)]
fn gathering_verdict(what: &str, ms: f64, target: f64, judged: bool) {
    let verdict = match (ms <= target, judged) {
        (true, _) => "ok",
        (false, true) => "SLOW",
        (false, false) => "reported: judged on 4+ cores",
    };
    println!("Scale check: {what} {ms:.2} ms a frame (target {target} ms) {verdict}");
    if judged {
        assert!(ms <= target, "{what}: {ms:.2} ms");
    }
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_gathering_100000_copies() {
    // A field 900 m wide and 1 km deep, seen from its near edge: about
    // two thirds are in view.
    let Some((ms, stats)) = gathering_ms(
        &sample(),
        100_000,
        300,
        Vec3::new(0.0, -10.0, 8.0),
        false,
        false,
    ) else {
        println!("Scale check: gathering 100,000 copies skipping: no GPU");
        return;
    };
    gathering_verdict(
        &format!(
            "gathering and handing over 100,000 copies ({} mesh copies in view)",
            stats.instances
        ),
        ms,
        8.0,
        enough_cores(),
    );
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_gathering_100000_copies_all_moving() {
    let Some((ms, _)) = gathering_ms(
        &sample(),
        100_000,
        300,
        Vec3::new(0.0, -10.0, 8.0),
        false,
        true,
    ) else {
        return;
    };
    gathering_verdict(
        "gathering and handing over 100,000 copies, every one moving",
        ms,
        8.0,
        enough_cores(),
    );
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_10000_units_a_tenth_in_view() {
    // 10,000 units on a field 300 m square, the camera low beside it,
    // looking along it: about a tenth are in view; shadows on.
    let Some((ms, stats)) = gathering_ms(
        &sample(),
        10_000,
        100,
        Vec3::new(-215.0, 0.0, 3.0),
        true,
        true,
    ) else {
        return;
    };
    gathering_verdict(
        &format!(
            "10,000 moving units, {} of 20,000 mesh copies in view and {} casting shadows",
            stats.instances, stats.casting
        ),
        ms,
        8.0,
        true,
    );
    assert!(stats.instances < 20_000 / 4, "most aren't drawn: {stats:?}");
}

// ------------------------------------------------------------ culling ---

/// Renders each (model, place) seen by `view_projection` from `eye`, lit
/// by `sun`, 128 pixels square; the pixels and what was drawn.
fn render_scene(
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

// ------------------------------------------------------------ LODs ---

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

// ------------------------------------------------------------ projectiles ---

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

// ------------------------------------------------------------ pixel stage ---

/// A colour for every source pixel of a picture up to 4096 wide and tall,
/// different for each, so the scaled-up image says which it shows.
fn source_colour(x: u32, y: u32) -> [u8; 4] {
    [
        (x & 255) as u8,
        (y & 255) as u8,
        ((x >> 8) | ((y >> 8) << 4)) as u8,
        255,
    ]
}

/// A target holding a `base` picture plus a pixel's margin all round, each
/// pixel its own colour ([`source_colour`]).
fn pattern_source(
    gpu: &Gpu,
    renderer: &ViewportRenderer,
    base: [u32; 2],
) -> render::ViewportTarget {
    let size = [base[0] + 2, base[1] + 2];
    let target = renderer.create_target(&gpu.device, size);
    let mut bytes = Vec::with_capacity((size[0] * size[1] * 4) as usize);
    for y in 0..size[1] {
        for x in 0..size[0] {
            bytes.extend_from_slice(&source_colour(x, y));
        }
    }
    gpu.queue.write_texture(
        target.color_texture().as_image_copy(),
        &bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size[0] * 4),
            rows_per_image: Some(size[1]),
        },
        wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
    );
    target
}

/// Scales the pattern up into a `screen` image as `scaling` and `slide`
/// say, and reads it back with its layout.
fn staged(
    gpu: &Gpu,
    renderer: &ViewportRenderer,
    source: &render::ViewportTarget,
    base: [u32; 2],
    screen: [u32; 2],
    scaling: PixelScaling,
    slide: [f32; 2],
) -> (Vec<u8>, render::PixelLayout) {
    let image = renderer.create_pixel_image(&gpu.device, screen);
    let layout = pixel_layout(base, screen, scaling, slide);
    renderer.draw_pixels(&gpu.device, &gpu.queue, source, &image, &layout);
    let pixels = read_texture(&gpu.device, &gpu.queue, image.texture(), screen).unwrap();
    (pixels, layout)
}

/// Every screen pixel of a whole-steps image is exactly the source pixel
/// the layout says (black in the bars): checked one by one.
fn check_whole(pixels: &[u8], layout: &render::PixelLayout) {
    let [w, h] = layout.screen;
    let mut wrong = 0;
    for y in 0..h {
        for x in 0..w {
            let at = ((y * w + x) * 4) as usize;
            let got = [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]];
            let want = match layout.source_pixel(x, y) {
                Some([sx, sy]) => source_colour(sx, sy),
                None => [0, 0, 0, 255],
            };
            if got != want {
                wrong += 1;
                assert!(
                    wrong < 5,
                    "{:?} at ({x}, {y}): {got:?}, not {want:?}",
                    layout.screen
                );
            }
        }
    }
    assert_eq!(wrong, 0, "{:?}: {wrong} pixels wrong", layout.screen);
}

#[test]
fn the_pixel_stage_is_exact_at_every_common_screen_size() {
    let Some(gpu) = gpu() else { return };
    let renderer = ViewportRenderer::new(&gpu.device, 1);
    for base in [[480, 270], [320, 180], [640, 360]] {
        let source = pattern_source(&gpu, &renderer, base);
        for screen in [
            [1280, 720],
            [1366, 768],
            [1600, 900],
            [1920, 1080],
            [1920, 1200],
            [2560, 1080],
            [2560, 1440],
            [3440, 1440],
            [3840, 2160],
        ] {
            let (pixels, layout) = staged(
                &gpu,
                &renderer,
                &source,
                base,
                screen,
                PixelScaling::WholeSteps,
                [0.0; 2],
            );
            assert!(layout.whole);
            check_whole(&pixels, &layout);
        }
    }
}

#[test]
fn a_camera_moving_part_of_a_pixel_a_frame_slides_the_picture_and_never_blurs_it() {
    let Some(gpu) = gpu() else { return };
    let renderer = ViewportRenderer::new(&gpu.device, 1);
    let base = [480, 270];
    let source = pattern_source(&gpu, &renderer, base);
    let screen = [1920, 1080];
    let mut last_x = 0.0;
    for tenth in -10..=10 {
        let f = tenth as f32 / 10.0;
        let (pixels, layout) = staged(
            &gpu,
            &renderer,
            &source,
            base,
            screen,
            PixelScaling::WholeSteps,
            [f, -f / 2.0],
        );
        // Never a blend: every pixel is exactly one source pixel.
        check_whole(&pixels, &layout);
        // Steadily along, a screen pixel at a time, never back.
        assert!(layout.slide[0] >= last_x || tenth == -10);
        last_x = layout.slide[0];
    }
}

#[test]
fn filling_the_screen_keeps_edges_sharp() {
    let Some(gpu) = gpu() else { return };
    let renderer = ViewportRenderer::new(&gpu.device, 1);
    let base = [480, 270];
    let source = pattern_source(&gpu, &renderer, base);
    let screen = [2560, 1440];
    let (pixels, layout) = staged(
        &gpu,
        &renderer,
        &source,
        base,
        screen,
        PixelScaling::Fill,
        [0.0; 2],
    );
    assert!(!layout.whole && layout.size == screen);
    // Along a row, each source pixel (5⅓ screen pixels wide) shows as at
    // least four screen pixels of exactly its colour, with at most one
    // blended pixel between it and the next.
    let row = 700;
    let mut exact = std::collections::BTreeMap::<u32, u32>::new();
    let mut blended = 0;
    for x in 0..screen[0] {
        let at = ((row * screen[0] + x) * 4) as usize;
        let got = [pixels[at], pixels[at + 1], pixels[at + 2], 255];
        let sx = (x as f32 + 0.5) / layout.scale + 1.0;
        let sy = ((row as f32 + 0.5) / layout.scale + 1.0).floor() as u32;
        if got == source_colour(sx.floor() as u32, sy) {
            *exact.entry(sx.floor() as u32).or_default() += 1;
        } else {
            blended += 1;
        }
    }
    assert!(exact.values().all(|&n| n >= 4), "{exact:?}");
    assert!(exact.len() >= 479, "{}", exact.len());
    assert!(blended <= 481, "{blended} blended pixels");
}

/// Part of the scale check (decision 0011): the pixel stage at 4K, which
/// the 2D plan budgets at 0.2 ms on the reference card. Reported, not
/// judged: software drivers are far slower.
#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
#[allow(clippy::print_stdout)]
fn scale_check_pixel_stage_at_4k() {
    let Some(gpu) = gpu() else { return };
    let renderer = ViewportRenderer::new(&gpu.device, 1);
    let base = [480, 270];
    let source = pattern_source(&gpu, &renderer, base);
    let image = renderer.create_pixel_image(&gpu.device, [3840, 2160]);
    let frames = 60;
    let start = std::time::Instant::now();
    for i in 0..frames {
        let layout = pixel_layout(
            base,
            [3840, 2160],
            PixelScaling::WholeSteps,
            [i as f32 / 60.0, 0.0],
        );
        renderer.draw_pixels(&gpu.device, &gpu.queue, &source, &image, &layout);
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .unwrap();
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    println!(
        "Scale check: pixel stage at 3840x2160 {ms:.2} ms (budget 0.2 ms on the reference card) on {}",
        gpu.adapter.get_info().name
    );
}

// ---------------------------------------------------------------- sprites ---

/// A 4 × 4 sheet: every pixel its own colour, except (3, 3), which is
/// see-through, and (2, 3), mostly see-through (alpha 100 of 255).
fn sheet_pixels() -> Vec<u8> {
    let mut rgba = Vec::new();
    for y in 0..4u8 {
        for x in 0..4u8 {
            let a = match (x, y) {
                (3, 3) => 0,
                (2, 3) => 100,
                _ => 255,
            };
            rgba.extend_from_slice(&[40 + x * 50, 20 + y * 60, 200 - x * 30 - y * 10, a]);
        }
    }
    rgba
}

/// Looking straight down at 64 × 64 metres of ground, one metre a pixel:
/// world (x, y) is pixel (x, 64 - y); height runs from -50 (far) to 50
/// (near).
fn flat_down() -> Mat4 {
    Mat4::from_cols(
        Vec4::new(1.0 / 32.0, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 1.0 / 32.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0 / 100.0, 0.0),
        Vec4::new(-1.0, -1.0, 0.5, 1.0),
    )
}

/// A sprite of the whole sheet with its top-left corner at pixel
/// (`x`, `y`), `flags` and a tint (rgb) with a flash.
fn sprite_at(x: f32, y: f32, z: f32, flags: u32, tint: [f32; 4]) -> render::SpriteInstance {
    render::SpriteInstance {
        corner: [x, 64.0 - y, z],
        flags,
        axis_x: [1.0, 0.0],
        axis_y: [0.0, -1.0],
        rect: [0.0, 0.0, 4.0, 4.0],
        tint,
        outline: [1.0, 0.0, 0.0, 0.0],
        foot: [x + 2.0, 64.0 - y - 4.0],
    }
}

/// Draws `sprites` of the test sheet over the backdrop, one sample a
/// pixel, and reads the 64 × 64 picture back.
fn render_sprites(sprites: &[render::SpriteInstance]) -> Option<Vec<u8>> {
    let gpu = gpu()?;
    let renderer = ViewportRenderer::new(&gpu.device, 1);
    let mut target = renderer.create_target(&gpu.device, [64, 64]);
    let sheet = renderer.upload_sheet(&gpu.device, &gpu.queue, [4, 4], &sheet_pixels());
    let (cut, blended): (Vec<_>, Vec<_>) = sprites.iter().partition(|s| !s.see_through());
    let runs = [
        render::SpriteRun {
            sheet: &sheet,
            instances: &cut,
        },
        render::SpriteRun {
            sheet: &sheet,
            instances: &blended,
        },
    ];
    let params = FrameParams {
        view_projection: flat_down(),
        camera_position: Vec3::new(32.0, 32.0, 50.0),
        focus: Vec3::new(32.0, 32.0, 0.0),
        sky_top: srgb(SKY_TOP),
        sky_horizon: srgb(SKY_HORIZON),
        ground: srgb(GROUND),
        shading: Shading::default(),
        size_px: [64, 64],
        sun: Sun::default(),
        environment: Environment::default(),
        editor_lines: false,
        grid_plane: GridPlane::Ground,
        grid_step: 1.0,
    };
    let stats = renderer.render_scene(
        &gpu.device,
        &gpu.queue,
        &mut target,
        &params,
        &render::Scene {
            sprites: &runs,
            ..render::Scene::default()
        },
    );
    assert_eq!(stats.sprites as usize, sprites.len());
    Some(read_pixels(&gpu.device, &gpu.queue, &target).unwrap())
}

fn sheet_colour(x: u8, y: u8) -> [u8; 3] {
    [40 + x * 50, 20 + y * 60, 200 - x * 30 - y * 10]
}

#[test]
fn sprites_show_their_pixels_exactly_as_drawn() {
    let white = [1.0, 1.0, 1.0, 0.0];
    let Some(pixels) = render_sprites(&[sprite_at(10.0, 10.0, 0.0, 0, white)]) else {
        return;
    };
    for y in 0..4u8 {
        for x in 0..4u8 {
            if (x, y) == (3, 3) || (x, y) == (2, 3) {
                continue;
            }
            let got = pixel(&pixels, 64, 10 + u32::from(x), 10 + u32::from(y));
            let want = sheet_colour(x, y);
            assert!(
                got.iter().zip(want).all(|(a, b)| a.abs_diff(b) <= 1),
                "({x}, {y}): {got:?}, not {want:?}"
            );
        }
    }
    // Cut out: the see-through pixels (even half) show what's behind.
    let behind = pixel(&pixels, 64, 40, 40);
    assert_eq!(pixel(&pixels, 64, 13, 13), behind);
    assert_eq!(pixel(&pixels, 64, 12, 13), behind);
    assert_eq!(pixel(&pixels, 64, 14, 10), behind, "nothing past its edge");
}

#[test]
fn flip_tint_flash_and_outline_change_a_sprite_as_asked() {
    let white = [1.0, 1.0, 1.0, 0.0];
    let Some(pixels) = render_sprites(&[
        sprite_at(4.0, 4.0, 0.0, render::SpriteInstance::FLIP_X, white),
        sprite_at(20.0, 4.0, 0.0, 0, [1.0, 0.0, 0.0, 0.0]),
        sprite_at(36.0, 4.0, 0.0, 0, [1.0, 1.0, 1.0, 1.0]),
        sprite_at(52.0, 4.0, 0.0, render::SpriteInstance::OUTLINE, white),
    ]) else {
        return;
    };
    // Flipped across: its first column shows the sheet's last.
    let near = |a: [u8; 3], b: [u8; 3]| a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 1);
    assert!(near(pixel(&pixels, 64, 4, 4), sheet_colour(3, 0)));
    // Tinted red: green and blue gone.
    let red = pixel(&pixels, 64, 20, 4);
    assert!(red[1] == 0 && red[2] == 0 && red[0] > 0, "{red:?}");
    // A full flash: white.
    assert_eq!(pixel(&pixels, 64, 36, 4), [255, 255, 255]);
    // The outline: red round what's drawn, not over it.
    assert_eq!(pixel(&pixels, 64, 51, 5), [255, 0, 0]);
    assert_eq!(pixel(&pixels, 64, 52, 3), [255, 0, 0]);
    assert!(near(pixel(&pixels, 64, 52, 4), sheet_colour(0, 0)));
    assert_eq!(
        pixel(&pixels, 64, 51, 3),
        pixel(&pixels, 64, 40, 40),
        "corners stay clear"
    );
}

#[test]
fn whoever_stands_lower_on_screen_is_in_front_on_the_same_layer() {
    let white = [1.0, 1.0, 1.0, 0.0];
    let red = [1.0, 0.0, 0.0, 0.0];
    // The white one stands a pixel lower: in front, whichever comes first.
    for order in [false, true] {
        let mut sprites = vec![
            sprite_at(10.0, 10.0, 0.0, 0, red),
            sprite_at(11.0, 11.0, 0.0, 0, white),
        ];
        if order {
            sprites.reverse();
        }
        let Some(pixels) = render_sprites(&sprites) else {
            return;
        };
        let p = pixel(&pixels, 64, 12, 12);
        assert!(p[1] > 0, "the lower one is in front: {p:?}");
    }
}

#[test]
fn the_nearer_sprite_is_in_front_and_see_through_ones_blend() {
    let white = [1.0, 1.0, 1.0, 0.0];
    let red = [1.0, 0.0, 0.0, 0.0];
    // Two overlapping: the higher one (nearer the camera) wins, whatever
    // the order they're given in.
    let Some(pixels) = render_sprites(&[
        sprite_at(10.0, 10.0, 1.0, 0, red),
        sprite_at(11.0, 11.0, 0.5, 0, white),
    ]) else {
        return;
    };
    let p = pixel(&pixels, 64, 12, 12);
    assert!(p[1] == 0 && p[2] == 0, "the red one is nearer: {p:?}");
    // See-through: half over what's behind.
    let Some(pixels) = render_sprites(&[sprite_at(
        30.0,
        30.0,
        0.0,
        render::SpriteInstance::SEE_THROUGH,
        white,
    )]) else {
        return;
    };
    let behind = pixel(&pixels, 64, 50, 50);
    let half = pixel(&pixels, 64, 32, 33);
    let full = pixel(&pixels, 64, 30, 30);
    assert!(near_between(half, behind, sheet_colour(2, 3)), "{half:?}");
    assert!(
        full.iter()
            .zip(sheet_colour(0, 0))
            .all(|(a, b)| a.abs_diff(b) <= 2)
    );
}

/// Whether `c` lies between `a` and `b` in every channel, give or take.
fn near_between(c: [u8; 3], a: [u8; 3], b: [u8; 3]) -> bool {
    (0..3).all(|i| c[i] + 3 >= a[i].min(b[i]) && c[i] <= a[i].max(b[i]) + 3)
}
