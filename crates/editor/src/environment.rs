//! The world around the models, as the editor holds it: the sky, fog, and
//! how the picture's brightness is shown (exposure and tone mapping).
//! Remembered between sessions with the editor's other settings, until
//! scenes arrive in Phase 2.

use crate::properties::row;
use egui::Ui;
use serde::{Deserialize, Serialize};

/// What's drawn behind everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SkyKind {
    /// The editor's calm grey backdrop, with a neutral sky light.
    #[default]
    Backdrop,
    /// A sky that follows the sun, and colours the shadows.
    Daylight,
}

impl SkyKind {
    /// Both, in menu order.
    pub const ALL: [Self; 2] = [Self::Backdrop, Self::Daylight];

    /// The name shown in Properties.
    pub fn name(self) -> &'static str {
        match self {
            Self::Backdrop => "Editor backdrop",
            Self::Daylight => "Daylight",
        }
    }
}

/// How bright light is mapped to the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ToneMapping {
    /// Blender's default: soft highlights, natural colours.
    #[default]
    Agx,
    /// The film-like curve many games use: punchier.
    Aces,
    /// None: colours past white are cut off.
    Standard,
}

impl ToneMapping {
    /// All three, in menu order.
    pub const ALL: [Self; 3] = [Self::Agx, Self::Aces, Self::Standard];

    /// The name shown in Properties.
    pub fn name(self) -> &'static str {
        match self {
            Self::Agx => "AgX",
            Self::Aces => "ACES",
            Self::Standard => "Standard",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Self::Agx => "Blender's default: bright light rolls off softly towards white",
            Self::Aces => "The film look many games use: more contrast and colour",
            Self::Standard => "No mapping: anything brighter than white is cut off",
        }
    }
}

/// The sky, fog and picture settings.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EnvironmentSettings {
    /// Which sky.
    pub sky: SkyKind,
    /// How bright the daylight sky is (1 natural).
    pub sky_strength: f32,
    /// Whether there's fog.
    pub fog: bool,
    /// The distance that hides half of something, in metres.
    pub fog_distance: f32,
    /// Whether the fog takes the sky's horizon colour.
    pub fog_matches_sky: bool,
    /// The fog's own colour, sRGB.
    pub fog_color: [u8; 3],
    /// Exposure in stops: +1 doubles the brightness.
    pub exposure: f32,
    /// How bright light is mapped to the screen.
    pub tone_mapping: ToneMapping,
}

impl Default for EnvironmentSettings {
    fn default() -> Self {
        Self {
            sky: SkyKind::Backdrop,
            sky_strength: 1.0,
            fog: false,
            fog_distance: 100.0,
            fog_matches_sky: true,
            fog_color: [180, 190, 205],
            exposure: 0.0,
            tone_mapping: ToneMapping::Agx,
        }
    }
}

impl EnvironmentSettings {
    /// The nearest and furthest fog distances the field allows, in metres.
    pub const FOG_DISTANCES: std::ops::RangeInclusive<f32> = 1.0..=10_000.0;
    /// The exposure range, in stops.
    pub const EXPOSURES: std::ops::RangeInclusive<f32> = -6.0..=6.0;
    /// The strongest daylight sky.
    pub const MAX_SKY_STRENGTH: f32 = 4.0;

    /// The settings with anything out of range put back in range (as read
    /// from a damaged settings file).
    pub fn sanitized(self) -> Self {
        let d = Self::default();
        let finite = |v: f32, fallback: f32| if v.is_finite() { v } else { fallback };
        Self {
            sky_strength: finite(self.sky_strength, d.sky_strength)
                .clamp(0.0, Self::MAX_SKY_STRENGTH),
            fog_distance: finite(self.fog_distance, d.fog_distance)
                .clamp(*Self::FOG_DISTANCES.start(), *Self::FOG_DISTANCES.end()),
            exposure: finite(self.exposure, d.exposure)
                .clamp(*Self::EXPOSURES.start(), *Self::EXPOSURES.end()),
            ..self
        }
    }
}

/// The sky, fog and picture fields in the Properties panel.
pub fn environment_properties(ui: &mut Ui, env: &mut EnvironmentSettings) {
    ui.label(egui::RichText::new("Sky").strong());
    row(ui, "Sky", |ui| {
        egui::ComboBox::from_id_salt("sky kind")
            .selected_text(env.sky.name())
            .show_ui(ui, |ui| {
                for kind in SkyKind::ALL {
                    ui.selectable_value(&mut env.sky, kind, kind.name());
                }
            })
            .response
            .on_hover_text(
                "Editor backdrop: the calm grey you work against. Daylight: a sky that \
                 follows the sun, from blue noon to orange sunset and dark night",
            );
    });
    if env.sky == SkyKind::Daylight {
        row(ui, "Strength", |ui| {
            ui.add(
                egui::DragValue::new(&mut env.sky_strength)
                    .range(0.0..=EnvironmentSettings::MAX_SKY_STRENGTH)
                    .speed(0.02),
            )
            .on_hover_text("How bright the sky and its light in the shadows are: 1 natural");
        });
    }
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Atmosphere").strong());
    row(ui, "Fog", |ui| ui.checkbox(&mut env.fog, "On"));
    if env.fog {
        row(ui, "Half hidden at", |ui| {
            ui.add(
                egui::DragValue::new(&mut env.fog_distance)
                    .range(EnvironmentSettings::FOG_DISTANCES)
                    .speed(0.5)
                    .suffix(" m"),
            )
            .on_hover_text(
                "At this distance, half of a thing is hidden by fog; twice as far, three quarters",
            )
        });
        row(ui, "Colour", |ui| {
            ui.checkbox(&mut env.fog_matches_sky, "Match the sky");
            if !env.fog_matches_sky {
                ui.color_edit_button_srgb(&mut env.fog_color);
            }
        });
    }
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Picture").strong());
    row(ui, "Exposure", |ui| {
        ui.add(
            egui::DragValue::new(&mut env.exposure)
                .range(EnvironmentSettings::EXPOSURES)
                .speed(0.02)
                .suffix(" EV"),
        )
        .on_hover_text("Brightness in camera stops: +1 is twice as bright, -1 half");
    });
    row(ui, "Tone mapping", |ui| {
        egui::ComboBox::from_id_salt("tone mapping")
            .selected_text(env.tone_mapping.name())
            .show_ui(ui, |ui| {
                for mapping in ToneMapping::ALL {
                    ui.selectable_value(&mut env.tone_mapping, mapping, mapping.name())
                        .on_hover_text(mapping.hint());
                }
            });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damaged_settings_are_tidied() {
        let s = EnvironmentSettings {
            sky_strength: f32::NAN,
            fog_distance: -5.0,
            exposure: f32::INFINITY,
            ..EnvironmentSettings::default()
        }
        .sanitized();
        assert_eq!(s.sky_strength, 1.0);
        assert_eq!(s.fog_distance, 1.0);
        assert_eq!(s.exposure, 0.0);
        let s = EnvironmentSettings {
            exposure: 99.0,
            sky_strength: 99.0,
            fog_distance: 1e9,
            ..s
        }
        .sanitized();
        assert_eq!(
            (s.exposure, s.sky_strength, s.fog_distance),
            (6.0, EnvironmentSettings::MAX_SKY_STRENGTH, 10_000.0)
        );
    }

    #[test]
    fn settings_round_trip_and_survive_rubbish() {
        let env = EnvironmentSettings {
            sky: SkyKind::Daylight,
            sky_strength: 0.5,
            fog: true,
            fog_distance: 42.0,
            fog_matches_sky: false,
            fog_color: [1, 2, 3],
            exposure: -1.5,
            tone_mapping: ToneMapping::Aces,
        };
        let text = ron::to_string(&env).unwrap();
        assert_eq!(ron::from_str::<EnvironmentSettings>(&text).unwrap(), env);
        assert_eq!(
            ron::from_str::<EnvironmentSettings>("(fog: true)")
                .unwrap()
                .fog_distance,
            100.0
        );
        let mut seed: u64 = 5;
        for _ in 0..2_000 {
            let mut bytes = text.clone().into_bytes();
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let at = (seed >> 33) as usize % bytes.len();
            bytes[at] = (seed >> 13) as u8;
            if let Ok(s) = std::str::from_utf8(&bytes)
                && let Ok(env) = ron::from_str::<EnvironmentSettings>(s)
            {
                let tidy = env.sanitized();
                assert!(tidy.exposure.is_finite() && tidy.fog_distance >= 1.0);
            }
        }
    }
}
