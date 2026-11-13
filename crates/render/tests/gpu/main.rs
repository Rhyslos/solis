//! Renders real frames on a GPU and checks the pixels.
//!
//! Machines without a GPU skip these tests with a note, unless the
//! environment variable `REQUIRE_GPU` is set, in which case a missing GPU is
//! a failure. The CI window job sets it and installs a software driver, so
//! these tests always run somewhere.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stderr)]

mod backdrop;
mod culling;
mod environment;
mod frames;
mod gathering;
mod instances;
mod lights;
mod lods;
mod looks;
mod pixel_stage;
mod projectiles;
mod sprites;

// Helpers more than one file uses.
use culling::render_scene;
use environment::{big_floor, env, showcase_on_floor};
use frames::wait_for_pick;
use gathering::{gathering_ms, gathering_verdict};
use lights::{brightness, floor, ground_pixel, looking_down, slanted_sun, square};
use looks::{render_env, render_lit, sample, showcase, unmapped};
use sprites::{flat_down, sheet_colour, sheet_pixels, sprite_at};

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
