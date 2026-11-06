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

// ui::text's SPREAD and ATLAS_SIDE.
const SPREAD: f32 = 6.0;
const ATLAS_SIDE: f32 = 1024.0;
// ui::pictures's SIDE.
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
};

const KIND_ELLIPSE: f32 = 1.0;
const KIND_LINE: f32 = 2.0;
const KIND_LETTER: f32 = 3.0;
const KIND_PICTURE: f32 = 4.0;

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
    lo = lo - vec2<f32>(1.0);
    hi = hi + vec2<f32>(1.0);
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

@fragment
fn ui_fs(o: Out) -> @location(0) vec4<f32> {
    // Cut off outside its canvas (whole pixels: a pixel's centre inside).
    if (o.at.x < o.cut.x || o.at.y < o.cut.y || o.at.x >= o.cut.z || o.at.y >= o.cut.w) {
        discard;
    }
    let centre = (o.rect.xy + o.rect.zw) * 0.5;
    let half = max((o.rect.zw - o.rect.xy) * 0.5, vec2<f32>(0.0));
    let p = o.at - centre;
    let kind = o.shape.z;
    let flags = u32(o.shape.w);
    let crisp = (flags & 1u) != 0u;
    var d: f32;
    if (kind == KIND_LETTER) {
        // Where in the letter's distance image, then how far from its
        // edge in the image's pixels, then in the screen's.
        let t = clamp((o.at - o.rect.xy) / max(o.rect.zw - o.rect.xy, vec2<f32>(0.0001)), vec2<f32>(0.0), vec2<f32>(1.0));
        let uv = mix(o.ends.xy, o.ends.zw, t) / ATLAS_SIDE;
        let v = textureSampleLevel(atlas, atlas_sampler, uv, 0.0).r;
        d = -((v * 255.0 - 128.0) / 127.0 * SPREAD) * o.shape.x;
    } else if (kind == KIND_LINE) {
        d = segment(o.at, o.ends.xy, o.ends.zw) - o.shape.x * 0.5;
    } else if (kind == KIND_ELLIPSE) {
        d = ellipse(p, half);
    } else {
        d = rounded_box(p, half, min(o.shape.x, min(half.x, half.y)));
    }
    // How much of the pixel the shape covers: a soft pixel-wide edge, or
    // all or nothing for pixel art.
    var cover = clamp(0.5 - d, 0.0, 1.0);
    if (crisp) {
        cover = select(0.0, 1.0, d < 0.0);
    }
    // Its colour along it: across or down.
    let span = max(o.rect.zw - o.rect.xy, vec2<f32>(0.0001));
    var t = clamp((o.at.x - o.rect.x) / span.x, 0.0, 1.0);
    if ((flags & 2u) != 0u) {
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
        if ((flags & 4u) != 0u) {
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
    return vec4<f32>(colour.rgb, colour.a * cover);
}
