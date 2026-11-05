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
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Screen {
    /// The screen's name (`Hud`, from `ui/hud.lst`); empty shows nothing.
    pub screen: String,
    /// Whether it's shown.
    pub shown: bool,
    /// Its place among the screens shown: higher ones are drawn on top.
    pub order: i64,
}

impl Screen {
    /// The longest screen name kept.
    pub const MAX_NAME: usize = 128;
    /// The furthest order kept, either way.
    pub const MAX_ORDER: i64 = 1_000_000;

    /// A screen showing `name`.
    pub fn showing(name: &str) -> Self {
        Self {
            screen: name.into(),
            shown: true,
            order: 0,
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
        Self {
            screen,
            order: self.order.clamp(-Self::MAX_ORDER, Self::MAX_ORDER),
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
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("screen", Value::Text(self.screen.clone()));
        data.set("shown", Value::Bool(self.shown));
        data.set("order", Value::Int(self.order));
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
        };
        assert_eq!(Screen::from_data(&s.to_data()), s);
        let odd = Screen {
            screen: "ui/../hud".into(),
            shown: true,
            order: i64::MAX,
        }
        .sanitized();
        assert_eq!(odd.screen, "");
        assert_eq!(odd.order, Screen::MAX_ORDER);
        assert!(
            Screen::from_data(&ComponentData::default()).shown,
            "shown unless said"
        );
    }
}
