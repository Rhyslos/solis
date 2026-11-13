//! The GPU images one viewport draws into, made by
//! [`ViewportRenderer::create_target`].

use super::frame_resources::FrameResources;
use super::{COLOR_FORMAT, DEPTH_FORMAT, HDR_FORMAT, ViewportRenderer};

/// GPU images one viewport draws into. Create with
/// [`ViewportRenderer::create_target`].
#[derive(Debug)]
pub struct ViewportTarget {
    pub(super) size: [u32; 2],
    /// Multisampled HDR image (only when multisampling).
    pub(super) msaa_view: Option<wgpu::TextureView>,
    /// The single-sample HDR image the last pass reads.
    pub(super) hdr_view: wgpu::TextureView,
    /// The last pass's view of `hdr_view`.
    pub(super) post_bind_group: wgpu::BindGroup,
    /// The finished, tone-mapped sRGB image.
    pub(super) resolved: wgpu::Texture,
    pub(super) resolved_view: wgpu::TextureView,
    pub(super) depth_view: wgpu::TextureView,
    /// The picking images, made the first time something is picked.
    pub(super) pick: Option<crate::pick::PickImages>,
    /// This viewport's own frame data, lamps, shadow map and lamp
    /// markers, so several viewports never overwrite each other's.
    pub(super) frame: FrameResources,
    /// Times the GPU's passes, once the first frame has asked (`None`
    /// inside: the card can't).
    pub(super) timer: Option<Option<crate::timing::GpuTimer>>,
    /// A layer's picture (decision 0094): no sky, and clear where nothing
    /// is drawn, so the pixel stage lays it over what's behind.
    pub(super) layered: bool,
}

impl ViewportTarget {
    /// How long the GPU took over this viewport's newest frame whose times
    /// have come back, a frame or two after it was drawn; `None` before
    /// any, or on a card without timestamps. Never waits.
    pub fn gpu_times(&self, device: &wgpu::Device) -> Option<crate::timing::GpuTimes> {
        self.timer.as_ref()?.as_ref()?.poll(device)
    }

    /// Makes this a layer's picture (decision 0094), or an ordinary one:
    /// a layer's has no sky and is clear (alpha 0) where nothing is
    /// drawn, its colours as drawn, for [`ViewportRenderer::draw_pixel_layers`].
    pub fn set_layered(&mut self, layered: bool) {
        self.layered = layered;
    }

    /// Whether this card can time its passes (known after the first
    /// frame).
    pub fn can_time(&self) -> Option<bool> {
        self.timer.as_ref().map(Option::is_some)
    }

    /// Size in pixels.
    pub fn size(&self) -> [u32; 2] {
        self.size
    }

    /// The finished colour image, ready for showing in egui.
    pub fn display_view(&self) -> &wgpu::TextureView {
        &self.resolved_view
    }

    /// The finished colour image as a texture (for copying).
    pub fn color_texture(&self) -> &wgpu::Texture {
        &self.resolved
    }

    /// The answer to [`ViewportRenderer::request_pick`], once the GPU has
    /// it (usually the frame after). Never waits.
    pub fn poll_pick(&mut self, device: &wgpu::Device) -> Option<crate::PickResult> {
        self.pick.as_mut()?.poll(device)
    }

    /// Whether a pick asked for is still waiting for its answer.
    pub fn is_picking(&self) -> bool {
        self.pick.as_ref().is_some_and(|p| p.is_waiting())
    }
}

impl ViewportRenderer {
    /// Makes the images for one viewport of `size` pixels. Sizes are
    /// clamped to what the GPU allows, and never below 1×1.
    pub fn create_target(&self, device: &wgpu::Device, size: [u32; 2]) -> ViewportTarget {
        let max = device.limits().max_texture_dimension_2d.max(1);
        let size = [size[0].clamp(1, max), size[1].clamp(1, max)];
        let extent = wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        };
        let texture = |label, format, usage, samples| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: extent,
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let resolved = texture(
            "viewport colour",
            COLOR_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            1,
        );
        let hdr_view = texture(
            "viewport HDR",
            HDR_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            1,
        )
        .create_view(&wgpu::TextureViewDescriptor::default());
        let msaa_view = (self.sample_count > 1).then(|| {
            texture(
                "viewport HDR (multisampled)",
                HDR_FORMAT,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
                self.sample_count,
            )
            .create_view(&wgpu::TextureViewDescriptor::default())
        });
        let post_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("last pass bind group"),
            layout: &self.post_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&hdr_view),
            }],
        });
        let depth_view = texture(
            "viewport depth",
            DEPTH_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
            self.sample_count,
        )
        .create_view(&wgpu::TextureViewDescriptor::default());
        let resolved_view = resolved.create_view(&wgpu::TextureViewDescriptor::default());
        ViewportTarget {
            frame: FrameResources::new(
                device,
                &self.frame_layout,
                &self.shadow_layout,
                &self.shadow_sampler,
                &self.no_shadows,
            ),
            size,
            msaa_view,
            hdr_view,
            post_bind_group,
            resolved,
            resolved_view,
            depth_view,
            pick: None,
            timer: None,
            layered: false,
        }
    }
}
