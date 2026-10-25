//! A model drawn at the entity: an imported asset (or one node of it).

use reflect::{
    AssetRef, Component, ComponentData, ComponentSchema, FieldInfo, FieldKind, Fields, Hint, Value,
};

/// A model drawn at the entity.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Model {
    /// The model asset (and, later, the node inside it); `None` draws
    /// nothing.
    pub asset: Option<AssetRef>,
}

impl Component for Model {
    const NAME: &'static str = "Model";

    fn schema() -> ComponentSchema {
        ComponentSchema {
            name: Self::NAME.into(),
            help: "Draws an imported model at the entity.".into(),
            fields: vec![
                FieldInfo::new("asset", "Model", FieldKind::Asset, Value::Asset(None)).hint(Hint {
                    help: "The imported model to draw.",
                    ..Hint::default()
                }),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("asset", Value::Asset(self.asset));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        Self {
            asset: Fields(data).asset("asset"),
        }
    }
}
