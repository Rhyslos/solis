//! Renders real frames on a GPU and checks the pixels.
//!
//! Machines without a GPU skip these tests with a note, unless the
//! environment variable `REQUIRE_GPU` is set, in which case a missing GPU is
//! a failure. The CI window job sets it and installs a software driver, so
//! these tests always run somewhere.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stderr)]

use glam::{Mat4, Vec3, Vec4};
use render::{FrameParams, Look, Shading, ViewportRenderer, best_sample_count, read_pixels};

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

/// A camera at `eye` looking straight along +Y (level with the horizon),
/// 60° tall view, square image.
fn looking_north(eye: Vec3) -> Mat4 {
    let (near, far) = (0.1, 1000.0);
    let scale = 1.0 / (30.0_f32.to_radians()).tan();
    let depth = far / (near - far);
    let projection = Mat4::from_cols(
        Vec4::new(scale, 0.0, 0.0, 0.0),
        Vec4::new(0.0, scale, 0.0, 0.0),
        Vec4::new(0.0, 0.0, depth, -1.0),
        Vec4::new(0.0, 0.0, depth * near, 0.0),
    );
    // Right is +X, up is +Z, forward is +Y (camera looks down its -Z).
    let view = Mat4::from_cols(
        Vec4::new(1.0, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, -1.0, 0.0),
        Vec4::new(0.0, 1.0, 0.0, 0.0),
        Vec4::new(-eye.x, -eye.z, eye.y, 1.0),
    );
    projection * view
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
    let target = renderer.create_target(&gpu.device, [size, size]);
    let eye = Vec3::new(0.0, 0.0, 2.0);
    renderer.render(
        &gpu.device,
        &gpu.queue,
        &target,
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
        },
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
    let target = renderer.create_target(&gpu.device, [size, size]);
    // Standing on the Y axis, so it runs straight ahead into the middle of
    // the lower half; the X axis crosses in front.
    let eye = Vec3::new(0.0, -10.0, 2.0);
    renderer.render(
        &gpu.device,
        &gpu.queue,
        &target,
        &FrameParams {
            view_projection: looking_north(eye),
            camera_position: eye,
            focus: Vec3::ZERO,
            sky_top: srgb(SKY_TOP),
            sky_horizon: srgb(SKY_HORIZON),
            ground: srgb(GROUND),
            shading: Shading::default(),
            size_px: [64, 64],
        },
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
    let target = renderer.create_target(&gpu.device, [1920, 1080]);
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
    };
    let frames = 60;
    let start = std::time::Instant::now();
    for _ in 0..frames {
        renderer.render(&gpu.device, &gpu.queue, &target, &params, &[]);
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
    let gpu = gpu()?;
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let target = renderer.create_target(&gpu.device, [size, size]);
    let uploaded = renderer.upload_model(&gpu.device, &gpu.queue, model);
    assert_eq!(uploaded.triangles(), model.triangles());
    renderer.render(
        &gpu.device,
        &gpu.queue,
        &target,
        &FrameParams {
            view_projection: looking_north(eye),
            camera_position: eye,
            focus: Vec3::new(1.0e7, 1.0e7, 0.0),
            sky_top: srgb(SKY_TOP),
            sky_horizon: srgb(SKY_HORIZON),
            ground: srgb(GROUND),
            shading,
            size_px: [size, size],
        },
        &[&uploaded],
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
}
