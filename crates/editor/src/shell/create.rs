//! The kinds of entity the Create group makes that the editor had no
//! one-step way to make before: each is an entity with a transform and
//! the kind's own component at its standard values, placed where the
//! camera looks and selected, as one undo step.

use crate::workbench::Workbench;
use area::EntityData;
use area::builtin::{ReverbZone, Sprite, Transform};
use reflect::EntityId;

impl Workbench {
    /// Adds an entity that draws a sprite sheet (none chosen yet: its
    /// sheet is picked in Properties), where the camera looks.
    pub fn add_sprite(&mut self) -> EntityId {
        let position = self.viewport.camera.focus();
        self.spawn_selected(
            EntityData::new("Sprite")
                .with(&Transform {
                    position,
                    ..Transform::default()
                })
                .with(&Sprite::default()),
        )
    }

    /// Adds a place where sounds echo, where the camera looks.
    pub fn add_reverb_zone(&mut self) -> EntityId {
        let position = self.viewport.camera.focus();
        self.spawn_selected(
            EntityData::new("Reverb zone")
                .with(&Transform {
                    position,
                    ..Transform::default()
                })
                .with(&ReverbZone::default()),
        )
    }
}
