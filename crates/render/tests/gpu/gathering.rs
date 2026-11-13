//! The CPU's side of drawing many copies, for the scale check.

use super::*;

/// The CPU's side of drawing `count` copies of `model` in rows of
/// `row` 3 m apart, seen from `eye` looking north, with or without the
/// sun's shadows: gathering, culling, sorting and handing them over (not
/// the GPU's drawing, which isn't waited for). With `moving`, every copy
/// moves every frame. The milliseconds a frame, and what the last drew.
pub(crate) fn gathering_ms(
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
pub(crate) fn gathering_verdict(what: &str, ms: f64, target: f64, judged: bool) {
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
