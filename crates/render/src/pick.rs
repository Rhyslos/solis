//! Picking: which placement is under the mouse, and how far away.
//!
//! When asked, the placements are drawn once more into a small image of
//! their own: each pixel holds the number of the placement drawn there
//! ([`crate::Placement::pick`], 0 for nothing) and its depth. Only the
//! rectangle asked about is drawn and copied back.
//!
//! Reading from the GPU takes a moment, so it never holds up a frame
//! (CONTRIBUTING rule 10): ask with [`crate::ViewportRenderer::request_pick`]
//! and collect the answer with [`crate::ViewportTarget::poll_pick`] on a
//! later frame (usually the next). One question is in flight per viewport
//! at a time.

use crate::instances::{Cull, Placement, gather};
use glam::{Mat4, Vec3, Vec4};
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

/// The picking image's format: the placement's number, and its depth's
/// bits (a whole-number format every GPU can draw into, unlike a float
/// one).
const PICK_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg32Uint;
/// Bytes per pixel of [`PICK_FORMAT`].
const PICK_BYTES: u32 = 8;
/// The picking pass's own depth image.
const PICK_DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// The picking shader, in WGSL.
pub const PICK_SHADER_SOURCE: &str = include_str!("pick.wgsl");

/// What was under a rectangle of the viewport image.
#[derive(Debug, Clone, PartialEq)]
pub struct PickResult {
    /// The rectangle, in image pixels from the top-left corner: x, y,
    /// width and height.
    pub rect: [u32; 4],
    /// Each pixel's placement number, row by row (0: nothing there).
    pub picks: Vec<u32>,
    /// Each pixel's depth (1 at the near plane, towards 0 far away; 0 for
    /// nothing).
    pub depths: Vec<f32>,
    /// The camera the picture was drawn with.
    pub view_projection: Mat4,
    /// The whole image's size in pixels.
    pub size: [u32; 2],
}

impl PickResult {
    /// The index into `picks` of image pixel (`x`, `y`), if it's inside
    /// the rectangle.
    fn index(&self, x: u32, y: u32) -> Option<usize> {
        let [rx, ry, w, h] = self.rect;
        (x >= rx && y >= ry && x < rx + w && y < ry + h).then(|| ((y - ry) * w + (x - rx)) as usize)
    }

    /// The placement number at image pixel (`x`, `y`), if something is
    /// there.
    pub fn pick_at(&self, x: u32, y: u32) -> Option<u32> {
        let i = self.index(x, y)?;
        self.picks.get(i).copied().filter(|&p| p != 0)
    }

    /// The world point drawn at image pixel (`x`, `y`), if something is
    /// there.
    pub fn point_at(&self, x: u32, y: u32) -> Option<Vec3> {
        let i = self.index(x, y)?;
        if self.picks.get(i).copied().unwrap_or(0) == 0 {
            return None;
        }
        let depth = *self.depths.get(i)?;
        let [w, h] = self.size;
        let ndc = Vec4::new(
            (x as f32 + 0.5) / w.max(1) as f32 * 2.0 - 1.0,
            1.0 - (y as f32 + 0.5) / h.max(1) as f32 * 2.0,
            depth,
            1.0,
        );
        let world = self.view_projection.inverse() * ndc;
        (world.w.abs() > 1e-12)
            .then(|| world.truncate() / world.w)
            .filter(|p| p.is_finite())
    }

    /// Every placement number with at least one pixel in the rectangle.
    pub fn all(&self) -> BTreeSet<u32> {
        self.picks.iter().copied().filter(|&p| p != 0).collect()
    }
}

/// The picking pass's pipeline, shared by every viewport.
#[derive(Debug)]
pub(crate) struct PickPipeline {
    pipeline: wgpu::RenderPipeline,
    /// Sprites, picked by their own pixels (Phase 9, D2).
    sprites: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

impl PickPipeline {
    /// `sheet_layout` is the sprite sheets' (sprites.rs), so a sheet
    /// uploaded once is picked from as it's drawn.
    pub(crate) fn new(device: &wgpu::Device, sheet_layout: &wgpu::BindGroupLayout) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("picking shader"),
            source: wgpu::ShaderSource::Wgsl(PICK_SHADER_SOURCE.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("picking layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("picking pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("picking pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("pick_vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    Some(crate::gpu_model::vertex_layout()),
                    Some(crate::instances::layout()),
                ],
            },
            // Both sides: a mirrored copy or a thin wall is picked from
            // either side, as it's drawn.
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: PICK_DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                // Depth is reversed (1 near, 0 far), as in the viewport.
                depth_compare: Some(wgpu::CompareFunction::Greater),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("pick_fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: PICK_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let sprite_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite picking pipeline layout"),
            bind_group_layouts: &[Some(&layout), Some(sheet_layout)],
            immediate_size: 0,
        });
        let pick_attribute = [wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Uint32,
            offset: 0,
            shader_location: 8,
        }];
        let sprites = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite picking pipeline"),
            layout: Some(&sprite_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("sprite_pick_vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<crate::SpriteInstance>()
                            as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &crate::sprites::ATTRIBUTES,
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 4,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &pick_attribute,
                    }),
                ],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: PICK_DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("sprite_pick_fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: PICK_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            sprites,
            layout,
        }
    }
}

/// One viewport's picking images and the question in flight.
#[derive(Debug)]
pub(crate) struct PickImages {
    size: [u32; 2],
    color: wgpu::Texture,
    color_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    instances: Option<wgpu::Buffer>,
    /// Sprites to pick from, and their numbers.
    sprite_instances: Option<wgpu::Buffer>,
    sprite_picks: Option<wgpu::Buffer>,
    waiting: Option<Waiting>,
}

/// A question sent to the GPU, waiting for its answer.
#[derive(Debug)]
struct Waiting {
    rect: [u32; 4],
    view_projection: Mat4,
    buffer: wgpu::Buffer,
    /// Bytes per row in `buffer` (rows are padded for the copy).
    row_bytes: u32,
    /// 0 waiting, 1 ready, 2 failed.
    state: Arc<AtomicU8>,
}

impl PickImages {
    pub(crate) fn new(device: &wgpu::Device, pipeline: &PickPipeline, size: [u32; 2]) -> Self {
        let extent = wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        };
        let texture = |label, format, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let color = texture(
            "picking image",
            PICK_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        );
        let depth_view = texture(
            "picking depth",
            PICK_DEPTH_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        )
        .create_view(&wgpu::TextureViewDescriptor::default());
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("picking camera"),
            size: std::mem::size_of::<Mat4>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("picking bind group"),
            layout: &pipeline.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        Self {
            size,
            color_view: color.create_view(&wgpu::TextureViewDescriptor::default()),
            color,
            depth_view,
            uniform,
            bind_group,
            instances: None,
            sprite_instances: None,
            sprite_picks: None,
            waiting: None,
        }
    }

    /// Whether a question is in flight.
    pub(crate) fn is_waiting(&self) -> bool {
        self.waiting.is_some()
    }

    /// Draws `placed` and `sprites` inside `rect` and starts copying it
    /// back. The rectangle is clamped to the image; an empty one asks
    /// nothing.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn request(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pipeline: &PickPipeline,
        view_projection: Mat4,
        placed: &[Placement<'_>],
        sprites: &[crate::SpritePicks<'_>],
        rect: [u32; 4],
    ) -> bool {
        if self.waiting.is_some() {
            return false;
        }
        let [w, h] = self.size;
        let x = rect[0].min(w);
        let y = rect[1].min(h);
        let rect = [x, y, rect[2].min(w - x), rect[3].min(h - y)];
        if rect[2] == 0 || rect[3] == 0 {
            return false;
        }
        // Only what's in view can be under the mouse.
        let crate::instances::Gathered {
            raw,
            batches,
            models,
            ..
        } = gather(placed, Some(&Cull::new(view_projection, None)), &[]);
        let bytes: &[u8] = bytemuck::cast_slice(&raw);
        // Too many to pick from at once (see the renderer's own check).
        let most = device.limits().max_buffer_size;
        if bytes.len() as u64 > most {
            return false;
        }
        if !bytes.is_empty() {
            let fits = self
                .instances
                .as_ref()
                .is_some_and(|b| b.size() >= bytes.len() as u64);
            if !fits {
                self.instances = Some(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("picking instances"),
                    size: (bytes.len() as u64).next_power_of_two().max(4096).min(most),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
            }
            if let Some(buffer) = &self.instances {
                queue.write_buffer(buffer, 0, bytes);
            }
        }
        // Sprites: every run's sprites and numbers, one after another.
        let mut sprite_bytes: Vec<u8> = Vec::new();
        let mut pick_bytes: Vec<u8> = Vec::new();
        let mut sprite_runs = Vec::new();
        for s in sprites {
            let n = s.run.instances.len().min(s.picks.len());
            if n == 0 {
                continue;
            }
            let first = (sprite_bytes.len() / std::mem::size_of::<crate::SpriteInstance>()) as u32;
            sprite_bytes.extend_from_slice(bytemuck::cast_slice(&s.run.instances[..n]));
            pick_bytes.extend_from_slice(bytemuck::cast_slice(&s.picks[..n]));
            sprite_runs.push((s.run.sheet, first..first + n as u32));
        }
        if sprite_bytes.len() as u64 > most {
            sprite_runs.clear();
        }
        if !sprite_runs.is_empty() {
            let fill = |slot: &mut Option<wgpu::Buffer>, bytes: &[u8], label| {
                if !slot
                    .as_ref()
                    .is_some_and(|b| b.size() >= bytes.len() as u64)
                {
                    *slot = Some(device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some(label),
                        size: (bytes.len() as u64).next_power_of_two().max(4096).min(most),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    }));
                }
                if let Some(buffer) = slot {
                    queue.write_buffer(buffer, 0, bytes);
                }
            };
            fill(&mut self.sprite_instances, &sprite_bytes, "picking sprites");
            fill(
                &mut self.sprite_picks,
                &pick_bytes,
                "picking sprite numbers",
            );
        }
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&view_projection));
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("picking"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("picking"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_scissor_rect(rect[0], rect[1], rect[2], rect[3]);
            pass.set_pipeline(&pipeline.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            if let Some(instances) = &self.instances {
                pass.set_vertex_buffer(1, instances.slice(..));
                for batch in &batches {
                    let model = placed[models[batch.model]].model;
                    model.draw_shape(&mut pass, batch.mesh, batch.instances.clone());
                }
            }
            if let (Some(instances), Some(picks)) = (&self.sprite_instances, &self.sprite_picks)
                && !sprite_runs.is_empty()
            {
                pass.set_pipeline(&pipeline.sprites);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_vertex_buffer(0, instances.slice(..));
                pass.set_vertex_buffer(1, picks.slice(..));
                for (sheet, range) in sprite_runs {
                    pass.set_bind_group(1, sheet.bind_group(), &[]);
                    pass.draw(0..6, range);
                }
            }
        }
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let row_bytes = (rect[2] * PICK_BYTES).div_ceil(align) * align;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("picking readback"),
            size: u64::from(row_bytes) * u64::from(rect[3]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.color,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: rect[0],
                    y: rect[1],
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row_bytes),
                    rows_per_image: Some(rect[3]),
                },
            },
            wgpu::Extent3d {
                width: rect[2],
                height: rect[3],
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        let state = Arc::new(AtomicU8::new(0));
        let done = Arc::clone(&state);
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                done.store(if result.is_ok() { 1 } else { 2 }, Ordering::Release);
            });
        self.waiting = Some(Waiting {
            rect,
            view_projection,
            buffer,
            row_bytes,
            state,
        });
        true
    }

    /// The answer, if it has arrived; never waits. A failed copy gives an
    /// empty answer (nothing found) rather than an error.
    pub(crate) fn poll(&mut self, device: &wgpu::Device) -> Option<PickResult> {
        let state = self.waiting.as_ref()?.state.load(Ordering::Acquire);
        if state == 0 {
            // Lets wgpu run the map callback if the copy has finished.
            let _ = device.poll(wgpu::PollType::Poll);
        }
        let waiting = self.waiting.as_ref()?;
        let state = waiting.state.load(Ordering::Acquire);
        if state == 0 {
            return None;
        }
        let waiting = self.waiting.take()?;
        let [_, _, w, h] = waiting.rect;
        let count = (w * h) as usize;
        let mut picks = Vec::with_capacity(count);
        let mut depths = Vec::with_capacity(count);
        if state == 1
            && let Ok(mapped) = waiting.buffer.slice(..).get_mapped_range()
        {
            for row in mapped.chunks(waiting.row_bytes as usize).take(h as usize) {
                for pixel in row[..(w * PICK_BYTES) as usize].chunks_exact(8) {
                    let word = |i: usize| {
                        u32::from_le_bytes([pixel[i], pixel[i + 1], pixel[i + 2], pixel[i + 3]])
                    };
                    picks.push(word(0));
                    depths.push(f32::from_bits(word(4)));
                }
            }
        }
        // Only a buffer that was mapped can be unmapped; anything else is a
        // GPU error.
        if state == 1 {
            waiting.buffer.unmap();
        }
        picks.resize(count, 0);
        depths.resize(count, 0.0);
        Some(PickResult {
            rect: waiting.rect,
            picks,
            depths,
            view_projection: waiting.view_projection,
            size: self.size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(picks: Vec<u32>, depths: Vec<f32>, rect: [u32; 4]) -> PickResult {
        PickResult {
            rect,
            picks,
            depths,
            view_projection: Mat4::IDENTITY,
            size: [4, 4],
        }
    }

    #[test]
    fn pixels_are_found_inside_the_rectangle_only() {
        let r = result(vec![0, 7, 3, 0], vec![0.0, 0.5, 0.5, 0.0], [1, 1, 2, 2]);
        assert_eq!(r.pick_at(1, 1), None, "nothing there");
        assert_eq!(r.pick_at(2, 1), Some(7));
        assert_eq!(r.pick_at(1, 2), Some(3));
        assert_eq!(r.pick_at(0, 0), None, "outside the rectangle");
        assert_eq!(r.pick_at(3, 3), None, "outside the rectangle");
        assert_eq!(r.all(), BTreeSet::from([3, 7]));
    }

    #[test]
    fn a_depth_becomes_a_world_point() {
        // With no camera transform, the point is the pixel's NDC position.
        let r = result(vec![1], vec![0.25], [2, 1, 1, 1]);
        let p = r.point_at(2, 1).unwrap();
        assert!(p.abs_diff_eq(Vec3::new(0.25, 0.25, 0.25), 1e-6), "{p}");
        let empty = result(vec![0], vec![0.0], [2, 1, 1, 1]);
        assert_eq!(empty.point_at(2, 1), None);
    }

    #[test]
    fn the_shader_is_valid() {
        use wgpu::naga;
        let module = naga::front::wgsl::parse_str(PICK_SHADER_SOURCE)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(PICK_SHADER_SOURCE)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(PICK_SHADER_SOURCE)));
    }
}
