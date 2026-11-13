//! Brushes on the GPU (Tools 1a, B1): each different brush model the
//! editor hands over uploaded once, by its key, and let go once no view
//! has drawn it for a while. A brush is uploaded again only when it
//! changes (its key does), never every frame.

use editor::ViewBrush;
use eframe::wgpu;
use render::{GpuModel, Placement};
use std::collections::HashMap;

/// How many frames an upload no view draws is kept, in case it's shown
/// again (a tab brought back to the front).
const KEEP_FRAMES: u64 = 120;

/// The brushes' models on the GPU, by key.
#[derive(Default)]
pub(crate) struct BrushUploads {
    /// Each model, and the frame it was last drawn in.
    uploaded: HashMap<u64, (GpuModel, u64)>,
    frame: u64,
}

impl BrushUploads {
    /// Starts a frame: lets go of models no view has drawn for a while.
    pub(crate) fn begin_frame(&mut self) {
        self.frame += 1;
        let now = self.frame;
        self.uploaded
            .retain(|_, (_, drawn)| now.saturating_sub(*drawn) <= KEEP_FRAMES);
    }

    /// Uploads the models of `brushes` not on the GPU yet, and notes that
    /// all of them are drawn this frame.
    pub(crate) fn upload(
        &mut self,
        renderer: &render::ViewportRenderer,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        brushes: &[ViewBrush],
    ) {
        for brush in brushes {
            let key = brush.model.key;
            match self.uploaded.get_mut(&key) {
                Some((_, drawn)) => *drawn = self.frame,
                None => {
                    let model = renderer.upload_model(device, queue, &brush.model.model);
                    self.uploaded.insert(key, (model, self.frame));
                }
            }
        }
    }

    /// `brushes` as placements, those uploaded.
    pub(crate) fn placements<'a>(
        &'a self,
        brushes: &'a [ViewBrush],
    ) -> impl Iterator<Item = Placement<'a>> + 'a {
        brushes.iter().filter_map(|b| {
            self.uploaded.get(&b.model.key).map(|(model, _)| Placement {
                model,
                node: None,
                transform: b.transform,
                pick: b.pick,
            })
        })
    }
}
