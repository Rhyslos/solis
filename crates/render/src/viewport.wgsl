// Viewport shaders: the backdrop, the ground grid and the ground axes.
//
// Colours are written in linear space; the render target is sRGB, so the
// GPU converts them when storing. Z is up and units are metres; the grid
// lies on the X-Y plane.

struct Frame {
    // World to clip space.
    view_proj: mat4x4<f32>,
    // Clip space back to world space, to find each pixel's view direction.
    inv_view_proj: mat4x4<f32>,
    // Camera position (xyz); w unused.
    camera: vec4<f32>,
    // The point the grid is centred on (xyz); w = how far around it the
    // grid reaches before it has faded out.
    focus: vec4<f32>,
    // Grid spacings: fine, major, super; w = half the grid's width.
    grid: vec4<f32>,
    // Opacity of each grid level (fine, major, super; w unused).
    grid_strength: vec4<f32>,
    // Backdrop, linear colour: the sky straight up, the sky at the horizon,
    // and the ground just below the horizon.
    sky_top: vec4<f32>,
    sky_horizon: vec4<f32>,
    ground: vec4<f32>,
};

@group(0) @binding(0) var<uniform> frame: Frame;

// ---------------------------------------------------------- backdrop ---

struct SkyOut {
    @builtin(position) clip: vec4<f32>,
    // Position on screen, -1 to 1 both ways.
    @location(0) ndc: vec2<f32>,
};

// Linear colour to sRGB, done here because the viewport image is plain
// RGBA (see COLOR_FORMAT in renderer.rs).
fn encode(c: vec3<f32>) -> vec3<f32> {
    let low = c * 12.92;
    let high = 1.055 * pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, low, c <= vec3<f32>(0.0031308));
}

// One triangle that covers the whole view.
@vertex
fn sky_vs(@builtin(vertex_index) index: u32) -> SkyOut {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0),
    );
    let xy = corners[index];
    var out: SkyOut;
    out.clip = vec4<f32>(xy, 1.0, 1.0);
    out.ndc = xy;
    return out;
}

// A calm backdrop with a horizon: the sky lightens gently towards the top,
// and below the horizon lies a slightly darker ground that darkens a little
// more towards your feet. The horizon follows the camera as it tilts.
@fragment
fn sky_fs(in: SkyOut) -> @location(0) vec4<f32> {
    let far = frame.inv_view_proj * vec4<f32>(in.ndc, 1.0, 1.0);
    let direction = normalize(far.xyz / far.w - frame.camera.xyz);
    let elevation = direction.z;
    let sky = mix(frame.sky_horizon.rgb, frame.sky_top.rgb, smoothstep(0.0, 0.7, elevation));
    let ground = frame.ground.rgb * mix(1.0, 0.8, smoothstep(0.0, 0.7, -elevation));
    // Soften the horizon over about one pixel, so it never looks jagged.
    let edge = max(fwidth(elevation), 1e-5);
    let above = smoothstep(-edge, edge, elevation);
    return vec4<f32>(encode(mix(ground, sky, above)), 1.0);
}

// -------------------------------------------------------------- grid ---

struct GridOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec2<f32>,
};

// One large square on the ground, drawn as two triangles. The grid lines
// themselves are worked out per pixel in the fragment shader.
@vertex
fn grid_vs(@builtin(vertex_index) index: u32) -> GridOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0),
    );
    let xy = corners[index] * frame.grid.w;
    var out: GridOut;
    out.clip = frame.view_proj * vec4<f32>(xy, 0.0, 1.0);
    out.world = xy;
    return out;
}

// How strongly a pixel lies on a grid line of the given spacing (0 to 1).
// Lines are one pixel wide whatever the distance, and a level fades out
// when its lines would be closer together than a few pixels, which avoids
// shimmering patterns in the distance.
fn grid_line(coord: vec2<f32>, spacing: f32) -> f32 {
    let cell = coord / spacing;
    let per_pixel = max(fwidth(cell), vec2<f32>(1e-6, 1e-6));
    let distance_to_line = abs(fract(cell - 0.5) - 0.5) / per_pixel;
    let on_line = 1.0 - min(min(distance_to_line.x, distance_to_line.y), 1.0);
    let density = max(per_pixel.x, per_pixel.y);
    let level_fade = 1.0 - smoothstep(0.08, 0.25, density);
    return on_line * level_fade;
}

@fragment
fn grid_fs(in: GridOut) -> @location(0) vec4<f32> {
    let fine = grid_line(in.world, frame.grid.x);
    let major = grid_line(in.world, frame.grid.y);
    let super_ = grid_line(in.world, frame.grid.z);

    // A grid around the focus point, fading softly to nothing, so it reads
    // as a work surface rather than an endless floor.
    let reach = frame.focus.w;
    let distance = length(in.world - frame.focus.xy);
    let distance_fade = 1.0 - smoothstep(reach * 0.25, reach, distance);

    let strength = frame.grid_strength;
    let alpha = max(max(fine * strength.x, major * strength.y), super_ * strength.z) * distance_fade;
    if alpha <= 0.002 {
        discard;
    }
    return vec4<f32>(1.0, 1.0, 1.0, alpha);
}

// ------------------------------------------------------------- lines ---

struct LineIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct LineOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world: vec3<f32>,
};

@vertex
fn line_vs(in: LineIn) -> LineOut {
    var out: LineOut;
    out.clip = frame.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    out.world = in.position;
    return out;
}

@fragment
fn line_fs(in: LineOut) -> @location(0) vec4<f32> {
    // Lines on the ground fade with distance like the grid.
    let reach = frame.focus.w;
    let distance = length(in.world.xy - frame.focus.xy);
    let fade = 1.0 - smoothstep(reach * 0.35, reach, distance);
    return vec4<f32>(encode(in.color.rgb), in.color.a * fade);
}
