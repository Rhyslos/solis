//! Models in each look: textures, outlines, glow, normal textures and metal.

use super::*;

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
pub(crate) fn render_lit(
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
pub(crate) fn unmapped() -> Environment {
    Environment {
        tonemap: Tonemap::Standard,
        ..Environment::default()
    }
}

/// As [`render_lit`], with the sky, fog and tone mapping in `environment`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_env(
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

pub(crate) fn sample() -> assets::Model {
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

pub(crate) fn showcase() -> assets::Model {
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
