//! The GPU data one viewport's frames use: its frame and lamp uniforms,
//! copies, projectiles, sprites, shadow map and lamp markers, each
//! buffer grown when it must be.

use super::LineVertex;
use crate::frame::FrameUniforms;
use crate::lighting::{LightsUniform, MARKER_VERTICES, SHADOW_FORMAT, SHADOW_MAP_SIZE};

/// The most lamps shown with a marker; more are drawn without one.
pub(super) const MAX_MARKED_LAMPS: usize = 1024;

/// The GPU data one viewport's frames use.
#[derive(Debug)]
pub(super) struct FrameResources {
    pub(super) uniform_buffer: wgpu::Buffer,
    pub(super) lights_buffer: wgpu::Buffer,
    /// The sun's shadow map (16 MiB), made the first frame that draws
    /// shadows: a layer's picture, or a view without shadows, never needs
    /// one. Until then `bind_group` binds the renderer's 1 × 1 stand-in.
    pub(super) shadow_view: Option<wgpu::TextureView>,
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) shadow_bind_group: wgpu::BindGroup,
    /// Room for every lamp's marker lines, filled each frame the editor's
    /// lines are drawn; made the first time there are any.
    pub(super) markers: Option<wgpu::Buffer>,
    /// Every mesh copy's placement, filled each frame; replaced by a
    /// bigger one when a frame has more (decision 0021).
    pub(super) instances: Option<wgpu::Buffer>,
    /// What the instance buffer holds now, so a frame sends only what
    /// changed.
    pub(super) sent: Vec<crate::instances::InstanceRaw>,
    /// Each placement's level of detail last frame.
    pub(super) levels: Vec<u8>,
    /// Every projectile, filled each frame; replaced by a bigger one
    /// when a frame has more.
    pub(super) bolts: Option<wgpu::Buffer>,
    /// Every sprite, filled each frame; replaced by a bigger one when a
    /// frame has more.
    pub(super) sprites: Option<wgpu::Buffer>,
    /// Whether last frame's sprites were too many to draw, so the warning
    /// is logged once when they stop fitting, not every frame.
    pub(super) sprites_dropped: bool,
    /// Whether last frame's model copies were too many to draw: their
    /// warning is logged once too.
    pub(super) copies_dropped: bool,
}

/// How many copies are compared and sent together: a run that changed
/// anywhere is sent whole.
const SENT_TOGETHER: usize = 512;

impl FrameResources {
    /// Puts every run's sprites into the sprite buffer, one after another,
    /// growing it if needed. Returns where each run's start in it (none if
    /// too many for one buffer).
    pub(super) fn write_sprites(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        runs: &[crate::sprites::SpriteRun<'_>],
    ) -> Option<Vec<u32>> {
        let size = std::mem::size_of::<crate::sprites::SpriteInstance>() as u64;
        let total: u64 = runs.iter().map(|r| r.instances.len() as u64).sum();
        let bytes = total * size;
        let most = device.limits().max_buffer_size;
        if total == 0 || bytes > most || total > u64::from(u32::MAX) {
            return (total == 0).then(Vec::new);
        }
        if !self.sprites.as_ref().is_some_and(|b| b.size() >= bytes) {
            let size = bytes.next_power_of_two().max(4096).min(most);
            self.sprites = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sprites"),
                size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        let buffer = self.sprites.as_ref()?;
        let mut starts = Vec::with_capacity(runs.len());
        let mut at = 0u64;
        for run in runs {
            starts.push(at as u32);
            if !run.instances.is_empty() {
                queue.write_buffer(buffer, at * size, bytemuck::cast_slice(run.instances));
            }
            at += run.instances.len() as u64;
        }
        Some(starts)
    }

    /// Puts `bolts` into the projectile buffer, growing it if needed.
    /// Returns how many will be drawn (none if too many for one buffer).
    pub(super) fn write_bolts(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bolts: &[crate::projectiles::Bolt],
    ) -> u32 {
        let bytes: &[u8] = bytemuck::cast_slice(bolts);
        let most = device.limits().max_buffer_size;
        if bytes.is_empty() || bytes.len() as u64 > most {
            return 0;
        }
        if !self
            .bolts
            .as_ref()
            .is_some_and(|b| b.size() >= bytes.len() as u64)
        {
            let size = (bytes.len() as u64).next_power_of_two().max(4096).min(most);
            self.bolts = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("projectiles"),
                size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        match &self.bolts {
            Some(buffer) => {
                queue.write_buffer(buffer, 0, bytes);
                bolts.len() as u32
            }
            None => 0,
        }
    }

    /// Puts `raw` into the instance buffer, growing it if needed, sending
    /// only the runs that differ from what it holds. `None`: too many for
    /// one buffer; otherwise the bytes sent.
    pub(super) fn write_instances(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        raw: Vec<crate::instances::InstanceRaw>,
    ) -> Option<u64> {
        use rayon::prelude::*;
        let bytes: &[u8] = bytemuck::cast_slice(&raw);
        if bytes.is_empty() {
            return Some(0);
        }
        // More copies than one buffer can hold would be a GPU error, which
        // stops the program: they aren't drawn instead.
        let most = device.limits().max_buffer_size;
        if bytes.len() as u64 > most {
            self.sent.clear();
            return None;
        }
        let fits = self
            .instances
            .as_ref()
            .is_some_and(|b| b.size() >= bytes.len() as u64);
        if !fits {
            // Room to grow, so a slowly growing scene isn't re-made every
            // frame.
            let size = (bytes.len() as u64).next_power_of_two().max(4096).min(most);
            self.instances = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("instances"),
                size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.sent.clear();
        }
        let Some(buffer) = &self.instances else {
            return Some(0);
        };
        let size = std::mem::size_of::<crate::instances::InstanceRaw>();
        let held: &[u8] = bytemuck::cast_slice(&self.sent);
        let run = SENT_TOGETHER * size;
        let changed: Vec<bool> = bytes
            .par_chunks(run)
            .enumerate()
            .map(|(i, chunk)| held.get(i * run..i * run + chunk.len()) != Some(chunk))
            .collect();
        let mut total = 0;
        let mut i = 0;
        while i < changed.len() {
            if !changed[i] {
                i += 1;
                continue;
            }
            let first = i;
            while i < changed.len() && changed[i] {
                i += 1;
            }
            let span = first * run..(i * run).min(bytes.len());
            queue.write_buffer(buffer, span.start as u64, &bytes[span.clone()]);
            total += span.len() as u64;
        }
        self.sent = raw;
        Some(total)
    }
}

impl FrameResources {
    /// `no_shadows` is bound for the shadow map until one is made
    /// ([`Self::shadow_map`]).
    pub(super) fn new(
        device: &wgpu::Device,
        frame_layout: &wgpu::BindGroupLayout,
        shadow_layout: &wgpu::BindGroupLayout,
        shadow_sampler: &wgpu::Sampler,
        no_shadows: &wgpu::TextureView,
    ) -> Self {
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame uniforms"),
            size: std::mem::size_of::<FrameUniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let lights_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lamps"),
            size: std::mem::size_of::<LightsUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = frame_bind_group(
            device,
            frame_layout,
            &uniform_buffer,
            &lights_buffer,
            no_shadows,
            shadow_sampler,
        );
        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow frame bind group"),
            layout: shadow_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        Self {
            uniform_buffer,
            lights_buffer,
            shadow_view: None,
            bind_group,
            shadow_bind_group,
            markers: None,
            instances: None,
            sent: Vec::new(),
            levels: Vec::new(),
            bolts: None,
            sprites: None,
            sprites_dropped: false,
            copies_dropped: false,
        }
    }

    /// The sun's shadow map, made (and bound for shading) the first time
    /// it's asked for.
    pub(super) fn shadow_map(
        &mut self,
        device: &wgpu::Device,
        frame_layout: &wgpu::BindGroupLayout,
        shadow_sampler: &wgpu::Sampler,
    ) -> &wgpu::TextureView {
        let view = match self.shadow_view.take() {
            Some(view) => view,
            None => {
                // A depth image the models are drawn into from the sun's
                // side, then looked up while shading.
                let view = depth_image(device, "sun shadow map", SHADOW_MAP_SIZE);
                self.bind_group = frame_bind_group(
                    device,
                    frame_layout,
                    &self.uniform_buffer,
                    &self.lights_buffer,
                    &view,
                    shadow_sampler,
                );
                view
            }
        };
        self.shadow_view.insert(view)
    }

    /// Room for every marked lamp's lines, made the first time it's
    /// asked for.
    pub(super) fn markers(&mut self, device: &wgpu::Device) -> &wgpu::Buffer {
        self.markers.get_or_insert_with(|| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("lamp markers"),
                size: (MAX_MARKED_LAMPS * MARKER_VERTICES * std::mem::size_of::<LineVertex>())
                    as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        })
    }
}

/// A square depth image `side` texels across, for the sun's shadows.
pub(super) fn depth_image(device: &wgpu::Device, label: &str, side: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: side,
                height: side,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SHADOW_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

/// A viewport's frame data as the shaders read it: its uniforms, lamps,
/// and the shadow map with its sampler.
pub(super) fn frame_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform_buffer: &wgpu::Buffer,
    lights_buffer: &wgpu::Buffer,
    shadow_view: &wgpu::TextureView,
    shadow_sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("frame bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: lights_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(shadow_view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(shadow_sampler),
            },
        ],
    })
}
