//! Game UI drawn on a real GPU (decision 0083): shapes land where the
//! layout says, sharp at any size, blended over the picture underneath,
//! and pixel-art screens have hard edges; pictures show from their atlas
//! (decision 0089).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stderr)]

use render::{UiPipeline, read_texture};

static GPU_TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn gpu() -> Option<(
    wgpu::Device,
    wgpu::Queue,
    std::sync::MutexGuard<'static, ()>,
)> {
    let turn = GPU_TURN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle().with_env());
    let found =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .ok()
            .and_then(|a| {
                pollster::block_on(a.request_device(&wgpu::DeviceDescriptor::default())).ok()
            });
    match found {
        Some((device, queue)) => Some((device, queue, turn)),
        None => {
            assert!(
                std::env::var_os("REQUIRE_GPU").is_none(),
                "REQUIRE_GPU is set but no GPU adapter was found"
            );
            eprintln!("No GPU adapter available; skipping GPU UI test.");
            None
        }
    }
}

/// Draws `shapes` over a grey image `size` big and reads it back.
fn draw(size: [u32; 2], shapes: &[ui::Shape]) -> Option<Vec<u8>> {
    draw_with(size, shapes, None)
}

fn draw_with(
    size: [u32; 2],
    shapes: &[ui::Shape],
    atlas: Option<&ui::text::Atlas>,
) -> Option<Vec<u8>> {
    draw_all(size, shapes, atlas, None)
}

fn draw_all(
    size: [u32; 2],
    shapes: &[ui::Shape],
    atlas: Option<&ui::text::Atlas>,
    pictures: Option<&ui::Pictures>,
) -> Option<Vec<u8>> {
    let (device, queue, _turn) = gpu()?;
    let (texture, view) = grey_image(&device, &queue, size);
    UiPipeline::new(&device, render::COLOR_FORMAT)
        .draw(&device, &queue, &view, size, shapes, atlas, pictures);
    Some(read_texture(&device, &queue, &texture, size).unwrap())
}

/// A grey image `size` big, ready to draw over and read back.
fn grey_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: [u32; 2],
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ui test"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: render::COLOR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("grey"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.5,
                    g: 0.5,
                    b: 0.5,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    queue.submit([encoder.finish()]);
    (texture, view)
}

fn px(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * width + x) * 4) as usize;
    [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]]
}

fn near(a: [u8; 4], b: [u8; 3]) -> bool {
    (0..3).all(|i| a[i].abs_diff(b[i]) <= 3)
}

const GREY: [u8; 3] = [128, 128, 128];

#[test]
fn the_sample_hud_lands_where_the_layout_says_on_two_screens() {
    for size in [[640u32, 360u32], [1280, 540]] {
        let screen = ui::sample::hud(ui::ScaleRule::Grow, 0.5);
        let shapes = ui::layout(&screen, size, None);
        let Some(pixels) = draw(size, &shapes) else {
            return;
        };
        let s = size[1] as f32 / 1080.0;
        // Inside the money box, towards its right: its dark fill (three
        // quarters over the grey), in the top right corner of any screen.
        let money = shapes
            .iter()
            .find(|x| (x.rect[2] - (size[0] as f32 - 32.0 * s)).abs() < 0.5)
            .unwrap();
        let mid_y = ((money.rect[1] + money.rect[3]) / 2.0) as u32;
        let inside = (money.rect[2] - 8.0) as u32;
        let p = px(&pixels, size[0], inside, mid_y);
        assert!(near(p, [55, 50, 44]), "{size:?}: {p:?}");
        // Outside every shape: the grey underneath, untouched.
        assert!(
            near(px(&pixels, size[0], size[0] / 2, size[1] / 2), GREY),
            "{size:?}"
        );
        // The energy bar's full half is green, its empty half dark.
        let bar = shapes
            .iter()
            .find(|x| x.colours[0] != x.colours[1])
            .unwrap();
        let y = ((bar.rect[1] + bar.rect[3]) / 2.0) as u32;
        let green = px(&pixels, size[0], (bar.rect[0] + 3.0) as u32, y);
        assert!(green[1] > 150 && green[0] < 160, "{green:?}");
    }
}

#[test]
fn pixel_art_screens_have_hard_edges_and_smooth_ones_soft() {
    let mut screen = ui::Screen::new("t", [100.0, 100.0], ui::ScaleRule::Keep);
    screen.elements = vec![
        ui::Element::new("dot", ui::ElementKind::Ellipse)
            .sized([60.0, 60.0])
            .at(ui::Anchor::TOP_LEFT, [20.0, 20.0])
            .filled(ui::Fill::Solid(ui::Colour::WHITE)),
    ];
    let soft = ui::layout(&screen, [100, 100], None);
    screen.scale = ui::ScaleRule::WholeSteps;
    let hard = ui::layout(&screen, [100, 100], Some(1));
    let (Some(soft), Some(hard)) = (draw([100, 100], &soft), draw([100, 100], &hard)) else {
        return;
    };
    let between = |p: &[u8]| {
        (0..100 * 100)
            .filter(|i| {
                let v = p[i * 4];
                v > 135 && v < 250
            })
            .count()
    };
    assert_eq!(
        between(&hard),
        0,
        "pixel art: grey or white, nothing between"
    );
    assert!(between(&soft) > 20, "smooth: a soft edge all round");
    // The middle is white either way.
    assert!(near(px(&hard, 100, 50, 50), [255, 255, 255]));
}

#[test]
fn half_seen_shapes_blend_and_no_shapes_draw_nothing() {
    let mut screen = ui::Screen::new("t", [10.0, 10.0], ui::ScaleRule::Keep);
    screen.elements = vec![
        ui::Element::new("veil", ui::ElementKind::Box { round: 0.0 })
            .at(
                ui::Anchor::new(ui::Edge::Stretch, ui::Edge::Stretch),
                [0.0, 0.0],
            )
            .filled(ui::Fill::Solid(ui::Colour([0.0, 0.0, 0.0, 0.5]))),
    ];
    let Some(pixels) = draw([16, 16], &ui::layout(&screen, [16, 16], None)) else {
        return;
    };
    let p = px(&pixels, 16, 8, 8);
    assert!(p[0].abs_diff(64) <= 3, "half black over grey: {p:?}");
    let Some(none) = draw([8, 8], &[]) else {
        return;
    };
    assert!(near(px(&none, 8, 4, 4), GREY));
}

#[test]
fn one_pipeline_draws_several_screens_a_frame_each_with_its_own_shapes() {
    // The pipeline keeps one buffer for the shapes and one for the
    // screen's size, filled each draw: three screens drawn one after
    // another, without waiting, must each show their own shapes, the
    // first two sharing the buffer, the last with so many that it grows.
    let Some((device, queue, _turn)) = gpu() else {
        return;
    };
    let pipeline = UiPipeline::new(&device, render::COLOR_FORMAT);
    let box_at = |name: &str, at: [f32; 2], size: [f32; 2], colour: ui::Colour| {
        ui::Element::new(name, ui::ElementKind::Box { round: 0.0 })
            .sized(size)
            .at(ui::Anchor::TOP_LEFT, at)
            .filled(ui::Fill::Solid(colour))
    };
    let shapes_of = |design: [f32; 2], elements: Vec<ui::Element>| {
        let mut screen = ui::Screen::new("t", design, ui::ScaleRule::Keep);
        screen.elements = elements;
        ui::layout(&screen, [design[0] as u32, design[1] as u32], None)
    };
    let white = shapes_of(
        [100.0, 100.0],
        vec![box_at(
            "white",
            [20.0, 20.0],
            [60.0, 60.0],
            ui::Colour::WHITE,
        )],
    );
    let mut many = vec![box_at(
        "black",
        [0.0, 0.0],
        [100.0, 100.0],
        ui::Colour([0.0, 0.0, 0.0, 1.0]),
    )];
    many.extend((0..3000).map(|i| {
        box_at(
            "speck",
            [199.0, 99.0],
            [1.0, 1.0],
            ui::Colour([0.0, 0.0, i as f32 / 3000.0, 1.0]),
        )
    }));
    let many = shapes_of([200.0, 100.0], many);
    assert!(many.len() > 3000);
    let red = shapes_of(
        [100.0, 100.0],
        vec![box_at(
            "red",
            [50.0, 0.0],
            [50.0, 100.0],
            ui::Colour([1.0, 0.0, 0.0, 1.0]),
        )],
    );
    let drawn: Vec<(wgpu::Texture, [u32; 2])> = [
        (&white, [100, 100]),
        (&red, [100, 100]),
        (&many, [200, 100]),
    ]
    .into_iter()
    .map(|(shapes, size)| {
        let (texture, view) = grey_image(&device, &queue, size);
        pipeline.draw(&device, &queue, &view, size, shapes, None, None);
        (texture, size)
    })
    .collect();
    let read: Vec<Vec<u8>> = drawn
        .iter()
        .map(|(texture, size)| read_texture(&device, &queue, texture, *size).unwrap())
        .collect();
    assert!(near(px(&read[0], 100, 50, 50), [255, 255, 255]));
    assert!(
        near(px(&read[0], 100, 75, 10), GREY),
        "not the next one's box"
    );
    assert!(near(px(&read[1], 100, 75, 10), [255, 0, 0]));
    assert!(
        near(px(&read[1], 100, 25, 50), GREY),
        "not the first one's box"
    );
    assert!(near(px(&read[2], 200, 50, 50), [0, 0, 0]));
    assert!(near(px(&read[2], 200, 150, 50), GREY));
}

fn fonts() -> ui::Fonts {
    let (fonts, notes) = ui::Fonts::new(vec![
        include_bytes!("../../editor/assets/fonts/Inter-Regular.ttf").to_vec(),
        include_bytes!("../../editor/assets/fonts/Inter-SemiBold.ttf").to_vec(),
    ]);
    assert!(notes.is_empty());
    fonts
}

#[test]
fn text_is_drawn_from_the_letters_atlas_in_its_colour() {
    let mut fonts = fonts();
    let mut screen = ui::Screen::new("t", [200.0, 100.0], ui::ScaleRule::Keep);
    screen.elements = vec![
        ui::Element::new(
            "big",
            ui::ElementKind::Text {
                text: "H".into(),
                size: 80.0,
                font: 1,
                align: ui::Edge::Start,
            },
        )
        .at(
            ui::Anchor::new(ui::Edge::Stretch, ui::Edge::Stretch),
            [10.0, 0.0],
        )
        .filled(ui::Fill::Solid(ui::Colour::rgb(255, 0, 0))),
    ];
    let shapes = ui::layout_with(&screen, [200, 100], None, Some(&mut fonts));
    assert_eq!(shapes.len(), 1);
    let Some(pixels) = draw_with([200, 100], &shapes, Some(&fonts.atlas)) else {
        return;
    };
    let h = shapes[0].rect;
    // A third of the way down the H: red somewhere in its left half (the
    // stem; the box has room around the letter, so it's searched for),
    // and the grey below in the middle, between the stems.
    let y = (h[1] + (h[3] - h[1]) / 3.0) as u32;
    let red = (h[0].max(0.0) as u32..((h[0] + h[2]) / 2.0) as u32)
        .map(|x| px(&pixels, 200, x, y))
        .any(|c| c[0] > 200 && c[1] < 60 && c[2] < 60);
    assert!(red, "no red stem on row {y} of {h:?}");
    let gap = px(&pixels, 200, ((h[0] + h[2]) / 2.0) as u32, y);
    assert!(near(gap, GREY), "{gap:?}");
}

#[test]
fn each_set_of_fonts_draws_from_its_own_atlas() {
    // One pipeline drawing two sets of fonts' text, each with one letter
    // made: the second's atlas is sent though it has as many letters (it
    // used to be taken for the first's, drawing the first's letter).
    let word = |text: &str, fonts: &mut ui::Fonts| {
        let mut screen = ui::Screen::new("t", [200.0, 100.0], ui::ScaleRule::Keep);
        screen.elements = vec![
            ui::Element::new(
                "word",
                ui::ElementKind::Text {
                    text: text.into(),
                    size: 80.0,
                    font: 0,
                    align: ui::Edge::Start,
                },
            )
            .at(
                ui::Anchor::new(ui::Edge::Stretch, ui::Edge::Stretch),
                [10.0, 0.0],
            )
            .filled(ui::Fill::Solid(ui::Colour::WHITE)),
        ];
        ui::layout_with(&screen, [200, 100], None, Some(fonts))
    };
    let (mut first, mut second) = (fonts(), fonts());
    let h = word("H", &mut first);
    let o = word("O", &mut second);
    let Some((device, queue, _turn)) = gpu() else {
        return;
    };
    let size = [200, 100];
    let pipeline = UiPipeline::new(&device, render::COLOR_FORMAT);
    let mut drawn = Vec::new();
    for (shapes, atlas) in [(&h, &first.atlas), (&o, &second.atlas)] {
        let (texture, view) = grey_image(&device, &queue, size);
        pipeline.draw(&device, &queue, &view, size, shapes, Some(atlas), None);
        drawn.push(read_texture(&device, &queue, &texture, size).unwrap());
    }
    // The O as a pipeline of its own draws it.
    let (texture, view) = grey_image(&device, &queue, size);
    UiPipeline::new(&device, render::COLOR_FORMAT).draw(
        &device,
        &queue,
        &view,
        size,
        &o,
        Some(&second.atlas),
        None,
    );
    let alone = read_texture(&device, &queue, &texture, size).unwrap();
    assert!(drawn[1] == alone, "the O from its own atlas");
    assert!(drawn[0] != drawn[1]);
    // An atlas of the wrong size isn't sent (the card would refuse it):
    // its text draws from nothing, and nothing fails.
    let mut broken = second.atlas.clone();
    broken.pixels.truncate(10);
    let (texture, view) = grey_image(&device, &queue, size);
    UiPipeline::new(&device, render::COLOR_FORMAT).draw(
        &device,
        &queue,
        &view,
        size,
        &o,
        Some(&broken),
        None,
    );
    let empty = read_texture(&device, &queue, &texture, size).unwrap();
    assert!(near(px(&empty, 200, 100, 50), GREY));
}

/// A HUD as Lust's view gives it (decision 0087): a full-screen stack
/// pinning a column to the bottom left, with a word and a green bar.
fn lust_hud() -> ui::view::Node {
    use ui::view::{Node, Style};
    let n = |v: f64| Style::Number(v);
    let on = Style::Flag(true);
    let piece = |kind: &str, styles: Vec<(&str, Style)>, children: Vec<Node>| Node {
        kind: kind.into(),
        styles: styles
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
        children,
        ..Node::default()
    };
    let mut bar = piece(
        "progress",
        vec![
            ("to", n(10.0)),
            ("width", n(300.0)),
            ("height", n(24.0)),
            ("green", on.clone()),
        ],
        vec![],
    );
    bar.value = Some(10.0);
    let words = Node {
        text: "Day 3".into(),
        ..piece(
            "text",
            vec![
                ("size", n(48.0)),
                ("bold", on.clone()),
                ("white", on.clone()),
            ],
            vec![],
        )
    };
    piece(
        "view",
        vec![("width", n(1920.0)), ("height", n(1080.0))],
        vec![piece(
            "stack",
            vec![
                ("width", Style::Text("fill".into())),
                ("height", Style::Text("fill".into())),
                ("bottom", on.clone()),
                ("pad", n(48.0)),
            ],
            vec![piece("column", vec![("gap", n(12.0))], vec![words, bar])],
        )],
    )
}

#[test]
fn a_lust_view_is_laid_out_and_drawn() {
    let mut fonts = fonts();
    let view = lust_hud();
    let mut screen = ui::Screen::new("hud", ui::view::design_of(&view), ui::ScaleRule::Grow);
    screen.view = Some(view);
    let size = [960, 540];
    let shapes = ui::layout_with(&screen, size, None, Some(&mut fonts));
    let Some(pixels) = draw_with(size, &shapes, Some(&fonts.atlas)) else {
        return;
    };
    if let Some(path) = std::env::var_os("SOLIS_UI_PICTURE") {
        // A picture to look at by hand (PPM).
        let mut out = format!("P6\n{} {}\n255\n", size[0], size[1]).into_bytes();
        for p in pixels.chunks(4) {
            out.extend_from_slice(&p[..3]);
        }
        std::fs::write(path, out).unwrap();
    }
    // Half size: the bar, 300 × 24 at 48 from the bottom left, is green
    // at its middle; above the words, the grey picture shows through.
    let green = px(&pixels, size[0], 24 + 75, 540 - 24 - 6);
    assert!(green[1] > 120 && green[0] < 100, "{green:?}");
    assert!(near(px(&pixels, size[0], 480, 100), GREY));
}

#[test]
fn a_canvas_cuts_off_what_s_outside_it() {
    // A white box half out of a 40 × 40 group that clips, at (30, 30).
    let mut screen = ui::Screen::new("t", [100.0, 100.0], ui::ScaleRule::Keep);
    let mut canvas = ui::Element::new("canvas", ui::ElementKind::Group)
        .sized([40.0, 40.0])
        .at(ui::Anchor::TOP_LEFT, [30.0, 30.0]);
    canvas.clips = true;
    canvas.children = vec![
        ui::Element::new("box", ui::ElementKind::Box { round: 0.0 })
            .sized([40.0, 40.0])
            .at(ui::Anchor::TOP_LEFT, [20.0, 20.0])
            .filled(ui::Fill::Solid(ui::Colour::WHITE)),
    ];
    screen.elements = vec![canvas];
    let shapes = ui::layout(&screen, [100, 100], None);
    assert_eq!(shapes[0].clip, [30.0, 30.0, 70.0, 70.0]);
    let Some(pixels) = draw([100, 100], &shapes) else {
        return;
    };
    assert!(near(px(&pixels, 100, 60, 60), [255, 255, 255]), "inside");
    assert!(near(px(&pixels, 100, 75, 75), GREY), "outside: cut off");
    assert!(near(px(&pixels, 100, 69, 55), [255, 255, 255]));
    assert!(near(px(&pixels, 100, 71, 55), GREY));
}

#[test]
fn a_picture_shows_its_pixels_blocky_when_grown_and_see_through_where_it_is() {
    // A 2 × 2 picture: red, green, blue, and a see-through corner, shown
    // 8 times as big at (8, 8).
    let mut pictures = ui::Pictures::new();
    let rgba = [
        255, 0, 0, 255, 0, 255, 0, 255, //
        0, 0, 255, 255, 255, 255, 255, 0,
    ];
    pictures.add("dots.png", [2, 2], &rgba).unwrap();
    let screen = ui::Screen {
        elements: vec![
            ui::Element::new(
                "dots",
                ui::ElementKind::Picture {
                    name: "dots.png".into(),
                    round: 0.0,
                    cover: false,
                },
            )
            .sized([16.0, 16.0])
            .at(ui::Anchor::TOP_LEFT, [8.0, 8.0]),
        ],
        pictures: Some(std::sync::Arc::new(pictures)),
        ..ui::Screen::new("Dots", [64.0, 64.0], ui::ScaleRule::Keep)
    };
    let shapes = ui::layout(&screen, [64, 64], None);
    assert_eq!(shapes.len(), 1);
    let Some(pixels) = draw_all([64, 64], &shapes, None, screen.pictures.as_deref()) else {
        return;
    };
    let at = |x, y| px(&pixels, 64, x, y);
    assert!(near(at(9, 9), [255, 0, 0]), "{:?}", at(9, 9));
    assert!(near(at(22, 9), [0, 255, 0]), "{:?}", at(22, 9));
    assert!(near(at(9, 22), [0, 0, 255]), "{:?}", at(9, 22));
    // Right up to the edge of a pixel, no blending with the next.
    assert!(near(at(15, 9), [255, 0, 0]), "{:?}", at(15, 9));
    assert!(near(at(16, 9), [0, 255, 0]), "{:?}", at(16, 9));
    assert!(near(at(22, 22), GREY), "see-through: {:?}", at(22, 22));
    assert!(near(at(40, 40), GREY), "outside");
}

/// A white 40 × 40 box at (30, 30) on a 100 × 100 screen, with `glow`
/// and `outline` (design pixels) and the screen's `pixel` size.
fn haloed(
    glow: Option<(f32, ui::Colour)>,
    outline: Option<(f32, ui::Colour)>,
    pixel: f32,
) -> Vec<ui::Shape> {
    let mut screen = ui::Screen::new("t", [100.0, 100.0], ui::ScaleRule::Keep);
    let mut b = ui::Element::new("box", ui::ElementKind::Box { round: 0.0 })
        .sized([40.0, 40.0])
        .at(ui::Anchor::TOP_LEFT, [30.0, 30.0])
        .filled(ui::Fill::Solid(ui::Colour::WHITE));
    b.glow = glow;
    b.outline = outline;
    screen.elements = vec![b];
    let mut shapes = ui::layout(&screen, [100, 100], None);
    for s in &mut shapes {
        s.pixel = pixel;
    }
    shapes
}

#[test]
fn an_outline_rings_a_shape_and_a_glow_fades_out_past_it() {
    let red = ui::Colour::rgb(255, 0, 0);
    let blue = ui::Colour::rgb(0, 0, 255);
    let shapes = haloed(Some((10.0, blue)), Some((3.0, red)), 0.0);
    let Some(pixels) = draw([100, 100], &shapes) else {
        return;
    };
    assert!(
        near(px(&pixels, 100, 50, 50), [255, 255, 255]),
        "inside: the box"
    );
    // Just outside the box's edge (x 70): its outline.
    let ring = px(&pixels, 100, 71, 50);
    assert!(ring[0] > 200 && ring[2] < 60, "the outline: {ring:?}");
    // Past the outline: blue fading out to the grey.
    let near_glow = px(&pixels, 100, 75, 50);
    let far_glow = px(&pixels, 100, 81, 50);
    assert!(
        near_glow[2] > far_glow[2],
        "fades: {near_glow:?} {far_glow:?}"
    );
    assert!(near_glow[2] > 150, "{near_glow:?}");
    assert!(
        near(px(&pixels, 100, 90, 50), GREY),
        "past its reach: untouched"
    );
    // Without them, the grey right beside the box.
    let Some(plain) = draw([100, 100], &haloed(None, None, 0.0)) else {
        return;
    };
    assert!(near(px(&plain, 100, 72, 50), GREY));
}

#[test]
fn a_pixel_size_draws_the_screen_in_blocks() {
    // A circle, drawn in blocks of 5 pixels: every block one colour, and
    // nothing between white and grey.
    let mut screen = ui::Screen::new("t", [100.0, 100.0], ui::ScaleRule::Keep);
    screen.elements = vec![
        ui::Element::new("dot", ui::ElementKind::Ellipse)
            .sized([62.0, 62.0])
            .at(ui::Anchor::TOP_LEFT, [19.0, 19.0])
            .filled(ui::Fill::Solid(ui::Colour::WHITE)),
    ];
    let mut shapes = ui::layout(&screen, [100, 100], None);
    for s in &mut shapes {
        s.pixel = 5.0;
    }
    let Some(pixels) = draw([100, 100], &shapes) else {
        return;
    };
    for by in 0..20 {
        for bx in 0..20 {
            let first = px(&pixels, 100, bx * 5, by * 5);
            for (dx, dy) in [(4, 0), (0, 4), (4, 4), (2, 3)] {
                assert_eq!(
                    px(&pixels, 100, bx * 5 + dx, by * 5 + dy),
                    first,
                    "block {bx}, {by}"
                );
            }
            assert!(
                near(first, [255, 255, 255]) || near(first, GREY),
                "hard: {first:?}"
            );
        }
    }
}

#[test]
fn a_hollow_shape_s_glow_and_outline_stay_outside_it() {
    // A box with only a border: the inside stays the grey underneath.
    let mut screen = ui::Screen::new("t", [100.0, 100.0], ui::ScaleRule::Keep);
    let mut ring = ui::Element::new("ring", ui::ElementKind::Box { round: 0.0 })
        .sized([40.0, 40.0])
        .at(ui::Anchor::TOP_LEFT, [30.0, 30.0]);
    ring.border = Some((2.0, ui::Colour::rgb(0, 0, 255)));
    screen.elements = vec![ring];
    screen.look = ui::ScreenLook {
        glow: 8.0,
        outline: 3.0,
        outline_colour: ui::Colour::rgb(255, 0, 0),
        pixel: 1.0,
    };
    let shapes = ui::layout(&screen, [100, 100], None);
    let Some(pixels) = draw([100, 100], &shapes) else {
        return;
    };
    assert!(near(px(&pixels, 100, 50, 50), GREY), "inside: still hollow");
    let ring = px(&pixels, 100, 71, 50);
    assert!(
        ring[0] > 200 && ring[2] < 60,
        "the outline outside: {ring:?}"
    );
}
