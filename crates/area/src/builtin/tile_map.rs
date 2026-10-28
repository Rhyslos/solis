//! A tile map in an area (Phase 9, D4): a tile map file (`.soltilemap`)
//! placed at the entity, its top-left corner on the entity's place.

use reflect::{
    AssetRef, Component, ComponentData, ComponentSchema, FieldInfo, FieldKind, Fields, Hint, Offer,
    Value,
};

/// A tile map file, drawn at the entity and changed as the game plays.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TileMap {
    /// The tile map file; `None` draws nothing.
    pub map: Option<AssetRef>,
}

impl Component for TileMap {
    const NAME: &'static str = "TileMap";

    fn schema() -> ComponentSchema {
        ComponentSchema {
            name: Self::NAME.into(),
            help: "Places a tile map: cells of terrain (grass, tilled, water) in layers, with \
                   values per cell, its top-left corner on the entity. Its cells change as the \
                   game plays."
                .into(),
            fields: vec![
                FieldInfo::new("map", "Map", FieldKind::Asset, Value::Asset(None)).hint(Hint {
                    help: "The tile map file (.soltilemap) in the project.",
                    offers: Offer::Assets("tile map"),
                    ..Hint::default()
                }),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("map", Value::Asset(self.map));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        Self {
            map: Fields(data).asset("map"),
        }
    }
}
