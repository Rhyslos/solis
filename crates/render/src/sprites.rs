//! Sprites (Phase 9, D2): frames of sprite sheets drawn as flat quads
//! lying on the ground, many at once.
//!
//! Each sheet's atlas is uploaded once ([`GpuSheet`]). Each frame, every
//! sprite is one [`SpriteInstance`] (88 bytes: where its frame's corner
//! is, a pixel's step across and down in the world, its place in the
//! atlas, flip, tint, flash, outline and where it stands), all in one
//! buffer; each sheet's
//! sprites are drawn in one call. Pixel art is cut out: every pixel is
//! drawn or not, so sprites write depth and need no sorting: the shader
//! draws one standing lower on screen a hair nearer, from its feet. Only
//! see-through
//! sprites are blended, drawn last, back to front. The colours are drawn
//! exactly: never lit or tone mapped (lamps and day and night on sprites
//! come in D9).

use bytemuck::{Pod, Zeroable};

/// One sprite to draw.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct SpriteInstance {
    /// Where its frame's top-left corner is in the world, in metres.
    pub corner: [f32; 3],
    /// [`SpriteInstance::FLIP_X`], [`SpriteInstance::FLIP_Y`],
    /// [`SpriteInstance::OUTLINE`], [`SpriteInstance::SEE_THROUGH`].
    pub flags: u32,
    /// One pixel to the right, in the world (x, y).
    pub axis_x: [f32; 2],
    /// One pixel down the picture, in the world (x, y).
    pub axis_y: [f32; 2],
    /// Its frame's place in the atlas: x, y, width and height, in texels.
    pub rect: [f32; 4],
    /// Linear tint (rgb), and how white it flashes (a, 0 to 1).
    pub tint: [f32; 4],
    /// The outline's linear colour (rgb; a unused).
    pub outline: [f32; 4],
    /// Where it stands (its pivot) in the world (x, y): the lower on
    /// screen, the nearer it's drawn, so whoever stands in front is in
    /// front with no sorting.
    pub foot: [f32; 2],
}

impl SpriteInstance {
    /// Mirrored left to right.
    pub const FLIP_X: u32 = 1;
    /// Mirrored top to bottom.
    pub const FLIP_Y: u32 = 2;
    /// A one-pixel outline.
    pub const OUTLINE: u32 = 4;
    /// Blended, sorted back to front.
    pub const SEE_THROUGH: u32 = 8;

    /// Whether it's drawn blended.
    pub fn see_through(&self) -> bool {
        self.flags & Self::SEE_THROUGH != 0
    }
}

const ATTRIBUTES: [wgpu::VertexAttribute; 8] = wgpu::vertex_attr_array![
    0 => Float32x3, 1 => Uint32, 2 => Float32x2, 3 => Float32x2,
    4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x2,
];

/// A sheet's atlas on the graphics card.
#[derive(Debug)]
pub struct GpuSheet {
    size: [u32; 2],
    bind_group: wgpu::BindGroup,
}

impl GpuSheet {
    /// The atlas's size in texels.
    pub fn size(&self) -> [u32; 2] {
        self.size
    }

    pub(crate) fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

/// The sprite pipelines and the sheet layout.
#[derive(Debug)]
pub(crate) struct SpritePipelines {
    pub(crate) sheet_layout: wgpu::BindGroupLayout,
    pub(crate) cut_out: wgpu::RenderPipeline,
    pub(crate) blended: wgpu::RenderPipeline,
}

impl SpritePipelines {
    pub(crate) fn new(
        device: &wgpu::Device,
        frame_layout: &wgpu::BindGroupLayout,
        shader: &wgpu::ShaderModule,
        sample_count: u32,
    ) -> Self {
        let sheet_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sprite sheet layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite layout"),
            bind_group_layouts: &[Some(frame_layout), None, None, Some(&sheet_layout)],
            immediate_size: 0,
        });
        let make = |blended: bool| {
            let buffers = [Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<SpriteInstance>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &ATTRIBUTES,
            })];
            let blend = blended.then_some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                // The picture's alpha (what the last pass tone maps) is
                // left alone.
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Zero,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(if blended {
                    "see-through sprite pipeline"
                } else {
                    "sprite pipeline"
                }),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: Some("sprite_vs"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &buffers,
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: crate::renderer::DEPTH_FORMAT,
                    depth_write_enabled: Some(!blended),
                    // Depth is reversed: nearer is greater.
                    depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: sample_count,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: Some("sprite_fs"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: crate::renderer::HDR_FORMAT,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        Self {
            cut_out: make(false),
            blended: make(true),
            sheet_layout,
        }
    }

    /// Uploads a sheet's atlas (`size` texels, straight RGBA rows, as
    /// `assets::sprite_sheet` packs it).
    pub(crate) fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: [u32; 2],
        rgba: &[u8],
    ) -> GpuSheet {
        let max = device.limits().max_texture_dimension_2d.max(1);
        let fits = size[0] >= 1
            && size[1] >= 1
            && size[0] <= max
            && size[1] <= max
            && rgba.len() == size[0] as usize * size[1] as usize * 4;
        // A sheet the card can't hold is drawn see-through (a 1×1 clear
        // atlas) rather than stopping the program.
        let (size, rgba) = if fits {
            (size, rgba)
        } else {
            log::warn!("a sprite sheet of {size:?} is too big for the graphics card; not drawn");
            ([1, 1], &[0u8; 4][..])
        };
        let extent = wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sprite sheet"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size[0] * 4),
                rows_per_image: Some(size[1]),
            },
            extent,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite sheet bind group"),
            layout: &self.sheet_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
        });
        GpuSheet { size, bind_group }
    }
}

/// Sprites of one sheet to draw together.
#[derive(Debug, Clone, Copy)]
pub struct SpriteRun<'a> {
    /// The sheet.
    pub sheet: &'a GpuSheet,
    /// Its sprites. Cut-out ones are drawn in any order; see-through ones
    /// in the order given (back to front), after everything else.
    pub instances: &'a [SpriteInstance],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instances_match_the_shader_layout() {
        assert_eq!(std::mem::size_of::<SpriteInstance>(), 88);
        let last = ATTRIBUTES.last().unwrap();
        assert_eq!(
            last.offset + 8,
            std::mem::size_of::<SpriteInstance>() as u64
        );
    }
}
