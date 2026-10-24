// Viewport shaders: the backdrop, models, the ground grid and the ground
// axes.
//
// Everything is drawn in linear HDR colour (brighter than white allowed)
// into a floating-point image. A last pass (`post_fs`) applies exposure and
// tone mapping and encodes to sRGB (see `encode`). The image's alpha
// channel marks which pixels are tone mapped: 1 for the lit world, 0 for
// the editor's own backdrop and the lines drawn over it, which are shown
// exactly as the theme has them. Z is up and units are metres; the grid
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
    // Opacity of each grid level (fine, major, super); w = the plane the
    // grid lies in: 0 the ground (X-Y), 1 upright facing the front (X-Z),
    // 2 upright facing the side (Y-Z).
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
    // The sun's view of the models, for its shadow map.
    sun_view_proj: mat4x4<f32>,
    // Towards the sun (xyz); w = 1 when its shadows are drawn.
    sun_direction: vec4<f32>,
    // The sun's linear colour times its strength (rgb).
    sun_color: vec4<f32>,
    // x: the width of one shadow-map texel in metres; y: the shadow map's
    // size in texels; z: how many lamps are in `lights`.
    shadow: vec4<f32>,
    // The soft sky light arriving from above and from below (rgb, linear).
    light_above: vec4<f32>,
    light_below: vec4<f32>,
    // Fog: colour (rgb, linear); w = how quickly it thickens (ln 2 over the
    // distance that hides half), 0 for none.
    fog: vec4<f32>,
    // x: exposure multiplier; y: tone mapping (0 AgX, 1 ACES, 2 none);
    // z: 1 for the daylight sky, 0 for the editor's backdrop.
    post: vec4<f32>,
};

// One lamp, as LightUniform in lighting.rs.
struct Light {
    // Position (xyz); w = range in metres, 0 for no limit.
    position: vec4<f32>,
    // Linear colour times strength in watts (rgb); w = 0 point, 1 spot.
    color: vec4<f32>,
    // The way a spotlight shines (xyz).
    direction: vec4<f32>,
    // Cosines of the spot's inner and outer angles (xy).
    cone: vec4<f32>,
};

struct Lights {
    lights: array<Light, 16>,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var<uniform> lamps: Lights;
@group(0) @binding(2) var shadow_map: texture_depth_2d;
@group(0) @binding(3) var shadow_sampler: sampler_comparison;

// ---------------------------------------------------------- backdrop ---

struct SkyOut {
    @builtin(position) clip: vec4<f32>,
    // Position on screen, -1 to 1 both ways.
    @location(0) ndc: vec2<f32>,
};

// Linear colour to sRGB, done in the last pass because the finished image
// is plain RGBA (see COLOR_FORMAT in renderer.rs).
fn encode(c: vec3<f32>) -> vec3<f32> {
    let low = c * 12.92;
    let high = 1.055 * pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, low, c <= vec3<f32>(0.0031308));
}

// sRGB to linear colour.
fn decode(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((max(c, vec3<f32>(0.0)) + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

// The opacity that, blending in linear colour over the backdrop's ground,
// looks the same as `alpha` blended in sRGB, as the editor's lines were
// tuned (and as egui draws). Linear blending makes faint light lines on a
// dark background much brighter, so they'd look heavier than the mockup.
fn overlay_alpha(color: vec3<f32>, alpha: f32) -> f32 {
    let under = frame.ground.rgb;
    let wanted = decode(encode(under) + alpha * (encode(color) - encode(under)));
    let reach = color - under;
    let per_channel = (wanted - under) / select(reach, vec3<f32>(1.0), abs(reach) < vec3<f32>(1e-5));
    return clamp(dot(per_channel, vec3<f32>(1.0 / 3.0)), 0.0, 1.0);
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
    var color = mix(ground, sky, above);
    let daylight = frame.post.z > 0.5;
    if daylight {
        // The daylight sky: a glow around the sun, brightest near the
        // horizon, and the sun's disc (about twice its real size, so it's
        // easy to find).
        let towards_sun = normalize(frame.sun_direction.xyz);
        let facing = max(dot(direction, towards_sun), 0.0);
        let sun = frame.sun_color.rgb;
        let glow = sun * (0.02 * pow(facing, 8.0) + 0.1 * pow(facing, 64.0));
        let haze = 1.0 - smoothstep(0.0, 0.4, abs(elevation));
        let disc = smoothstep(0.99985, 0.99990, facing) * above;
        color = color + glow * (0.4 + haze) * above + sun * 8.0 * disc;
    }
    return vec4<f32>(color, select(0.0, 1.0, daylight));
}

// ------------------------------------------------------------------ fog ---

// How much of something `distance` metres from the camera shows through
// the fog (1 all of it, 0 none).
fn fog_visibility(world: vec3<f32>) -> f32 {
    return exp(-frame.fog.w * distance(world, frame.camera.xyz));
}

// The mask for the last pass: things far into the fog over the editor's
// backdrop fade to the backdrop exactly as the theme shows it.
fn world_mask(visibility: f32) -> f32 {
    return select(visibility, 1.0, frame.post.z > 0.5);
}

// -------------------------------------------------------------- grid ---

struct GridOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec2<f32>,
};

// The point of the grid's plane at the plane's own coordinates `ab`.
fn grid_point(ab: vec2<f32>) -> vec3<f32> {
    let plane = frame.grid_strength.w;
    if plane < 0.5 {
        return vec3<f32>(ab, 0.0);
    } else if plane < 1.5 {
        return vec3<f32>(ab.x, 0.0, ab.y);
    }
    return vec3<f32>(0.0, ab.x, ab.y);
}

// A world point in the grid plane's own coordinates.
fn grid_coords(p: vec3<f32>) -> vec2<f32> {
    let plane = frame.grid_strength.w;
    if plane < 0.5 {
        return p.xy;
    } else if plane < 1.5 {
        return p.xz;
    }
    return p.yz;
}

// One large square in the grid's plane (the ground, or upright for a flat
// view from the front or side), drawn as two triangles. The grid lines
// themselves are worked out per pixel in the fragment shader.
@vertex
fn grid_vs(@builtin(vertex_index) index: u32) -> GridOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0),
    );
    let ab = corners[index] * frame.grid.w;
    var out: GridOut;
    out.clip = frame.view_proj * vec4<f32>(grid_point(ab), 1.0);
    out.world = ab;
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
    let distance = length(in.world - grid_coords(frame.focus.xyz));
    let distance_fade = 1.0 - smoothstep(reach * 0.25, reach, distance);

    let strength = frame.grid_strength;
    let line = max(max(fine * strength.x, major * strength.y), super_ * strength.z) * distance_fade;

    // The sun's shadows fall on the ground too, as a soft darkening, so
    // models sit on the grid instead of floating over it.
    var shade = 0.0;
    if frame.grid_strength.w < 0.5 && frame.sun_direction.w > 0.5 && frame.sun_direction.z > 0.0 {
        let sun = frame.sun_color.rgb;
        let sun_strength = clamp(dot(sun, vec3<f32>(0.2126, 0.7152, 0.0722)) / 2.0, 0.0, 1.0);
        shade = (1.0 - sun_shadow(vec3<f32>(in.world, 0.0), vec3<f32>(0.0, 0.0, 1.0)))
            * 0.45 * sun_strength;
    }
    let alpha = line + shade * (1.0 - line);
    if alpha <= 0.002 {
        discard;
    }
    // A cool grey-blue (sRGB #9aa4b4), as in the mockup, over the shadow.
    let grid_color = vec3<f32>(0.323, 0.371, 0.456);
    let color = grid_color * (line / alpha);
    // Fog hides the grid with everything else.
    return vec4<f32>(color, overlay_alpha(color, alpha) * fog_visibility(grid_point(in.world)));
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
    let distance = length(grid_coords(in.world) - grid_coords(frame.focus.xyz));
    let fade = 1.0 - smoothstep(reach * 0.35, reach, distance);
    return vec4<f32>(in.color.rgb, overlay_alpha(in.color.rgb, in.color.a * fade) * fog_visibility(in.world));
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

// Where one copy of a mesh is drawn (an instance; InstanceRaw in
// instances.rs): the first three rows of its placement in the world.
struct Instance {
    @location(3) row_0: vec4<f32>,
    @location(4) row_1: vec4<f32>,
    @location(5) row_2: vec4<f32>,
};

fn instance_world(instance: Instance, position: vec3<f32>) -> vec3<f32> {
    let p = vec4<f32>(position, 1.0);
    return vec3<f32>(dot(instance.row_0, p), dot(instance.row_1, p), dot(instance.row_2, p));
}

// Turns a normal with the placement: by the inverse transpose of its turn
// and stretch, so stretched shapes still light right. That's the matrix of
// cofactors (crossed columns) divided by the determinant; only the
// determinant's sign matters, since the normal is made length 1 after. A
// placement squashed flat keeps the normal as it was.
fn instance_normal(instance: Instance, normal: vec3<f32>) -> vec3<f32> {
    let c0 = vec3<f32>(instance.row_0.x, instance.row_1.x, instance.row_2.x);
    let c1 = vec3<f32>(instance.row_0.y, instance.row_1.y, instance.row_2.y);
    let c2 = vec3<f32>(instance.row_0.z, instance.row_1.z, instance.row_2.z);
    let a = cross(c1, c2);
    let det = dot(c0, a);
    let cofactors = mat3x3<f32>(a, cross(c2, c0), cross(c0, c1));
    let n = cofactors * normal * select(1.0, -1.0, det < 0.0);
    let len = length(n);
    let fallback = select(vec3<f32>(0.0, 0.0, 1.0), normalize(normal), length(normal) > 1e-12);
    return select(fallback, n / len, abs(det) > 1e-12 && len > 1e-12);
}

struct MeshOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world: vec3<f32>,
};

@vertex
fn mesh_vs(in: MeshIn, instance: Instance) -> MeshOut {
    let world = instance_world(instance, in.position);
    var out: MeshOut;
    out.clip = frame.view_proj * vec4<f32>(world, 1.0);
    out.normal = instance_normal(instance, in.normal);
    out.uv = in.uv;
    out.world = world;
    return out;
}

const PI: f32 = 3.14159265;

// The soft light arriving along direction `d` from the sky above and the
// ground below: the "sky light" of decision 0008, the first source of
// indirect light. It follows the sky in use.
fn sky_light(d: vec3<f32>) -> vec3<f32> {
    return mix(frame.light_below.rgb, frame.light_above.rgb, clamp(d.z * 0.5 + 0.5, 0.0, 1.0));
}

// How much of the sun reaches `world` (0 in shadow, 1 in sunlight), from
// the shadow map, softened over nine nearby texels. Looked up a little way
// out along the surface's normal, so surfaces don't shadow themselves.
fn sun_shadow(world: vec3<f32>, n: vec3<f32>) -> f32 {
    if frame.sun_direction.w < 0.5 {
        return 1.0;
    }
    let lifted = world + n * frame.shadow.x * 1.5;
    let p = frame.sun_view_proj * vec4<f32>(lifted, 1.0);
    let ndc = p.xyz / p.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || ndc.z > 1.0 {
        return 1.0;
    }
    let texel = 1.0 / max(frame.shadow.y, 1.0);
    var sum = 0.0;
    for (var x = -1; x <= 1; x++) {
        for (var y = -1; y <= 1; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel;
            sum += textureSampleCompareLevel(shadow_map, shadow_sampler, uv + offset, ndc.z - 0.0005);
        }
    }
    return sum / 9.0;
}

// The direction towards lamp `i` from `world`, and the light arriving
// from it: its strength falls off with the square of the distance, ends
// smoothly at its range, and for a spotlight fades towards the cone's edge.
// Watts become light as in Blender (P / 4π² d²), so a lamp of the same
// strength lights a scene about as brightly in both.
struct LampLight {
    l: vec3<f32>,
    radiance: vec3<f32>,
};

fn lamp_light(i: u32, world: vec3<f32>) -> LampLight {
    let lamp = lamps.lights[i];
    let to_lamp = lamp.position.xyz - world;
    let d2 = max(dot(to_lamp, to_lamp), 0.0001);
    let l = to_lamp * inverseSqrt(d2);
    var falloff = 1.0 / (4.0 * PI * PI * d2);
    let range = lamp.position.w;
    if range > 0.0 {
        let r = d2 / (range * range);
        let window = clamp(1.0 - r * r, 0.0, 1.0);
        falloff *= window * window;
    }
    if lamp.color.w > 0.5 {
        let cos_angle = dot(-l, normalize(lamp.direction.xyz));
        falloff *= smoothstep(lamp.cone.y, max(lamp.cone.x, lamp.cone.y + 0.0001), cos_angle);
    }
    return LampLight(l, lamp.color.rgb * falloff);
}

// Light reflected towards the viewer from one light arriving from `l` with
// `radiance`: glTF's metal-and-roughness material with GGX highlights, the
// same maths as most engines.
fn brdf(base: vec3<f32>, metallic: f32, roughness: f32, n: vec3<f32>, v: vec3<f32>,
        l: vec3<f32>, radiance: vec3<f32>) -> vec3<f32> {
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
    return (diffuse + specular) * radiance * n_dot_l;
}

// The standard look: the sun (with its shadow), every lamp, and the sky
// light.
fn standard(base: vec3<f32>, metallic: f32, roughness: f32, occlusion: f32,
            n: vec3<f32>, v: vec3<f32>, world: vec3<f32>, shadow: f32) -> vec3<f32> {
    var direct = brdf(base, metallic, roughness, n, v, normalize(frame.sun_direction.xyz),
        frame.sun_color.rgb) * shadow;
    let count = u32(frame.shadow.z);
    for (var i = 0u; i < count; i++) {
        let lamp = lamp_light(i, world);
        direct += brdf(base, metallic, roughness, n, v, lamp.l, lamp.radiance);
    }

    // Indirect light from the sky: diffuse from around the normal, and a
    // reflection of the sky, weighted by a well-known fit (Karis) of how
    // much a surface of this roughness reflects.
    let n_dot_v = max(dot(n, v), 0.0001);
    let f0 = mix(vec3<f32>(0.04), base, metallic);
    let c0 = vec4<f32>(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4<f32>(1.0, 0.0425, 1.04, -0.04);
    let r = roughness * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * n_dot_v)) * r.x + r.y;
    let ab = vec2<f32>(-1.04, 1.04) * a004 + r.zw;
    let reflected = sky_light(reflect(-v, n)) * (f0 * ab.x + ab.y);
    let indirect = (base * (1.0 - metallic) * sky_light(n) + reflected) * occlusion;
    return direct + indirect;
}

// Light in `bands` flat steps, `edge` wide where one step meets the next.
fn banded(x: f32, bands: f32, edge: f32) -> f32 {
    let y = clamp(x, 0.0, 1.0) * bands;
    return (floor(y) + smoothstep(0.5 - edge, 0.5 + edge, fract(y))) / bands;
}

// The stylized look: the sun's light in a few flat bands with crisp
// edges (shadows crisp too), lamps banded the same way, a sharp highlight
// on glossy surfaces, and a rim of sky light around the edges.
// `band_edge` is how wide one pixel is in the banded light, so band edges
// stay smooth instead of jagged.
fn stylized(base: vec3<f32>, roughness: f32, occlusion: f32, n: vec3<f32>,
            v: vec3<f32>, world: vec3<f32>, shadow: f32, bands: f32,
            band_edge: f32) -> vec3<f32> {
    let l = normalize(frame.sun_direction.xyz);
    let crisp_shadow = smoothstep(0.35, 0.65, shadow);
    let n_dot_l = max(dot(n, l), 0.0) * crisp_shadow;
    var light = frame.sun_color.rgb / PI * banded(n_dot_l, bands, band_edge);
    let count = u32(frame.shadow.z);
    for (var i = 0u; i < count; i++) {
        let lamp = lamp_light(i, world);
        let strength = max(lamp.radiance.r, max(lamp.radiance.g, lamp.radiance.b));
        let colour = lamp.radiance / max(strength, 0.0001);
        light += colour * banded(max(dot(n, lamp.l), 0.0) * strength / PI, bands, 0.02);
    }
    let lit = base * (sky_light(n) * 1.3 * occlusion + light * 0.95);
    let h = normalize(v + l);
    let gloss = 1.0 - roughness;
    let highlight = smoothstep(0.5 - band_edge, 0.5 + band_edge,
        pow(max(dot(n, h), 0.0), 8.0 + 120.0 * gloss * gloss)) * gloss * 0.6;
    let rim = smoothstep(0.55, 0.65, 1.0 - max(dot(n, v), 0.0)) * 0.35;
    let sun_tint = frame.sun_color.rgb / max(frame.sun_color.r + frame.sun_color.g + frame.sun_color.b, 0.0001) * 3.0;
    return lit + sun_tint * highlight * step(0.001, n_dot_l) + frame.light_above.rgb * rim;
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
    let band_edge = 0.5 * bands * fwidth(dot(n, normalize(frame.sun_direction.xyz))) + 0.01;
    // Cut-out parts are dropped only now, after the slopes above.
    if material.alpha.y == 1.0 && color.a < material.alpha.x {
        discard;
    }

    var lit: vec3<f32>;
    if frame.shading.x == 2.0 {
        lit = color.rgb;
    } else {
        let shadow = sun_shadow(in.world, ng);
        if frame.shading.x == 1.0 {
            lit = stylized(color.rgb, roughness, occlusion, n, v, in.world, shadow, bands, band_edge);
        } else {
            lit = standard(color.rgb, metallic, roughness, occlusion, n, v, in.world, shadow);
        }
    }
    // Fog, then the mask for the last pass: the unlit look's colours are
    // shown as painted, never tone mapped. (Blended surfaces use their
    // alpha for blending instead.)
    let visibility = fog_visibility(in.world);
    let fogged = mix(frame.fog.rgb, lit + glow, visibility);
    let mask = select(world_mask(visibility), 0.0, frame.shading.x == 2.0);
    let alpha = select(mask, color.a, material.alpha.y == 2.0);
    return vec4<f32>(fogged, alpha);
}

// ---------------------------------------------------------- outlines ---

// The stylized look's outlines: each solid model drawn again, its back
// faces pushed outwards by a few pixels and filled near-black, so a dark
// edge shows around it (the "inverted hull" outline many cel-shaded games
// use).
struct OutlineOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
};

@vertex
fn outline_vs(in: MeshIn, instance: Instance) -> OutlineOut {
    let world = instance_world(instance, in.position);
    let clip = frame.view_proj * vec4<f32>(world, 1.0);
    let normal_clip = (frame.view_proj * vec4<f32>(instance_normal(instance, in.normal), 0.0)).xy;
    let len = length(normal_clip);
    let direction = select(vec2<f32>(0.0), normal_clip / len, len > 1e-6);
    let offset = direction * frame.shading.z * 2.0 / max(frame.screen.xy, vec2<f32>(1.0)) * clip.w;
    var out: OutlineOut;
    out.clip = vec4<f32>(clip.xy + offset, clip.zw);
    out.world = world;
    return out;
}

@fragment
fn outline_fs(in: OutlineOut) -> @location(0) vec4<f32> {
    let visibility = fog_visibility(in.world);
    let color = mix(frame.fog.rgb, vec3<f32>(0.0, 0.0, 0.0003), visibility);
    return vec4<f32>(color, world_mask(visibility));
}

// ----------------------------------------------------------- shadows ---

// The shadow map: each model drawn from the sun's side, depth only.
@vertex
fn shadow_vs(in: MeshIn, instance: Instance) -> @builtin(position) vec4<f32> {
    return frame.sun_view_proj * vec4<f32>(instance_world(instance, in.position), 1.0);
}

// ------------------------------------------------------- last pass ---

// The finished HDR image (group 2; group 1 is the models' materials).
@group(2) @binding(0) var hdr_image: texture_2d<f32>;

struct PostOut {
    @builtin(position) clip: vec4<f32>,
};

@vertex
fn post_vs(@builtin(vertex_index) index: u32) -> PostOut {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0),
    );
    var out: PostOut;
    out.clip = vec4<f32>(corners[index], 0.0, 1.0);
    return out;
}

// AgX, Blender's default view since 4.0, using the well-known fit of its
// curve by Benjamin Wrensch: colours are squeezed into a log range, put
// through an S-curve, and turned back to linear. Bright colours drift
// towards white, as on film.
fn agx(c: vec3<f32>) -> vec3<f32> {
    let inset = mat3x3<f32>(
        vec3<f32>(0.842479062253094, 0.0423282422610123, 0.0423756549057051),
        vec3<f32>(0.0784335999999992, 0.878468636469772, 0.0784336),
        vec3<f32>(0.0792237451477643, 0.0791661274605434, 0.879142973793104),
    );
    let outset = mat3x3<f32>(
        vec3<f32>(1.19687900512017, -0.0528968517574562, -0.0529716355144438),
        vec3<f32>(-0.0980208811401368, 1.15190312990417, -0.0980434501171241),
        vec3<f32>(-0.0990297440797205, -0.0989611768448433, 1.15107367264116),
    );
    let min_ev = -12.47393;
    let max_ev = 4.026069;
    var v = inset * max(c, vec3<f32>(1e-10));
    v = clamp(log2(v), vec3<f32>(min_ev), vec3<f32>(max_ev));
    v = (v - min_ev) / (max_ev - min_ev);
    let v2 = v * v;
    let v4 = v2 * v2;
    v = 15.5 * v4 * v2 - 40.14 * v4 * v + 31.96 * v4 - 6.868 * v2 * v + 0.4298 * v2 + 0.1191 * v - 0.00232;
    v = outset * v;
    return pow(max(v, vec3<f32>(0.0)), vec3<f32>(2.2));
}

// The ACES film curve as most games fit it (Krzysztof Narkowicz's fit).
fn aces(c: vec3<f32>) -> vec3<f32> {
    let x = c * 0.6;
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), vec3<f32>(0.0), vec3<f32>(1.0));
}

// Exposure and tone mapping for the lit world, then sRGB for the screen.
// Pixels marked 0 (the editor's backdrop and its lines) are left as they
// are; partly covered ones blend between the two.
@fragment
fn post_fs(in: PostOut) -> @location(0) vec4<f32> {
    let hdr = textureLoad(hdr_image, vec2<i32>(in.clip.xy), 0);
    let exposed = max(hdr.rgb, vec3<f32>(0.0)) * frame.post.x;
    var mapped: vec3<f32>;
    if frame.post.y == 0.0 {
        mapped = agx(exposed);
    } else if frame.post.y == 1.0 {
        mapped = aces(exposed);
    } else {
        mapped = exposed;
    }
    let shown = mix(hdr.rgb, mapped, clamp(hdr.a, 0.0, 1.0));
    return vec4<f32>(encode(clamp(shown, vec3<f32>(0.0), vec3<f32>(1.0))), 1.0);
}

// ------------------------------------------------------------ projectiles ---

// One projectile (Bolt in projectiles.rs): a glowing streak from its head
// back along its trail, turned to face the camera around its own line.
struct BoltIn {
    @location(0) head: vec3<f32>,
    @location(1) trail: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) width: f32,
};

struct BoltOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec3<f32>,
    // -1 at one edge, 1 at the other.
    @location(1) across: f32,
};

@vertex
fn bolt_vs(@builtin(vertex_index) corner: u32, bolt: BoltIn) -> BoltOut {
    // Two triangles: how far back along the trail (0 head, 1 tail), and
    // which side.
    var along = array<f32, 6>(0.0, 0.0, 1.0, 1.0, 0.0, 1.0);
    var side = array<f32, 6>(-1.0, 1.0, -1.0, -1.0, 1.0, 1.0);
    let a = along[corner % 6u];
    let s = side[corner % 6u];
    let point = bolt.head + bolt.trail * a;
    let toward = frame.camera.xyz - point;
    // Sideways: across the trail as the camera sees it; for a projectile
    // flying straight at the camera, any way across.
    var across = cross(bolt.trail, toward);
    if dot(across, across) < 1e-12 {
        across = cross(toward, vec3<f32>(0.0, 0.0, 1.0));
        if dot(across, across) < 1e-12 {
            across = vec3<f32>(1.0, 0.0, 0.0);
        }
    }
    let world = point + normalize(across) * s * bolt.width * 0.5;
    var out: BoltOut;
    out.clip = frame.view_proj * vec4<f32>(world, 1.0);
    // Brightest at the head, fading towards the tail.
    out.color = bolt.color * (1.0 - 0.7 * a);
    out.across = s;
    return out;
}

@fragment
fn bolt_fs(in: BoltOut) -> @location(0) vec4<f32> {
    // Soft edges; added to what's behind, so crossing streaks glow.
    let soft = 1.0 - in.across * in.across;
    return vec4<f32>(in.color * soft, 1.0);
}
