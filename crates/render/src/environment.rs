//! The world around the models: the sky (the editor's calm backdrop, or a
//! daylight sky that follows the sun), fog, and how the finished picture's
//! brightness is mapped to the screen (exposure and tone mapping).
//!
//! Light is worked out in HDR (high dynamic range): a sunlit white wall can
//! be many times brighter than "white" on a screen. Tone mapping squeezes
//! that range into what a screen shows, as cameras and Blender do, so
//! bright highlights roll off softly instead of flattening to white.

use crate::frame::srgb_to_linear;
use glam::Vec3;

/// How HDR brightness is mapped to the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tonemap {
    /// Blender's default since 4.0: soft highlights that drift towards
    /// white, as film does, with natural colours.
    #[default]
    Agx,
    /// The film-like curve many games use: punchier contrast and colour.
    Aces,
    /// No mapping: colours past white are cut off ("Standard" in Blender).
    Standard,
}

impl Tonemap {
    pub(crate) fn id(self) -> f32 {
        match self {
            Self::Agx => 0.0,
            Self::Aces => 1.0,
            Self::Standard => 2.0,
        }
    }
}

/// What's drawn behind everything, and where soft sky light comes from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sky {
    /// The editor's calm grey backdrop (the colours in `FrameParams`), with
    /// a neutral sky light. Shown exactly as the theme has it.
    Backdrop,
    /// A sky that follows the sun: blue by day, warm at sunrise and sunset,
    /// dark at night, with the sun's disc. Its light colours the shadows.
    Daylight {
        /// How bright the sky is compared with the sun (1 as in nature,
        /// lower for darker shadows).
        strength: f32,
    },
}

/// Fog: distant things fade into the fog colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fog {
    /// The distance at which half of a thing is hidden, in metres.
    pub half_distance: f32,
    /// The fog's colour, sRGB 0 to 1; `None` matches the sky's horizon.
    pub color: Option<[f32; 3]>,
}

/// The sky, fog and picture settings for a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Environment {
    /// The sky.
    pub sky: Sky,
    /// Fog, if any.
    pub fog: Option<Fog>,
    /// Exposure in stops (EV): +1 doubles the brightness, -1 halves it.
    pub exposure: f32,
    /// How brightness is mapped to the screen.
    pub tonemap: Tonemap,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            sky: Sky::Backdrop,
            fog: None,
            exposure: 0.0,
            tonemap: Tonemap::Agx,
        }
    }
}

/// The soft light from the sky above and the ground below, with the
/// editor's backdrop (decision 0008's first source of indirect light).
pub(crate) const BACKDROP_SKY_LIGHT: [f32; 3] = [0.30, 0.34, 0.40];
/// The light bounced up from the ground, with the editor's backdrop.
pub(crate) const BACKDROP_GROUND_LIGHT: [f32; 3] = [0.10, 0.09, 0.08];

/// The sky's colours for one frame, linear.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SkyColors {
    /// Straight up.
    pub top: [f32; 3],
    /// At the horizon.
    pub horizon: [f32; 3],
    /// Below the horizon.
    pub ground: [f32; 3],
    /// The soft light arriving from above.
    pub light_above: [f32; 3],
    /// The soft light arriving from below.
    pub light_below: [f32; 3],
    /// Whether this is the daylight sky (drawn with the sun's disc, and
    /// tone mapped like the models).
    pub daylight: bool,
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn scale(c: [f32; 3], k: f32) -> [f32; 3] {
    c.map(|v| v * k)
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The sky's colours: the backdrop's (sRGB `backdrop` top, horizon,
/// ground), or the daylight sky for a sun shining along `sun_direction`
/// at `sun_strength` W/m².
pub(crate) fn sky_colors(
    sky: Sky,
    backdrop: [[f32; 3]; 3],
    sun_direction: Vec3,
    sun_strength: f32,
) -> SkyColors {
    match sky {
        Sky::Backdrop => SkyColors {
            top: backdrop[0].map(srgb_to_linear),
            horizon: backdrop[1].map(srgb_to_linear),
            ground: backdrop[2].map(srgb_to_linear),
            light_above: BACKDROP_SKY_LIGHT,
            light_below: BACKDROP_GROUND_LIGHT,
            daylight: false,
        },
        Sky::Daylight { strength } => {
            let strength = if strength.is_finite() {
                strength.clamp(0.0, 10.0)
            } else {
                1.0
            };
            let towards_sun = (-sun_direction).try_normalize().unwrap_or(Vec3::Z);
            // The sun's height above the horizon, in degrees.
            let height = towards_sun.z.clamp(-1.0, 1.0).asin().to_degrees();
            // Day fades to night between 10° above and 6° below the horizon
            // (civil twilight), and the warm colours of sunrise and sunset
            // give way to blue between 5° and 30°.
            let day = smoothstep(-6.0, 10.0, height);
            let blue = smoothstep(5.0, 30.0, height);
            let sun = if sun_strength.is_finite() {
                (sun_strength / 3.0).clamp(0.0, 30.0)
            } else {
                1.0
            };
            let night_top = [0.002, 0.003, 0.008];
            let night_horizon = [0.004, 0.006, 0.012];
            let day_top = mix([0.14, 0.17, 0.36], [0.05, 0.16, 0.60], blue);
            let day_horizon = mix([0.95, 0.50, 0.25], [0.42, 0.58, 0.88], blue);
            let k = strength * (night_or_day(sun, day));
            let top = scale(mix(night_top, day_top, day), k);
            let horizon = scale(mix(night_horizon, day_horizon, day), k);
            let ground = scale(mix([0.003, 0.003, 0.003], [0.10, 0.09, 0.08], day), k);
            SkyColors {
                top,
                horizon,
                ground,
                light_above: mix(horizon, top, 0.6),
                light_below: scale(ground, 1.2),
                daylight: true,
            }
        }
    }
}

/// How bright the sky is: the sun's strength by day, a little starlight
/// by night.
fn night_or_day(sun: f32, day: f32) -> f32 {
    1.0 + (sun - 1.0) * day
}

/// The fog for the shader: linear colour (xyz) and how quickly it thickens
/// (w: ln 2 over the half distance; 0 for no fog).
pub(crate) fn fog_uniform(fog: Option<Fog>, sky: &SkyColors) -> [f32; 4] {
    let Some(fog) = fog.filter(|f| f.half_distance.is_finite() && f.half_distance > 0.0) else {
        return [0.0; 4];
    };
    let [r, g, b] = match fog.color {
        Some(c) => c.map(srgb_to_linear),
        None => sky.horizon,
    };
    [
        r,
        g,
        b,
        std::f32::consts::LN_2 / fog.half_distance.max(0.01),
    ]
}

/// The picture settings for the shader: x the exposure multiplier, y the
/// tone mapping.
pub(crate) fn post_uniform(env: &Environment) -> [f32; 4] {
    let ev = if env.exposure.is_finite() {
        env.exposure.clamp(-10.0, 10.0)
    } else {
        0.0
    };
    [2f32.powf(ev), env.tonemap.id(), 0.0, 0.0]
}

#[cfg(test)]
mod tests {
    use super::*;

    const BACKDROP: [[f32; 3]; 3] = [[0.1, 0.1, 0.1], [0.2, 0.2, 0.2], [0.3, 0.3, 0.3]];

    fn sun_at(height_deg: f32) -> Vec3 {
        let h = height_deg.to_radians();
        -Vec3::new(0.0, h.cos(), h.sin())
    }

    fn brightness(c: [f32; 3]) -> f32 {
        c.iter().sum()
    }

    #[test]
    fn the_backdrop_keeps_the_theme_colours() {
        let s = sky_colors(Sky::Backdrop, BACKDROP, sun_at(45.0), 3.0);
        assert!(!s.daylight);
        assert_eq!(s.top, [0.1f32; 3].map(srgb_to_linear));
        assert_eq!(s.light_above, BACKDROP_SKY_LIGHT);
    }

    #[test]
    fn the_daylight_sky_is_blue_by_day_warm_at_sunset_and_dark_at_night() {
        let sky = Sky::Daylight { strength: 1.0 };
        let noon = sky_colors(sky, BACKDROP, sun_at(60.0), 3.0);
        assert!(noon.daylight);
        assert!(
            noon.top[2] > noon.top[0] * 2.0,
            "blue overhead: {:?}",
            noon.top
        );
        let sunset = sky_colors(sky, BACKDROP, sun_at(2.0), 3.0);
        assert!(
            sunset.horizon[0] > sunset.horizon[2],
            "warm horizon: {:?}",
            sunset.horizon
        );
        let night = sky_colors(sky, BACKDROP, sun_at(-20.0), 3.0);
        assert!(brightness(night.top) * 20.0 < brightness(noon.top));
        // A stronger sun brightens the sky with it.
        let bright = sky_colors(sky, BACKDROP, sun_at(60.0), 6.0);
        assert!(brightness(bright.top) > brightness(noon.top) * 1.9);
    }

    #[test]
    fn odd_values_give_a_usable_sky() {
        for (strength, dir, sun) in [
            (f32::NAN, Vec3::NAN, f32::NAN),
            (f32::INFINITY, Vec3::ZERO, f32::INFINITY),
            (-5.0, Vec3::Z, -1.0),
        ] {
            let s = sky_colors(Sky::Daylight { strength }, BACKDROP, dir, sun);
            for c in [s.top, s.horizon, s.ground, s.light_above, s.light_below] {
                assert!(c.iter().all(|v| v.is_finite() && *v >= 0.0), "{c:?}");
            }
        }
    }

    #[test]
    fn fog_thickens_by_half_every_half_distance() {
        let sky = sky_colors(Sky::Backdrop, BACKDROP, sun_at(45.0), 3.0);
        assert_eq!(fog_uniform(None, &sky), [0.0; 4]);
        let f = fog_uniform(
            Some(Fog {
                half_distance: 50.0,
                color: None,
            }),
            &sky,
        );
        assert_eq!(f[..3], sky.horizon, "matches the horizon");
        assert!(((-f[3] * 50.0).exp() - 0.5).abs() < 1e-5);
        let broken = Fog {
            half_distance: f32::NAN,
            color: Some([1.0, 0.0, 0.0]),
        };
        assert_eq!(fog_uniform(Some(broken), &sky), [0.0; 4]);
    }

    #[test]
    fn exposure_doubles_per_stop() {
        let mut env = Environment::default();
        assert_eq!(post_uniform(&env), [1.0, 0.0, 0.0, 0.0]);
        env.exposure = 1.0;
        env.tonemap = Tonemap::Standard;
        assert_eq!(post_uniform(&env), [2.0, 2.0, 0.0, 0.0]);
        env.exposure = f32::NAN;
        assert_eq!(post_uniform(&env)[0], 1.0);
    }
}
