//! What the renderer needs to know each frame, and how it is laid out for
//! the GPU.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

/// Half the width of the drawn grid, in metres (1 km each way).
pub const GRID_HALF_EXTENT: f32 = 1_000.0;
/// Spacing of the three grid levels, in metres: 1 m, 10 m and 100 m.
pub const GRID_SPACINGS: [f32; 3] = [1.0, 10.0, 100.0];
/// How strongly each grid level is drawn (opacity of a white line, 0 to 1),
/// in the same order as [`GRID_SPACINGS`]. Kept low so the grid stays a
/// quiet guide rather than a pattern.
pub const GRID_STRENGTHS: [f32; 3] = [0.07, 0.14, 0.2];
/// The least distance around the focus point that the grid reaches, in
/// metres: even close in, the ground stretches well beyond what is in view.
pub const GRID_MIN_REACH: f32 = 60.0;
/// How far the grid reaches compared with the camera's distance from the
/// focus point: further away shows more of it.
pub const GRID_REACH_PER_DISTANCE: f32 = 3.0;

/// Everything that changes from frame to frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameParams {
    /// World-to-screen transform (depth 0 at near, 1 at far).
    pub view_projection: Mat4,
    /// Where the camera is.
    pub camera_position: Vec3,
    /// The point on the ground the grid is centred on.
    pub focus: Vec3,
    /// The sky straight up, sRGB 0 to 1 (as colour pickers and the
    /// interface theme give them).
    pub sky_top: [f32; 3],
    /// The sky at the horizon, sRGB 0 to 1.
    pub sky_horizon: [f32; 3],
    /// The ground just below the horizon, sRGB 0 to 1.
    pub ground: [f32; 3],
    /// How models are shaded.
    pub shading: Shading,
    /// The image's size in pixels (for outlines a set number of pixels
    /// wide).
    pub size_px: [u32; 2],
    /// The sun's direction, colour, strength and shadows.
    pub sun: crate::lighting::Sun,
}

/// The ways models can be shaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Look {
    /// Realistic: metal and roughness, highlights, soft sky light.
    #[default]
    Standard,
    /// Cel-shaded: flat bands of light, crisp highlights, outlines.
    Stylized,
    /// Colours and textures only, without light.
    Unlit,
}

/// How models are shaded in a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shading {
    /// Which look.
    pub look: Look,
    /// The stylized look's bands of light (2 to 4).
    pub bands: u32,
    /// The stylized look's outline width in pixels; 0 for none.
    pub outline_px: f32,
}

impl Default for Shading {
    fn default() -> Self {
        Self {
            look: Look::Standard,
            bands: 3,
            outline_px: 0.0,
        }
    }
}

impl Shading {
    /// Whether outlines are drawn.
    pub fn outlines(&self) -> bool {
        self.look == Look::Stylized && self.outline_px > 0.0
    }
}

/// Converts one sRGB channel (0 to 1) to linear light, as the shaders work.
pub fn srgb_to_linear(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear(rgb: [f32; 3]) -> [f32; 4] {
    [
        srgb_to_linear(rgb[0]),
        srgb_to_linear(rgb[1]),
        srgb_to_linear(rgb[2]),
        1.0,
    ]
}

/// How far around the focus point the grid reaches, for a camera at
/// `distance` metres from it.
pub fn grid_reach(distance: f32) -> f32 {
    let distance = if distance.is_finite() { distance } else { 0.0 };
    (distance.abs() * GRID_REACH_PER_DISTANCE).max(GRID_MIN_REACH)
}

/// The frame data exactly as the shader's `Frame` struct expects it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct FrameUniforms {
    view_projection: [[f32; 4]; 4],
    inverse_view_projection: [[f32; 4]; 4],
    camera: [f32; 4],
    focus: [f32; 4],
    grid: [f32; 4],
    grid_strength: [f32; 4],
    sky_top: [f32; 4],
    sky_horizon: [f32; 4],
    ground: [f32; 4],
    shading: [f32; 4],
    screen: [f32; 4],
    sun_view_proj: [[f32; 4]; 4],
    sun_direction: [f32; 4],
    sun_color: [f32; 4],
    shadow: [f32; 4],
}

/// What the renderer worked out about the sun and lamps for one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FrameLight {
    /// The sun's view, for its shadow map.
    pub sun_view_projection: Mat4,
    /// One shadow-map texel's width in metres.
    pub texel: f32,
    /// Whether the shadow map was drawn this frame.
    pub shadows_drawn: bool,
    /// How many lamps are in the lamp list.
    pub lamps: u32,
}

impl Default for FrameLight {
    fn default() -> Self {
        Self {
            sun_view_projection: Mat4::IDENTITY,
            texel: 0.0,
            shadows_drawn: false,
            lamps: 0,
        }
    }
}

impl FrameUniforms {
    pub(crate) fn new(params: &FrameParams, light: &FrameLight) -> Self {
        let (sun_direction, sun_color) =
            crate::lighting::sun_uniforms(&params.sun, light.shadows_drawn);
        let reach = grid_reach(params.camera_position.distance(params.focus));
        let [fine, major, super_level] = GRID_SPACINGS;
        let [a, b, c] = GRID_STRENGTHS;
        Self {
            view_projection: params.view_projection.to_cols_array_2d(),
            inverse_view_projection: params.view_projection.inverse().to_cols_array_2d(),
            camera: params.camera_position.extend(1.0).to_array(),
            focus: [params.focus.x, params.focus.y, params.focus.z, reach],
            grid: [fine, major, super_level, GRID_HALF_EXTENT],
            grid_strength: [a, b, c, 0.0],
            sky_top: linear(params.sky_top),
            sky_horizon: linear(params.sky_horizon),
            ground: linear(params.ground),
            shading: [
                match params.shading.look {
                    Look::Standard => 0.0,
                    Look::Stylized => 1.0,
                    Look::Unlit => 2.0,
                },
                params.shading.bands.clamp(1, 8) as f32,
                if params.shading.outlines() {
                    params.shading.outline_px.clamp(0.0, 16.0)
                } else {
                    0.0
                },
                0.0,
            ],
            screen: [params.size_px[0] as f32, params.size_px[1] as f32, 0.0, 0.0],
            sun_view_proj: light.sun_view_projection.to_cols_array_2d(),
            sun_direction,
            sun_color,
            shadow: [
                light.texel,
                crate::lighting::SHADOW_MAP_SIZE as f32,
                light.lamps as f32,
                0.0,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> FrameParams {
        FrameParams {
            view_projection: Mat4::IDENTITY,
            camera_position: Vec3::new(0.0, -10.0, 5.0),
            focus: Vec3::ZERO,
            sky_top: [0.29, 0.29, 0.29],
            sky_horizon: [0.24, 0.24, 0.24],
            ground: [0.22, 0.22, 0.22],
            shading: Shading::default(),
            size_px: [800, 600],
            sun: crate::lighting::Sun::default(),
        }
    }

    #[test]
    fn uniforms_match_the_shader_layout() {
        // Three matrices and twelve vectors of 16 bytes.
        assert_eq!(std::mem::size_of::<FrameUniforms>(), 3 * 64 + 12 * 16);
    }

    #[test]
    fn srgb_conversion_keeps_the_ends_and_darkens_the_middle() {
        assert_eq!(srgb_to_linear(0.0), 0.0);
        assert!((srgb_to_linear(1.0) - 1.0).abs() < 1e-6);
        assert!(srgb_to_linear(0.5) < 0.5);
        assert_eq!(srgb_to_linear(-1.0), 0.0);
        assert!((srgb_to_linear(2.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_grid_reaches_further_from_further_away() {
        assert_eq!(grid_reach(1.0), GRID_MIN_REACH);
        assert!(grid_reach(500.0) > grid_reach(100.0));
        assert_eq!(grid_reach(f32::NAN), GRID_MIN_REACH);
        assert_eq!(grid_reach(-1000.0), grid_reach(1000.0));
    }

    #[test]
    fn uniforms_carry_the_frame() {
        let u = FrameUniforms::new(&params(), &FrameLight::default());
        assert_eq!(u.camera, [0.0, -10.0, 5.0, 1.0]);
        assert_eq!(u.focus[3], grid_reach(params().camera_position.length()));
        assert_eq!(u.grid, [1.0, 10.0, 100.0, GRID_HALF_EXTENT]);
        assert!(u.sky_top[0] < 0.29, "colours are converted to linear");
    }

    #[test]
    fn the_look_reaches_the_shader() {
        let mut p = params();
        p.shading = Shading {
            look: Look::Stylized,
            bands: 2,
            outline_px: 3.0,
        };
        let u = FrameUniforms::new(&p, &FrameLight::default());
        assert_eq!(u.shading, [1.0, 2.0, 3.0, 0.0]);
        assert_eq!(u.screen[..2], [800.0, 600.0]);
        // Outlines belong to the stylized look only.
        p.shading.look = Look::Standard;
        assert_eq!(
            FrameUniforms::new(&p, &FrameLight::default()).shading[2],
            0.0
        );
        // Silly values are kept in range.
        p.shading = Shading {
            look: Look::Stylized,
            bands: 999,
            outline_px: f32::INFINITY,
        };
        let u = FrameUniforms::new(&p, &FrameLight::default());
        assert_eq!(u.shading[1], 8.0);
        assert_eq!(u.shading[2], 16.0);
    }
}

#[cfg(test)]
mod light_tests {
    use super::*;

    #[test]
    fn the_sun_and_lamps_reach_the_shader() {
        let params = FrameParams {
            view_projection: Mat4::IDENTITY,
            camera_position: Vec3::ZERO,
            focus: Vec3::ZERO,
            sky_top: [0.0; 3],
            sky_horizon: [0.0; 3],
            ground: [0.0; 3],
            shading: Shading::default(),
            size_px: [1, 1],
            sun: crate::lighting::Sun::default(),
        };
        let light = FrameLight {
            sun_view_projection: Mat4::from_scale(Vec3::splat(2.0)),
            texel: 0.01,
            shadows_drawn: true,
            lamps: 3,
        };
        let u = FrameUniforms::new(&params, &light);
        assert_eq!(u.shadow[0], 0.01);
        assert_eq!(u.shadow[2], 3.0);
        assert_eq!(u.sun_direction[3], 1.0);
        assert_eq!(u.sun_view_proj[0][0], 2.0);
        assert!(u.sun_direction[2] > 0.0, "towards the sun, which is up");
    }
}
