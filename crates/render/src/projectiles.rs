//! Projectiles drawn in one go: each a glowing streak (a quad turned to
//! face the camera around its own line), all of them instances of one
//! draw call, added to the picture so crossing streaks glow brighter.

use bytemuck::{Pod, Zeroable};

/// One projectile to draw: where its head is, the line back to its tail,
/// its colour (linear RGB; above 1 glows) and how wide it is, in metres.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Bolt {
    /// Its head, in metres.
    pub head: [f32; 3],
    /// From its head back to its tail.
    pub trail: [f32; 3],
    /// Its colour.
    pub colour: [f32; 3],
    /// Its width.
    pub width: f32,
}

const ATTRIBUTES: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
    0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32,
];

/// The pipeline that draws them, into the main pass (`sample_count`
/// samples, the world's HDR image), with the frame's data as group 0.
pub(crate) fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    sample_count: u32,
) -> wgpu::RenderPipeline {
    let buffers = [Some(wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Bolt>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &ATTRIBUTES,
    })];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("projectile pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("bolt_vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &buffers,
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: crate::renderer::DEPTH_FORMAT,
            depth_write_enabled: Some(false),
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
            entry_point: Some("bolt_fs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: crate::renderer::HDR_FORMAT,
                // Added to what's behind; the alpha below (which marks
                // what the last pass tone maps) is left alone.
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::Zero,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bolts_match_the_shader_layout() {
        assert_eq!(std::mem::size_of::<Bolt>(), 10 * 4);
        let last = ATTRIBUTES.last().unwrap();
        assert_eq!(last.offset + 4, std::mem::size_of::<Bolt>() as u64);
    }
}
