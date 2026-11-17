//! An animator on an entity (Phase 9, D3b): an animator file (`.solanim`)
//! that chooses which clip the entity's Sprite plays, from parameters game
//! code sets.

use reflect::{
    AssetRef, Component, ComponentData, ComponentSchema, FieldInfo, FieldKind, Fields, Hint, Offer,
    Value,
};

use super::field;

/// A state machine choosing the clip of the Sprite on the same entity.
#[derive(Debug, Clone, PartialEq)]
pub struct Animator {
    /// The animator file (`.solanim`); `None` does nothing.
    pub animator: Option<AssetRef>,
    /// The state it's in, by name: empty for the animator's entry state.
    /// The running game keeps it up to date.
    pub state: String,
    /// How fast its states play: 1 as the animator says, 2 twice as fast.
    pub speed: f32,
}

impl Default for Animator {
    fn default() -> Self {
        Self {
            animator: None,
            state: String::new(),
            speed: 1.0,
        }
    }
}

impl Animator {
    /// The fastest an animator plays.
    pub const MAX_SPEED: f32 = 16.0;
    /// The longest state name kept.
    pub const MAX_STATE_NAME: usize = 64;
}

impl Component for Animator {
    const NAME: &'static str = "Animator";

    fn schema() -> ComponentSchema {
        let d = Self::default();
        ComponentSchema {
            name: Self::NAME.into(),
            help: "Chooses which clip the entity's Sprite plays: an animator file's states and \
                   the transitions between them, driven by values game code sets."
                .into(),
            fields: vec![
                FieldInfo::new("animator", "Animator", FieldKind::Asset, Value::Asset(None)).hint(
                    Hint {
                        help: "The animator file (.solanim) in the project.",
                        offers: Offer::Assets("animator"),
                        ..Hint::default()
                    },
                ),
                FieldInfo::new(
                    "state",
                    "State",
                    FieldKind::Text,
                    Value::Text(String::new()),
                )
                .hint(Hint {
                    help: "The state it starts in: empty for the animator's own start. \
                               While the game runs, the state it's in.",
                    offers: Offer::StatesOf("animator"),
                    ..Hint::default()
                }),
                field::float(
                    "speed",
                    "Speed",
                    d.speed,
                    field::hint(
                        (0.0, f64::from(Self::MAX_SPEED)),
                        0.01,
                        "×",
                        "How fast its states play: 1 as the animator says, 2 twice as fast.",
                    ),
                ),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("animator", Value::Asset(self.animator));
        data.set("state", Value::Text(self.state.clone()));
        data.set("speed", Value::Float(f64::from(self.speed)));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        let d = Self::default();
        let f = Fields(data);
        let speed = f.float("speed", d.speed);
        Self {
            animator: f.asset("animator"),
            state: f
                .text("state")
                .unwrap_or_default()
                .chars()
                .filter(|c| !c.is_control())
                .collect::<String>()
                .trim()
                .chars()
                .take(Self::MAX_STATE_NAME)
                .collect::<String>()
                .trim_end()
                .to_owned(),
            speed: if speed.is_finite() {
                speed.clamp(0.0, Self::MAX_SPEED)
            } else {
                d.speed
            },
        }
    }
}
