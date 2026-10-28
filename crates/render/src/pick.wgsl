// Picking: each placement's number and depth, drawn into two small images
// only when the editor asks what is under the mouse (pick.rs). Its own
// module, so the viewport shader stays as it is.

struct PickFrame {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> frame: PickFrame;

struct MeshIn {
    @location(0) position: vec3<f32>,
};

// The same instance buffer the viewport draws with (InstanceRaw in
// instances.rs); only the placement and its number are read here.
struct Instance {
    @location(3) row_0: vec4<f32>,
    @location(4) row_1: vec4<f32>,
    @location(5) row_2: vec4<f32>,
    @location(10) pick: u32,
};

struct PickOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) @interpolate(flat) pick: u32,
};

@vertex
fn pick_vs(in: MeshIn, instance: Instance) -> PickOut {
    let p = vec4<f32>(in.position, 1.0);
    let world = vec3<f32>(dot(instance.row_0, p), dot(instance.row_1, p), dot(instance.row_2, p));
    var out: PickOut;
    out.clip = frame.view_projection * vec4<f32>(world, 1.0);
    out.pick = instance.pick;
    return out;
}

// One pixel: the placement's number, and its depth as the depth test saw
// it (1 near, 0 far) as bits, to find the world point under the mouse.
@fragment
fn pick_fs(in: PickOut) -> @location(0) vec2<u32> {
    return vec2<u32>(in.pick, bitcast<u32>(in.clip.z));
}

// ------------------------------------------------------------- sprites ---

// The same as in viewport.wgsl, so a sprite is picked where it's drawn.
const FOOT_DEPTH: f32 = 0.00005;

// The sprite's sheet (sprites.rs).
@group(1) @binding(0) var sheet_image: texture_2d<f32>;

// SpriteInstance (sprites.rs), with its number from a buffer of its own.
struct SpriteIn {
    @location(0) corner: vec3<f32>,
    @location(1) flags: u32,
    @location(2) axis_x: vec2<f32>,
    @location(3) axis_y: vec2<f32>,
    @location(4) rect: vec4<f32>,
    @location(7) foot: vec2<f32>,
    @location(8) pick: u32,
};

struct SpritePickOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) rect: vec4<f32>,
    @location(2) @interpolate(flat) flags: u32,
    @location(3) @interpolate(flat) pick: u32,
};

@vertex
fn sprite_pick_vs(@builtin(vertex_index) index: u32, s: SpriteIn) -> SpritePickOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
    );
    let local = corners[index] * s.rect.zw;
    let world = s.corner + vec3<f32>(s.axis_x * local.x + s.axis_y * local.y, 0.0);
    var out: SpritePickOut;
    out.clip = frame.view_projection * vec4<f32>(world, 1.0);
    let foot = frame.view_projection * vec4<f32>(s.foot, s.corner.z, 1.0);
    out.clip.z = out.clip.z - FOOT_DEPTH * (foot.y / foot.w) * out.clip.w;
    out.local = local;
    out.rect = s.rect;
    out.flags = s.flags;
    out.pick = s.pick;
    return out;
}

// Only the sprite's own pixels are picked: clicking the gap between a
// character's legs picks what's behind.
@fragment
fn sprite_pick_fs(in: SpritePickOut) -> @location(0) vec2<u32> {
    let p = floor(in.local);
    if p.x < 0.0 || p.y < 0.0 || p.x >= in.rect.z || p.y >= in.rect.w {
        discard;
    }
    var q = p;
    if (in.flags & 1u) != 0u {
        q.x = in.rect.z - 1.0 - p.x;
    }
    if (in.flags & 2u) != 0u {
        q.y = in.rect.w - 1.0 - p.y;
    }
    let a = textureLoad(sheet_image, vec2<i32>(in.rect.xy + q), 0).a;
    let see_through = (in.flags & 8u) != 0u;
    if a < 0.5 && !see_through || a <= 0.0 {
        discard;
    }
    return vec2<u32>(in.pick, bitcast<u32>(in.clip.z));
}
