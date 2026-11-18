//! The running game's state written out part by part, and its checksums,
//! for tools and tests that run the game without a window (stage 1, the
//! same game on two PCs): the world's parts ([`world::StateParts`]) with
//! each running component's values and each game-wide object's.

use crate::workbench::Workbench;
use world::StateParts;
use world::state_parts::data_text;

impl Workbench {
    /// The running game's checksums now: the world's, and game code's
    /// (every running component's values and the game-wide objects'), as
    /// a recording keeps them. `None` when no game runs.
    pub fn game_checksums(&mut self) -> Option<(u64, u64)> {
        let playing = self.open.playing.as_mut()?;
        let world = playing.world.checksum();
        let game = playing.game.as_mut().map_or(0, script::Game::checksum);
        Some((world, game))
    }

    /// The running game's state, part by part: the world's, then each
    /// running component's values (`<ID> "<name>" <Component> (running)`)
    /// and each game-wide object's (`object <Name>`). It asks every
    /// component, so it's for tests and tools, not every frame. `None`
    /// when no game runs.
    pub fn game_state(&mut self) -> Option<StateParts> {
        let playing = self.open.playing.as_mut()?;
        let mut parts = playing.world.state_parts();
        let Some(game) = playing.game.as_mut() else {
            return Some(parts);
        };
        let names: std::collections::HashMap<reflect::EntityId, String> =
            playing.world.entities().into_iter().flatten().collect();
        for (entity, component, data) in game.all_values() {
            let name = names.get(&entity).map(String::as_str).unwrap_or_default();
            parts.insert(
                format!("{entity} {name:?} {component} (running)"),
                data_text(&data),
            );
        }
        for (object, data) in game.object_values() {
            parts.insert(format!("object {object}"), data_text(&data));
        }
        Some(parts)
    }
}

/// What other crates' tests need of a running game (the `test-support`
/// feature, only ever turned on by a dev-dependency).
#[cfg(feature = "test-support")]
impl Workbench {
    /// Plays this workbench's games unhurried: game code's and physics'
    /// time limits are kept on the real clock, so on a busy machine a tick
    /// of one run is cut short and the game pauses, and a test comparing it
    /// with another run fails though nothing is wrong. For tests that
    /// compare runs only.
    pub fn play_unhurried(&mut self) {
        self.unhurried = crate::game_code::Unhurried::Everything;
    }

    /// Moves `id` by `by` in the running game, as a deliberately broken
    /// rule would: for tests that check a difference is found and named.
    /// Whether it was there to move.
    pub fn nudge(&mut self, id: reflect::EntityId, by: glam::Vec3) -> bool {
        let Some(playing) = self.open.playing.as_mut() else {
            return false;
        };
        playing.world.change_transform(id, |_, mut t| {
            t.position += by;
            Some(t)
        })
    }
}

#[cfg(test)]
#[path = "state_dump_tests.rs"]
mod tests;
