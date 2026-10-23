//! Renders real frames on a GPU and checks the pixels.
//!
//! Machines without a GPU skip these tests with a note, unless the
//! environment variable `REQUIRE_GPU` is set, in which case a missing GPU is
//! a failure. The CI window job sets it and installs a software driver, so
//! these tests always run somewhere.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stderr)]

use glam::{Mat4, Vec3, Vec4};
use render::{
    Environment, Fog, FrameParams, Light, LightKind, Look, Placement, Shading, Sky, Sun, Tonemap,
    ViewportRenderer, best_sample_count, read_pixels,
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
                let (device, queue) =
                    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                        .ok()?;
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
fn the_ground_axes_show_in_red_and_green() {
    let Some(gpu) = gpu() else { return };
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let size = 64;
    let mut target = renderer.create_target(&gpu.device, [size, size]);
    // Standing on the Y axis, so it runs straight ahead into the middle of
    // the lower half; the X axis crosses in front.
    let eye = Vec3::new(0.0, -10.0, 2.0);
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
    assert!(greenish, "no green Y axis down the middle");
    assert!(reddish, "no red X axis");
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
        },
        &[Placement {
            model: &uploaded,
            node: None,
            transform: glam::Mat4::IDENTITY,
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
    // The left copy shows red (the base) where the single one doesn't.
    let red = |p: &[u8], x: u32| {
        let i = ((36 * 64 + x) * 4) as usize;
        p[i] > p[i + 1] + 40
    };
    let left = (0..12).any(|x| red(&twice, x));
    let right = (52..64).any(|x| red(&twice, x));
    assert!(left && right, "both copies drawn");
    assert!(
        !(0..12).any(|x| red(&once, x)),
        "one copy stays in the middle"
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
