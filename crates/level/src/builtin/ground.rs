//! The ground in a level (Tools 1b, G1; decisions 0111 and 0116): a
//! ground's settings file (`.ground.lst`), whose voxels are in the region
//! files beside it. A level has at most one: the editor refuses a second
//! with why ([`ONE_PER_LEVEL`]).

use super::field;
use reflect::{
    AssetRef, Component, ComponentData, ComponentSchema, FieldInfo, FieldKind, Fields, Hint, Offer,
    Value,
};

/// Why a second ground isn't added to a level, in plain words.
pub const ONE_PER_LEVEL: &str = "A level has one ground (it can be up to 2 × 2 km): this level \
                                 already has one. Pick it in the Level list to change it.";

/// A ground, drawn where its voxels say, and solid when Solid is on.
#[derive(Debug, Clone, PartialEq)]
pub struct Ground {
    /// The ground's settings file; `None` draws nothing.
    pub ground: Option<AssetRef>,
    /// Whether characters and bodies stand on it.
    pub solid: bool,
}

impl Default for Ground {
    fn default() -> Self {
        Self {
            ground: None,
            solid: true,
        }
    }
}

impl Component for Ground {
    const NAME: &'static str = "Ground";

    fn schema() -> ComponentSchema {
        ComponentSchema {
            name: Self::NAME.into(),
            help: "The level's ground: hills, and later caves and overhangs, kept as voxels \
                   (one-metre cubes) in its own files. A level has one."
                .into(),
            fields: vec![
                FieldInfo::new("ground", "Ground", FieldKind::Asset, Value::Asset(None)).hint(
                    Hint {
                        help: "The ground's settings file (.ground.lst) in the project; its \
                               voxels are in the folder beside it.",
                        offers: Offer::Assets("ground"),
                        ..Hint::default()
                    },
                ),
                field::boolean(
                    "solid",
                    "Solid",
                    true,
                    "Characters and bodies stand on it and bump into it.",
                ),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("ground", Value::Asset(self.ground));
        data.set("solid", Value::Bool(self.solid));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        let fields = Fields(data);
        Self {
            ground: fields.asset("ground"),
            solid: fields.bool("solid", true),
        }
    }
}
