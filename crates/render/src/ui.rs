//! Game UI's shapes on the GPU ([decision 0083](../../../docs/decisions/0083-game-ui-on-any-screen.md)):
//! each [`ui::Shape`] is one instance of a quad, its edge worked out in
//! the shader from its distance, so boxes, rounded corners, ellipses and
//! lines stay sharp at any size. Drawn over a finished image (the Game
//! panel's, after tone mapping), in the order given, in one draw call.
//! The shapes' buffer, the screen's size and the bind group are made once
//! and filled each draw, so a frame allocates nothing on the card.

/// One shape as the shader reads it (160 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiInstance {
    rect: [f32; 4],
    ends: [f32; 4],
    /// Round (or a line's width), border width, kind, flags.
    shape: [f32; 4],
    colour_a: [f32; 4],
    colour_b: [f32; 4],
    border: [f32; 4],
    /// Where it may be drawn: left, top, right, bottom.
    clip: [f32; 4],
    /// Glow reach, outline width, pixel size, unused (decision 0089).
    halo: [f32; 4],
    glow: [f32; 4],
    outline: [f32; 4],
}

/// The most shapes drawn at once (more are left out).
pub const MAX_SHAPES: usize = 65_536;

/// Each kind of shape's number in the shader (`KIND_…` in `ui.wgsl`).
const KIND_BOX: f32 = 0.0;
const KIND_ELLIPSE: f32 = 1.0;
const KIND_LINE: f32 = 2.0;
const KIND_LETTER: f32 = 3.0;
const KIND_PICTURE: f32 = 4.0;
/// A shape's flags (`FLAG_…` in `ui.wgsl`): on the game's pixels, a
/// gradient running down, a picture shown blocky.
const FLAG_CRISP: u32 = 1;
const FLAG_DOWN: u32 = 2;
const FLAG_BLOCKY: u32 = 4;

impl UiInstance {
    /// `shape` for the shader.
    pub fn of(shape: &ui::Shape) -> Self {
        let kind = match shape.kind {
            ui::ShapeKind::Box => KIND_BOX,
            ui::ShapeKind::Ellipse => KIND_ELLIPSE,
            ui::ShapeKind::Line => KIND_LINE,
            ui::ShapeKind::Letter => KIND_LETTER,
            ui::ShapeKind::Picture => KIND_PICTURE,
        };
        let mut flags = 0u32;
        if shape.crisp {
            flags |= FLAG_CRISP;
        }
        if shape.stretch == ui::Stretch::Down {
            flags |= FLAG_DOWN;
        }
        if shape.blocky {
            flags |= FLAG_BLOCKY;
        }
        Self {
            rect: shape.rect,
            ends: [shape.from[0], shape.from[1], shape.to[0], shape.to[1]],
            shape: [shape.round, shape.border.0, kind, flags as f32],
            colour_a: shape.colours[0].0,
            colour_b: shape.colours[1].0,
            border: shape.border.1.0,
            clip: shape.clip,
            halo: [shape.glow.0, shape.outline.0, shape.pixel, 0.0],
            glow: shape.glow.1.0,
            outline: shape.outline.1.0,
        }
    }
}

/// The pipeline game UI is drawn with, and the letters' and pictures'
/// atlases on the card.
#[derive(Debug)]
pub struct UiPipeline {
    pipeline: wgpu::RenderPipeline,
    atlas: wgpu::Texture,
    /// The atlas revision on the card (`u64::MAX`: none yet).
    uploaded: std::sync::atomic::AtomicU64,
    /// The pictures' atlas (decision 0089), and its revision on the card.
    pictures: wgpu::Texture,
    pictures_uploaded: std::sync::atomic::AtomicU64,
    /// The screen's size, filled each draw.
    screen: wgpu::Buffer,
    /// The screen's size, the atlases and the sampler, bound once.
    bind_group: wgpu::BindGroup,
    /// Every shape of a draw; replaced by a bigger one when a draw has
    /// more.
    shapes: std::sync::Mutex<Option<wgpu::Buffer>>,
}

impl UiPipeline {
    /// Made once, for images of `format`.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("game ui shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ui.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("game ui layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("game ui pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let attributes = wgpu::vertex_attr_array![
            0 => Float32x4, 1 => Float32x4, 2 => Float32x4,
            3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4,
            7 => Float32x4, 8 => Float32x4, 9 => Float32x4
        ];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("game ui pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("ui_vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<UiInstance>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &attributes,
                })],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("ui_fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let side = ui::text::ATLAS_SIDE;
        let atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("game ui letters"),
            size: wgpu::Extent3d {
                width: side,
                height: side,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let atlas_view = atlas.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("game ui letters"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let side = ui::pictures::SIDE;
        let pictures = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("game ui pictures"),
            size: wgpu::Extent3d {
                width: side,
                height: side,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Colours as the screen has them (sRGB numbers), as shapes' are.
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let pictures_view = pictures.create_view(&wgpu::TextureViewDescriptor::default());
        let screen = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("game ui screen"),
            size: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("game ui bind group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: screen.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&pictures_view),
                },
            ],
        });
        Self {
            screen,
            bind_group,
            shapes: std::sync::Mutex::new(None),
            pipeline,
            atlas,
            uploaded: std::sync::atomic::AtomicU64::new(u64::MAX),
            pictures,
            pictures_uploaded: std::sync::atomic::AtomicU64::new(u64::MAX),
        }
    }

    /// Sends the pictures' atlas to the card when it's another or changed.
    fn upload_pictures(&self, queue: &wgpu::Queue, pictures: &ui::Pictures) {
        use std::sync::atomic::Ordering;
        let side = ui::pictures::SIDE;
        if self.pictures_uploaded.load(Ordering::Relaxed) == pictures.revision
            || pictures.pixels.len() != (side * side * 4) as usize
        {
            return;
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.pictures,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pictures.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(side * 4),
                rows_per_image: Some(side),
            },
            wgpu::Extent3d {
                width: side,
                height: side,
                depth_or_array_layers: 1,
            },
        );
        self.pictures_uploaded
            .store(pictures.revision, Ordering::Relaxed);
    }

    /// Sends the letters' atlas to the card when it's another or changed
    /// (every atlas has its own revisions). One of the wrong size isn't
    /// sent: its pixels are public, and the card's copy would refuse it.
    fn upload(&self, queue: &wgpu::Queue, atlas: &ui::text::Atlas) {
        use std::sync::atomic::Ordering;
        let side = ui::text::ATLAS_SIDE;
        if self.uploaded.load(Ordering::Relaxed) == atlas.revision
            || atlas.pixels.len() != (side * side) as usize
        {
            return;
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.atlas,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(side),
                rows_per_image: Some(side),
            },
            wgpu::Extent3d {
                width: side,
                height: side,
                depth_or_array_layers: 1,
            },
        );
        self.uploaded.store(atlas.revision, Ordering::Relaxed);
    }

    /// Draws `shapes` over `view` (an image `size` pixels big, kept as it
    /// is underneath), in order, with letters from `atlas` and pictures
    /// from `pictures`, and submits it. Shapes past [`MAX_SHAPES`] are left
    /// out. Never waits for the GPU.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        size: [u32; 2],
        shapes: &[ui::Shape],
        atlas: Option<&ui::text::Atlas>,
        pictures: Option<&ui::Pictures>,
    ) {
        if shapes.is_empty() || size[0] == 0 || size[1] == 0 {
            return;
        }
        if let Some(atlas) = atlas {
            self.upload(queue, atlas);
        }
        if let Some(pictures) = pictures {
            self.upload_pictures(queue, pictures);
        }
        let instances: Vec<UiInstance> =
            shapes.iter().take(MAX_SHAPES).map(UiInstance::of).collect();
        let bytes: &[u8] = bytemuck::cast_slice(&instances);
        let most = device.limits().max_buffer_size;
        if bytes.len() as u64 > most {
            return;
        }
        // Several screens a frame share the buffer: each draw's writes
        // land before its own submit and after the one before.
        let mut held = self
            .shapes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !held
            .as_ref()
            .is_some_and(|b| b.size() >= bytes.len() as u64)
        {
            // Room to grow, so a slowly growing screen isn't re-made
            // every frame.
            let size = (bytes.len() as u64).next_power_of_two().max(4096).min(most);
            *held = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("game ui shapes"),
                size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        let Some(buffer) = held.as_ref() else {
            return;
        };
        queue.write_buffer(buffer, 0, bytes);
        let screen = [size[0] as f32, size[1] as f32, 0.0, 0.0];
        queue.write_buffer(&self.screen, 0, bytemuck::cast_slice(&screen));
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("game ui"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("game ui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, buffer.slice(..bytes.len() as u64));
            pass.draw(0..4, 0..instances.len() as u32);
        }
        queue.submit([encoder.finish()]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_instance_is_160_bytes_and_says_its_kind_and_flags() {
        assert_eq!(std::mem::size_of::<UiInstance>(), 160);
        let screen = ui::sample::hud(ui::ScaleRule::WholeSteps, 0.5);
        let shapes = ui::layout(&screen, [1920, 1080], Some(4));
        let i = UiInstance::of(&shapes[1]);
        assert_eq!(i.shape[2], 1.0, "the sun is an ellipse");
        assert_eq!(i.shape[3], 1.0, "crisp");
    }

    #[test]
    fn the_shader_s_numbers_are_the_rust_ones() {
        use crate::shader_layout::{assert_struct, constant};
        let source = include_str!("ui.wgsl");
        // The screen's size, as `draw` writes it.
        assert_struct(
            source,
            "Screen",
            std::mem::size_of::<[f32; 4]>(),
            &[("size", 0), ("_pad", 8)],
        );
        let numbers = [
            ("SPREAD", f64::from(ui::text::SPREAD)),
            ("ATLAS_SIDE", f64::from(ui::text::ATLAS_SIDE)),
            ("PICTURES_SIDE", f64::from(ui::pictures::SIDE)),
            ("KIND_ELLIPSE", f64::from(KIND_ELLIPSE)),
            ("KIND_LINE", f64::from(KIND_LINE)),
            ("KIND_LETTER", f64::from(KIND_LETTER)),
            ("KIND_PICTURE", f64::from(KIND_PICTURE)),
            ("FLAG_CRISP", f64::from(FLAG_CRISP)),
            ("FLAG_DOWN", f64::from(FLAG_DOWN)),
            ("FLAG_BLOCKY", f64::from(FLAG_BLOCKY)),
        ];
        for (name, rust) in numbers {
            assert_eq!(constant(source, name), rust, "{name}");
        }
        // A box is any kind the shader doesn't name.
        assert!(
            [KIND_ELLIPSE, KIND_LINE, KIND_LETTER, KIND_PICTURE]
                .iter()
                .all(|&k| k != KIND_BOX)
        );
    }
}
