//! The registry: every component Solis knows, by name.

use crate::component::Component;
use crate::schema::{ComponentSchema, FieldKind};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

/// Why a component couldn't be registered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// Another component already has this name.
    Duplicate(String),
    /// The name is empty, or has characters other than letters, digits
    /// and `_`.
    BadName(String),
    /// A field's starting value doesn't fit its kind, or two fields share
    /// a name.
    BadField {
        /// The component.
        component: String,
        /// The field.
        field: String,
    },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Duplicate(name) => write!(f, "there are two components called {name}"),
            Self::BadName(name) => write!(
                f,
                "\"{name}\" can't be a component's name: use letters, digits and _"
            ),
            Self::BadField { component, field } => write!(
                f,
                "{component}.{field} is defined twice, or its starting value doesn't fit it"
            ),
        }
    }
}

impl std::error::Error for RegistryError {}

/// Every component Solis knows, by name: built-in ones, and later the
/// game's own (Lust, Rust, Lua).
#[derive(Debug, Clone, Default)]
pub struct Registry {
    schemas: BTreeMap<String, Arc<ComponentSchema>>,
}

/// Whether `name` can name a component or field.
fn good_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !name.starts_with(|c: char| c.is_ascii_digit())
}

impl Registry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `schema`, refusing a duplicate name, a bad name, or a field
    /// whose starting value doesn't fit it.
    pub fn register(&mut self, schema: ComponentSchema) -> Result<(), RegistryError> {
        if !good_name(&schema.name) {
            return Err(RegistryError::BadName(schema.name));
        }
        if self.schemas.contains_key(&schema.name) {
            return Err(RegistryError::Duplicate(schema.name));
        }
        let mut seen = std::collections::BTreeSet::new();
        for field in &schema.fields {
            let fits = field.kind.accepts(&field.default)
                || matches!(field.kind, FieldKind::Choice(ref o) if o.is_empty());
            if !good_name(&field.name) || !seen.insert(field.name.as_str()) || !fits {
                return Err(RegistryError::BadField {
                    component: schema.name.clone(),
                    field: field.name.clone(),
                });
            }
        }
        self.schemas.insert(schema.name.clone(), Arc::new(schema));
        Ok(())
    }

    /// Adds a Rust component's schema.
    pub fn register_type<T: Component>(&mut self) -> Result<(), RegistryError> {
        self.register(T::schema())
    }

    /// The schema of the component called `name`.
    pub fn get(&self, name: &str) -> Option<&Arc<ComponentSchema>> {
        self.schemas.get(name)
    }

    /// Every schema, sorted by name.
    pub fn iter(&self) -> impl Iterator<Item = &Arc<ComponentSchema>> {
        self.schemas.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::FieldInfo;
    use crate::value::Value;

    fn schema(name: &str) -> ComponentSchema {
        ComponentSchema {
            name: name.into(),
            help: String::new(),
            fields: vec![FieldInfo::new("hp", "HP", FieldKind::Int, Value::Int(100))],
        }
    }

    #[test]
    fn components_are_found_by_name() {
        let mut r = Registry::new();
        r.register(schema("Health")).unwrap();
        assert_eq!(r.get("Health").unwrap().fields.len(), 1);
        assert!(r.get("health").is_none(), "names are exact");
        assert_eq!(r.iter().count(), 1);
    }

    #[test]
    fn duplicates_bad_names_and_bad_fields_are_refused() {
        let mut r = Registry::new();
        r.register(schema("Health")).unwrap();
        assert_eq!(
            r.register(schema("Health")),
            Err(RegistryError::Duplicate("Health".into()))
        );
        for bad in ["", "Has space", "1st", "Dot.ted", &"x".repeat(65)] {
            assert!(
                matches!(r.register(schema(bad)), Err(RegistryError::BadName(_))),
                "{bad}"
            );
        }
        let mut twice = schema("Twice");
        twice.fields.push(twice.fields[0].clone());
        assert!(matches!(
            r.register(twice),
            Err(RegistryError::BadField { .. })
        ));
        let mut wrong = schema("Wrong");
        wrong.fields[0].default = Value::Text("lots".into());
        let err = r.register(wrong).unwrap_err();
        assert!(err.to_string().contains("Wrong.hp"), "{err}");
    }
}
