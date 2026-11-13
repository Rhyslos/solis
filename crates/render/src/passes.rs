//! One frame of a scene ([`ViewportRenderer::render_scene`]): preparing it
//! (what's in view, sent to the GPU and counted), then its three passes:
//! the sun's shadow map, the viewport itself, and the last pass (exposure,
//! tone mapping and sRGB) into the finished image.

use super::frame_resources::{FrameResources, MAX_MARKED_LAMPS};
use crate::frame::{FrameLight, FrameParams, FrameUniforms};
use crate::gpu_model::{GpuModel, Pass};
use crate::instances::{Batch, Placement};
use crate::lighting::{LightsUniform, SHADOW_REACH, light_markers, sun_view_fitted};
use crate::renderer::{Scene, ViewportRenderer, ViewportTarget, overlap, view_box};
use crate::sprites::{SpriteLayer, SpriteRun};
use crate::timing::{FrameStats, TimedPass};
use glam::Vec3;

/// The timestamps a pass writes, when the frame is timed.
type Stamps<'t> = Option<wgpu::RenderPassTimestampWrites<'t>>;

/// What preparing a frame found and sent to the GPU, for its passes.
struct Prepared<'a> {
    /// The models, where they are.
    placed: &'a [Placement<'a>],
    /// The sprites, a run per sheet.
    sprites: &'a [SpriteRun<'a>],
    /// The batches drawn, each one mesh for a run of copies (none when
    /// the copies couldn't be sent).
    batches: Vec<Batch>,
    /// For each batch's model number, the placement holding that model.
    models: Vec<usize>,
    /// Where each sprite run starts in the sprite buffer; none when they
    /// were too many to send.
    sprite_starts: Option<Vec<u32>>,
    /// Projectiles sent.
    bolt_count: u32,
    /// The lamp markers' vertices sent.
    marker_count: u32,
    /// Whether the sun's shadow map is drawn this frame.
    shadows_drawn: bool,
}

impl<'a> Prepared<'a> {
    /// The model `batch` draws.
    fn model_of(&self, batch: &Batch) -> &'a GpuModel {
        self.placed[self.models[batch.model]].model
    }
}

impl ViewportRenderer {
    /// The pipelines for copies wound as made, or mirrored.
    fn facing(&self, mirrored: bool) -> &crate::gpu_model::Facing {
        if mirrored {
            &self.models.mirrored
        } else {
            &self.models.normal
        }
    }

    /// Draws every batch whose mesh belongs in `kind`.
    fn draw_batches<'m>(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        batches: &[crate::instances::Batch],
        model_of: &impl Fn(&crate::instances::Batch) -> &'m GpuModel,
        kind: Pass,
    ) {
        for batch in batches.iter().filter(|b| b.seen) {
            let model = model_of(batch);
            if model.pass_of(batch.mesh) != Some(kind) {
                continue;
            }
            pass.set_pipeline(self.facing(batch.mirrored).for_pass(kind));
            model.draw(pass, batch.mesh, batch.lod, batch.instances.clone());
        }
    }

    /// Draws one frame of `scene` into `target`, and submits it, saying
    /// what it drew. Never waits for the GPU to finish.
    pub fn render_scene(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &mut ViewportTarget,
        params: &FrameParams,
        scene: &Scene<'_>,
    ) -> FrameStats {
        if target.timer.is_none() {
            target.timer = Some(crate::timing::GpuTimer::new(device, queue));
        }
        let (prepared, stats) = self.prepare(device, queue, target, params, scene);
        let target = &*target;
        let timer = target.timer.as_ref().and_then(Option::as_ref);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("viewport frame"),
        });
        if let Some(timer) = timer {
            timer.begin(prepared.shadows_drawn);
        }
        let stamps = |pass| timer.and_then(|t| t.writes(pass));
        self.shadow_pass(
            &mut encoder,
            &target.frame,
            &prepared,
            stamps(TimedPass::Shadows),
        );
        self.main_pass(
            &mut encoder,
            target,
            params,
            &prepared,
            stamps(TimedPass::Models),
        );
        self.last_pass(&mut encoder, target, stamps(TimedPass::LastPass));
        if let Some(timer) = timer {
            timer.finish(&mut encoder);
        }
        queue.submit([encoder.finish()]);
        if let Some(timer) = timer {
            timer.submitted();
        }
        stats
    }

    /// Works out what `scene` draws into `target` this frame and sends it
    /// to the GPU: the copies in view (and those casting into the shadows
    /// in view), projectiles, sprites, lamp markers and the frame's
    /// uniforms. Also counts what the frame draws.
    fn prepare<'a>(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &mut ViewportTarget,
        params: &FrameParams,
        scene: &Scene<'a>,
    ) -> (Prepared<'a>, FrameStats) {
        let Scene {
            placed,
            lights,
            bolts,
            sprites,
        } = *scene;
        let (lamps, lamp_count) = LightsUniform::new(lights, params.camera_position);
        let receivers = shadow_receivers(params, placed);
        let light = params.sun.direction.try_normalize().unwrap_or(-Vec3::Z);
        let cull = crate::instances::Cull::new(
            params.view_projection,
            receivers.map(|(min, max)| (min, max, light)),
        )
        .with_detail(params.view_projection, params.size_px[1]);
        let crate::instances::Gathered {
            raw,
            mut batches,
            models,
            seen,
            casting,
            casters,
            levels,
        } = crate::instances::gather(placed, Some(&cull), &target.frame.levels);
        target.frame.levels = levels;
        let count = raw.len();
        let sent = target.frame.write_instances(device, queue, raw);
        let bolt_count = target.frame.write_bolts(device, queue, bolts);
        let sprite_starts = target.frame.write_sprites(device, queue, sprites);
        if sprite_starts.is_none() && !target.frame.sprites_dropped {
            log::warn!(
                "more sprites than the graphics card can draw at once; sprites aren't drawn"
            );
        }
        target.frame.sprites_dropped = sprite_starts.is_none();
        // Only sprites that are drawn count, and are sent.
        let sprites_drawn: u64 = match sprite_starts {
            Some(_) => sprites.iter().map(|r| r.instances.len() as u64).sum(),
            None => 0,
        };
        if sent.is_none() {
            log::warn!(
                "{count} model copies are more than the graphics card can draw at once; models \
                 aren't drawn"
            );
            batches.clear();
        }
        let shadows_drawn = receivers.is_some() && casters.is_some() && sent.is_some();
        if shadows_drawn {
            target
                .frame
                .shadow_map(device, &self.frame_layout, &self.shadow_sampler);
        }
        let marker_count = send_markers(device, queue, &mut target.frame, params, lights);
        let prepared = Prepared {
            placed,
            sprites,
            batches,
            models,
            sprite_starts,
            bolt_count,
            marker_count,
            shadows_drawn,
        };
        let stats = frame_stats(&prepared, seen, casting, sent, sprites_drawn);
        let mut light = FrameLight {
            lamps: lamp_count,
            shadows_drawn,
            ..FrameLight::default()
        };
        if let Some(receivers) = receivers.filter(|_| shadows_drawn) {
            (light.sun_view_projection, light.texel) =
                sun_view_fitted(&params.sun, receivers, casters);
        }
        let frame = &target.frame;
        queue.write_buffer(
            &frame.uniform_buffer,
            0,
            bytemuck::bytes_of(&FrameUniforms::new(params, &light).layered(target.layered)),
        );
        queue.write_buffer(&frame.lights_buffer, 0, bytemuck::bytes_of(&lamps));
        (prepared, stats)
    }

    /// The sun's shadow map: every batch that casts, drawn for its depth
    /// alone (nothing when no shadows are drawn this frame).
    fn shadow_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &FrameResources,
        prepared: &Prepared<'_>,
        stamps: Stamps<'_>,
    ) {
        let Some(shadow_view) = frame
            .shadow_view
            .as_ref()
            .filter(|_| prepared.shadows_drawn)
        else {
            return;
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sun shadow map"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: shadow_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: stamps,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &frame.shadow_bind_group, &[]);
        pass.set_pipeline(&self.shadow_pipeline);
        if let Some(instances) = &frame.instances {
            pass.set_vertex_buffer(1, instances.slice(..));
            for batch in prepared.batches.iter().filter(|b| b.casts) {
                prepared.model_of(batch).draw_depth(
                    &mut pass,
                    batch.mesh,
                    batch.lod,
                    batch.instances.clone(),
                );
            }
        }
    }

    /// The viewport itself, in HDR: the sky, a tile map's ground, solid
    /// models, outlines, cut-out sprites, the editor's lines, blended
    /// models, projectiles, then see-through sprites and what's drawn over
    /// everything.
    fn main_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &ViewportTarget,
        params: &FrameParams,
        prepared: &Prepared<'_>,
        stamps: Stamps<'_>,
    ) {
        let frame = &target.frame;
        let mut pass = begin_main_pass(encoder, target, stamps);
        pass.set_bind_group(0, &frame.bind_group, &[]);
        // A layer's picture has no sky: what's behind shows through.
        if !target.layered {
            pass.set_pipeline(&self.sky_pipeline);
            pass.draw(0..3, 0..1);
        }
        let layered = target.layered;
        let model_of = |b: &Batch| prepared.model_of(b);
        let batches = &prepared.batches;
        // A tile map's ground first, under everything (Phase 9, D4).
        self.draw_sprites(&mut pass, frame, prepared, SpriteLayer::Under, layered);
        if let Some(instances) = &frame.instances {
            pass.set_vertex_buffer(1, instances.slice(..));
        }
        // Solid shapes first, so the see-through grid and axes are
        // hidden behind them; blended shapes last, over everything.
        self.draw_batches(&mut pass, batches, &model_of, Pass::Solid);
        self.draw_batches(&mut pass, batches, &model_of, Pass::SolidDoubleSided);
        if params.shading.outlines() {
            for batch in batches.iter().filter(|b| b.seen) {
                let facing = self.facing(batch.mirrored);
                pass.set_pipeline(&facing.outline);
                model_of(batch).draw_outline(
                    &mut pass,
                    batch.mesh,
                    batch.lod,
                    batch.instances.clone(),
                );
            }
        }
        // Cut-out sprites write depth like solids, so the grid hides
        // behind them too.
        self.draw_sprites(&mut pass, frame, prepared, SpriteLayer::CutOut, layered);
        if params.editor_lines {
            self.draw_editor_lines(&mut pass, frame, prepared.marker_count);
        }
        if let Some(instances) = &frame.instances {
            pass.set_vertex_buffer(1, instances.slice(..));
        }
        self.draw_batches(&mut pass, batches, &model_of, Pass::Blended);
        if let Some(buffer) = frame.bolts.as_ref().filter(|_| prepared.bolt_count > 0) {
            pass.set_pipeline(if layered {
                &self.bolt_pipeline_layered
            } else {
                &self.bolt_pipeline
            });
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..6, 0..prepared.bolt_count);
        }
        self.draw_sprites(&mut pass, frame, prepared, SpriteLayer::SeeThrough, layered);
        self.draw_sprites(&mut pass, frame, prepared, SpriteLayer::Over, layered);
    }

    /// The sprite runs on `layer`, each drawn whole by one pipeline.
    fn draw_sprites(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &FrameResources,
        prepared: &Prepared<'_>,
        layer: SpriteLayer,
        layered: bool,
    ) {
        let sprites = prepared.sprites;
        let (Some(buffer), Some(starts)) =
            (frame.sprites.as_ref(), prepared.sprite_starts.as_ref())
        else {
            return;
        };
        if !sprites
            .iter()
            .any(|r| r.instances.first().is_some_and(|f| f.layer() == layer))
        {
            return;
        }
        pass.set_pipeline(match (layer, layered) {
            (SpriteLayer::CutOut, _) => &self.sprites.cut_out,
            (SpriteLayer::SeeThrough, false) => &self.sprites.blended,
            (SpriteLayer::SeeThrough, true) => &self.sprites.blended_layered,
            (SpriteLayer::Under | SpriteLayer::Over, false) => &self.sprites.flat,
            (SpriteLayer::Under | SpriteLayer::Over, true) => &self.sprites.flat_layered,
        });
        pass.set_vertex_buffer(0, buffer.slice(..));
        for (run, &start) in sprites.iter().zip(starts) {
            // A run is drawn whole by one pipeline, by its first
            // sprite (the caller keeps runs apart).
            let Some(first) = run.instances.first() else {
                continue;
            };
            if first.layer() != layer {
                continue;
            }
            pass.set_bind_group(3, run.sheet.bind_group(), &[]);
            pass.draw(0..6, start..start + run.instances.len() as u32);
        }
    }

    /// The editor's lines: the grid, the axes and the lamps' markers
    /// (`marker_count` vertices of them).
    fn draw_editor_lines(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &FrameResources,
        marker_count: u32,
    ) {
        pass.set_pipeline(&self.grid_pipeline);
        pass.draw(0..6, 0..1);
        pass.set_pipeline(&self.line_pipeline);
        pass.set_vertex_buffer(0, self.axes.slice(..));
        pass.draw(0..self.axis_count, 0..1);
        if let Some(buffer) = frame.markers.as_ref().filter(|_| marker_count > 0) {
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..marker_count, 0..1);
        }
    }

    /// The last pass: exposure, tone mapping and sRGB, into the finished
    /// image.
    fn last_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &ViewportTarget,
        stamps: Stamps<'_>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("viewport last pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.resolved_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: stamps,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.post_pipeline);
        pass.set_bind_group(0, &target.frame.bind_group, &[]);
        pass.set_bind_group(2, &target.post_bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// Where shadows fall this frame, if the sun casts any: the part of the
/// world in view, up to `SHADOW_REACH` away. What can cast into it is
/// drawn into the sun's shadow map, and only what's in view is drawn.
fn shadow_receivers(params: &FrameParams, placed: &[Placement<'_>]) -> Option<(Vec3, Vec3)> {
    let shadows_wanted = params.sun.shadows && params.shading.look != crate::frame::Look::Unlit;
    shadows_wanted
        .then(|| crate::instances::world_bounds(placed))
        .flatten()
        .and_then(|(min, max)| {
            // Down to the ground, so shadows reach the grid below
            // models.
            let world = (min.min(min.with_z(0.0)), max.max(max.with_z(0.0)));
            overlap(world, view_box(params.view_projection, SHADOW_REACH)?)
        })
}

/// Sends the lamps' markers, which are drawn only among the editor's
/// lines, and says how many vertices they have.
fn send_markers(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    frame: &mut FrameResources,
    params: &FrameParams,
    lights: &[crate::lighting::Light],
) -> u32 {
    let marked = &lights[..lights.len().min(MAX_MARKED_LAMPS)];
    let markers = if params.editor_lines {
        light_markers(marked)
    } else {
        Vec::new()
    };
    if !markers.is_empty() {
        let buffer = frame.markers(device);
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(&markers));
    }
    markers.len() as u32
}

/// What the frame draws, counted for the Profiler: `seen` copies in view,
/// `casting` into the shadow map, `sent` bytes of copies (none when they
/// couldn't be) and `sprites_drawn`.
fn frame_stats(
    prepared: &Prepared<'_>,
    seen: u32,
    casting: u32,
    sent: Option<u64>,
    sprites_drawn: u64,
) -> FrameStats {
    let batches = &prepared.batches;
    FrameStats {
        instances: seen,
        casting,
        batches: batches.len() as u32,
        triangles: batches
            .iter()
            .filter(|b| b.seen)
            .map(|b| {
                u64::from(prepared.model_of(b).triangles_of(b.mesh, b.lod))
                    * u64::from(b.instances.end - b.instances.start)
            })
            .sum(),
        lods: batches
            .iter()
            .filter(|b| b.seen)
            .fold([0; 5], |mut lods, b| {
                lods[b.lod.min(4)] += b.instances.end - b.instances.start;
                lods
            }),
        sent_bytes: sent.unwrap_or(0)
            + u64::from(prepared.bolt_count)
                * std::mem::size_of::<crate::projectiles::Bolt>() as u64
            + sprites_drawn * std::mem::size_of::<crate::sprites::SpriteInstance>() as u64,
        bolts: prepared.bolt_count,
        sprites: u32::try_from(sprites_drawn).unwrap_or(u32::MAX),
    }
}

/// Begins the viewport's pass. With multisampling, drawing goes into the
/// multisampled image, which is smoothed into the single-sample HDR one
/// at the end.
fn begin_main_pass<'e>(
    encoder: &'e mut wgpu::CommandEncoder,
    target: &ViewportTarget,
    stamps: Stamps<'_>,
) -> wgpu::RenderPass<'e> {
    let (view, resolve_target) = match &target.msaa_view {
        Some(msaa) => (msaa, Some(&target.hdr_view)),
        None => (&target.hdr_view, None),
    };
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("viewport"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(if target.layered {
                    wgpu::Color::TRANSPARENT
                } else {
                    wgpu::Color::BLACK
                }),
                store: if resolve_target.is_some() {
                    wgpu::StoreOp::Discard
                } else {
                    wgpu::StoreOp::Store
                },
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &target.depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(0.0),
                store: wgpu::StoreOp::Discard,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: stamps,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}
