//! The kinds of entity the Create group makes that the editor had no
//! one-step way to make before: each is an entity with a transform and
//! the kind's own component at its standard values, placed where the
//! camera looks and selected, as one undo step.

use crate::workbench::Workbench;
use area::EntityData;
use area::builtin::{Brush, ReverbZone, Sprite, Transform};
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

    /// Adds a solid box brush (Tools 1a, B1) standing where the camera
    /// looks, named as the brush list names them (`Box_01`, `Box_02`…).
    /// Until the brush list arrives (B2), this is how a brush is made.
    pub fn add_box_brush(&mut self) -> EntityId {
        let position = self.viewport.camera.focus();
        let boxes = self
            .open
            .document
            .area()
            .iter()
            .filter(|(_, e)| {
                e.components
                    .get(<Brush as reflect::Component>::NAME)
                    .and_then(|c| c.get("shape"))
                    == Some(&reflect::Value::Text("Box".into()))
            })
            .count();
        self.spawn_selected(
            EntityData::new(&format!("Box_{:02}", boxes + 1))
                .with(&Transform {
                    position,
                    ..Transform::default()
                })
                .with(&Brush::default()),
        )
    }
}
