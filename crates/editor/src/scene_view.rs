//! What the viewport draws, worked out from the scene each frame: the
//! models, the sun, the lamps and the environment, in the world's terms.

use crate::models::ModelInfo;
use glam::Vec3;
use reflect::AssetId;
use scene::Scene;
use scene::builtin::{Environment, Light, LightKind, Model};
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
pub struct SceneView {
    /// The models to draw.
    pub models: Vec<AssetId>,
    /// The sun (the first sun in the scene), if there is one.
    pub sun: Option<ViewLight>,
    /// Every other light.
    pub lamps: Vec<ViewLight>,
    /// The sky, fog and picture settings (the first in the scene).
    pub environment: Environment,
    /// How many triangles the models have together.
    pub triangles: usize,
}

impl SceneView {
    /// What to draw of `scene`, with `models` saying which assets are
    /// loaded.
    pub fn of(scene: &Scene, models: &BTreeMap<AssetId, ModelInfo>) -> Self {
        let mut view = Self {
            environment: scene
                .with::<Environment>()
                .next()
                .map(|(_, e)| e)
                .unwrap_or_default(),
            ..Self::default()
        };
        for (_, model) in scene.with::<Model>() {
            if let Some(info) = model.asset.and_then(|r| models.get(&r.asset)) {
                view.models.push(info.asset);
                view.triangles += info.triangles;
            }
        }
        for (id, light) in scene.with::<Light>() {
            let world = scene.world_matrix(id);
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
    use scene::EntityData;
    use scene::builtin::Transform;

    #[test]
    fn the_starter_scene_has_a_sun_shining_down_and_no_lamps() {
        let view = SceneView::of(&scene::builtin::starter_scene(), &BTreeMap::new());
        let sun = view.sun.unwrap();
        assert!(sun.direction.z < -0.7);
        assert!(view.lamps.is_empty());
        assert!(view.models.is_empty());
    }

    #[test]
    fn lamps_are_placed_by_their_parents() {
        let mut scene = Scene::new();
        let parent = scene.spawn(EntityData::new("Post").with(&Transform {
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
        scene.spawn(bulb);
        let view = SceneView::of(&scene, &BTreeMap::new());
        assert!(view.sun.is_none());
        let lamp = view.lamps[0];
        assert!(lamp.position.abs_diff_eq(Vec3::new(5.0, 0.0, 3.0), 1e-5));
        assert!(lamp.direction.abs_diff_eq(Vec3::NEG_Z, 1e-5));
        assert_eq!(lamp.range, Some(4.0));
        assert!(lamp.inner <= lamp.outer);
    }

    #[test]
    fn models_draw_only_when_their_asset_is_loaded() {
        let mut scene = Scene::new();
        let asset = AssetId::random();
        let r = reflect::AssetRef { asset, part: None };
        scene.spawn(EntityData::new("Crate").with(&Model { asset: Some(r) }));
        assert!(SceneView::of(&scene, &BTreeMap::new()).models.is_empty());
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
        let view = SceneView::of(&scene, &models);
        assert_eq!(view.models, [asset]);
        assert_eq!(view.triangles, 12);
    }
}
