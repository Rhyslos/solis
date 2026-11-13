//! The sky, fog, exposure and tone mapping, and two viewports keeping their own frames.

use super::*;

/// The showcase scene standing on a big floor, for looking at.
pub(crate) fn showcase_on_floor() -> assets::Model {
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
pub(crate) fn big_floor(half: f32) -> assets::Model {
    let mut model = floor(false);
    model.meshes = vec![square(0.0, 0.05, half)];
    model.show_every_mesh();
    model.bounds = assets::Bounds {
        min: Vec3::new(-half, -half, 0.0),
        max: Vec3::new(half, half, 0.05),
    };
    model
}

pub(crate) fn env(sky: Sky, fog: Option<Fog>, exposure: f32, tonemap: Tonemap) -> Environment {
    Environment {
        sky,
        fog,
        exposure,
        tonemap,
        ..Environment::default()
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
