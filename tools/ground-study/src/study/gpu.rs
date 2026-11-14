//! Through the engine's own renderer: chunk meshes uploaded (the last
//! step of a stroke), and the report's pictures of each look.

use super::scale::mesh_all;
use super::stats::{Spread, Table, ms, timed};
use glam::{Mat4, Vec3, Vec4};
use ground_study::ground::{Ground, Samples};
use ground_study::mesh::{ChunkMesh, Method};
use ground_study::shapes::hill_with_cave;
use ground_study::store::Full;
use ground_study::voxel::{Dense, Voxel};
use render::{
    Environment, FrameParams, GridPlane, Placement, Shading, Sky, Sun, Tonemap, ViewportRenderer,
    best_sample_count, read_pixels,
};
use std::path::Path;

/// The palette's colours (linear), in the shapes' material order: grass,
/// rock, sand, dirt.
const PALETTE: [[f32; 4]; 4] = [
    [0.105, 0.235, 0.055, 1.0],
    [0.24, 0.23, 0.215, 1.0],
    [0.52, 0.42, 0.24, 1.0],
    [0.17, 0.085, 0.04, 1.0],
];

struct Gpu {
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn gpu() -> Option<Gpu> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle().with_env());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .ok()?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
    Some(Gpu {
        adapter,
        device,
        queue,
    })
}

/// `meshes` as one model, a part per material so each has its colour.
fn model(meshes: &[&ChunkMesh]) -> assets::Model {
    let mut parts: Vec<assets::Mesh> = (0..PALETTE.len())
        .map(|m| assets::Mesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            material: Some(m),
            lods: Vec::new(),
        })
        .collect();
    for mesh in meshes {
        for t in mesh.indices.chunks_exact(3) {
            let m = usize::from(mesh.materials[t[0] as usize]).min(PALETTE.len() - 1);
            let part = &mut parts[m];
            for i in t {
                let i = *i as usize;
                part.indices.push(part.vertices.len() as u32);
                part.vertices.push(assets::Vertex {
                    position: mesh.positions[i].to_array(),
                    normal: mesh.normals[i].to_array(),
                    uv: [0.0, 0.0],
                });
            }
        }
    }
    let points = meshes.iter().flat_map(|m| m.positions.iter().copied());
    let bounds = assets::Bounds::around(points).unwrap_or(assets::Bounds {
        min: Vec3::ZERO,
        max: Vec3::ONE,
    });
    assets::Model {
        name: "Ground".into(),
        nodes: vec![assets::Node {
            name: "Ground".into(),
            parent: None,
            local: Mat4::IDENTITY,
            world: Mat4::IDENTITY,
            meshes: (0..parts.len()).collect(),
        }],
        meshes: parts,
        materials: PALETTE
            .iter()
            .map(|c| assets::Material {
                base_color: *c,
                roughness: 0.9,
                ..assets::Material::default()
            })
            .collect(),
        textures: Vec::new(),
        lamps: Vec::new(),
        bounds,
        warnings: Vec::new(),
    }
}

/// One chunk as the editor would hand it over: its own points, shared.
fn chunk_model(mesh: &ChunkMesh) -> assets::Model {
    let vertices = mesh
        .positions
        .iter()
        .zip(&mesh.normals)
        .map(|(p, n)| assets::Vertex {
            position: p.to_array(),
            normal: n.to_array(),
            uv: [0.0, 0.0],
        })
        .collect();
    let mut model = model(&[]);
    model.meshes = vec![assets::Mesh {
        vertices,
        indices: mesh.indices.clone(),
        material: None,
        lods: Vec::new(),
    }];
    model.nodes[0].meshes = vec![0];
    model
}

/// Uploading chunk meshes, each way.
pub(crate) fn uploads(full: &Ground<Full>) {
    let Some(gpu) = gpu() else {
        println!("\nNo graphics driver: uploads not measured.");
        return;
    };
    let info = gpu.adapter.get_info();
    println!("\nUploads through {} ({:?}).", info.name, info.backend);
    let renderer = ViewportRenderer::new(&gpu.device, 1);
    let table = Table::start(
        "Uploading a chunk's mesh through the engine's renderer (300 surface chunks)",
        &[
            "Way",
            "Upload, mean (ms)",
            "Worst (ms)",
            "Bytes a chunk, mean",
        ],
    );
    for method in Method::ALL {
        let (meshes, _) = mesh_all(full, method);
        let mut times = Vec::new();
        let mut bytes = 0;
        let mut kept = Vec::new();
        for (_, mesh) in meshes.iter().step_by((meshes.len() / 300).max(1)).take(300) {
            let (time, uploaded) = timed(|| {
                let model = chunk_model(mesh);
                renderer.upload_model(&gpu.device, &gpu.queue, &model)
            });
            times.push(time);
            bytes += mesh.upload_bytes();
            kept.push(uploaded);
        }
        let spread = Spread::of(&times);
        table.row(&[
            method.name().into(),
            ms(spread.mean),
            ms(spread.worst),
            format!(
                "{:.0} KB",
                bytes as f64 / times.len().max(1) as f64 / 1024.0
            ),
        ]);
    }
}

/// The report's pictures: the hill with its cave and overhang, each way,
/// written as PPM files into `folder` (the run script makes them PNGs).
pub(crate) fn pictures(folder: &Path) -> Result<(), String> {
    let gpu = gpu().ok_or("no graphics driver")?;
    let ground: Ground<Full> = Ground::build(glam::UVec3::new(3, 3, 2), |key, dense| {
        ground_study::shapes::fill(key, dense, hill_with_cave)
    });
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let size = [480, 360];
    let mut target = renderer.create_target(&gpu.device, size);
    let eye = Vec3::new(44.0, -6.0, 34.0);
    let view = glam::camera::rh::view::look_at_mat4(eye, Vec3::new(57.0, 50.0, 15.0), Vec3::Z);
    let view_projection = perspective(size[0] as f32 / size[1] as f32) * view;
    let (mut scratch, mut samples) = (Dense::filled(Voxel::AIR), Samples::default());
    for method in Method::ALL {
        let meshes: Vec<ChunkMesh> = ground
            .keys()
            .map(|key| {
                ground.gather(key, &mut scratch, &mut samples);
                method.mesh(&samples)
            })
            .collect();
        let model = model(&meshes.iter().collect::<Vec<_>>());
        let uploaded = renderer.upload_model(&gpu.device, &gpu.queue, &model);
        renderer.render(
            &gpu.device,
            &gpu.queue,
            &mut target,
            &FrameParams {
                view_projection,
                camera_position: eye,
                focus: Vec3::new(57.0, 50.0, 15.0),
                sky_top: [0.09, 0.09, 0.11],
                sky_horizon: [0.11, 0.12, 0.14],
                ground: [0.12, 0.13, 0.15],
                shading: Shading::default(),
                size_px: size,
                sun: Sun {
                    direction: Vec3::new(0.45, 0.6, -0.65).normalize(),
                    ..Sun::default()
                },
                environment: Environment {
                    sky: Sky::Daylight { strength: 0.8 },
                    tonemap: Tonemap::Agx,
                    ..Environment::default()
                },
                editor_lines: false,
                grid_plane: GridPlane::Ground,
                grid_step: 1.0,
            },
            &[Placement {
                model: &uploaded,
                node: None,
                transform: Mat4::IDENTITY,
                pick: 1,
            }],
            &[],
        );
        let pixels = read_pixels(&gpu.device, &gpu.queue, &target)?;
        let name = method
            .name()
            .to_lowercase()
            .replace([' ', ','], "-")
            .replace("--", "-");
        write_ppm(&folder.join(format!("{name}.ppm")), size, &pixels)?;
        println!(
            "{name}: {} triangles",
            meshes.iter().map(ChunkMesh::triangles).sum::<usize>()
        );
    }
    Ok(())
}

/// A 50° tall perspective with depth reversed, as the editor's camera.
fn perspective(aspect: f32) -> Mat4 {
    let (near, far) = (0.1, 1000.0);
    let scale = 1.0 / 25.0_f32.to_radians().tan();
    Mat4::from_cols(
        Vec4::new(scale / aspect, 0.0, 0.0, 0.0),
        Vec4::new(0.0, scale, 0.0, 0.0),
        Vec4::new(0.0, 0.0, near / (far - near), -1.0),
        Vec4::new(0.0, 0.0, far * near / (far - near), 0.0),
    )
}

fn write_ppm(path: &Path, [w, h]: [u32; 2], rgba: &[u8]) -> Result<(), String> {
    let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
    for pixel in rgba.chunks_exact(4) {
        out.extend_from_slice(&pixel[..3]);
    }
    std::fs::write(path, out).map_err(|e| format!("{}: {e}", path.display()))
}
