//! The pixel stage: exact at every common size, sliding by part of a pixel, filling the screen, and layers.

use super::*;

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

#[test]
fn layers_lie_over_each_other_each_slid_by_its_own_bit_of_a_pixel() {
    // Decision 0094: the back picture as it is, a layer over it clear but
    // for a 2 × 2 white block (its colours weighed by how covered they
    // are), slid half a pixel: two screen pixels at ×4.
    let Some(gpu) = gpu() else { return };
    let renderer = ViewportRenderer::new(&gpu.device, 1);
    let base = [480, 270];
    let back = pattern_source(&gpu, &renderer, base);
    let size = [base[0] + 2, base[1] + 2];
    let front = renderer.create_target(&gpu.device, size);
    let mut bytes = vec![0u8; (size[0] * size[1] * 4) as usize];
    for (x, y) in [(11, 11), (12, 11), (11, 12), (12, 12)] {
        let at = ((y * size[0] + x) * 4) as usize;
        bytes[at..at + 4].copy_from_slice(&[255, 255, 255, 255]);
    }
    gpu.queue.write_texture(
        front.color_texture().as_image_copy(),
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
    let screen = [1920, 1080];
    let image = renderer.create_pixel_image(&gpu.device, screen);
    let still = pixel_layout(base, screen, PixelScaling::WholeSteps, [0.0; 2]);
    let slid = pixel_layout(base, screen, PixelScaling::WholeSteps, [0.5, 0.0]);
    renderer.draw_pixel_layers(
        &gpu.device,
        &gpu.queue,
        &[(&back, still), (&front, slid)],
        &image,
    );
    let pixels = read_texture(&gpu.device, &gpu.queue, image.texture(), screen).unwrap();
    let at = |x: u32, y: u32| {
        let i = ((y * screen[0] + x) * 4) as usize;
        [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
    };
    // Source pixel 11 starts at screen pixel 40 still; slid, at 38.
    assert_eq!(at(38, 41), [255, 255, 255, 255], "the block, slid");
    assert_eq!(at(45, 47), [255, 255, 255, 255]);
    assert_eq!(
        at(46, 41),
        source_colour(12, 11),
        "the back picture after it"
    );
    assert_eq!(at(37, 41), source_colour(10, 11), "and before it");
    // Everywhere else, exactly the back picture.
    assert_eq!(at(200, 300), source_colour(51, 76));
    // The image keeps each picture's place from frame to frame: the next
    // frames lay them the other way round, then the block's alone, and
    // each shows what it was given.
    let frames: [&[(&render::ViewportTarget, render::PixelLayout)]; 2] =
        [&[(&front, still), (&back, still)], &[(&front, still)]];
    let mut shown = Vec::new();
    for laid in frames {
        renderer.draw_pixel_layers(&gpu.device, &gpu.queue, laid, &image);
        let pixels = read_texture(&gpu.device, &gpu.queue, image.texture(), screen).unwrap();
        let i = ((41 * screen[0] + 41) * 4) as usize;
        shown.push([pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]);
    }
    assert_eq!(shown[0], source_colour(11, 11), "the back picture over all");
    assert_eq!(shown[1], [255, 255, 255, 255], "the block alone");
}

#[test]
fn a_layer_s_picture_is_clear_where_nothing_is_drawn() {
    // Decision 0094: no sky, alpha 0 where nothing is, whole where a
    // sprite is.
    let Some(gpu) = gpu() else { return };
    let renderer = ViewportRenderer::new(&gpu.device, 1);
    let sheet = renderer.upload_sheet(&gpu.device, &gpu.queue, [4, 4], &sheet_pixels());
    let instances = [sprite_at(10.0, 10.0, 0.0, 0, [1.0, 1.0, 1.0, 0.0])];
    let under = [sprite_at(
        30.0,
        30.0,
        0.0,
        render::SpriteInstance::UNDER | render::SpriteInstance::PARALLAX,
        [1.0, 1.0, 1.0, 0.0],
    )];
    let runs = [
        render::SpriteRun {
            sheet: &sheet,
            instances: &instances,
        },
        render::SpriteRun {
            sheet: &sheet,
            instances: &under,
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
    let mut target = renderer.create_target(&gpu.device, [64, 64]);
    target.set_layered(true);
    renderer.render_scene(
        &gpu.device,
        &gpu.queue,
        &mut target,
        &params,
        &render::Scene {
            sprites: &runs,
            ..render::Scene::default()
        },
    );
    let pixels = read_pixels(&gpu.device, &gpu.queue, &target).unwrap();
    let alpha = |x: u32, y: u32| pixels[((y * 64 + x) * 4 + 3) as usize];
    assert_eq!(alpha(0, 0), 0, "clear where nothing is");
    assert_eq!(alpha(50, 50), 0);
    assert_eq!(alpha(10, 10), 255, "a sprite covers");
    assert_eq!(alpha(30, 30), 255, "a layer's flat sprite covers too");
    // Its colours as drawn.
    let got = pixel(&pixels, 64, 10, 10);
    let want = sheet_colour(0, 0);
    assert!(
        got.iter().zip(want).all(|(a, b)| a.abs_diff(b) <= 1),
        "{got:?}"
    );
    // An ordinary picture again: the sky, every pixel whole.
    target.set_layered(false);
    renderer.render_scene(
        &gpu.device,
        &gpu.queue,
        &mut target,
        &params,
        &render::Scene {
            sprites: &runs,
            ..render::Scene::default()
        },
    );
    let pixels = read_pixels(&gpu.device, &gpu.queue, &target).unwrap();
    assert_eq!(pixels[3], 255);
}

/// Part of the scale check (decision 0011): eight parallax layers laid
/// over the world's picture at 4K (decision 0094), each its own small
/// picture slid by its own bit of a pixel. Reported, not judged: software
/// drivers are far slower than a card.
#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_eight_parallax_layers_at_4k() {
    let Some(gpu) = gpu() else { return };
    let renderer = ViewportRenderer::new(&gpu.device, 1);
    let base = [480, 270];
    let sources: Vec<render::ViewportTarget> = (0..9)
        .map(|_| pattern_source(&gpu, &renderer, base))
        .collect();
    let image = renderer.create_pixel_image(&gpu.device, [3840, 2160]);
    let frames = 60u32;
    let start = std::time::Instant::now();
    for i in 0..frames {
        let laid: Vec<(&render::ViewportTarget, render::PixelLayout)> = sources
            .iter()
            .enumerate()
            .map(|(n, source)| {
                let slide = [(i as usize * (n + 1)) as f32 / 60.0 % 1.0, 0.0];
                (
                    source,
                    pixel_layout(base, [3840, 2160], PixelScaling::WholeSteps, slide),
                )
            })
            .collect();
        renderer.draw_pixel_layers(&gpu.device, &gpu.queue, &laid, &image);
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .unwrap();
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
    println!(
        "Scale check: eight parallax layers and the world laid at 3840x2160 {ms:.2} ms (about nine times the pixel stage alone) on {}",
        gpu.adapter.get_info().name
    );
}

/// Part of the scale check (decision 0011): the CPU's side of a busy 2D
/// game's frame after its pictures are drawn: eight parallax layers and
/// the world laid into a 1080p image, then three screens of game UI with
/// 20,000 shapes between them drawn over it. Only handing the work over
/// is timed (the GPU's drawing is waited for between frames, untimed).
/// Reported, not judged: drivers differ in what handing over costs.
#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_layers_and_game_ui_handed_over() {
    let Some(gpu) = gpu() else { return };
    let renderer = ViewportRenderer::new(&gpu.device, 1);
    let base = [480, 270];
    let screen = [1920, 1080];
    let sources: Vec<render::ViewportTarget> = (0..9)
        .map(|_| pattern_source(&gpu, &renderer, base))
        .collect();
    let image = renderer.create_pixel_image(&gpu.device, screen);
    let hud = ui::layout(&ui::sample::hud(ui::ScaleRule::Grow, 0.5), screen, None);
    let shapes: Vec<ui::Shape> = hud.iter().cycle().take(20_000 / 3).cloned().collect();
    let frames = 60u32;
    let mut spent = std::time::Duration::ZERO;
    for i in 0..frames {
        let laid: Vec<(&render::ViewportTarget, render::PixelLayout)> = sources
            .iter()
            .enumerate()
            .map(|(n, source)| {
                let slide = [(i as usize * (n + 1)) as f32 / 60.0 % 1.0, 0.0];
                (
                    source,
                    pixel_layout(base, screen, PixelScaling::WholeSteps, slide),
                )
            })
            .collect();
        let start = std::time::Instant::now();
        renderer.draw_pixel_layers(&gpu.device, &gpu.queue, &laid, &image);
        for _ in 0..3 {
            renderer.draw_ui_on_pixels(&gpu.device, &gpu.queue, &image, &shapes, None, None);
        }
        spent += start.elapsed();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .unwrap();
    }
    let ms = spent.as_secs_f64() * 1000.0 / f64::from(frames);
    println!(
        "Scale check: eight parallax layers and three screens of game UI (20,000 shapes) handed over {ms:.3} ms a frame on {}",
        gpu.adapter.get_info().name
    );
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
