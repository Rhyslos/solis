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
