//! The study's measurements, in the report's order. Each prints a
//! Markdown table.

mod frames;
#[cfg(feature = "gpu")]
pub(crate) mod gpu;
mod scale;
mod solid;
mod stats;
mod strokes;
pub(crate) mod worlds;

use ground_study::store::{Octree, Sparse};
use std::sync::Arc;

/// Every measurement but the graphics card's.
pub(crate) fn run() {
    println!(
        "Ground study: release build, {} cores here, {} threads in the job pool.",
        std::thread::available_parallelism().map_or(1, |n| n.get()),
        rayon::current_num_threads()
    );
    let full = scale::target_ground();
    let table = scale::storage_table(&full);
    scale::store_row(&table, &full);
    scale::store_row::<Sparse>(&table, &scale::kept_as(&full));
    scale::store_row::<Octree>(&table, &scale::kept_as(&full));
    scale::files(&full);
    scale::meshing(&full);
    strokes::by_store(&full);
    strokes::by_method(&full);
    solid::colliders(&full);
    solid::walking(&full);
    let full = Arc::new(full);
    frames::worker_or_frame(&full);
    frames::interference();
}

/// Uploads through the engine's renderer.
#[cfg(feature = "gpu")]
pub(crate) fn uploads() {
    let full = scale::target_ground();
    gpu::uploads(&full);
}
