//! The open-worlds report's pictures, drawn by the engine's renderer: a
//! 20 km view in each look with far ground coarser, the same with each
//! level tinted, close-ups of a seam between levels each way, and the
//! editor's map of the world's regions (drawn here on the processor).

use super::super::stats::{ms, timed};
use super::Looks;
use super::view::{Arrange, Way, camera, draw};
use glam::{Mat4, Vec2, Vec3, Vec4};
use ground_study::far::Heights;
use ground_study::lod::layout::Layout;
use ground_study::lod::nets::Seams;
use ground_study::mesh::ChunkMesh;
use ground_study::voxel::SIZE_I;
use ground_study::world;
use render::{
    Environment, Fog, FrameParams, GridPlane, Placement, Shading, Sky, Sun, Tonemap,
    ViewportRenderer, best_sample_count, read_pixels,
};
use std::collections::HashMap;
use std::path::Path;

/// The ground kinds' colours (linear): grass, rock, sand, dirt, snow.
const PALETTE: [[f32; 4]; 5] = [
    [0.105, 0.235, 0.055, 1.0],
    [0.24, 0.23, 0.215, 1.0],
    [0.52, 0.42, 0.24, 1.0],
    [0.17, 0.085, 0.04, 1.0],
    [0.8, 0.82, 0.85, 1.0],
];

/// A colour per level, for the tinted picture.
const LEVELS: [[f32; 4]; 8] = [
    [0.75, 0.2, 0.15, 1.0],
    [0.85, 0.55, 0.1, 1.0],
    [0.75, 0.75, 0.15, 1.0],
    [0.25, 0.6, 0.2, 1.0],
    [0.15, 0.5, 0.65, 1.0],
    [0.25, 0.3, 0.75, 1.0],
    [0.5, 0.25, 0.65, 1.0],
    [0.6, 0.6, 0.6, 1.0],
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

/// `meshes` as one model, a part per colour; `colour` picks each
/// triangle's (from its mesh and its first point's kind).
fn model_of(
    meshes: &[(usize, &ChunkMesh)],
    colours: &[[f32; 4]],
    colour: impl Fn(usize, u8) -> usize,
) -> assets::Model {
    let mut parts: Vec<assets::Mesh> = (0..colours.len())
        .map(|m| assets::Mesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            material: Some(m),
            lods: Vec::new(),
        })
        .collect();
    let mut index: HashMap<(usize, usize, u32), u32> = HashMap::new();
    for (n, (tag, mesh)) in meshes.iter().enumerate() {
        for t in mesh.indices.chunks_exact(3) {
            let c = colour(*tag, mesh.materials[t[0] as usize]).min(colours.len() - 1);
            for i in t {
                let at = *index.entry((c, n, *i)).or_insert_with(|| {
                    let part = &mut parts[c];
                    part.vertices.push(assets::Vertex {
                        position: mesh.positions[*i as usize].to_array(),
                        normal: mesh.normals[*i as usize].to_array(),
                        uv: [0.0, 0.0],
                    });
                    part.vertices.len() as u32 - 1
                });
                parts[c].indices.push(at);
            }
        }
    }
    let points = meshes.iter().flat_map(|(_, m)| m.positions.iter().copied());
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
        materials: colours
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

/// A 50° tall perspective with depth reversed, as the editor's camera,
/// out to 40 km.
fn perspective(aspect: f32) -> Mat4 {
    let (near, far) = (0.5, 40_000.0);
    let scale = 1.0 / 25.0_f32.to_radians().tan();
    Mat4::from_cols(
        Vec4::new(scale / aspect, 0.0, 0.0, 0.0),
        Vec4::new(0.0, scale, 0.0, 0.0),
        Vec4::new(0.0, 0.0, near / (far - near), -1.0),
        Vec4::new(0.0, 0.0, far * near / (far - near), 0.0),
    )
}

/// A shot: where from, where to, and the sky behind.
struct Shot {
    eye: Vec3,
    at: Vec3,
    /// A bright backdrop instead of the sky, so cracks show.
    loud_sky: bool,
    fog: Option<f32>,
}

fn shoot(gpu: &Gpu, model: &assets::Model, shot: &Shot, path: &Path) -> Result<f64, String> {
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let size = [960, 540];
    let mut target = renderer.create_target(&gpu.device, size);
    let view = glam::camera::rh::view::look_at_mat4(shot.eye, shot.at, Vec3::Z);
    let uploaded = renderer.upload_model(&gpu.device, &gpu.queue, model);
    let (sky, environment) = if shot.loud_sky {
        (
            [0.9, 0.1, 0.8],
            Environment {
                sky: Sky::Backdrop,
                tonemap: Tonemap::Agx,
                ..Environment::default()
            },
        )
    } else {
        (
            [0.3, 0.4, 0.55],
            Environment {
                sky: Sky::Daylight { strength: 0.8 },
                fog: shot.fog.map(|half| Fog {
                    half_distance: half,
                    color: None,
                }),
                tonemap: Tonemap::Agx,
                ..Environment::default()
            },
        )
    };
    let (time, _) = timed(|| {
        renderer.render(
            &gpu.device,
            &gpu.queue,
            &mut target,
            &FrameParams {
                view_projection: perspective(size[0] as f32 / size[1] as f32) * view,
                camera_position: shot.eye,
                focus: shot.at,
                sky_top: sky,
                sky_horizon: sky,
                ground: sky,
                shading: Shading::default(),
                size_px: size,
                sun: Sun {
                    direction: Vec3::new(0.55, 0.45, -0.45).normalize(),
                    ..Sun::default()
                },
                environment,
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
    });
    let pixels = read_pixels(&gpu.device, &gpu.queue, &target)?;
    write_ppm(path, size, &pixels)?;
    Ok(time)
}

fn write_ppm(path: &Path, [w, h]: [u32; 2], rgba: &[u8]) -> Result<(), String> {
    let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
    for pixel in rgba.chunks_exact(4) {
        out.extend_from_slice(&pixel[..3]);
    }
    std::fs::write(path, out).map_err(|e| format!("{}: {e}", path.display()))
}

/// The recommended way: rings of 8, voxels to 2 km with transition
/// bands, the far field beyond.
const RECOMMENDED: Way = Way {
    name: "",
    layout: Arrange::Rings(8),
    seams: Seams::Transition,
    far_from: Some(4),
};

/// Every picture, as PPM files in `folder`.
pub(crate) fn all(folder: &Path) -> Result<(), String> {
    let gpu = gpu().ok_or("no graphics driver")?;
    let info = gpu.adapter.get_info();
    println!("Drawing through {} ({:?}).", info.name, info.backend);
    let camera = camera();
    let eye = camera + Vec3::new(0.0, -300.0, 140.0);
    let at = eye + Vec3::new(0.0, 1000.0, -95.0);
    let view = Shot {
        eye,
        at,
        loud_sky: false,
        fog: Some(6000.0),
    };
    for look in Looks::ALL {
        let drawn = draw(RECOMMENDED, look, camera);
        let tagged: Vec<(usize, &ChunkMesh)> = drawn.meshes.iter().map(|m| (0, m)).collect();
        let model = model_of(&tagged, &PALETTE, |_, kind| usize::from(kind));
        let triangles: usize = drawn.meshes.iter().map(ChunkMesh::triangles).sum();
        let name = format!("open-worlds-{}.ppm", look.name().to_lowercase());
        let time = shoot(&gpu, &model, &view, &folder.join(&name))?;
        println!("{name}: {triangles} triangles, drawn in {} ms", ms(time));
        if look == Looks::Smooth {
            let levelled: Vec<(usize, &ChunkMesh)> = drawn
                .meshes
                .iter()
                .map(|m| {
                    let level = m
                        .positions
                        .first()
                        .map_or(0, |p| drawn.layout.level_at(p.truncate()));
                    (level as usize, m)
                })
                .collect();
            let tinted = model_of(&levelled, &LEVELS, |level, _| level);
            shoot(&gpu, &tinted, &view, &folder.join("open-worlds-levels.ppm"))?;
        }
    }
    seams(&gpu, folder, camera)?;
    map(folder, camera.truncate())
}

/// Close-ups of the border between levels 0 and 1, low and grazing,
/// with a loud backdrop so any crack shows.
fn seams(gpu: &Gpu, folder: &Path, camera: Vec3) -> Result<(), String> {
    let layout = RECOMMENDED.layout(camera);
    // The level-0 square's east edge, on the ground there.
    let edge_x = (0..world::SIDE / SIZE_I)
        .rev()
        .find(|x| {
            layout.level_of(glam::IVec2::new(*x, (camera.y / SIZE_I as f32) as i32)) == Some(0)
        })
        .map_or(camera.x, |x| ((x + 1) * SIZE_I) as f32);
    let spot = Vec2::new(edge_x, camera.y + 40.0);
    let ground = world::height(spot);
    let shot = Shot {
        eye: spot.extend(ground + 9.0) + Vec3::new(-45.0, -70.0, 0.0),
        at: spot.extend(ground - 2.0),
        loud_sky: true,
        fog: None,
    };
    let ways = [
        (Looks::Smooth, Seams::None, "smooth-nothing"),
        (Looks::Smooth, Seams::Skirts, "smooth-skirts"),
        (Looks::Smooth, Seams::Transition, "smooth-transition"),
        (Looks::Blocky, Seams::None, "blocky-nothing"),
        (Looks::Blocky, Seams::Transition, "blocky-walls"),
    ];
    for (look, seams, name) in ways {
        let way = Way {
            seams,
            ..RECOMMENDED
        };
        let drawn = draw(way, look, camera);
        let near: Vec<(usize, &ChunkMesh)> = drawn
            .meshes
            .iter()
            .filter(|m| {
                m.positions
                    .first()
                    .is_some_and(|p| p.truncate().distance(spot) < 2500.0)
            })
            .map(|m| (0, m))
            .collect();
        let model = model_of(&near, &PALETTE, |_, kind| usize::from(kind));
        shoot(
            gpu,
            &model,
            &shot,
            &folder.join(format!("open-worlds-seam-{name}.ppm")),
        )?;
        println!("open-worlds-seam-{name}.ppm");
    }
    Ok(())
}

/// The editor's map of a 20 km world: its heights shaded, its regions
/// (256 m) as a faint grid, every 2 km a stronger line, and the rings
/// drawn round the camera.
fn map(folder: &Path, camera: Vec2) -> Result<(), String> {
    const PX: usize = 640;
    let spacing = world::SIDE as f32 / PX as f32;
    let heights = Heights::sampled(world::SIDE as f32, spacing, |p| (world::height(p), 0));
    let layout = Layout::rings(glam::IVec2::splat(world::SIDE / SIZE_I), camera, 7, 8);
    let mut out = format!("P6\n{PX} {PX}\n255\n").into_bytes();
    let light = Vec3::new(-0.5, 0.5, 0.7).normalize();
    for row in 0..PX {
        let y = PX - 1 - row;
        for x in 0..PX {
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * spacing;
            let h = heights.at(p);
            let dx = heights.at(p + Vec2::X * spacing) - heights.at(p - Vec2::X * spacing);
            let dy = heights.at(p + Vec2::Y * spacing) - heights.at(p - Vec2::Y * spacing);
            let normal = Vec3::new(-dx, -dy, 2.0 * spacing).normalize();
            let shade = 0.35 + 0.65 * normal.dot(light).max(0.0);
            let base = if h > 96.0 {
                Vec3::new(0.85, 0.87, 0.9)
            } else if h < 18.0 {
                Vec3::new(0.76, 0.68, 0.48)
            } else {
                Vec3::new(0.32, 0.5, 0.22).lerp(
                    Vec3::new(0.5, 0.48, 0.44),
                    ((h - 18.0) / 78.0).clamp(0.0, 1.0),
                )
            };
            let mut c = base * shade;
            let on = |step: f32| (p.x % step) < spacing || (p.y % step) < spacing;
            if on(2048.0) {
                c *= 0.45;
            } else if on(256.0) {
                c *= 0.85;
            }
            let here = layout.level_at(p);
            let there = layout.level_at(p + Vec2::splat(spacing));
            if here != there || layout.level_at(p + Vec2::new(spacing, 0.0)) != here {
                c = Vec3::ONE;
            }
            out.extend(
                (c.clamp(Vec3::ZERO, Vec3::ONE) * 255.0)
                    .to_array()
                    .map(|v| v as u8),
            );
        }
    }
    let path = folder.join("open-worlds-map.ppm");
    std::fs::write(&path, out).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("open-worlds-map.ppm");
    Ok(())
}
