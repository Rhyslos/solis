//! A sprite (Phase 9, D2): one frame of a sprite sheet (an Aseprite
//! file), lying on the ground at the entity, its pivot (where it stands)
//! on the entity's place.

use reflect::{
    AssetRef, Component, ComponentData, ComponentSchema, FieldInfo, FieldKind, Fields, Hint, Offer,
    Value,
};

use super::field;

/// One frame of a sprite sheet, drawn at the entity.
#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    /// The sprite sheet; `None` draws nothing.
    pub sheet: Option<AssetRef>,
    /// Which frame, counting from 1 as Aseprite does.
    pub frame: i64,
    /// The slice it stands on (its pivot), by name; empty for the bottom
    /// middle of the frame.
    pub pivot: String,
    /// Mirrored left to right.
    pub flip_x: bool,
    /// Mirrored top to bottom.
    pub flip_y: bool,
    /// Multiplied into its colours, sRGB 0 to 1: white leaves it as drawn.
    pub tint: [f32; 3],
    /// How white it flashes, 0 to 1 (a hit).
    pub flash: f32,
    /// Whether it has a one-pixel outline.
    pub outline: bool,
    /// The outline's colour, sRGB 0 to 1.
    pub outline_color: [f32; 3],
    /// Whether it's see-through (glass, smoke): drawn blended, after the
    /// rest, sorted back to front. Otherwise every pixel is either drawn
    /// or not, which needs no sorting.
    pub see_through: bool,
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            sheet: None,
            frame: 1,
            pivot: String::new(),
            flip_x: false,
            flip_y: false,
            tint: [1.0; 3],
            flash: 0.0,
            outline: false,
            outline_color: [0.0; 3],
            see_through: false,
        }
    }
}

impl Sprite {
    /// The most frames a sprite counts to.
    pub const MAX_FRAME: i64 = 65_536;

    /// Its frame from 0, for a sheet of `frames` frames (the first when
    /// it's out of range).
    pub fn frame_index(&self, frames: usize) -> usize {
        usize::try_from(self.frame.saturating_sub(1))
            .ok()
            .filter(|&f| f < frames)
            .unwrap_or(0)
    }
}

impl Component for Sprite {
    const NAME: &'static str = "Sprite";

    fn schema() -> ComponentSchema {
        let d = Self::default();
        ComponentSchema {
            name: Self::NAME.into(),
            help: "Draws a frame of a sprite sheet (an Aseprite file) lying on the ground at \
                   the entity, standing on its pivot. Lower on the screen is drawn in front."
                .into(),
            fields: vec![
                FieldInfo::new("sheet", "Sheet", FieldKind::Asset, Value::Asset(None)).hint(Hint {
                    help: "The sprite sheet: an Aseprite file in the project.",
                    offers: Offer::Assets("sprite sheet"),
                    ..Hint::default()
                }),
                FieldInfo::new("frame", "Frame", FieldKind::Int, Value::Int(d.frame)).hint(Hint {
                    range: Some((1.0, Self::MAX_FRAME as f64)),
                    speed: Some(0.1),
                    help: "Which frame it shows, counting from 1 as Aseprite does.",
                    ..Hint::default()
                }),
                FieldInfo::new(
                    "pivot",
                    "Pivot",
                    FieldKind::Text,
                    Value::Text(d.pivot.clone()),
                )
                .hint(Hint {
                    help: "The slice it stands on, such as feet: its pivot (or its middle) \
                               sits on the entity's place. Empty: the bottom middle of the frame.",
                    offers: Offer::SlicesOf("sheet"),
                    ..Hint::default()
                }),
                field::boolean("flip_x", "Flip across", d.flip_x, "Mirrored left to right."),
                field::boolean("flip_y", "Flip up", d.flip_y, "Mirrored top to bottom."),
                field::color(
                    "tint",
                    "Tint",
                    d.tint,
                    "Multiplied into its colours: white leaves it as drawn.",
                ),
                field::float(
                    "flash",
                    "Flash",
                    d.flash,
                    field::hint(
                        (0.0, 1.0),
                        0.01,
                        "",
                        "How white it flashes, 0 to 1: a hit, for a moment.",
                    ),
                ),
                field::boolean(
                    "outline",
                    "Outline",
                    d.outline,
                    "A one-pixel outline round what's drawn.",
                ),
                field::color(
                    "outline_color",
                    "Outline colour",
                    d.outline_color,
                    "The outline's colour.",
                ),
                field::boolean(
                    "see_through",
                    "See-through",
                    d.see_through,
                    "For glass, smoke and the like: drawn blended, sorted back to front. \
                     Leave it off for ordinary pixel art, which needs no sorting.",
                ),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("sheet", Value::Asset(self.sheet));
        data.set("frame", Value::Int(self.frame));
        data.set("pivot", Value::Text(self.pivot.clone()));
        data.set("flip_x", Value::Bool(self.flip_x));
        data.set("flip_y", Value::Bool(self.flip_y));
        data.set("tint", Value::floats(&self.tint));
        data.set("flash", Value::Float(f64::from(self.flash)));
        data.set("outline", Value::Bool(self.outline));
        data.set("outline_color", Value::floats(&self.outline_color));
        data.set("see_through", Value::Bool(self.see_through));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        let d = Self::default();
        let f = Fields(data);
        let colour = |name: &str, default: [f32; 3]| {
            data.get(name)
                .and_then(Value::as_floats::<3>)
                .filter(|c| c.iter().all(|v| v.is_finite()))
                .map_or(default, |c| c.map(|v| v.clamp(0.0, 1.0)))
        };
        let flash = f.float("flash", d.flash);
        Self {
            sheet: f.asset("sheet"),
            frame: f.int("frame", d.frame).clamp(1, Self::MAX_FRAME),
            pivot: f.text("pivot").unwrap_or_default().to_owned(),
            flip_x: f.bool("flip_x", d.flip_x),
            flip_y: f.bool("flip_y", d.flip_y),
            tint: colour("tint", d.tint),
            flash: if flash.is_finite() {
                flash.clamp(0.0, 1.0)
            } else {
                0.0
            },
            outline: f.bool("outline", d.outline),
            outline_color: colour("outline_color", d.outline_color),
            see_through: f.bool("see_through", d.see_through),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sprite_round_trips_and_damaged_values_are_made_sensible() {
        let s = Sprite {
            sheet: Some(AssetRef {
                asset: reflect::AssetId::random(),
                part: None,
            }),
            frame: 7,
            pivot: "feet".into(),
            flip_x: true,
            tint: [0.5, 1.0, 0.25],
            flash: 0.5,
            outline: true,
            see_through: true,
            ..Sprite::default()
        };
        assert_eq!(Sprite::from_data(&s.to_data()), s);
        let mut odd = s.to_data();
        odd.set("frame", Value::Int(-4));
        odd.set("flash", Value::Float(f64::NAN));
        odd.set("tint", Value::floats(&[9.0, -1.0, f32::INFINITY]));
        let read = Sprite::from_data(&odd);
        assert_eq!(read.frame, 1);
        assert_eq!(read.flash, 0.0);
        assert_eq!(read.tint, [1.0; 3], "a broken colour is white");
        let mut odd = s.to_data();
        odd.set("tint", Value::floats(&[9.0, -1.0, 0.5]));
        assert_eq!(Sprite::from_data(&odd).tint, [1.0, 0.0, 0.5]);
        // Out of a sheet's range: its first frame.
        assert_eq!(
            Sprite {
                frame: 3,
                ..Sprite::default()
            }
            .frame_index(10),
            2
        );
        assert_eq!(
            Sprite {
                frame: 30,
                ..Sprite::default()
            }
            .frame_index(10),
            0
        );
        assert_eq!(Sprite::default().frame_index(0), 0);
    }
}
