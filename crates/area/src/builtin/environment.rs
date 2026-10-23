//! The world around the models: the sky, fog, exposure and tone mapping
//! ([decision 0017](../../../../docs/decisions/0017-hdr-tone-mapping-sky-and-fog.md)).
//! Saved with each area.

use super::field;
use reflect::{Component, ComponentData, ComponentSchema, Fields, Value};

/// What's drawn behind everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sky {
    /// The editor's calm grey backdrop.
    #[default]
    Backdrop,
    /// A sky that follows the sun.
    Daylight,
}

/// How bright light is mapped to the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToneMapping {
    /// Blender's default: soft highlights, natural colours.
    #[default]
    Agx,
    /// The film-like curve many games use.
    Aces,
    /// None: colours past white are cut off.
    Standard,
}

const SKIES: [&str; 2] = ["Backdrop", "Daylight"];
const TONE_MAPPINGS: [&str; 3] = ["AgX", "ACES", "Standard"];

/// The sky, fog and picture settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Environment {
    /// Which sky.
    pub sky: Sky,
    /// How bright the daylight sky is (1 natural).
    pub sky_strength: f32,
    /// Whether there's fog.
    pub fog: bool,
    /// The distance that hides half of something, in metres.
    pub fog_distance: f32,
    /// Whether the fog takes the sky's horizon colour.
    pub fog_matches_sky: bool,
    /// The fog's own colour, sRGB 0 to 1.
    pub fog_color: [f32; 3],
    /// Exposure in stops: +1 doubles the brightness.
    pub exposure: f32,
    /// How bright light is mapped to the screen.
    pub tone_mapping: ToneMapping,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            sky: Sky::Backdrop,
            sky_strength: 1.0,
            fog: false,
            fog_distance: 100.0,
            fog_matches_sky: true,
            fog_color: [180.0 / 255.0, 190.0 / 255.0, 205.0 / 255.0],
            exposure: 0.0,
            tone_mapping: ToneMapping::Agx,
        }
    }
}

impl Component for Environment {
    const NAME: &'static str = "Environment";

    fn schema() -> ComponentSchema {
        let d = Self::default();
        ComponentSchema {
            name: Self::NAME.into(),
            help: "The sky, fog, exposure and tone mapping of the area.".into(),
            fields: vec![
                field::choice(
                    "sky",
                    "Sky",
                    &SKIES,
                    SKIES[d.sky as usize],
                    "Backdrop: the calm grey you work against. Daylight: a sky that follows the sun.",
                ),
                field::float(
                    "sky_strength",
                    "Sky strength",
                    d.sky_strength,
                    field::hint(
                        (0.0, 4.0),
                        0.02,
                        "",
                        "How bright the daylight sky and its light are: 1 natural.",
                    ),
                ),
                field::boolean(
                    "fog",
                    "Fog",
                    d.fog,
                    "Distant things fade into the fog's colour.",
                ),
                field::float(
                    "fog_distance",
                    "Half hidden at",
                    d.fog_distance,
                    field::hint(
                        (1.0, 10_000.0),
                        0.5,
                        " m",
                        "At this distance, half of a thing is hidden by fog.",
                    ),
                ),
                field::boolean(
                    "fog_matches_sky",
                    "Fog matches sky",
                    d.fog_matches_sky,
                    "Take the fog's colour from the horizon.",
                ),
                field::color(
                    "fog_color",
                    "Fog colour",
                    d.fog_color,
                    "The fog's own colour.",
                ),
                field::float(
                    "exposure",
                    "Exposure",
                    d.exposure,
                    field::hint(
                        (-6.0, 6.0),
                        0.02,
                        " EV",
                        "Brightness in camera stops: +1 twice as bright.",
                    ),
                ),
                field::choice(
                    "tone_mapping",
                    "Tone mapping",
                    &TONE_MAPPINGS,
                    TONE_MAPPINGS[d.tone_mapping as usize],
                    "AgX: Blender's default. ACES: the film look of many games. Standard: none.",
                ),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("sky", Value::Text(SKIES[self.sky as usize].into()));
        data.set("sky_strength", Value::Float(f64::from(self.sky_strength)));
        data.set("fog", Value::Bool(self.fog));
        data.set("fog_distance", Value::Float(f64::from(self.fog_distance)));
        data.set("fog_matches_sky", Value::Bool(self.fog_matches_sky));
        data.set("fog_color", Value::floats(&self.fog_color));
        data.set("exposure", Value::Float(f64::from(self.exposure)));
        data.set(
            "tone_mapping",
            Value::Text(TONE_MAPPINGS[self.tone_mapping as usize].into()),
        );
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        let f = Fields(data);
        let d = Self::default();
        Self {
            sky: match f.text("sky") {
                Some("Daylight") => Sky::Daylight,
                _ => Sky::Backdrop,
            },
            sky_strength: f.float("sky_strength", d.sky_strength),
            fog: f.bool("fog", d.fog),
            fog_distance: f.float("fog_distance", d.fog_distance),
            fog_matches_sky: f.bool("fog_matches_sky", d.fog_matches_sky),
            fog_color: f.color("fog_color", d.fog_color),
            exposure: f.float("exposure", d.exposure),
            tone_mapping: match f.text("tone_mapping") {
                Some("ACES") => ToneMapping::Aces,
                Some("Standard") => ToneMapping::Standard,
                _ => ToneMapping::Agx,
            },
        }
    }
}
