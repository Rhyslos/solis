//! The screens entities show and the sounds they play, as the running game
//! sets them.

use crate::{Id, Values, World, game_set};
use area::builtin::AudioSource;
use reflect::EntityId;

impl World {
    /// Every entity's [`AudioSource`], by ID (Phase 7, S5). Only the
    /// entities that have one are looked at, so a big world without
    /// sounds costs nothing here.
    pub fn audio_sources(&self) -> Vec<(EntityId, AudioSource)> {
        let mut sources: Vec<(EntityId, AudioSource)> = self
            .ecs
            .query::<(&Id, &AudioSource)>()
            .iter()
            .map(|(id, source)| (id.0, source.clone()))
            .collect();
        sources.sort_unstable_by_key(|(id, _)| *id);
        sources
    }

    /// The [`AudioSource`]s that are playing and name an event, by entity
    /// in id order: what the app keeps sounding each frame. Only these are
    /// copied, so a world full of silent sources costs little.
    pub fn sounding_audio_sources(&self) -> Vec<(EntityId, AudioSource)> {
        let mut sources: Vec<(EntityId, AudioSource)> = self
            .ecs
            .query::<(&Id, &AudioSource)>()
            .iter()
            .filter(|(_, source)| source.playing && !source.event.is_empty())
            .map(|(id, source)| (id.0, source.clone()))
            .collect();
        sources.sort_unstable_by_key(|(id, _)| *id);
        sources
    }

    /// Turns an entity's [`AudioSource`] on or off (game code's
    /// `setSoundPlaying`). Nothing happens for an entity without one.
    pub fn set_sound_playing(&mut self, id: EntityId, playing: bool) {
        if let Some(&entity) = self.by_id.get(&id)
            && let Ok(mut source) = self.ecs.get::<&mut AudioSource>(entity)
            && source.playing != playing
        {
            source.playing = playing;
            self.touched = true;
        }
    }

    /// Shows or hides a screen for an entity (game code's `showScreen` and
    /// `hideScreen`, decision 0087). Showing sets the entity's Screen to
    /// `screen` and shows it, giving it a Screen if it has none; hiding
    /// hides it only if it shows `screen`. Nothing happens for an entity
    /// not in the world, or a name a Screen can't hold.
    pub fn set_screen(&mut self, id: EntityId, screen: &str, shown: bool) {
        use area::builtin::Screen;
        use reflect::Component;
        let Some(&entity) = self.by_id.get(&id) else {
            return;
        };
        let now = self
            .ecs
            .get::<&Values>(entity)
            .ok()
            .and_then(|v| v.0.get(Screen::NAME).map(Screen::from_data));
        let wanted = match (&now, shown) {
            (Some(now), true) => Screen {
                screen: screen.to_string(),
                shown: true,
                ..now.clone()
            },
            (None, true) => Screen::showing(screen),
            (Some(now), false) if now.screen.eq_ignore_ascii_case(screen) => Screen {
                shown: false,
                ..now.clone()
            },
            _ => return,
        }
        .sanitized();
        if wanted.screen.is_empty() || now.as_ref() == Some(&wanted) {
            return;
        }
        // Kept over edits while playing like the rest game code sets on
        // an entity (see `game_set`).
        game_set::put_screen(&mut self.ecs, entity, &wanted.screen, wanted.shown);
        // The Screen is among the values the structure's sum holds.
        self.structure = std::sync::OnceLock::new();
        self.touched = true;
    }

    /// The screen an entity's Screen shows, if it has one and it's shown.
    pub fn screen_shown(&self, id: EntityId) -> Option<String> {
        use area::builtin::Screen;
        use reflect::Component;
        let entity = *self.by_id.get(&id)?;
        let values = self.ecs.get::<&Values>(entity).ok()?;
        let screen = Screen::from_data(values.0.get(Screen::NAME)?);
        (screen.shown && !screen.screen.is_empty()).then_some(screen.screen)
    }

    /// The screen each entity's Screen shows, for those shown.
    pub fn screens_shown(&self) -> std::collections::HashMap<EntityId, String> {
        use area::builtin::Screen;
        use reflect::Component;
        self.values_named(Screen::NAME)
            .into_iter()
            .map(|(id, data)| (id, Screen::from_data(&data)))
            .filter(|(_, s)| s.shown && !s.screen.is_empty())
            .map(|(id, s)| (id, s.screen))
            .collect()
    }
}
