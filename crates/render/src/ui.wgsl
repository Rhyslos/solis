// Game UI shapes (decision 0083): boxes with rounded corners, ellipses and
// lines, each an instance drawn as a quad whose fragments work out how far
// they are from the shape's edge (a signed distance), so edges are sharp
// at any size. Drawn over the finished picture, never tone mapped.

struct Screen {
    size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> screen: Screen;
// Letters as distance images (decision 0084): 128 on a letter's edge.
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;
// Pictures (decision 0089): RGBA, colours times their alpha, each with a
// pixel of its edge copied round it.
@group(0) @binding(3) var pictures: texture_2d<f32>;

// ui::text's SPREAD and ATLAS_SIDE, and ui::pictures's SIDE (a test in
// ui.rs checks these and the kinds and flags below against Rust's).
const SPREAD: f32 = 6.0;
const ATLAS_SIDE: f32 = 1024.0;
const PICTURES_SIDE: f32 = 2048.0;

struct Instance {
    @location(0) rect: vec4<f32>,
    @location(1) ends: vec4<f32>,
    // round (or a line's width), border width, kind, flags
    @location(2) shape: vec4<f32>,
    @location(3) colour_a: vec4<f32>,
    @location(4) colour_b: vec4<f32>,
    @location(5) border: vec4<f32>,
    // Where it may be drawn: left, top, right, bottom (a canvas's box).
    @location(6) clip: vec4<f32>,
    // Glow reach, outline width, pixel size (decision 0089).
    @location(7) halo: vec4<f32>,
    @location(8) glow: vec4<f32>,
    @location(9) outline: vec4<f32>,
};

struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) at: vec2<f32>,
    @location(1) @interpolate(flat) rect: vec4<f32>,
    @location(2) @interpolate(flat) ends: vec4<f32>,
    @location(3) @interpolate(flat) shape: vec4<f32>,
    @location(4) @interpolate(flat) colour_a: vec4<f32>,
    @location(5) @interpolate(flat) colour_b: vec4<f32>,
    @location(6) @interpolate(flat) border: vec4<f32>,
    @location(7) @interpolate(flat) cut: vec4<f32>,
    @location(8) @interpolate(flat) halo: vec4<f32>,
    @location(9) @interpolate(flat) glow: vec4<f32>,
    @location(10) @interpolate(flat) outline: vec4<f32>,
};

// Each kind of shape's number (any other is a box), and a shape's flags.
const KIND_ELLIPSE: f32 = 1.0;
const KIND_LINE: f32 = 2.0;
const KIND_LETTER: f32 = 3.0;
const KIND_PICTURE: f32 = 4.0;
const FLAG_CRISP: u32 = 1u;
const FLAG_DOWN: u32 = 2u;
const FLAG_BLOCKY: u32 = 4u;

@vertex
fn ui_vs(@builtin(vertex_index) v: u32, i: Instance) -> Out {
    // The quad covering the shape, a pixel wider for its soft edge (a
    // line's box is its ends', widened by its width).
    var lo = i.rect.xy;
    var hi = i.rect.zw;
    if (i.shape.z == KIND_LINE) {
        lo = min(i.ends.xy, i.ends.zw) - vec2<f32>(i.shape.x);
        hi = max(i.ends.xy, i.ends.zw) + vec2<f32>(i.shape.x);
    }
    // Room for its glow and outline, and a whole block of pixels.
    let reach = 1.0 + i.halo.x + i.halo.y + max(i.halo.z, 0.0);
    lo = lo - vec2<f32>(reach);
    hi = hi + vec2<f32>(reach);
    let corner = vec2<f32>(f32(v & 1u), f32((v >> 1u) & 1u));
    let at = mix(lo, hi, corner);
    let ndc = vec2<f32>(at.x / screen.size.x * 2.0 - 1.0, 1.0 - at.y / screen.size.y * 2.0);
    var o: Out;
    o.clip = vec4<f32>(ndc, 0.0, 1.0);
    o.at = at;
    o.rect = i.rect;
    o.ends = i.ends;
    o.shape = i.shape;
    o.colour_a = i.colour_a;
    o.colour_b = i.colour_b;
    o.border = i.border;
    o.cut = i.clip;
    o.halo = i.halo;
    o.glow = i.glow;
    o.outline = i.outline;
    return o;
}

fn rounded_box(p: vec2<f32>, half: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - half + vec2<f32>(r);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn ellipse(p: vec2<f32>, half: vec2<f32>) -> f32 {
    let h = max(half, vec2<f32>(0.0001));
    // Close to the true distance near the edge, which is all that's drawn.
    let k = length(p / h);
    return (k - 1.0) * min(h.x, h.y);
}

fn segment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let ab = b - a;
    let t = clamp(dot(p - a, ab) / max(dot(ab, ab), 0.0001), 0.0, 1.0);
    return length(p - a - ab * t);
}

// `a` over `b`, both with straight (not premultiplied) colours.
fn over(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    let alpha = a.a + b.a * (1.0 - a.a);
    if (alpha <= 0.0) {
        return vec4<f32>(0.0);
    }
    return vec4<f32>((a.rgb * a.a + b.rgb * b.a * (1.0 - a.a)) / alpha, alpha);
}

@fragment
fn ui_fs(o_in: Out) -> @location(0) vec4<f32> {
    var o = o_in;
    // The screen's pixel size: every pixel of a block is worked out at
    // the block's middle, so the whole screen is drawn in blocks lined up
    // from its top left.
    let block = o.halo.z;
    if (block > 1.0) {
        o.at = (floor(o.at / block) + vec2<f32>(0.5)) * block;
    }
    // Cut off outside its canvas (whole pixels: a pixel's centre inside).
    if (o.at.x < o.cut.x || o.at.y < o.cut.y || o.at.x >= o.cut.z || o.at.y >= o.cut.w) {
        discard;
    }
    let centre = (o.rect.xy + o.rect.zw) * 0.5;
    let half = max((o.rect.zw - o.rect.xy) * 0.5, vec2<f32>(0.0));
    let p = o.at - centre;
    let kind = o.shape.z;
    let flags = u32(o.shape.w);
    let crisp = (flags & FLAG_CRISP) != 0u;
    var d: f32;
    if (kind == KIND_LETTER) {
        // Where in the letter's distance image, then how far from its
        // edge in the image's pixels, then in the screen's.
        let t = clamp((o.at - o.rect.xy) / max(o.rect.zw - o.rect.xy, vec2<f32>(0.0001)), vec2<f32>(0.0), vec2<f32>(1.0));
        let uv = mix(o.ends.xy, o.ends.zw, t) / ATLAS_SIDE;
        let v = textureSampleLevel(atlas, atlas_sampler, uv, 0.0).r;
        d = -((v * 255.0 - 128.0) / 127.0 * SPREAD) * o.shape.x;
        // Past the letter's own image (a glow reaching further): its
        // distance from the image's edge added on.
        d = d + length(max(abs(p) - half, vec2<f32>(0.0)));
    } else if (kind == KIND_LINE) {
        d = segment(o.at, o.ends.xy, o.ends.zw) - o.shape.x * 0.5;
    } else if (kind == KIND_ELLIPSE) {
        d = ellipse(p, half);
    } else {
        d = rounded_box(p, half, min(o.shape.x, min(half.x, half.y)));
    }
    // How much of the pixel the shape covers: a soft pixel-wide edge, or
    // all or nothing for pixel art (and for blocks of the screen's pixel
    // size).
    let hard = crisp || block > 1.0;
    var cover = clamp(0.5 - d, 0.0, 1.0);
    if (hard) {
        cover = select(0.0, 1.0, d < 0.0);
    }
    // Its colour along it: across or down.
    let span = max(o.rect.zw - o.rect.xy, vec2<f32>(0.0001));
    var t = clamp((o.at.x - o.rect.x) / span.x, 0.0, 1.0);
    if ((flags & FLAG_DOWN) != 0u) {
        t = clamp((o.at.y - o.rect.y) / span.y, 0.0, 1.0);
    }
    var colour = mix(o.colour_a, o.colour_b, t);
    if (kind == KIND_PICTURE) {
        // Where in the picture, in the atlas's pixels: a whole pixel for
        // blocky pictures, else smoothed between the four nearest (kept
        // inside the picture's own pixels).
        let along = clamp((o.at - o.rect.xy) / span, vec2<f32>(0.0), vec2<f32>(1.0));
        let px = mix(o.ends.xy, o.ends.zw, along);
        var c: vec4<f32>;
        if ((flags & FLAG_BLOCKY) != 0u) {
            let lo = vec2<i32>(floor(o.ends.xy));
            let hi = max(vec2<i32>(ceil(o.ends.zw)) - vec2<i32>(1), lo);
            c = textureLoad(pictures, clamp(vec2<i32>(floor(px)), lo, hi), 0);
        } else {
            let q = clamp(px, o.ends.xy + vec2<f32>(0.5), max(o.ends.zw - vec2<f32>(0.5), o.ends.xy + vec2<f32>(0.5)));
            c = textureSampleLevel(pictures, atlas_sampler, q / PICTURES_SIDE, 0.0);
        }
        // Back to straight colours, then the shape's colour over it.
        let straight = select(vec3<f32>(0.0), c.rgb / max(c.a, 0.0001), c.a > 0.0);
        colour = vec4<f32>(straight, c.a) * o.colour_a;
    }
    // The border: the band inside the edge.
    let width = o.shape.y;
    if (width > 0.0 && kind != KIND_LINE) {
        var inside = clamp(0.5 - (d + width), 0.0, 1.0);
        if (crisp) {
            inside = select(0.0, 1.0, d + width < 0.0);
        }
        colour = mix(o.border, colour, inside);
    }
    var out = vec4<f32>(colour.rgb, colour.a * cover);
    // Its outline, just outside its edge, then its glow beyond that, both
    // under it.
    // A letter's distance reaches only as far as its distance image's
    // spread, so its outline and glow do too.
    var line = o.halo.y;
    var reach = o.halo.x;
    if (kind == KIND_LETTER) {
        let most = SPREAD * o.shape.x;
        line = min(line, most);
        reach = min(reach, max(most - line, 0.0));
    }
    // Only outside its edge: a hollow shape (a border, a focus ring) stays
    // hollow, and a see-through fill doesn't show them through it.
    var outside = clamp(d + 0.5, 0.0, 1.0);
    if (hard) {
        outside = select(0.0, 1.0, d >= 0.0);
    }
    if (line > 0.0) {
        var band = clamp(0.5 - (d - line), 0.0, 1.0);
        if (hard) {
            band = select(0.0, 1.0, d - line < 0.0);
        }
        band = band * outside;
        out = over(out, vec4<f32>(o.outline.rgb, o.outline.a * band));
    }
    if (reach > 0.0) {
        // Brightest at the edge, fading to nothing at its reach.
        let t = clamp((d - line) / reach, 0.0, 1.0);
        let g = (1.0 - t) * (1.0 - t) * outside;
        out = over(out, vec4<f32>(o.glow.rgb, o.glow.a * g * 0.75));
    }
    return out;
}
