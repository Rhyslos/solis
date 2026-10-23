//! Where an entity is: position, rotation and scale, relative to its
//! parent (or the world). Z is up, units are metres.

use glam::{Mat4, Quat, Vec3};
use reflect::{
    Component, ComponentData, ComponentSchema, FieldInfo, FieldKind, Fields, Hint, Value,
};

/// Where an entity is, relative to its parent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// Position, in metres.
    pub position: Vec3,
    /// Rotation.
    pub rotation: Quat,
    /// Size along each axis (1 as made).
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    /// The transform as a matrix.
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    /// The way the entity faces: its own -Z (lights shine this way).
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::NEG_Z
    }
}

impl Component for Transform {
    const NAME: &'static str = "Transform";

    fn schema() -> ComponentSchema {
        ComponentSchema {
            name: Self::NAME.into(),
            help: "Where the entity is: position, rotation and scale, relative to its parent."
                .into(),
            fields: vec![
                FieldInfo::new(
                    "position",
                    "Position",
                    FieldKind::Vec3,
                    Value::vec3(Vec3::ZERO),
                )
                .hint(Hint {
                    speed: Some(0.05),
                    unit: " m",
                    help: "Where it is, in metres (Z is up).",
                    ..Hint::default()
                }),
                FieldInfo::new(
                    "rotation",
                    "Rotation",
                    FieldKind::Rotation,
                    Value::quat(Quat::IDENTITY),
                )
                .hint(Hint {
                    speed: Some(1.0),
                    unit: "°",
                    help: "How it's turned, in degrees around X, Y and Z.",
                    ..Hint::default()
                }),
                FieldInfo::new("scale", "Scale", FieldKind::Vec3, Value::vec3(Vec3::ONE)).hint(
                    Hint {
                        speed: Some(0.01),
                        help: "Its size along each axis: 1 as made.",
                        ..Hint::default()
                    },
                ),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("position", Value::vec3(self.position));
        data.set("rotation", Value::quat(self.rotation));
        data.set("scale", Value::vec3(self.scale));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        let f = Fields(data);
        let d = Self::default();
        Self {
            position: f.vec3("position", d.position),
            rotation: f.rotation("rotation", d.rotation),
            scale: f.vec3("scale", d.scale),
        }
    }
}
