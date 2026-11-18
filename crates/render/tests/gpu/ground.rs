//! The ground (Tools 1b, G1): its chunks uploaded with shared materials
//! and pooled buffers, drawn in the Smooth look, flat and gently rolling,
//! without a crack between chunks.

use super::*;

/// The chunk's mesh as a model of one mesh in the first kind's material.
fn chunk_model(mesh: &ground::ChunkMesh) -> assets::Model {
    let vertices = mesh
        .positions
        .iter()
        .zip(&mesh.normals)
        .map(|(p, n)| assets::Vertex {
            position: *p,
            normal: *n,
            uv: [p[0], p[1]],
        })
        .collect();
    let points = mesh.positions.iter().map(|p| Vec3::from_array(*p));
    assets::Model {
        name: "Chunk".into(),
        meshes: vec![assets::Mesh {
            vertices,
            indices: mesh.indices.clone(),
            material: Some(0),
            lods: Vec::new(),
        }],
        nodes: vec![assets::Node {
            name: "Chunk".into(),
            parent: None,
            local: Mat4::IDENTITY,
            world: Mat4::IDENTITY,
            meshes: vec![0],
        }],
        materials: Vec::new(),
        textures: Vec::new(),
        lamps: Vec::new(),
        bounds: assets::Bounds::around(points).unwrap(),
        warnings: Vec::new(),
    }
}

/// Draws `start` as a ground of 3 × 3 chunks, one high, seen from `eye`
/// looking north, `size` pixels square; also says how many buffers the
/// pool made and reused for drawing it twice.
fn draw(start: ground::Start, eye: Vec3, size: u32) -> Option<(Vec<u8>, (usize, usize))> {
    let gpu = gpu()?;
    let renderer = ViewportRenderer::new(&gpu.device, best_sample_count(&gpu.adapter));
    let mut target = renderer.create_target(&gpu.device, [size, size]);
    let g = start.ground(glam::UVec3::new(3, 3, 1));
    let materials = renderer.upload_ground_materials(&gpu.device, &gpu.queue, &[[92, 138, 64]]);
    let mut pool = render::BufferPool::default();
    let mut samples = ground::Samples::default();
    let models: Vec<(assets::Model, Mat4)> = ground::store::keys(g.extent())
        .filter_map(|key| {
            g.gather(key, &mut samples).known()?;
            let mesh = ground::smooth::mesh(&samples);
            (!mesh.is_empty()).then(|| {
                (
                    chunk_model(&mesh),
                    Mat4::from_translation(mesh.origin.as_vec3()),
                )
            })
        })
        .collect();
    let upload = |pool: &mut render::BufferPool| -> Vec<render::GpuModel> {
        models
            .iter()
            .map(|(m, _)| {
                renderer.upload_ground_chunk((&gpu.device, &gpu.queue), pool, &materials, m)
            })
            .collect()
    };
    // Uploaded, given back, and uploaded again: the second time from the
    // pool.
    for model in upload(&mut pool) {
        pool.recycle(model);
    }
    let uploaded = upload(&mut pool);
    let placed: Vec<Placement> = uploaded
        .iter()
        .zip(&models)
        .map(|(model, (_, at))| Placement {
            model,
            node: None,
            transform: *at,
            pick: 0,
        })
        .collect();
    renderer.render(
        &gpu.device,
        &gpu.queue,
        &mut target,
        &FrameParams {
            view_projection: looking_north(eye),
            camera_position: eye,
            focus: Vec3::new(1.0e7, 1.0e7, 0.0),
            sky_top: srgb(SKY_TOP),
            sky_horizon: srgb(SKY_HORIZON),
            ground: srgb(GROUND),
            shading: Shading::default(),
            size_px: [size, size],
            sun: Sun {
                direction: Vec3::new(0.3, 0.4, -0.8).normalize(),
                shadows: false,
                ..Sun::default()
            },
            environment: unmapped(),
            editor_lines: false,
            grid_plane: GridPlane::Ground,
            grid_step: 1.0,
        },
        &placed,
        &[],
    );
    let pixels = read_pixels(&gpu.device, &gpu.queue, &target).unwrap();
    Some((pixels, pool.counts()))
}

/// Every pixel of the rows from `top` down is the ground's green, lit:
/// none is the backdrop showing through a crack.
fn green_below(pixels: &[u8], size: u32, top: u32) {
    for y in top..size {
        for x in 0..size {
            let [r, g, b] = pixel(pixels, size, x, y);
            assert!(
                g > r && g > b && g > 40,
                "({x}, {y}) is {:?}, not the ground's green",
                [r, g, b]
            );
        }
    }
}

#[test]
fn a_flat_ground_is_drawn_green_without_cracks() {
    // Over the middle chunk's seam with its neighbours, 4 m above the
    // ground, looking north: the lower half is ground.
    let eye = Vec3::new(32.0, 2.0, 24.0);
    let Some((pixels, (made, reused))) = draw(ground::Start::Flat { height: 20.0 }, eye, 96)
    else {
        return;
    };
    green_below(&pixels, 96, 56);
    // Above the horizon: the backdrop.
    assert!(between(pixel(&pixels, 96, 48, 4), SKY_TOP, SKY_HORIZON));
    assert!(reused >= made, "the second upload came from the pool ({made} made, {reused} reused)");
}

#[test]
fn a_gently_rolling_ground_is_drawn_without_cracks() {
    let eye = Vec3::new(48.0, 4.0, 40.0);
    let start = ground::Start::Gentle {
        height: 16.0,
        seed: 3,
    };
    let Some((pixels, _)) = draw(start, eye, 96) else {
        return;
    };
    green_below(&pixels, 96, 80);
}
