//! What a component's settings are when nobody has changed them: what
//! Properties' dot and **↺** compare a setting with and put back
//! (decision 0085), and what the card's Reset puts back.
//!
//! For most components that's the schema's own value for each field. A
//! component made set up for a job starts from that job's values: a
//! light from its kind's (a sun, a lamp, a spotlight), and the
//! Environment from a new level's (sprites lit, decision 0092). Without
//! that, a level's own Sun showed four changed settings that nobody had
//! changed, each with a way back to a lamp's value, and read as a
//! module's copy with changes of its own (Shared 1f's loose ends).

use level::builtin::{Environment, Light, LightKind};
use reflect::{Component, ComponentData, FieldInfo, Value};

/// The component named `component` as it starts, for an entity whose
/// component is `data`; `None` where that's the schema's values.
pub(crate) fn of(component: &str, data: &ComponentData) -> Option<ComponentData> {
    if component == Light::NAME {
        let standard = match Light::from_data(data).kind {
            LightKind::Sun => Light::sun(),
            LightKind::Point => Light::point(),
            LightKind::Spot => Light::spot(),
        };
        return Some(standard.to_data());
    }
    if component == Environment::NAME {
        return Some(level::builtin::starter_environment().to_data());
    }
    None
}

/// The standard value of `field` in `standard` (from [`of`]), or the
/// schema's.
pub(crate) fn value<'a>(standard: Option<&'a ComponentData>, field: &'a FieldInfo) -> &'a Value {
    standard
        .and_then(|s| s.get(&field.name))
        .unwrap_or(&field.default)
}

/// Whether `a` and `b` are the same value as the level keeps it: numbers
/// are kept to a float's precision in files (`0.98039216`), so a colour
/// read back from a file is still its standard one.
pub(crate) fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Float(x), Value::Float(y)) => (*x as f32) == (*y as f32),
        (Value::List(xs), Value::List(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| same(x, y))
        }
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn light(l: Light) -> ComponentData {
        l.to_data()
    }

    #[test]
    fn a_light_starts_as_its_kind() {
        for made in [Light::sun(), Light::point(), Light::spot()] {
            let standard = of(Light::NAME, &light(made)).unwrap();
            assert_eq!(standard, made.to_data());
        }
        // A sun turned up keeps a sun's standard, not a lamp's.
        let bright = Light {
            strength: 9.0,
            ..Light::sun()
        };
        let standard = of(Light::NAME, &light(bright)).unwrap();
        assert_eq!(standard.get("strength"), Some(&Value::Float(3.0)));
    }

    #[test]
    fn the_environment_starts_as_a_new_level_s() {
        let data = Environment::default().to_data();
        let standard = of(Environment::NAME, &data).unwrap();
        assert_eq!(standard.get("light_sprites"), Some(&Value::Bool(true)));
        assert_eq!(of("Transform", &ComponentData::default()), None);
    }

    #[test]
    fn numbers_read_back_from_a_file_are_the_same() {
        let made = Value::floats(&[1.0, 250.0 / 255.0, 242.0 / 255.0]);
        let read = Value::List(
            [1.0, 0.98039216, 0.9490196]
                .into_iter()
                .map(Value::Float)
                .collect(),
        );
        assert!(same(&made, &read));
        assert!(!same(&Value::Float(3.0), &Value::Float(3.001)));
        assert!(!same(&made, &Value::List(Vec::new())));
        assert!(same(&Value::Bool(true), &Value::Bool(true)));
        assert!(!same(
            &Value::Text("Sun".into()),
            &Value::Text("Point".into())
        ));
    }
}
