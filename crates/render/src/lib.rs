//! Draws viewports with wgpu: the backdrop, models (uploaded once with
//! [`ViewportRenderer::upload_model`]), the ground grid and the axes. Full
//! materials, the sun with its shadows, lamps, the sky and fog, drawn in HDR
//! and tone mapped.
//!
//! Layer 1 (engine). Knows nothing about the editor or any interface: it
//! receives a [`FrameParams`] (camera and colours) and produces an image in
//! a [`ViewportTarget`]. See this crate's README for its purpose and limits.

mod environment;
mod frame;
mod gpu_model;
mod instances;
mod lighting;
mod pick;
mod pixel;
mod projectiles;
mod readback;
mod renderer;
mod sprites;
mod timing;
mod ui;

pub use environment::{Environment, Fog, Sky, Tonemap};
pub use frame::{
    FrameParams, GRID_HALF_EXTENT, GRID_MIN_REACH, GRID_REACH_PER_DISTANCE, GRID_SPACINGS,
    GRID_STRENGTHS, GridPlane, Look, Shading, grid_reach, srgb_to_linear,
};
pub use gpu_model::GpuModel;
pub use instances::{LOD_PIXELS, Placement};
pub use lighting::{
    Light, LightKind, MAX_LIGHTS, SHADOW_FORMAT, SHADOW_MAP_SIZE, SHADOW_REACH, Sun, light_markers,
    sun_view_fitted, sun_view_projection,
};
pub use pick::{PICK_SHADER_SOURCE, PickResult};
pub use pixel::{
    MAX_SLIDE, PIXEL_SHADER_SOURCE, PixelImage, PixelLayout, PixelScaling, pixel_layout,
};
pub use projectiles::Bolt;
pub use readback::{read_pixels, read_texture};
pub use renderer::{
    AXIS_X_COLOR, AXIS_Y_COLOR, COLOR_FORMAT, DEPTH_FORMAT, HDR_FORMAT, LineVertex, SHADER_SOURCE,
    Scene, ViewportRenderer, ViewportTarget, best_sample_count, ground_axes,
};
pub use sprites::{GpuSheet, SpriteInstance, SpriteLayer, SpritePicks, SpriteRun};
pub use timing::{FrameStats, GpuTimes};
pub use ui::{MAX_SHAPES as MAX_UI_SHAPES, UiInstance, UiPipeline};

/// The shaders as the compiler reads them, for the tests that hold the
/// Rust side to them: a uniform's members and offsets, and the constants
/// a shader repeats from Rust, so a change on one side only fails a test.
#[cfg(test)]
pub(crate) mod shader_layout {
    use wgpu::naga;

    fn module(source: &str) -> naga::Module {
        naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)))
    }

    /// Checks that the struct `name` in `source` has exactly `members`
    /// (each its name in the shader and the byte offset Rust gives it), in
    /// order, and is `size` bytes long.
    pub(crate) fn assert_struct(source: &str, name: &str, size: usize, members: &[(&str, usize)]) {
        let module = module(source);
        let (found, span) = module
            .types
            .iter()
            .find_map(|(_, ty)| match &ty.inner {
                naga::TypeInner::Struct { members, span } if ty.name.as_deref() == Some(name) => {
                    let found: Vec<(String, usize)> = members
                        .iter()
                        .map(|m| (m.name.clone().unwrap_or_default(), m.offset as usize))
                        .collect();
                    Some((found, *span as usize))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("the shader has no struct {name}"));
        let want: Vec<(String, usize)> = members
            .iter()
            .map(|&(member, offset)| (member.to_owned(), offset))
            .collect();
        assert_eq!(found, want, "{name}: the shader's members and offsets");
        assert_eq!(span, size, "{name}: the shader's size");
    }

    /// The value of the `const` called `name` in `source`.
    pub(crate) fn constant(source: &str, name: &str) -> f64 {
        let module = module(source);
        let (_, found) = module
            .constants
            .iter()
            .find(|(_, c)| c.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("the shader has no const {name}"));
        match module.global_expressions[found.init] {
            naga::Expression::Literal(naga::Literal::F32(v)) => f64::from(v),
            naga::Expression::Literal(naga::Literal::U32(v)) => f64::from(v),
            naga::Expression::Literal(naga::Literal::I32(v)) => f64::from(v),
            ref other => panic!("{name} is {other:?}, not a number"),
        }
    }

    #[test]
    fn a_struct_or_constant_that_differs_is_caught() {
        let source = "struct S { a: vec4<f32>, b: f32, }\nconst N: u32 = 7u;\n";
        assert_struct(source, "S", 32, &[("a", 0), ("b", 16)]);
        assert_eq!(constant(source, "N"), 7.0);
        let swapped = std::panic::catch_unwind(|| {
            assert_struct(source, "S", 32, &[("b", 0), ("a", 16)]);
        });
        assert!(swapped.is_err(), "members in another order");
        let longer =
            std::panic::catch_unwind(|| assert_struct(source, "S", 48, &[("a", 0), ("b", 16)]));
        assert!(longer.is_err(), "another size");
    }
}

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
    fn the_ui_shader_is_valid() {
        let source = include_str!("ui.wgsl");
        let module = naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
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
                "bolt_fs",
                "bolt_vs",
                "grid_fs",
                "grid_vs",
                "line_fs",
                "line_vs",
                "mesh_fs",
                "mesh_vs",
                "outline_fs",
                "outline_vs",
                "post_fs",
                "post_vs",
                "shadow_vs",
                "sky_fs",
                "sky_vs",
                "sprite_fs",
                "sprite_vs"
            ]
        );
    }
}
