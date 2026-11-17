//! A sound that keeps playing at an entity (Phase 7, S5): an engine's hum,
//! a campfire, a radio. It names a [sound event](../../../../docs/manual/sound-events.md)
//! and plays it over and over, following its entity, from Play (or when
//! game code spawns the entity) until the entity goes or game code stops
//! it (`setSoundPlaying`).

use reflect::{
    Component, ComponentData, ComponentSchema, FieldInfo, FieldKind, Fields, Hint, Value,
};

use super::field;

/// A sound playing over and over at the entity.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioSource {
    /// The sound event, by name: its file in assets without `.solsound`
    /// (`"sfx/engine"`).
    pub event: String,
    /// Whether it's playing.
    pub playing: bool,
    /// How loud, in decibels, on top of the event's own volume.
    pub volume_db: f32,
}

impl Default for AudioSource {
    fn default() -> Self {
        Self {
            event: String::new(),
            playing: true,
            volume_db: 0.0,
        }
    }
}

impl AudioSource {
    /// The quietest and loudest it may be set, in decibels.
    pub const VOLUME_RANGE_DB: (f32, f32) = (-60.0, 12.0);
    /// The longest event name kept.
    pub const MAX_EVENT_NAME: usize = 256;

    /// The same with anything unusable put right: a volume in range, and
    /// an event name that's a sensible length with nothing unprintable.
    #[must_use]
    pub fn sanitized(self) -> Self {
        let volume_db = if self.volume_db.is_finite() {
            self.volume_db
                .clamp(Self::VOLUME_RANGE_DB.0, Self::VOLUME_RANGE_DB.1)
        } else {
            0.0
        };
        let event =
            if self.event.len() > Self::MAX_EVENT_NAME || self.event.contains(char::is_control) {
                String::new()
            } else {
                self.event.trim().to_owned()
            };
        Self {
            event,
            playing: self.playing,
            volume_db,
        }
    }
}

impl Component for AudioSource {
    const NAME: &'static str = "AudioSource";

    fn schema() -> ComponentSchema {
        let d = Self::default();
        ComponentSchema {
            name: Self::NAME.into(),
            help: "A sound that keeps playing at the entity: an engine, a campfire, a radio. \
                   It plays its sound event over and over, following the entity, while the \
                   game runs."
                .into(),
            fields: vec![
                FieldInfo::new(
                    "event",
                    "Sound event",
                    FieldKind::Text,
                    Value::Text(String::new()),
                )
                .hint(Hint {
                    help: "The sound event to play, by its file in assets without \
                               .solsound: \"sfx/engine\".",
                    ..Hint::default()
                }),
                field::boolean(
                    "playing",
                    "Playing",
                    d.playing,
                    "Whether it plays. Game code turns it on and off with setSoundPlaying.",
                ),
                field::float(
                    "volume",
                    "Volume",
                    d.volume_db,
                    field::hint(
                        (
                            f64::from(Self::VOLUME_RANGE_DB.0),
                            f64::from(Self::VOLUME_RANGE_DB.1),
                        ),
                        0.1,
                        " dB",
                        "How loud, on top of the sound event's own volume: -6 dB is about half.",
                    ),
                ),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("event", Value::Text(self.event.clone()));
        data.set("playing", Value::Bool(self.playing));
        data.set("volume", Value::Float(f64::from(self.volume_db)));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        let d = Self::default();
        let f = Fields(data);
        Self {
            event: f.text("event").unwrap_or_default().to_owned(),
            playing: f.bool("playing", d.playing),
            volume_db: f.float("volume", d.volume_db),
        }
        .sanitized()
    }
}
