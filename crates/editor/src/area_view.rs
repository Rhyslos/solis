//! What the viewport draws, worked out from the area each frame: the
//! models, the sun, the lamps and the environment, in the world's terms.

use crate::models::ModelInfo;
use area::Area;
use area::builtin::{Environment, Light, LightKind, Model};
use glam::Vec3;
use reflect::AssetId;
use std::collections::BTreeMap;

/// A light, placed in the world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewLight {
    /// Sun, point or spot.
    pub kind: LightKind,
    /// Where it is.
    pub position: Vec3,
    /// The way it shines (length 1).
    pub direction: Vec3,
    /// Its colour, sRGB 0 to 1.
    pub color: [f32; 3],
    /// Watts, or W/m² for the sun.
    pub strength: f32,
    /// How far it reaches; `None` fades on forever.
    pub range: Option<f32>,
    /// A spotlight's cone: full strength inside `inner`, nothing past
    /// `outer`, in radians.
    pub inner: f32,
    /// See `inner`.
    pub outer: f32,
    /// Whether it casts shadows.
    pub shadows: bool,
}

/// What the viewport draws.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AreaView {
    /// The models to draw, each where its entity is in the world.
    pub models: Vec<(AssetId, glam::Mat4)>,
    /// The sun (the first sun in the area), if there is one.
    pub sun: Option<ViewLight>,
    /// Every other light.
    pub lamps: Vec<ViewLight>,
    /// The sky, fog and picture settings (the first in the area).
    pub environment: Environment,
    /// How many triangles the models have together.
    pub triangles: usize,
}

impl AreaView {
    /// What to draw of `area`, with `models` saying which assets are
    /// loaded.
    pub fn of(area: &Area, models: &BTreeMap<AssetId, ModelInfo>) -> Self {
        let mut view = Self {
            environment: area
                .with::<Environment>()
                .next()
                .map(|(_, e)| e)
                .unwrap_or_default(),
            ..Self::default()
        };
        for (id, model) in area.with::<Model>() {
            if let Some(info) = model.asset.and_then(|r| models.get(&r.asset)) {
                view.models.push((info.asset, area.world_matrix(id)));
                view.triangles += info.triangles;
            }
        }
        for (id, light) in area.with::<Light>() {
            let world = area.world_matrix(id);
            let placed = place(&light, world);
            if light.kind == LightKind::Sun && view.sun.is_none() {
                view.sun = Some(placed);
            } else if light.kind != LightKind::Sun {
                view.lamps.push(placed);
            }
        }
        view
    }
}

/// `light` placed by `world`: shining along the entity's -Z.
fn place(light: &Light, world: glam::Mat4) -> ViewLight {
    let outer = light.cone.clamp(0.5, 89.5).to_radians();
    let inner = (light.cone - light.soft_edge.max(0.0))
        .clamp(0.0, light.cone)
        .to_radians();
    ViewLight {
        kind: light.kind,
        position: world.transform_point3(Vec3::ZERO),
        direction: world
            .transform_vector3(Vec3::NEG_Z)
            .try_normalize()
            .unwrap_or(Vec3::NEG_Z),
        color: light.color,
        strength: light.strength,
        range: (light.range > 0.0).then_some(light.range),
        inner: inner.min(outer),
        outer,
        shadows: light.shadows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use area::EntityData;
    use area::builtin::Transform;

    #[test]
    fn the_starter_area_has_a_sun_shining_down_and_no_lamps() {
        let view = AreaView::of(&area::builtin::starter_area(), &BTreeMap::new());
        let sun = view.sun.unwrap();
        assert!(sun.direction.z < -0.7);
        assert!(view.lamps.is_empty());
        assert!(view.models.is_empty());
    }

    #[test]
    fn lamps_are_placed_by_their_parents() {
        let mut area = Area::new();
        let parent = area.spawn(EntityData::new("Post").with(&Transform {
            position: Vec3::new(5.0, 0.0, 0.0),
            ..Transform::default()
        }));
        let mut bulb = EntityData::new("Bulb")
            .with(&Transform {
                position: Vec3::new(0.0, 0.0, 3.0),
                ..Transform::default()
            })
            .with(&Light {
                range: 4.0,
                ..Light::spot()
            });
        bulb.parent = Some(parent);
        area.spawn(bulb);
        let view = AreaView::of(&area, &BTreeMap::new());
        assert!(view.sun.is_none());
        let lamp = view.lamps[0];
        assert!(lamp.position.abs_diff_eq(Vec3::new(5.0, 0.0, 3.0), 1e-5));
        assert!(lamp.direction.abs_diff_eq(Vec3::NEG_Z, 1e-5));
        assert_eq!(lamp.range, Some(4.0));
        assert!(lamp.inner <= lamp.outer);
    }

    #[test]
    fn models_draw_only_when_their_asset_is_loaded() {
        let mut area = Area::new();
        let asset = AssetId::random();
        let r = reflect::AssetRef { asset, part: None };
        area.spawn(EntityData::new("Crate").with(&Model { asset: Some(r) }));
        assert!(AreaView::of(&area, &BTreeMap::new()).models.is_empty());
        let mut models = BTreeMap::new();
        models.insert(
            asset,
            ModelInfo {
                asset,
                name: "Crate".into(),
                triangles: 12,
                min: Vec3::ZERO,
                max: Vec3::ONE,
            },
        );
        let view = AreaView::of(&area, &models);
        assert_eq!(view.models, [(asset, glam::Mat4::IDENTITY)]);
        assert_eq!(view.triangles, 12);
    }
}
