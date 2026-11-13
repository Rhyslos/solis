//! Sprites: their pixels exactly as drawn, flips and tints, order, tile layers, picking and lit sprites.

use super::*;

/// A 4 × 4 sheet: every pixel its own colour, except (3, 3), which is
/// see-through, and (2, 3), mostly see-through (alpha 100 of 255).
pub(crate) fn sheet_pixels() -> Vec<u8> {
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
pub(crate) fn flat_down() -> Mat4 {
    Mat4::from_cols(
        Vec4::new(1.0 / 32.0, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 1.0 / 32.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0 / 100.0, 0.0),
        Vec4::new(-1.0, -1.0, 0.5, 1.0),
    )
}

/// A sprite of the whole sheet with its top-left corner at pixel
/// (`x`, `y`), `flags` and a tint (rgb) with a flash.
pub(crate) fn sprite_at(
    x: f32,
    y: f32,
    z: f32,
    flags: u32,
    tint: [f32; 4],
) -> render::SpriteInstance {
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
    let (cut, blended): (Vec<_>, Vec<_>) = sprites.iter().partition(|s| !s.see_through());
    render_runs(&[cut, blended])
}

/// Draws each of `groups` of sprites as a run of its own (a run is drawn
/// by its first sprite's pipeline), and reads the picture back.
fn render_runs(groups: &[Vec<render::SpriteInstance>]) -> Option<Vec<u8>> {
    render_sprites_lit(groups, Environment::default(), &[])
}

/// [`render_runs`], in `environment` with `lights`.
fn render_sprites_lit(
    groups: &[Vec<render::SpriteInstance>],
    environment: Environment,
    lights: &[render::Light],
) -> Option<Vec<u8>> {
    let sprites: Vec<&render::SpriteInstance> = groups.iter().flatten().collect();
    let gpu = gpu()?;
    let renderer = ViewportRenderer::new(&gpu.device, 1);
    let mut target = renderer.create_target(&gpu.device, [64, 64]);
    let sheet = renderer.upload_sheet(&gpu.device, &gpu.queue, [4, 4], &sheet_pixels());
    let runs: Vec<render::SpriteRun> = groups
        .iter()
        .map(|instances| render::SpriteRun {
            sheet: &sheet,
            instances,
        })
        .collect();
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
        environment,
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
            lights,
            ..render::Scene::default()
        },
    );
    assert_eq!(stats.sprites as usize, sprites.len());
    Some(read_pixels(&gpu.device, &gpu.queue, &target).unwrap())
}

pub(crate) fn sheet_colour(x: u8, y: u8) -> [u8; 3] {
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

#[test]
fn tile_layers_lie_under_and_over_every_sprite() {
    let white = [1.0, 1.0, 1.0, 0.0];
    let red = [1.0, 0.0, 0.0, 0.0];
    let blue = [0.0, 0.0, 1.0, 0.0];
    // A tile layer under (red), a sprite (white), a layer over (blue),
    // each nearer the camera than the last, given in any order: under
    // stays under and over stays over, whatever their height.
    let mut under = sprite_at(8.0, 8.0, 5.0, render::SpriteInstance::UNDER, red);
    under.axis_x = [4.0, 0.0];
    under.axis_y = [0.0, -4.0];
    let mut over = sprite_at(40.0, 40.0, -5.0, render::SpriteInstance::OVER, blue);
    over.axis_x = [2.0, 0.0];
    over.axis_y = [0.0, -2.0];
    for order in [false, true] {
        let mut sprites = [
            under,
            sprite_at(10.0, 10.0, 0.0, 0, white),
            over,
            sprite_at(41.0, 41.0, 0.0, 0, white),
        ];
        if order {
            sprites.reverse();
        }
        let runs: Vec<Vec<render::SpriteInstance>> = sprites.iter().map(|s| vec![*s]).collect();
        let Some(pixels) = render_runs(&runs) else {
            return;
        };
        // The under layer shows where nothing stands on it...
        let p = pixel(&pixels, 64, 15, 15);
        assert!(p[0] > 0 && p[1] == 0 && p[2] == 0, "under shows: {p:?}");
        // ...and the sprite stands on it.
        assert!(
            pixel(&pixels, 64, 10, 10)[1] > 0,
            "the sprite is over the under layer"
        );
        // The over layer covers the sprite.
        let p = pixel(&pixels, 64, 41, 41);
        assert!(p[0] == 0 && p[1] == 0 && p[2] > 0, "over covers: {p:?}");
    }
}

/// Whether `c` lies between `a` and `b` in every channel, give or take.
fn near_between(c: [u8; 3], a: [u8; 3], b: [u8; 3]) -> bool {
    (0..3).all(|i| c[i] + 3 >= a[i].min(b[i]) && c[i] <= a[i].max(b[i]) + 3)
}

#[test]
fn sprites_are_picked_by_their_own_pixels_nearest_first() {
    let Some(gpu) = gpu() else {
        return;
    };
    let renderer = ViewportRenderer::new(&gpu.device, 1);
    let mut target = renderer.create_target(&gpu.device, [64, 64]);
    let sheet = renderer.upload_sheet(&gpu.device, &gpu.queue, [4, 4], &sheet_pixels());
    let white = [1.0, 1.0, 1.0, 0.0];
    // A at (10, 10); B two pixels right of it, a metre higher (nearer).
    let instances = [
        sprite_at(10.0, 10.0, 0.0, 0, white),
        sprite_at(12.0, 10.0, 1.0, 0, white),
    ];
    let picks = [3, 4];
    let runs = [render::SpritePicks {
        run: render::SpriteRun {
            sheet: &sheet,
            instances: &instances,
        },
        picks: &picks,
    }];
    assert!(renderer.request_pick_with_sprites(
        &gpu.device,
        &gpu.queue,
        &mut target,
        flat_down(),
        &[],
        &runs,
        [0, 0, 64, 64],
    ));
    let result = wait_for_pick(&gpu, &mut target);
    assert_eq!(result.pick_at(10, 10), Some(3));
    assert_eq!(result.pick_at(11, 13), Some(3));
    // Where they overlap, the nearer one.
    assert_eq!(result.pick_at(13, 11), Some(4));
    // B's clear pixel (3, 3) and its mostly clear one (2, 3) pick nothing:
    // A doesn't reach that far.
    assert_eq!(result.pick_at(15, 13), None);
    assert_eq!(result.pick_at(14, 13), None);
    assert_eq!(result.pick_at(0, 0), None);
    assert_eq!(result.all(), std::collections::BTreeSet::from([3, 4]));
}

#[test]
fn parallax_layers_are_picked_behind_everything_or_over_it() {
    // Decision 0094: a layer behind is picked only where nothing else is;
    // one in front over everything.
    let Some(gpu) = gpu() else {
        return;
    };
    let renderer = ViewportRenderer::new(&gpu.device, 1);
    let mut target = renderer.create_target(&gpu.device, [64, 64]);
    let sheet = renderer.upload_sheet(&gpu.device, &gpu.queue, [4, 4], &sheet_pixels());
    let white = [1.0, 1.0, 1.0, 0.0];
    let behind = render::SpriteInstance::PARALLAX | render::SpriteInstance::UNDER;
    let front = render::SpriteInstance::PARALLAX | render::SpriteInstance::OVER;
    // A layer behind, a metre higher than the sprite (which would win
    // without the flag); the sprite two pixels right; a layer in front
    // over the sprite's right half, lower down.
    let back = [sprite_at(10.0, 10.0, 1.0, behind, white)];
    let middle = [sprite_at(12.0, 10.0, 0.0, 0, white)];
    let near = [sprite_at(14.0, 10.0, -1.0, front, white)];
    let runs = [
        render::SpritePicks {
            run: render::SpriteRun {
                sheet: &sheet,
                instances: &back,
            },
            picks: &[5],
        },
        render::SpritePicks {
            run: render::SpriteRun {
                sheet: &sheet,
                instances: &middle,
            },
            picks: &[6],
        },
        render::SpritePicks {
            run: render::SpriteRun {
                sheet: &sheet,
                instances: &near,
            },
            picks: &[7],
        },
    ];
    assert!(renderer.request_pick_with_sprites(
        &gpu.device,
        &gpu.queue,
        &mut target,
        flat_down(),
        &[],
        &runs,
        [0, 0, 64, 64],
    ));
    let result = wait_for_pick(&gpu, &mut target);
    assert_eq!(
        result.pick_at(10, 10),
        Some(5),
        "the layer, where it's alone"
    );
    assert_eq!(
        result.pick_at(12, 11),
        Some(6),
        "the sprite over the layer behind"
    );
    assert_eq!(
        result.pick_at(14, 11),
        Some(7),
        "the layer in front over the sprite"
    );
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_100000_sprites() {
    // The CPU's side of drawing 100,000 sprites of one sheet, the plan's
    // budget for a 2D game: handing them over in one go (the GPU's drawing
    // isn't waited for, so it's reported by the Profiler instead).
    let Some(gpu) = gpu() else {
        return;
    };
    let renderer = ViewportRenderer::new(&gpu.device, 1);
    let mut target = renderer.create_target(&gpu.device, [64, 64]);
    let sheet = renderer.upload_sheet(&gpu.device, &gpu.queue, [4, 4], &sheet_pixels());
    let sprites: Vec<render::SpriteInstance> = (0..100_000)
        .map(|i| {
            sprite_at(
                (i % 60) as f32,
                (i / 60 % 60) as f32,
                0.0,
                0,
                [1.0, 1.0, 1.0, 0.0],
            )
        })
        .collect();
    let runs = [render::SpriteRun {
        sheet: &sheet,
        instances: &sprites,
    }];
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
    let frames = 10;
    let mut spent = std::time::Duration::ZERO;
    for _ in 0..frames {
        let start = std::time::Instant::now();
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
        spent += start.elapsed();
        assert_eq!(stats.sprites, 100_000);
        let _ = read_pixels(&gpu.device, &gpu.queue, &target);
    }
    let ms = spent.as_secs_f64() * 1000.0 / f64::from(frames);
    gathering_verdict("handing over 100,000 sprites", ms, 2.0, true);
}

#[test]
fn lit_sprites_take_the_day_s_tint_and_a_lamp_s_pool_in_steps() {
    // Two sprites far apart; a lamp 2 m above the first.
    let white = [1.0, 1.0, 1.0, 0.0];
    let sprites = vec![
        sprite_at(10.0, 10.0, 0.0, 0, white),
        sprite_at(50.0, 50.0, 0.0, 0, white),
    ];
    let night = Environment {
        tint: [0.25, 0.25, 0.5],
        sprite_light: Some(0),
        ..Environment::default()
    };
    let lamp = render::Light {
        kind: render::LightKind::Point,
        position: Vec3::new(12.0, 64.0 - 12.0, 2.0),
        direction: Vec3::NEG_Z,
        color: [1.0, 0.9, 0.6],
        strength: 300.0,
        range: Some(8.0),
    };
    let Some(plain) = render_runs(std::slice::from_ref(&sprites)) else {
        return;
    };
    let Some(lit) = render_sprites_lit(std::slice::from_ref(&sprites), night, &[lamp]) else {
        return;
    };
    let far = |p: &[u8]| pixel(p, 64, 50, 50);
    let near = |p: &[u8]| pixel(p, 64, 11, 11);
    // Far from the lamp: the night's tint, darker and bluer.
    let (a, b) = (far(&plain), far(&lit));
    assert!(b[0] < a[0] && b[1] < a[1], "darker: {a:?} {b:?}");
    assert!(
        f32::from(b[2]) / f32::from(b[0].max(1)) > f32::from(a[2]) / f32::from(a[0].max(1)),
        "bluer"
    );
    // Under the lamp: brighter than the far one.
    let lamp_lit = near(&lit);
    assert!(
        lamp_lit[0] > b[0] + 20,
        "the lamp's pool: {lamp_lit:?} {b:?}"
    );
    // Unlit (the switch off): exactly as drawn, tint or not.
    let off = Environment {
        sprite_light: None,
        ..night
    };
    let Some(as_drawn) = render_sprites_lit(std::slice::from_ref(&sprites), off, &[lamp]) else {
        return;
    };
    assert_eq!(far(&as_drawn), far(&plain));
    assert_eq!(near(&as_drawn), near(&plain));
    // In steps: the light comes in bands, so fewer shades along a line
    // out from the lamp.
    let banded = Environment {
        sprite_light: Some(2),
        tint: [0.0; 3],
        ..night
    };
    let wide = vec![sprite_at(10.0, 10.0, 0.0, 0, white); 1];
    let Some(steps) = render_sprites_lit(std::slice::from_ref(&wide), banded, &[lamp]) else {
        return;
    };
    let Some(smooth) = render_sprites_lit(
        std::slice::from_ref(&wide),
        Environment {
            sprite_light: Some(0),
            ..banded
        },
        &[lamp],
    ) else {
        return;
    };
    assert_ne!(near(&steps), near(&smooth), "banded differs from smooth");
    // A dark night in steps stays dark, never black: only the lamps'
    // light is banded (a review finding).
    let dark = Environment {
        sprite_light: Some(8),
        tint: [0.16, 0.18, 0.35],
        ..night
    };
    let Some(dark_night) = render_sprites_lit(std::slice::from_ref(&sprites), dark, &[lamp]) else {
        return;
    };
    let d = far(&dark_night);
    assert!(d[2] > 0, "not black: {d:?}");
}
