//! A game screen shown (decision 0087): which of the project's screens
//! (`assets/ui/<name>.lst`), whether it's shown, and its order among
//! several. Game code shows and hides it by changing `shown`, so which
//! screens are open is part of the game: saved, replayed and the same on
//! every computer. The screen reads the entity it's on through the engine.

use reflect::{
    Component, ComponentData, ComponentSchema, FieldInfo, FieldKind, Fields, Hint, NameKind, Offer,
    Value,
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
    /// The room from one line of its text to the next, in percent of the
    /// font's own (100; stage 1, text layout).
    pub line_height: i64,
    /// Room added after each letter of its text, in design pixels (0).
    pub letter_spacing: f32,
    /// How many letters a second its text types out at (0: it shows at
    /// once; stage 1, text effects).
    pub typing_speed: f32,
    /// The sound event played as its text types out (its name as game
    /// code has it, `Sfx.Blip`; empty for none).
    pub typing_sound: String,
    /// Whether the typing sound plays on each letter or each word.
    pub sound_every_word: bool,
}

/// The choices of the Screen's "Sound on" setting.
const SOUND_ON: [&str; 2] = ["Each letter", "Each word"];

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
            line_height: 100,
            letter_spacing: 0.0,
            typing_speed: 0.0,
            typing_sound: String::new(),
            sound_every_word: false,
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
    /// The least and most line height, in percent.
    pub const LINE_HEIGHTS: (i64, i64) = (50, 300);
    /// The least and most letter spacing, in design pixels.
    pub const LETTER_SPACINGS: (f32, f32) = (-10.0, 50.0);
    /// The fastest text types out, in letters a second.
    pub const MAX_TYPING_SPEED: f32 = 1000.0;
    /// The longest typing sound's name kept.
    pub const MAX_SOUND_NAME: usize = 256;

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
            line_height: self
                .line_height
                .clamp(Self::LINE_HEIGHTS.0, Self::LINE_HEIGHTS.1),
            letter_spacing: if self.letter_spacing.is_finite() {
                self.letter_spacing
                    .clamp(Self::LETTER_SPACINGS.0, Self::LETTER_SPACINGS.1)
            } else {
                0.0
            },
            typing_speed: if self.typing_speed.is_finite() {
                self.typing_speed.clamp(0.0, Self::MAX_TYPING_SPEED)
            } else {
                0.0
            },
            typing_sound: if self.typing_sound.len() > Self::MAX_SOUND_NAME
                || self.typing_sound.contains(char::is_control)
            {
                String::new()
            } else {
                self.typing_sound
            },
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
                    help: "Which screen: a file in the Library's ui folder, by its name (ui/hud.lst is Hud).".into(),
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
                    help: "Its place among the screens shown: higher ones are drawn on top.".into(),
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
                               pixel-art look (1: smooth).".into(),
                        ..Hint::default()
                    },
                ),
                FieldInfo::new("lineHeight", "Line height", FieldKind::Int, Value::Int(100)).hint(
                    Hint {
                        range: Some((
                            Self::LINE_HEIGHTS.0 as f64,
                            Self::LINE_HEIGHTS.1 as f64,
                        )),
                        unit: "%".into(),
                        help: "The room from one line of the screen's text to the next, as a \
                               share of the font's own (100%). More spreads lines out; less \
                               packs them closer.".into(),
                        ..Hint::default()
                    },
                ),
                field::float(
                    "letterSpacing",
                    "Letter spacing",
                    0.0,
                    field::hint(
                        (
                            f64::from(Self::LETTER_SPACINGS.0),
                            f64::from(Self::LETTER_SPACINGS.1),
                        ),
                        0.1,
                        "px",
                        "Room added after each letter of the screen's text (on a 1920 × 1080 \
                         design), for wide titles or tight labels. 0 as the font spaces them.",
                    ),
                ),
                field::float(
                    "typingSpeed",
                    "Typing speed",
                    0.0,
                    Hint {
                        range: Some((0.0, f64::from(Self::MAX_TYPING_SPEED))),
                        speed: Some(1.0),
                        unit: "letters/s".into(),
                        help: "Its text types itself out, this many letters a second, each time \
                               the words change (30 to 60 suits dialogue). 0 shows it at once. \
                               Game code hears TypingFinished when it's done, and can call \
                               finishTyping to show it all."
                            .into(),
                        ..Hint::default()
                    },
                ),
                FieldInfo::new(
                    "typingSound",
                    "Typing sound",
                    FieldKind::Text,
                    Value::Text(String::new()),
                )
                .hint(Hint {
                    help: "A sound event played as the text types out, for a voice of blips. \
                           None for silence."
                        .into(),
                    offers: Offer::Names(NameKind::Sound),
                    ..Hint::default()
                }),
                field::choice(
                    "soundOn",
                    "Sound on",
                    &SOUND_ON,
                    SOUND_ON[0],
                    "Whether the typing sound plays for each letter or only for the first \
                     letter of each word (softer for fast text). At most once a tick.",
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
        data.set("lineHeight", Value::Int(self.line_height));
        data.set(
            "letterSpacing",
            Value::Float(f64::from(self.letter_spacing)),
        );
        data.set("typingSpeed", Value::Float(f64::from(self.typing_speed)));
        data.set("typingSound", Value::Text(self.typing_sound.clone()));
        data.set(
            "soundOn",
            Value::Text(SOUND_ON[usize::from(self.sound_every_word)].to_string()),
        );
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
            line_height: match data.get("lineHeight") {
                Some(Value::Int(n)) => *n,
                _ => 100,
            },
            letter_spacing: f.float("letterSpacing", 0.0),
            typing_speed: f.float("typingSpeed", 0.0),
            typing_sound: f.text("typingSound").unwrap_or_default().to_owned(),
            sound_every_word: f.text("soundOn") == Some(SOUND_ON[1]),
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
            line_height: 140,
            letter_spacing: 1.5,
            typing_speed: 40.0,
            typing_sound: "Sfx.Blip".into(),
            sound_every_word: true,
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
            line_height: 9_999,
            letter_spacing: f32::NAN,
            typing_speed: f32::INFINITY,
            typing_sound: "a\nb".into(),
            sound_every_word: false,
        }
        .sanitized();
        assert_eq!(odd.typing_speed, 0.0);
        assert_eq!(odd.typing_sound, "");
        assert_eq!(odd.screen, "");
        assert_eq!(odd.order, Screen::MAX_ORDER);
        assert_eq!(odd.glow, 0.0);
        assert_eq!(odd.outline, Screen::MAX_HALO);
        assert_eq!(odd.outline_color, [0.0, 0.0, 0.5]);
        assert_eq!(odd.pixel_size, 1);
        let old = Screen::from_data(&ComponentData::default());
        assert_eq!((old.glow, old.pixel_size), (0.0, 1), "older files: no look");
        assert_eq!(
            (
                old.typing_speed,
                old.typing_sound.as_str(),
                old.sound_every_word
            ),
            (0.0, "", false),
            "older files: shown at once, silent"
        );
        assert!(
            Screen::from_data(&ComponentData::default()).shown,
            "shown unless said"
        );
    }
}
