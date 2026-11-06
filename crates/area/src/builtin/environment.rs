//! The world around the models: the sky, fog, exposure and tone mapping
//! ([decision 0017](../../../../docs/decisions/0017-hdr-tone-mapping-sky-and-fog.md)).
//! Saved with each area.

use super::field;
use reflect::{
    Component, ComponentData, ComponentSchema, FieldInfo, FieldKind, Fields, Hint, Value,
};

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
    /// Day and night (decision 0092): whether the day's colours tint the
    /// world as the game's time of day runs.
    pub day_night: bool,
    /// How long a day lasts, in minutes of game time.
    pub day_minutes: f32,
    /// The hour the game starts at (0 up to 24).
    pub day_start: f32,
    /// The day's colours.
    pub day_colours: super::DayColours,
    /// Whether sprites and tile maps are lit: by the day's tint and the
    /// lamps.
    pub light_sprites: bool,
    /// How many steps the light on sprites is drawn in (0 smooth).
    pub light_steps: i64,
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
            day_night: false,
            day_minutes: 24.0,
            day_start: 8.0,
            day_colours: super::DayColours::default(),
            light_sprites: false,
            light_steps: 0,
        }
    }
}

impl Environment {
    /// The most light steps.
    pub const MAX_STEPS: i64 = 8;

    /// The game's hour at `seconds` of game time, moved on by `shift`
    /// hours (game code's `setTimeOfDay`): 0 up to 24.
    pub fn hour_at(&self, seconds: f64, shift: f64) -> f64 {
        let minutes = f64::from(self.day_minutes.clamp(0.1, 100_000.0));
        let h = f64::from(self.day_start) + seconds / (minutes * 60.0) * 24.0 + shift;
        if h.is_finite() {
            h.rem_euclid(24.0)
        } else {
            0.0
        }
    }

    /// The day's tint at `hour` (sRGB 0 to 1); white with day and night off.
    pub fn tint_at(&self, hour: f64) -> [f32; 3] {
        if self.day_night {
            self.day_colours.at(hour as f32)
        } else {
            [1.0; 3]
        }
    }

    /// Whether lamps that shine only at night are on at `hour`: always,
    /// with day and night off.
    pub fn is_night(&self, hour: f64) -> bool {
        !self.day_night || self.day_colours.is_night(hour as f32)
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
                field::boolean(
                    "day_night",
                    "Day and night",
                    d.day_night,
                    "The day's colours tint the world as the game's time of day runs (World:timeOfDay).",
                ),
                field::float(
                    "day_minutes",
                    "A day lasts",
                    d.day_minutes,
                    field::hint(
                        (0.1, 100_000.0),
                        0.1,
                        " min",
                        "How long a whole day lasts, in minutes of play. A paused game's day stands still.",
                    ),
                ),
                field::float(
                    "day_start",
                    "Starts at",
                    d.day_start,
                    field::hint(
                        (0.0, 24.0),
                        0.05,
                        " h",
                        "The hour the game starts at: 8 is eight in the morning, 19.5 half past seven at night.",
                    ),
                ),
                FieldInfo::new(
                    "day_colours",
                    "Colours",
                    FieldKind::Text,
                    Value::Text(d.day_colours.text()),
                )
                .hint(Hint {
                    help: "The day's colours at their hours: the world is tinted by them, smoothly from one to the next.",
                    offers: reflect::Offer::DayColours,
                    ..Hint::default()
                }),
                field::boolean(
                    "light_sprites",
                    "Light sprites",
                    d.light_sprites,
                    "Sprites and tile maps are lit: by the day's colours and by lamps.",
                ),
                FieldInfo::new("light_steps", "Light steps", FieldKind::Int, Value::Int(0)).hint(
                    Hint {
                        range: Some((0.0, Self::MAX_STEPS as f64)),
                        help: "0 for smooth light on sprites; 2 to 8 for bands of light, a stylized look.",
                        ..Hint::default()
                    },
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
        data.set("day_night", Value::Bool(self.day_night));
        data.set("day_minutes", Value::Float(f64::from(self.day_minutes)));
        data.set("day_start", Value::Float(f64::from(self.day_start)));
        data.set("day_colours", Value::Text(self.day_colours.text()));
        data.set("light_sprites", Value::Bool(self.light_sprites));
        data.set("light_steps", Value::Int(self.light_steps));
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
            day_night: f.bool("day_night", d.day_night),
            day_minutes: {
                let m = f.float("day_minutes", d.day_minutes);
                if m.is_finite() {
                    m.clamp(0.1, 100_000.0)
                } else {
                    d.day_minutes
                }
            },
            day_start: {
                let h = f.float("day_start", d.day_start);
                if h.is_finite() {
                    h.rem_euclid(24.0)
                } else {
                    d.day_start
                }
            },
            day_colours: f
                .text("day_colours")
                .map_or(d.day_colours, super::DayColours::parse),
            light_sprites: f.bool("light_sprites", d.light_sprites),
            light_steps: match data.get("light_steps") {
                // One step would be all or nothing: two is the fewest.
                Some(Value::Int(1)) => 2,
                Some(Value::Int(n)) => (*n).clamp(0, Self::MAX_STEPS),
                _ => 0,
            },
        }
    }
}
