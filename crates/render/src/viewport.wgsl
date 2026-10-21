// Viewport shaders: the backdrop, models, the ground grid and the ground
// axes.
//
// Colours are worked out in linear space and encoded to sRGB as they are
// written (see `encode`). Z is up and units are metres; the grid
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
    // How models are shaded: x = 0 standard, 1 stylized, 2 unlit; y = the
    // stylized look's bands of light; z = its outline width in pixels
    // (0 for none).
    shading: vec4<f32>,
    // The image size in pixels (xy).
    screen: vec4<f32>,
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

// A calm backdrop, as in the mockup: dark above, lightening gently towards
// the horizon, and the ground lightening a little more towards your feet,
// so the horizon is barely seen. It follows the camera as it tilts.
@fragment
fn sky_fs(in: SkyOut) -> @location(0) vec4<f32> {
    let far = frame.inv_view_proj * vec4<f32>(in.ndc, 1.0, 1.0);
    let direction = normalize(far.xyz / far.w - frame.camera.xyz);
    let elevation = direction.z;
    let sky = mix(frame.sky_horizon.rgb, frame.sky_top.rgb, smoothstep(0.0, 0.7, elevation));
    let ground = frame.ground.rgb * mix(1.0, 1.15, smoothstep(0.0, 0.7, -elevation));
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
    // A cool grey-blue (sRGB #9aa4b4), as in the mockup.
    return vec4<f32>(encode(vec3<f32>(0.323, 0.371, 0.456)), alpha);
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

// ------------------------------------------------------------ models ---

// One material's settings (group 1), as MaterialUniform in gpu_model.rs.
struct Material {
    // Linear RGBA, multiplied with the colour texture.
    base_color: vec4<f32>,
    // Glow, linear RGB, multiplied with the glow texture.
    emissive: vec4<f32>,
    // x metallic, y roughness, z normal strength, w occlusion strength.
    surface: vec4<f32>,
    // x: alpha cutoff; y: 0 solid, 1 cut out, 2 blended.
    alpha: vec4<f32>,
};

@group(1) @binding(0) var<uniform> material: Material;
@group(1) @binding(1) var base_texture: texture_2d<f32>;
@group(1) @binding(2) var surface_texture: texture_2d<f32>;
@group(1) @binding(3) var normal_texture: texture_2d<f32>;
@group(1) @binding(4) var occlusion_texture: texture_2d<f32>;
@group(1) @binding(5) var emissive_texture: texture_2d<f32>;
@group(1) @binding(6) var base_sampler: sampler;
@group(1) @binding(7) var surface_sampler: sampler;
@group(1) @binding(8) var normal_sampler: sampler;
@group(1) @binding(9) var occlusion_sampler: sampler;
@group(1) @binding(10) var emissive_sampler: sampler;

struct MeshIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct MeshOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world: vec3<f32>,
};

@vertex
fn mesh_vs(in: MeshIn) -> MeshOut {
    var out: MeshOut;
    out.clip = frame.view_proj * vec4<f32>(in.position, 1.0);
    out.normal = in.normal;
    out.uv = in.uv;
    out.world = in.position;
    return out;
}

// The light, until real lights arrive in milestone 4: a fixed sun, and
// soft light from the sky above and the ground below (the "sky light" of
// decision 0008, the first source of indirect light).
const SUN_DIRECTION: vec3<f32> = vec3<f32>(0.35, -0.55, 0.76);
const SUN_COLOR: vec3<f32> = vec3<f32>(1.0, 0.96, 0.9);
const SUN_STRENGTH: f32 = 3.0;
const SKY_LIGHT: vec3<f32> = vec3<f32>(0.30, 0.34, 0.40);
const GROUND_LIGHT: vec3<f32> = vec3<f32>(0.10, 0.09, 0.08);
const PI: f32 = 3.14159265;

// The light arriving from the sky and ground along direction `d`.
fn sky_light(d: vec3<f32>) -> vec3<f32> {
    return mix(GROUND_LIGHT, SKY_LIGHT, clamp(d.z * 0.5 + 0.5, 0.0, 1.0));
}

// The standard look: glTF's metal-and-roughness material, lit by the sun
// (GGX highlights, the same maths as most engines) and the sky light.
fn standard(base: vec3<f32>, metallic: f32, roughness: f32, occlusion: f32,
            n: vec3<f32>, v: vec3<f32>) -> vec3<f32> {
    let l = normalize(SUN_DIRECTION);
    let h = normalize(v + l);
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_v = max(dot(n, v), 0.0001);
    let n_dot_h = max(dot(n, h), 0.0);
    let v_dot_h = max(dot(v, h), 0.0);
    let a = max(roughness * roughness, 0.002);
    let a2 = a * a;
    let d_term = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    let distribution = a2 / (PI * d_term * d_term);
    let k = (roughness + 1.0) * (roughness + 1.0) / 8.0;
    let geometry = (n_dot_v / (n_dot_v * (1.0 - k) + k)) * (n_dot_l / (n_dot_l * (1.0 - k) + k));
    let f0 = mix(vec3<f32>(0.04), base, metallic);
    let fresnel = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - v_dot_h, 5.0);
    let specular = distribution * geometry * fresnel / (4.0 * n_dot_v * n_dot_l + 0.0001);
    let diffuse = (vec3<f32>(1.0) - fresnel) * (1.0 - metallic) * base / PI;
    let direct = (diffuse + specular) * SUN_COLOR * SUN_STRENGTH * n_dot_l;

    // Indirect light from the sky: diffuse from around the normal, and a
    // reflection of the sky, weighted by a well-known fit (Karis) of how
    // much a surface of this roughness reflects.
    let c0 = vec4<f32>(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4<f32>(1.0, 0.0425, 1.04, -0.04);
    let r = roughness * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * n_dot_v)) * r.x + r.y;
    let ab = vec2<f32>(-1.04, 1.04) * a004 + r.zw;
    let reflected = sky_light(reflect(-v, n)) * (f0 * ab.x + ab.y);
    let indirect = (base * (1.0 - metallic) * sky_light(n) + reflected) * occlusion;
    return direct + indirect;
}

// The stylized look: light in a few flat bands with crisp edges, a sharp
// highlight on glossy surfaces, and a rim of sky light around the edges.
// `band_edge` is how wide one pixel is in the banded light, so band edges
// stay smooth instead of jagged.
fn stylized(base: vec3<f32>, roughness: f32, occlusion: f32, n: vec3<f32>,
            v: vec3<f32>, bands: f32, band_edge: f32) -> vec3<f32> {
    let l = normalize(SUN_DIRECTION);
    let n_dot_l = max(dot(n, l), 0.0);
    let x = n_dot_l * bands;
    let banded = (floor(x) + smoothstep(0.5 - band_edge, 0.5 + band_edge, fract(x))) / bands;
    let shadow = sky_light(n) * 1.3 * occlusion;
    let lit = base * (shadow + SUN_COLOR * 0.95 * banded);
    let h = normalize(v + l);
    let gloss = 1.0 - roughness;
    let highlight = smoothstep(0.5 - band_edge, 0.5 + band_edge,
        pow(max(dot(n, h), 0.0), 8.0 + 120.0 * gloss * gloss)) * gloss * 0.6;
    let rim = smoothstep(0.55, 0.65, 1.0 - max(dot(n, v), 0.0)) * 0.35;
    return lit + SUN_COLOR * highlight * step(0.001, n_dot_l) + SKY_LIGHT * rim;
}

@fragment
fn mesh_fs(in: MeshOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    // Everything that needs screen-space slopes comes first, before any
    // branch or discard, as WGSL requires.
    let base_sample = textureSample(base_texture, base_sampler, in.uv);
    let surface_sample = textureSample(surface_texture, surface_sampler, in.uv);
    let normal_sample = textureSample(normal_texture, normal_sampler, in.uv).xyz;
    let occlusion_sample = textureSample(occlusion_texture, occlusion_sampler, in.uv).r;
    let emissive_sample = textureSample(emissive_texture, emissive_sampler, in.uv).rgb;
    let dp_dx = dpdx(in.world);
    let dp_dy = dpdy(in.world);
    let duv_dx = dpdx(in.uv);
    let duv_dy = dpdy(in.uv);

    let color = material.base_color * base_sample;

    // The surface's own direction, turned around on the back of a
    // double-sided surface, then bent by the normal texture. With no
    // tangents in the file, the texture's directions come from how the
    // texture coordinates change across the screen (as the glTF sample
    // viewer does).
    let side = select(-1.0, 1.0, front);
    let ng = normalize(in.normal) * side;
    let det = duv_dx.x * duv_dy.y - duv_dy.x * duv_dx.y;
    let t_raw = (duv_dy.y * dp_dx - duv_dx.y * dp_dy) / select(det, 1.0, abs(det) < 1e-12);
    let t_flat = t_raw - ng * dot(ng, t_raw);
    let has_tangent = dot(t_flat, t_flat) > 1e-12;
    let t = select(vec3<f32>(1.0, 0.0, 0.0), normalize(t_flat), has_tangent);
    let b = cross(ng, t);
    let bent = (normal_sample * 2.0 - 1.0) * vec3<f32>(material.surface.z, material.surface.z, 1.0);
    let mapped = normalize(t * bent.x + b * bent.y + ng * bent.z);
    let n = select(ng, mapped, has_tangent);

    let v = normalize(frame.camera.xyz - in.world);
    let metallic = clamp(material.surface.x * surface_sample.b, 0.0, 1.0);
    let roughness = clamp(material.surface.y * surface_sample.g, 0.03, 1.0);
    let occlusion = mix(1.0, occlusion_sample, material.surface.w);
    let glow = material.emissive.rgb * emissive_sample;

    let bands = max(frame.shading.y, 1.0);
    let band_edge = 0.5 * bands * fwidth(dot(n, normalize(SUN_DIRECTION))) + 0.01;
    // Cut-out parts are dropped only now, after the slopes above.
    if material.alpha.y == 1.0 && color.a < material.alpha.x {
        discard;
    }

    var lit: vec3<f32>;
    if frame.shading.x == 1.0 {
        lit = stylized(color.rgb, roughness, occlusion, n, v, bands, band_edge);
    } else if frame.shading.x == 2.0 {
        lit = color.rgb;
    } else {
        lit = standard(color.rgb, metallic, roughness, occlusion, n, v);
    }
    let alpha = select(1.0, color.a, material.alpha.y == 2.0);
    return vec4<f32>(encode(min(lit + glow, vec3<f32>(1.0))), alpha);
}

// ---------------------------------------------------------- outlines ---

// The stylized look's outlines: each solid model drawn again, its back
// faces pushed outwards by a few pixels and filled near-black, so a dark
// edge shows around it (the "inverted hull" outline many cel-shaded games
// use).
@vertex
fn outline_vs(in: MeshIn) -> @builtin(position) vec4<f32> {
    let clip = frame.view_proj * vec4<f32>(in.position, 1.0);
    let normal_clip = (frame.view_proj * vec4<f32>(in.normal, 0.0)).xy;
    let len = length(normal_clip);
    let direction = select(vec2<f32>(0.0), normal_clip / len, len > 1e-6);
    let offset = direction * frame.shading.z * 2.0 / max(frame.screen.xy, vec2<f32>(1.0)) * clip.w;
    return vec4<f32>(clip.xy + offset, clip.zw);
}

@fragment
fn outline_fs() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.004, 1.0);
}
