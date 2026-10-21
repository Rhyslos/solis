//! The scene's light as the editor holds it: the sun's settings, and the
//! lamps (point lights and spotlights), whether placed by hand or brought
//! in with a model ([decision 0014](../../../docs/decisions/0014-lights-placed-two-ways.md)).
//!
//! Strengths use Blender's units, so values carry over: watts for lamps,
//! watts per square metre for the sun. Colours are sRGB, as colour pickers
//! show them. Until scenes arrive in Phase 2, lamps aren't saved; the sun's
//! settings are, with the editor's other settings.

use crate::models::ModelId;
use egui::{RichText, Ui};
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// The sun: its place in the sky, colour, strength and shadows.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SunSettings {
    /// Which way round the sky it is, in degrees clockwise from north
    /// (+Y): 90 is east (+X), 180 south.
    pub heading: f32,
    /// How high it is, in degrees above the horizon: 90 is straight
    /// overhead.
    pub height: f32,
    /// Its colour, sRGB.
    pub color: [u8; 3],
    /// Its strength in watts per square metre (Blender's unit).
    pub strength: f32,
    /// Whether it casts shadows.
    pub shadows: bool,
}

impl Default for SunSettings {
    fn default() -> Self {
        Self {
            heading: 150.0,
            height: 50.0,
            color: [255, 250, 242],
            strength: 3.0,
            shadows: true,
        }
    }
}

impl SunSettings {
    /// The strongest sun the fields allow, in W/m².
    pub const MAX_STRENGTH: f32 = 100.0;

    /// The direction its light travels (from the sky towards the ground).
    pub fn direction(&self) -> Vec3 {
        let (heading, height) = (self.heading.to_radians(), self.height.to_radians());
        -Vec3::new(
            heading.sin() * height.cos(),
            heading.cos() * height.cos(),
            height.sin(),
        )
    }

    /// A sun as a model file describes it: the way its light travels, its
    /// linear colour and strength; shadows stay as `current` has them.
    pub fn from_file(direction: Vec3, color: [f32; 3], strength: f32, current: Self) -> Self {
        let towards = (-direction).try_normalize().unwrap_or(Vec3::Z);
        Self {
            heading: towards.x.atan2(towards.y).to_degrees(),
            height: towards.z.clamp(-1.0, 1.0).asin().to_degrees(),
            color: color.map(linear_to_srgb_byte),
            strength,
            shadows: current.shadows,
        }
        .sanitized()
    }

    /// The settings with anything out of range put back in range (as read
    /// from a damaged settings file).
    pub fn sanitized(self) -> Self {
        let d = Self::default();
        let finite = |v: f32, fallback: f32| if v.is_finite() { v } else { fallback };
        Self {
            heading: finite(self.heading, d.heading).rem_euclid(360.0),
            height: finite(self.height, d.height).clamp(-90.0, 90.0),
            color: self.color,
            strength: finite(self.strength, d.strength).clamp(0.0, Self::MAX_STRENGTH),
            shadows: self.shadows,
        }
    }
}

/// Which lamp, as the editor numbers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LampId(pub u64);

/// What kind of lamp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LampKind {
    /// Light in every direction from a point, like a bulb.
    Point,
    /// A cone of light, like a torch or stage light. Angles in degrees
    /// from the middle of the cone: full strength inside `inner`, fading
    /// to nothing at `outer`.
    Spot {
        /// Where the fade starts.
        inner: f32,
        /// Where the light ends.
        outer: f32,
    },
}

impl LampKind {
    /// "Point light" or "Spotlight".
    pub fn name(self) -> &'static str {
        match self {
            Self::Point => "Point light",
            Self::Spot { .. } => "Spotlight",
        }
    }
}

/// A lamp in the viewport.
#[derive(Debug, Clone, PartialEq)]
pub struct Lamp {
    /// Its number.
    pub id: LampId,
    /// A name to show.
    pub name: String,
    /// Point or spot.
    pub kind: LampKind,
    /// Where it is, in metres.
    pub position: Vec3,
    /// The way a spotlight shines (length 1).
    pub direction: Vec3,
    /// Its colour, sRGB.
    pub color: [u8; 3],
    /// Its strength in watts (Blender's unit).
    pub strength: f32,
    /// How far it reaches in metres; `None` fades on forever.
    pub range: Option<f32>,
    /// The model it came in with, if any (removed along with it).
    pub from_model: Option<ModelId>,
}

impl Lamp {
    /// The strongest lamp the fields allow, in watts.
    pub const MAX_STRENGTH: f32 = 100_000.0;
    /// A new lamp's strength in watts: a bright room light, as in Blender.
    pub const DEFAULT_STRENGTH: f32 = 1000.0;
    /// How far above the camera's focus point a new point light goes.
    pub const POINT_HEIGHT: f32 = 2.0;

    /// A point light `POINT_HEIGHT` above `focus`.
    pub fn point(id: LampId, focus: Vec3) -> Self {
        Self {
            id,
            name: format!("Point light {}", id.0),
            kind: LampKind::Point,
            position: focus + Vec3::Z * Self::POINT_HEIGHT,
            direction: -Vec3::Z,
            color: [255, 255, 255],
            strength: Self::DEFAULT_STRENGTH,
            range: None,
            from_model: None,
        }
    }

    /// A spotlight at the camera, shining where it looks, like a torch.
    pub fn spot(id: LampId, camera: Vec3, forward: Vec3) -> Self {
        Self {
            id,
            name: format!("Spotlight {}", id.0),
            kind: LampKind::Spot {
                inner: 20.0,
                outer: 30.0,
            },
            position: camera,
            direction: forward.try_normalize().unwrap_or(-Vec3::Z),
            ..Self::point(id, camera)
        }
    }

    /// Moves it to where the camera is looking: a point light above the
    /// focus point, a spotlight to the camera, shining the same way.
    pub fn move_here(&mut self, camera: Vec3, forward: Vec3, focus: Vec3) {
        match self.kind {
            LampKind::Point => self.position = focus + Vec3::Z * Self::POINT_HEIGHT,
            LampKind::Spot { .. } => {
                self.position = camera;
                self.direction = forward.try_normalize().unwrap_or(-Vec3::Z);
            }
        }
    }

    /// A lamp read from a model file (linear colour, cone in radians).
    pub fn from_file(id: LampId, lamp: &LampFromFile, model: ModelId) -> Self {
        let kind = match lamp.cone {
            None => LampKind::Point,
            Some((inner, outer)) => LampKind::Spot {
                inner: inner.to_degrees(),
                outer: outer.to_degrees(),
            },
        };
        Self {
            id,
            name: if lamp.name.trim().is_empty() {
                format!("{} {}", kind.name(), id.0)
            } else {
                lamp.name.clone()
            },
            kind,
            position: lamp.position,
            direction: lamp.direction.try_normalize().unwrap_or(-Vec3::Z),
            color: lamp.color.map(linear_to_srgb_byte),
            strength: lamp.strength,
            range: lamp.range,
            from_model: Some(model),
        }
        .sanitized()
    }

    /// The lamp with anything out of range put back in range.
    pub fn sanitized(mut self) -> Self {
        let finite = |v: f32, fallback: f32| if v.is_finite() { v } else { fallback };
        if !self.position.is_finite() {
            self.position = Vec3::ZERO;
        }
        self.direction = self.direction.try_normalize().unwrap_or(-Vec3::Z);
        self.strength =
            finite(self.strength, Self::DEFAULT_STRENGTH).clamp(0.0, Self::MAX_STRENGTH);
        self.range = self.range.filter(|r| r.is_finite() && *r > 0.0);
        if let LampKind::Spot { inner, outer } = &mut self.kind {
            *outer = finite(*outer, 30.0).clamp(1.0, 89.0);
            *inner = finite(*inner, 0.0).clamp(0.0, *outer);
        }
        self
    }
}

/// A lamp as a model file describes it, handed over by the app.
#[derive(Debug, Clone, PartialEq)]
pub struct LampFromFile {
    /// Its name in the file.
    pub name: String,
    /// The spot's inner and outer angles in radians; `None` for a point
    /// light.
    pub cone: Option<(f32, f32)>,
    /// Where it is.
    pub position: Vec3,
    /// The way it shines.
    pub direction: Vec3,
    /// Its colour, linear.
    pub color: [f32; 3],
    /// Its strength in watts.
    pub strength: f32,
    /// How far it reaches.
    pub range: Option<f32>,
}

/// One linear colour channel as an sRGB byte.
fn linear_to_srgb_byte(c: f32) -> u8 {
    let c = if c.is_finite() {
        c.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let s = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round() as u8
}

/// What's selected in the Scene panel, shown in Properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Selection {
    /// Nothing.
    #[default]
    Nothing,
    /// The sun.
    Sun,
    /// A lamp.
    Lamp(LampId),
    /// An imported model.
    Model(ModelId),
}

/// A labelled row in Properties: the name on the left, the field after it.
fn row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.add_sized(
            egui::vec2(78.0, 18.0),
            egui::Label::new(RichText::new(label).color(crate::theme::TEXT_WEAK)),
        );
        add(ui)
    })
    .inner
}

/// The sun's fields in the Properties panel.
pub fn sun_properties(ui: &mut Ui, sun: &mut SunSettings) {
    row(ui, "Heading", |ui| {
        ui.add(
            egui::DragValue::new(&mut sun.heading)
                .range(0.0..=360.0)
                .speed(1.0)
                .suffix("°"),
        )
        .on_hover_text("Which way round the sky: 0° north (+Y), 90° east (+X), 180° south");
    });
    row(ui, "Height", |ui| {
        ui.add(
            egui::DragValue::new(&mut sun.height)
                .range(-10.0..=90.0)
                .speed(0.5)
                .suffix("°"),
        )
        .on_hover_text("How high above the horizon: low for long evening shadows, 90° at noon");
    });
    row(ui, "Colour", |ui| ui.color_edit_button_srgb(&mut sun.color));
    row(ui, "Strength", |ui| {
        ui.add(
            egui::DragValue::new(&mut sun.strength)
                .range(0.0..=SunSettings::MAX_STRENGTH)
                .speed(0.05)
                .suffix(" W/m²"),
        )
        .on_hover_text("Blender's unit: 3 is the editor's clear day, 0 turns the sun off");
    });
    row(ui, "Shadows", |ui| ui.checkbox(&mut sun.shadows, ""));
    if sun.heading >= 360.0 {
        sun.heading = 0.0;
    }
}

/// What the lamp fields asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LampRequest {
    /// Nothing.
    None,
    /// Move it to where the camera is looking.
    MoveHere,
    /// Delete it.
    Delete,
}

/// A lamp's fields in the Properties panel.
pub fn lamp_properties(ui: &mut Ui, lamp: &mut Lamp) -> LampRequest {
    let mut request = LampRequest::None;
    row(ui, "Name", |ui| ui.text_edit_singleline(&mut lamp.name));
    row(ui, "Kind", |ui| ui.label(lamp.kind.name()));
    row(ui, "Colour", |ui| {
        ui.color_edit_button_srgb(&mut lamp.color)
    });
    row(ui, "Strength", |ui| {
        ui.add(
            egui::DragValue::new(&mut lamp.strength)
                .range(0.0..=Lamp::MAX_STRENGTH)
                .speed(5.0)
                .suffix(" W"),
        )
        .on_hover_text("Watts, as in Blender: 1000 W lights a room");
    });
    row(ui, "Range", |ui| {
        let mut limited = lamp.range.is_some();
        ui.checkbox(&mut limited, "")
            .on_hover_text("Off: the light fades on forever, as in real life");
        let mut range = lamp.range.unwrap_or(10.0);
        if limited {
            ui.add(
                egui::DragValue::new(&mut range)
                    .range(0.1..=10_000.0)
                    .speed(0.1)
                    .suffix(" m"),
            );
            lamp.range = Some(range);
        } else {
            lamp.range = None;
        }
    });
    if let LampKind::Spot { inner, outer } = &mut lamp.kind {
        row(ui, "Cone", |ui| {
            ui.add(
                egui::DragValue::new(outer)
                    .range(1.0..=89.0)
                    .speed(0.5)
                    .suffix("°"),
            )
            .on_hover_text("How wide the light reaches, from the middle of the cone");
        });
        row(ui, "Soft edge", |ui| {
            let mut softness = *outer - *inner;
            ui.add(
                egui::DragValue::new(&mut softness)
                    .range(0.0..=*outer)
                    .speed(0.5)
                    .suffix("°"),
            )
            .on_hover_text("How gradually the light fades at the cone's edge");
            *inner = (*outer - softness).clamp(0.0, *outer);
        });
    }
    row(ui, "Position", |ui| {
        for value in [
            &mut lamp.position.x,
            &mut lamp.position.y,
            &mut lamp.position.z,
        ] {
            ui.add(egui::DragValue::new(value).speed(0.05).max_decimals(2));
        }
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let hint = match lamp.kind {
            LampKind::Point => "Moves it above the point the camera looks at",
            LampKind::Spot { .. } => "Moves it to the camera, shining where the camera looks",
        };
        if ui.button("Move here").on_hover_text(hint).clicked() {
            request = LampRequest::MoveHere;
        }
        if ui.button("Delete").clicked() {
            request = LampRequest::Delete;
        }
    });
    if lamp.from_model.is_some() {
        ui.add_space(4.0);
        ui.label(
            RichText::new("Came in with a model; removed along with it.")
                .color(crate::theme::TEXT_WEAK)
                .size(11.0),
        );
    }
    request
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_sun_shines_down_from_the_south_east() {
        let d = SunSettings::default().direction();
        assert!(d.z < -0.7, "downwards");
        assert!(d.x < 0.0 && d.y > 0.0, "from +X and -Y: {d}");
        assert!((d.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn a_sun_from_a_file_keeps_its_direction() {
        let d = Vec3::new(0.3, -0.4, -0.8).normalize();
        let sun = SunSettings::from_file(d, [1.0, 1.0, 1.0], 4.0, SunSettings::default());
        assert!(sun.direction().abs_diff_eq(d, 1e-4), "{}", sun.direction());
        assert_eq!(sun.color, [255, 255, 255]);
        assert_eq!(sun.strength, 4.0);
        let broken =
            SunSettings::from_file(Vec3::NAN, [f32::NAN; 3], f32::NAN, SunSettings::default());
        assert!(broken.direction().is_finite());
    }

    #[test]
    fn a_noon_sun_shines_straight_down() {
        let noon = SunSettings {
            height: 90.0,
            ..SunSettings::default()
        };
        assert!(noon.direction().abs_diff_eq(-Vec3::Z, 1e-5));
    }

    #[test]
    fn damaged_sun_settings_are_tidied() {
        let s = SunSettings {
            heading: -30.0,
            height: f32::NAN,
            color: [1, 2, 3],
            strength: f32::INFINITY,
            shadows: false,
        }
        .sanitized();
        assert_eq!(s.heading, 330.0);
        assert_eq!(s.height, SunSettings::default().height);
        assert_eq!(s.strength, SunSettings::default().strength);
        assert!(!s.shadows);
        let s = SunSettings {
            strength: 1e9,
            height: 400.0,
            ..s
        }
        .sanitized();
        assert_eq!(s.strength, SunSettings::MAX_STRENGTH);
        assert_eq!(s.height, 90.0);
    }

    #[test]
    fn sun_settings_round_trip_and_survive_rubbish() {
        let sun = SunSettings {
            heading: 12.0,
            height: 34.0,
            color: [9, 8, 7],
            strength: 5.5,
            shadows: false,
        };
        let text = ron::to_string(&sun).unwrap();
        assert_eq!(ron::from_str::<SunSettings>(&text).unwrap(), sun);
        // Missing fields take their defaults.
        assert_eq!(
            ron::from_str::<SunSettings>("(heading: 1.0)")
                .unwrap()
                .height,
            SunSettings::default().height
        );
        // Randomly damaged text: an answer or an error, never a crash.
        let mut seed: u64 = 11;
        for _ in 0..2_000 {
            let mut bytes = text.clone().into_bytes();
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let at = (seed >> 33) as usize % bytes.len();
            bytes[at] = (seed >> 13) as u8;
            if let Ok(s) = std::str::from_utf8(&bytes)
                && let Ok(sun) = ron::from_str::<SunSettings>(s)
            {
                let tidy = sun.sanitized();
                assert!(tidy.direction().is_finite());
            }
        }
    }

    #[test]
    fn new_lamps_go_where_the_camera_looks() {
        let focus = Vec3::new(3.0, 4.0, 0.0);
        let point = Lamp::point(LampId(1), focus);
        assert_eq!(point.position, focus + Vec3::Z * Lamp::POINT_HEIGHT);
        let camera = Vec3::new(0.0, -10.0, 5.0);
        let mut spot = Lamp::spot(LampId(2), camera, Vec3::new(0.0, 2.0, -1.0));
        assert_eq!(spot.position, camera);
        assert!((spot.direction.length() - 1.0).abs() < 1e-5);
        assert_eq!(spot.name, "Spotlight 2");
        spot.move_here(Vec3::ONE, Vec3::X, Vec3::ZERO);
        assert_eq!((spot.position, spot.direction), (Vec3::ONE, Vec3::X));
        let mut point = point;
        point.move_here(Vec3::ONE, Vec3::X, Vec3::new(1.0, 1.0, 0.0));
        assert_eq!(point.position, Vec3::new(1.0, 1.0, Lamp::POINT_HEIGHT));
    }

    #[test]
    fn lamps_from_files_are_converted_and_tidied() {
        let file = LampFromFile {
            name: String::new(),
            cone: Some((f32::NAN, 2.0)),
            position: Vec3::new(f32::NAN, 0.0, 0.0),
            direction: Vec3::ZERO,
            color: [1.0, 0.214, 0.0],
            strength: f32::NAN,
            range: Some(-3.0),
        };
        let lamp = Lamp::from_file(LampId(7), &file, ModelId(3));
        assert_eq!(lamp.name, "Spotlight 7");
        assert_eq!(lamp.position, Vec3::ZERO);
        assert_eq!(lamp.direction, -Vec3::Z);
        assert_eq!(lamp.color, [255, 127, 0], "linear to sRGB");
        assert_eq!(lamp.strength, Lamp::DEFAULT_STRENGTH);
        assert_eq!(lamp.range, None);
        assert_eq!(
            lamp.kind,
            LampKind::Spot {
                inner: 0.0,
                outer: 89.0
            }
        );
        assert_eq!(lamp.from_model, Some(ModelId(3)));
    }
}
