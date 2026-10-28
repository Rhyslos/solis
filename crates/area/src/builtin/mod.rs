//! The built-in components. Each is a typed Rust struct that turns into
//! values and back (`reflect::Component`); missing or damaged values take
//! their defaults, so an older or damaged area still loads.

mod audio_source;
mod camera;
mod environment;
mod light;
mod listener;
mod model;
mod physics;
mod reverb_zone;
mod transform;

pub use audio_source::AudioSource;
pub use camera::Camera;
pub use environment::{Environment, Sky, ToneMapping};
pub use light::{Light, LightKind, aim, sun_rotation};
pub use listener::Listener;
pub use model::Model;
pub use physics::{BodyKind, CharacterBody, Collider, ColliderShape, RigidBody};
pub use reverb_zone::{ReverbZone, ZoneShape};
pub use transform::Transform;

use crate::document::{Area, EntityData};
use reflect::{Registry, RegistryError};

/// A registry holding every built-in component.
pub fn registry() -> Result<Registry, RegistryError> {
    let mut registry = Registry::new();
    registry.register_type::<Transform>()?;
    registry.register_type::<Light>()?;
    registry.register_type::<Environment>()?;
    registry.register_type::<Model>()?;
    registry.register_type::<Camera>()?;
    registry.register_type::<RigidBody>()?;
    registry.register_type::<Collider>()?;
    registry.register_type::<CharacterBody>()?;
    registry.register_type::<AudioSource>()?;
    registry.register_type::<ReverbZone>()?;
    registry.register_type::<Listener>()?;
    Ok(registry)
}

/// A new area, as File > New area makes: a sun and an environment (the
/// sun, sky and fog live in each area).
pub fn starter_area() -> Area {
    let mut area = Area::new();
    area.spawn(
        EntityData::new("Sun")
            .with(&Transform {
                rotation: light::sun_rotation(150.0, 50.0),
                ..Transform::default()
            })
            .with(&Light::sun()),
    );
    area.spawn(EntityData::new("Environment").with(&Environment::default()));
    area
}

/// Field-building shorthand for the built-in schemas.
pub(crate) mod field {
    use reflect::{FieldInfo, FieldKind, Hint, Value};

    pub(crate) fn float(name: &str, label: &str, default: f32, hint: Hint) -> FieldInfo {
        FieldInfo::new(
            name,
            label,
            FieldKind::Float,
            Value::Float(f64::from(default)),
        )
        .hint(hint)
    }

    pub(crate) fn boolean(name: &str, label: &str, default: bool, help: &'static str) -> FieldInfo {
        FieldInfo::new(name, label, FieldKind::Bool, Value::Bool(default)).hint(Hint {
            help,
            ..Hint::default()
        })
    }

    pub(crate) fn choice(
        name: &str,
        label: &str,
        options: &[&str],
        default: &str,
        help: &'static str,
    ) -> FieldInfo {
        FieldInfo::new(
            name,
            label,
            FieldKind::Choice(options.iter().map(|o| (*o).to_string()).collect()),
            Value::Text(default.to_string()),
        )
        .hint(Hint {
            help,
            ..Hint::default()
        })
    }

    pub(crate) fn color(
        name: &str,
        label: &str,
        default: [f32; 3],
        help: &'static str,
    ) -> FieldInfo {
        FieldInfo::new(name, label, FieldKind::Color, Value::floats(&default)).hint(Hint {
            help,
            ..Hint::default()
        })
    }

    pub(crate) fn hint(
        range: (f64, f64),
        speed: f64,
        unit: &'static str,
        help: &'static str,
    ) -> Hint {
        Hint {
            range: Some(range),
            speed: Some(speed),
            unit,
            help,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflect::{Component, ComponentData, Value};

    /// Every field of every built-in schema has a starting value that fits.
    #[test]
    fn the_built_in_components_register() {
        let registry = registry().unwrap();
        let names: Vec<&str> = registry.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "AudioSource",
                "Camera",
                "CharacterBody",
                "Collider",
                "Environment",
                "Light",
                "Listener",
                "Model",
                "ReverbZone",
                "RigidBody",
                "Transform"
            ]
        );
    }

    fn round_trip<T: Component + PartialEq + std::fmt::Debug>(value: &T) {
        let data = value.to_data();
        let schema = T::schema();
        for field in &schema.fields {
            let v = data.get(&field.name).unwrap();
            assert!(field.kind.accepts(v), "{}.{} = {v:?}", T::NAME, field.name);
        }
        assert_eq!(&T::from_data(&data), value);
        // Every field written, and nothing else.
        assert_eq!(data.fields.len(), schema.fields.len());
    }

    #[test]
    fn every_built_in_round_trips_through_values() {
        round_trip(&Transform::default());
        round_trip(&Camera::default());
        round_trip(&AudioSource {
            event: "sfx/engine".into(),
            playing: false,
            volume_db: -3.0,
        });
        round_trip(&Light::sun());
        round_trip(&Light::point());
        round_trip(&Light::spot());
        round_trip(&Environment::default());
        round_trip(&Model::default());
        round_trip(&RigidBody::default());
        round_trip(&Collider::default());
        round_trip(&CharacterBody::default());
        round_trip(&RigidBody {
            kind: BodyKind::Kinematic,
            upright: true,
            ..RigidBody::default()
        });
        round_trip(&Collider {
            shape: ColliderShape::Capsule,
            ..Collider::default()
        });
        let env = Environment {
            sky: Sky::Daylight,
            fog: true,
            tone_mapping: ToneMapping::Aces,
            ..Environment::default()
        };
        round_trip(&env);
    }

    #[test]
    fn empty_or_damaged_data_gives_defaults() {
        let mut junk = ComponentData::default();
        junk.set("kind", Value::Text("Moon".into()));
        junk.set("strength", Value::Float(f64::NAN));
        junk.set("rotation", Value::Bool(true));
        assert_eq!(Light::from_data(&junk).kind, LightKind::Point);
        assert_eq!(Light::from_data(&junk).strength, Light::point().strength);
        assert_eq!(Transform::from_data(&junk), Transform::default());
        assert_eq!(
            Environment::from_data(&ComponentData::default()),
            Environment::default()
        );
    }

    #[test]
    fn a_new_area_has_a_sun_and_an_environment() {
        let area = starter_area();
        assert_eq!(area.len(), 2);
        let (_, sun) = area.with::<Light>().next().unwrap();
        assert_eq!(sun.kind, LightKind::Sun);
        assert_eq!(area.with::<Environment>().count(), 1);
    }
}
