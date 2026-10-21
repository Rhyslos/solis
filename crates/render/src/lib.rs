//! Draws viewports with wgpu: the backdrop, models (uploaded once with
//! [`ViewportRenderer::upload_model`]), the ground grid and the axes. Full
//! materials and lights arrive later in Phase 1 (ROADMAP.md).
//!
//! Layer 1 (engine). Knows nothing about the editor or any interface: it
//! receives a [`FrameParams`] (camera and colours) and produces an image in
//! a [`ViewportTarget`]. See this crate's README for its purpose and limits.

mod frame;
mod gpu_model;
mod readback;
mod renderer;

pub use frame::{
    FrameParams, GRID_HALF_EXTENT, GRID_MIN_REACH, GRID_REACH_PER_DISTANCE, GRID_SPACINGS,
    GRID_STRENGTHS, grid_reach, srgb_to_linear,
};
pub use gpu_model::GpuModel;
pub use readback::read_pixels;
pub use renderer::{
    AXIS_X_COLOR, AXIS_Y_COLOR, COLOR_FORMAT, DEPTH_FORMAT, LineVertex, SHADER_SOURCE,
    ViewportRenderer, ViewportTarget, best_sample_count, ground_axes,
};

#[cfg(test)]
mod shader_tests {
    use super::SHADER_SOURCE;
    use wgpu::naga;

    /// Parses and validates the WGSL with the same compiler wgpu uses, so
    /// shader mistakes fail the tests even on machines without a GPU.
    #[test]
    fn shader_is_valid() {
        let module = naga::front::wgsl::parse_str(SHADER_SOURCE)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(SHADER_SOURCE)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(SHADER_SOURCE)));
    }

    #[test]
    fn shader_has_the_expected_entry_points() {
        let module = naga::front::wgsl::parse_str(SHADER_SOURCE).unwrap();
        let mut names: Vec<_> = module
            .entry_points
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        names.sort_unstable();
        assert_eq!(
            names,
            [
                "grid_fs", "grid_vs", "line_fs", "line_vs", "mesh_fs", "mesh_vs", "sky_fs",
                "sky_vs"
            ]
        );
    }
}
