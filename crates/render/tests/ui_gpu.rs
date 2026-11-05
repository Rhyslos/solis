//! Game UI drawn on a real GPU (decision 0083): shapes land where the
//! layout says, sharp at any size, blended over the picture underneath,
//! and pixel-art screens have hard edges.

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
    let (device, queue, _turn) = gpu()?;
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
    UiPipeline::new(&device, render::COLOR_FORMAT)
        .draw(&device, &queue, &view, size, shapes, atlas);
    Some(read_texture(&device, &queue, &texture, size).unwrap())
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
