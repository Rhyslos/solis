// The pixel stage (Phase 9, D1): a 2D game's small picture, drawn at its
// base resolution with a one-pixel margin all round, scaled up to the
// screen. See pixel.rs for the layout these numbers come from.

struct Stage {
    // The picture's top-left corner on the screen and its size there, in
    // screen pixels.
    origin: vec2<f32>,
    size: vec2<f32>,
    // The base resolution (the source holds this plus a pixel each side).
    base: vec2<f32>,
    // How many screen pixels make one source pixel.
    scale: f32,
    // 1: whole steps (every screen pixel shows one source pixel exactly);
    // 0: fill (the last bit between whole steps smoothed, edges sharp).
    whole: f32,
    // How far the picture slides across the source: in whole screen
    // pixels for whole steps, in source pixels for fill.
    slide: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> stage: Stage;
@group(0) @binding(1) var source: texture_2d<f32>;
@group(0) @binding(2) var smooth_sampler: sampler;

struct Out {
    @builtin(position) clip: vec4<f32>,
};

@vertex
fn pixel_vs(@builtin(vertex_index) index: u32) -> Out {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0),
    );
    var out: Out;
    out.clip = vec4<f32>(corners[index], 0.0, 1.0);
    return out;
}

@fragment
fn pixel_fs(in: Out) -> @location(0) vec4<f32> {
    // This screen pixel, from the picture's corner.
    let at = floor(in.clip.xy) - stage.origin;
    if at.x < 0.0 || at.y < 0.0 || at.x >= stage.size.x || at.y >= stage.size.y {
        // The bars around a picture that doesn't fill the screen.
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    if stage.whole > 0.5 {
        // Whole steps: the one source pixel under it, never a blend. The
        // slide is whole screen pixels, so the picture still moves by
        // less than a source pixel without blurring.
        let texel = floor((at + stage.slide) / stage.scale) + vec2<f32>(1.0, 1.0);
        let most = stage.base + vec2<f32>(1.0, 1.0);
        let clamped = clamp(texel, vec2<f32>(0.0, 0.0), most);
        return vec4<f32>(textureLoad(source, vec2<i32>(clamped), 0).rgb, 1.0);
    }
    // Fill: where the middle of this screen pixel falls in the source,
    // with the blend squeezed into the last screen pixel of each source
    // pixel ("sharp bilinear"), so edges stay sharp at any size.
    let src = (at + vec2<f32>(0.5, 0.5)) / stage.scale + stage.slide + vec2<f32>(1.0, 1.0);
    let texel = src - vec2<f32>(0.5, 0.5);
    let corner = floor(texel);
    let f = texel - corner;
    let sharp = clamp((f - vec2<f32>(0.5, 0.5)) * stage.scale + vec2<f32>(0.5, 0.5), vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
    let size = stage.base + vec2<f32>(2.0, 2.0);
    let uv = (corner + vec2<f32>(0.5, 0.5) + sharp) / size;
    return vec4<f32>(textureSampleLevel(source, smooth_sampler, uv, 0.0).rgb, 1.0);
}
