//! Game UI's shapes on the GPU ([decision 0083](../../../docs/decisions/0083-game-ui-on-any-screen.md)):
//! each [`ui::Shape`] is one instance of a quad, its edge worked out in
//! the shader from its distance, so boxes, rounded corners, ellipses and
//! lines stay sharp at any size. Drawn over a finished image (the Game
//! panel's, after tone mapping), in the order given, in one draw call.

use wgpu::util::DeviceExt as _;

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

impl UiInstance {
    /// `shape` for the shader.
    pub fn of(shape: &ui::Shape) -> Self {
        let kind = match shape.kind {
            ui::ShapeKind::Box => 0.0,
            ui::ShapeKind::Ellipse => 1.0,
            ui::ShapeKind::Line => 2.0,
            ui::ShapeKind::Letter => 3.0,
            ui::ShapeKind::Picture => 4.0,
        };
        let mut flags = 0u32;
        if shape.crisp {
            flags |= 1;
        }
        if shape.stretch == ui::Stretch::Down {
            flags |= 2;
        }
        if shape.blocky {
            flags |= 4;
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
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    atlas: wgpu::Texture,
    atlas_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    /// The atlas revision on the card (`u64::MAX`: none yet).
    uploaded: std::sync::atomic::AtomicU64,
    /// The pictures' atlas (decision 0089), and its revision on the card.
    pictures: wgpu::Texture,
    pictures_view: wgpu::TextureView,
    pictures_uploaded: std::sync::atomic::AtomicU64,
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
        Self {
            layout,
            pipeline,
            atlas,
            atlas_view,
            sampler,
            uploaded: std::sync::atomic::AtomicU64::new(u64::MAX),
            pictures,
            pictures_view,
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

    /// Sends the letters' atlas to the card when it changed since last.
    fn upload(&self, queue: &wgpu::Queue, atlas: &ui::text::Atlas) {
        use std::sync::atomic::Ordering;
        if self.uploaded.load(Ordering::Relaxed) == atlas.revision {
            return;
        }
        let side = ui::text::ATLAS_SIDE;
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
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("game ui shapes"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let screen = [size[0] as f32, size[1] as f32, 0.0, 0.0];
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("game ui screen"),
            contents: bytemuck::cast_slice(&screen),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("game ui bind group"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.pictures_view),
                },
            ],
        });
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
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_vertex_buffer(0, buffer.slice(..));
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
}
