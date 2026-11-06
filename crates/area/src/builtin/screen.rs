//! A game screen shown (decision 0087): which of the project's screens
//! (`assets/ui/<name>.lst`), whether it's shown, and its order among
//! several. Game code shows and hides it by changing `shown`, so which
//! screens are open is part of the game: saved, replayed and the same on
//! every computer. The screen reads the entity it's on through the engine.

use reflect::{
    Component, ComponentData, ComponentSchema, FieldInfo, FieldKind, Fields, Hint, Offer, Value,
};

use super::field;

/// A game screen shown for an entity.
#[derive(Debug, Clone, PartialEq)]
pub struct Screen {
    /// The screen's name (`Hud`, from `ui/hud.lst`); empty shows nothing.
    pub screen: String,
    /// Whether it's shown.
    pub shown: bool,
    /// Its place among the screens shown: higher ones are drawn on top.
    pub order: i64,
    /// Its look (decision 0089, M6): a glow round everything on it, in
    /// each piece's own colour, this many design pixels out (0 for none).
    pub glow: f32,
    /// An outline just outside everything on it, this thick in design
    /// pixels (0 for none).
    pub outline: f32,
    /// The outline's colour (sRGB, 0 to 1).
    pub outline_color: [f32; 3],
    /// The screen drawn in blocks this many design pixels big, for a
    /// pixel-art look (1: smooth).
    pub pixel_size: i64,
}

impl Default for Screen {
    fn default() -> Self {
        Self {
            screen: String::new(),
            shown: false,
            order: 0,
            glow: 0.0,
            outline: 0.0,
            outline_color: [0.0, 0.0, 0.0],
            pixel_size: 1,
        }
    }
}

impl Screen {
    /// The longest screen name kept.
    pub const MAX_NAME: usize = 128;
    /// The furthest order kept, either way.
    pub const MAX_ORDER: i64 = 1_000_000;
    /// The furthest a glow or an outline reaches, in design pixels.
    pub const MAX_HALO: f32 = 64.0;
    /// The biggest pixel size.
    pub const MAX_PIXEL: i64 = 32;

    /// A screen showing `name`.
    pub fn showing(name: &str) -> Self {
        Self {
            screen: name.into(),
            shown: true,
            ..Self::default()
        }
    }

    /// The same with anything unusable put right.
    #[must_use]
    pub fn sanitized(self) -> Self {
        let screen = if self.screen.len() > Self::MAX_NAME
            || !self.screen.chars().all(|c| c.is_ascii_alphanumeric())
        {
            String::new()
        } else {
            self.screen
        };
        let halo = |v: f32| {
            if v.is_finite() {
                v.clamp(0.0, Self::MAX_HALO)
            } else {
                0.0
            }
        };
        Self {
            screen,
            order: self.order.clamp(-Self::MAX_ORDER, Self::MAX_ORDER),
            glow: halo(self.glow),
            outline: halo(self.outline),
            outline_color: self.outline_color.map(|c| {
                if c.is_finite() {
                    c.clamp(0.0, 1.0)
                } else {
                    0.0
                }
            }),
            pixel_size: self.pixel_size.clamp(1, Self::MAX_PIXEL),
            ..self
        }
    }
}

impl Component for Screen {
    const NAME: &'static str = "Screen";

    fn schema() -> ComponentSchema {
        ComponentSchema {
            name: Self::NAME.into(),
            help: "Shows one of the game's screens (a HUD, a menu) over the game while it plays. \
                   A screen is a Lust file in the ui folder, and it can read this entity."
                .into(),
            fields: vec![
                FieldInfo::new(
                    "screen",
                    "Screen",
                    FieldKind::Text,
                    Value::Text(String::new()),
                )
                .hint(Hint {
                    help: "Which screen: a file in assets/ui, by its name (ui/hud.lst is Hud).",
                    offers: Offer::Screens,
                    ..Hint::default()
                }),
                field::boolean(
                    "shown",
                    "Shown",
                    true,
                    "Whether it's on screen. Game code shows and hides it by changing this.",
                ),
                FieldInfo::new("order", "Order", FieldKind::Int, Value::Int(0)).hint(Hint {
                    help: "Its place among the screens shown: higher ones are drawn on top.",
                    ..Hint::default()
                }),
                field::float(
                    "glow",
                    "Glow",
                    0.0,
                    field::hint(
                        (0.0, f64::from(Self::MAX_HALO)),
                        0.2,
                        "px",
                        "A soft light round everything on the screen, in each piece's own colour, \
                         this many pixels out (on a 1920 × 1080 design). 0 for none.",
                    ),
                ),
                field::float(
                    "outline",
                    "Outline",
                    0.0,
                    field::hint(
                        (0.0, f64::from(Self::MAX_HALO)),
                        0.1,
                        "px",
                        "A line just outside everything on the screen, this thick. 0 for none.",
                    ),
                ),
                field::color(
                    "outlineColor",
                    "Outline colour",
                    [0.0, 0.0, 0.0],
                    "The outline's colour.",
                ),
                FieldInfo::new("pixelSize", "Pixel size", FieldKind::Int, Value::Int(1)).hint(
                    Hint {
                        range: Some((1.0, Self::MAX_PIXEL as f64)),
                        help: "Draws the whole screen in blocks this many pixels big, for a \
                               pixel-art look (1: smooth).",
                        ..Hint::default()
                    },
                ),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("screen", Value::Text(self.screen.clone()));
        data.set("shown", Value::Bool(self.shown));
        data.set("order", Value::Int(self.order));
        data.set("glow", Value::Float(f64::from(self.glow)));
        data.set("outline", Value::Float(f64::from(self.outline)));
        data.set("outlineColor", Value::floats(&self.outline_color));
        data.set("pixelSize", Value::Int(self.pixel_size));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        let f = Fields(data);
        Self {
            screen: f.text("screen").unwrap_or_default().to_owned(),
            shown: f.bool("shown", true),
            order: match data.get("order") {
                Some(Value::Int(n)) => *n,
                _ => 0,
            },
            glow: f.float("glow", 0.0),
            outline: f.float("outline", 0.0),
            outline_color: f.color("outlineColor", [0.0, 0.0, 0.0]),
            pixel_size: match data.get("pixelSize") {
                Some(Value::Int(n)) => *n,
                _ => 1,
            },
        }
        .sanitized()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_screen_round_trips_and_odd_values_are_put_right() {
        let s = Screen {
            screen: "Hud".into(),
            shown: false,
            order: 3,
            glow: 6.5,
            outline: 2.0,
            outline_color: [1.0, 0.5, 0.0],
            pixel_size: 3,
        };
        assert_eq!(Screen::from_data(&s.to_data()), s);
        let odd = Screen {
            screen: "ui/../hud".into(),
            shown: true,
            order: i64::MAX,
            glow: f32::NAN,
            outline: 1e9,
            outline_color: [f32::INFINITY, -1.0, 0.5],
            pixel_size: -4,
        }
        .sanitized();
        assert_eq!(odd.screen, "");
        assert_eq!(odd.order, Screen::MAX_ORDER);
        assert_eq!(odd.glow, 0.0);
        assert_eq!(odd.outline, Screen::MAX_HALO);
        assert_eq!(odd.outline_color, [0.0, 0.0, 0.5]);
        assert_eq!(odd.pixel_size, 1);
        let old = Screen::from_data(&ComponentData::default());
        assert_eq!((old.glow, old.pixel_size), (0.0, 1), "older files: no look");
        assert!(
            Screen::from_data(&ComponentData::default()).shown,
            "shown unless said"
        );
    }
}
