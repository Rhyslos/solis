//! What the viewport draws, worked out from the area each frame: the
//! models, the sun, the lamps and the environment, in the world's terms.

use crate::models::ModelInfo;
use area::Area;
use area::builtin::{Camera, Environment, Light, LightKind, Model};
use glam::Vec3;
use reflect::AssetId;
use std::collections::{BTreeMap, HashMap, HashSet};

/// A light, placed in the world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewLight {
    /// The entity it belongs to (for picking its marker).
    pub entity: reflect::EntityId,
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

/// A model to draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewModel {
    /// The model asset.
    pub asset: AssetId,
    /// One of its parts (a node's index), or the whole model.
    pub node: Option<usize>,
    /// Where its entity is in the world.
    pub transform: glam::Mat4,
    /// Its number for picking: its entity is `pickable[pick - 1]` of the
    /// [`AreaView`]; 0 when it can't be picked.
    pub pick: u32,
}

/// What the viewport draws.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AreaView {
    /// The models to draw.
    pub models: Vec<ViewModel>,
    /// The entity behind each pick number (number 1 is the first).
    pub pickable: Vec<reflect::EntityId>,
    /// Entities that can't be picked (locked in the Area panel).
    pub locked: HashSet<reflect::EntityId>,
    /// The sun (the first sun in the area), if there is one.
    pub sun: Option<ViewLight>,
    /// Every other light.
    pub lamps: Vec<ViewLight>,
    /// The sky, fog and picture settings (the first in the area).
    pub environment: Environment,
    /// How many triangles the models have together.
    pub triangles: usize,
    /// The cameras, where they are (for their markers).
    pub cameras: Vec<(reflect::EntityId, glam::Mat4)>,
    /// The 2D cameras, where they are, each with whether it's the one the
    /// game looks through (the first camera of all): the 2D view frames
    /// what each shows.
    pub screens: Vec<(Vec3, bool)>,
}

impl AreaView {
    /// What to draw of `area`, with `models` saying which assets are
    /// loaded.
    pub fn of(area: &Area, models: &BTreeMap<AssetId, ModelInfo>) -> Self {
        Self::of_shown(area, models, &HashSet::new(), &HashSet::new())
    }

    /// [`Self::of`], leaving out entities `hidden` in the editor, and
    /// making `locked` ones impossible to pick.
    pub fn of_shown(
        area: &Area,
        models: &BTreeMap<AssetId, ModelInfo>,
        hidden: &HashSet<reflect::EntityId>,
        locked: &HashSet<reflect::EntityId>,
    ) -> Self {
        let tree = area::Tree::of(area);
        let world = tree.world_matrices(area);
        Self::of_placed(area, &tree, &world, models, hidden, locked)
    }

    /// [`Self::of_shown`], with the area's `tree` and `world` matrices
    /// already worked out (the document keeps them), so a frame doesn't
    /// walk up every entity's parents again.
    pub fn of_placed(
        area: &Area,
        tree: &area::Tree,
        world: &HashMap<reflect::EntityId, glam::Mat4>,
        models: &BTreeMap<AssetId, ModelInfo>,
        hidden: &HashSet<reflect::EntityId>,
        locked: &HashSet<reflect::EntityId>,
    ) -> Self {
        let at = |id| world.get(&id).copied().unwrap_or_default();
        let mut view = Self {
            environment: tree
                .first_of(area.with::<Environment>().map(|(id, _)| id))
                .and_then(|id| area.get(id)?.get::<Environment>())
                .unwrap_or_default(),
            ..Self::default()
        };
        view.locked = locked.clone();
        for (id, model) in area.with::<Model>() {
            if hidden.contains(&id) {
                continue;
            }
            let pickable = !locked.contains(&id);
            view.add_model(id, model, at(id), models, pickable);
        }
        // The sun is the first in tree order, as the game has it.
        let mut suns = Vec::new();
        for (id, light) in area.with::<Light>() {
            if hidden.contains(&id) {
                continue;
            }
            if light.kind == LightKind::Sun {
                suns.push((id, light));
            } else {
                view.add_light(id, &light, at(id));
            }
        }
        if let Some(first) = tree.first_of(suns.iter().map(|(id, _)| *id))
            && let Some((id, light)) = suns.iter().find(|(id, _)| *id == first)
        {
            view.add_light(*id, light, at(*id));
        }
        let first = tree.first_of(area.with::<Camera>().map(|(id, _)| id));
        for (id, camera) in area.with::<Camera>() {
            if !hidden.contains(&id) {
                view.cameras.push((id, at(id)));
            }
            if camera.view == area::builtin::CameraView::TwoD {
                view.screens
                    .push((at(id).w_axis.truncate(), Some(id) == first));
            }
        }
        view
    }

    /// Every marker that can be clicked in the viewport (lamps and
    /// cameras, which have no surface to pick), with where it is.
    pub fn markers(&self) -> impl Iterator<Item = (reflect::EntityId, Vec3)> + '_ {
        self.lamps
            .iter()
            .map(|l| (l.entity, l.position))
            .chain(
                self.cameras
                    .iter()
                    .map(|(id, m)| (*id, m.transform_point3(Vec3::ZERO))),
            )
            .filter(|(id, _)| !self.locked.contains(id))
    }

    /// What the running game shows of `world`: everything (hidden is only
    /// for the editor), nothing pickable, no camera markers; each entity
    /// `t` of the way between the last tick and the next
    /// (`world::World::placed_between`; 1 for where it is now).
    pub fn of_world(world: &world::World, models: &BTreeMap<AssetId, ModelInfo>, t: f32) -> Self {
        use rayon::prelude::*;
        let mut view = Self {
            environment: world.environment().unwrap_or_default(),
            ..Self::default()
        };
        // Models in jobs (decision 0031), kept in tree order. None of the
        // game's can be picked, so no numbers are handed out.
        let placed = world.placed_between::<Model>(t);
        let drawn: Vec<(ViewModel, usize)> = placed
            .par_iter()
            .filter_map(|(_, model, global)| view_model(model, *global, models))
            .collect();
        view.triangles = drawn.iter().map(|(_, t)| t).sum();
        view.models = drawn.into_iter().map(|(m, _)| m).collect();
        for (id, light, global) in world.placed_between::<Light>(t) {
            view.add_light(id, &light, global);
        }
        view
    }

    fn add_model(
        &mut self,
        id: reflect::EntityId,
        model: Model,
        transform: glam::Mat4,
        models: &BTreeMap<AssetId, ModelInfo>,
        pickable: bool,
    ) {
        let Some((mut drawn, triangles)) = view_model(&model, transform, models) else {
            return;
        };
        self.triangles += triangles;
        if pickable {
            self.pickable.push(id);
            drawn.pick = self.pickable.len() as u32;
        }
        self.models.push(drawn);
    }

    fn add_light(&mut self, id: reflect::EntityId, light: &Light, world: glam::Mat4) {
        let placed = place(id, light, world);
        if light.kind == LightKind::Sun && self.sun.is_none() {
            self.sun = Some(placed);
        } else if light.kind != LightKind::Sun {
            self.lamps.push(placed);
        }
    }
}

/// `model` drawn at `transform` (not pickable), with the triangles it adds
/// (a part's aren't counted), if its asset is loaded and has the part.
fn view_model(
    model: &Model,
    transform: glam::Mat4,
    models: &BTreeMap<AssetId, ModelInfo>,
) -> Option<(ViewModel, usize)> {
    let r = model.asset?;
    let info = models.get(&r.asset)?;
    let node = match r.part {
        None => None,
        // One part: drawn only if the model still has it.
        Some(part) => Some(info.node_of(part)?),
    };
    let triangles = if node.is_none() { info.triangles } else { 0 };
    Some((
        ViewModel {
            asset: info.asset,
            node,
            transform,
            pick: 0,
        },
        triangles,
    ))
}

/// `light` placed by `world`: shining along the entity's -Z.
fn place(entity: reflect::EntityId, light: &Light, world: glam::Mat4) -> ViewLight {
    let outer = light.cone.clamp(0.5, 89.5).to_radians();
    let inner = (light.cone - light.soft_edge.max(0.0))
        .clamp(0.0, light.cone)
        .to_radians();
    ViewLight {
        entity,
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
                lods: Vec::new(),
                min: Vec3::ZERO,
                max: Vec3::ONE,
                nodes: Vec::new(),
            },
        );
        let view = AreaView::of(&area, &models);
        assert_eq!(
            view.models,
            [ViewModel {
                asset,
                node: None,
                transform: glam::Mat4::IDENTITY,
                pick: 1,
            }]
        );
        assert_eq!(view.pickable.len(), 1);
        assert_eq!(view.triangles, 12);
    }

    #[test]
    fn the_game_s_view_is_the_same_on_one_core_or_many() {
        // Decision 0031: jobs never change the result.
        let mut area = Area::new();
        let asset = AssetId::random();
        let mut info = crate::test_support::crate_model();
        info.asset = asset;
        let models = BTreeMap::from([(asset, info)]);
        for i in 0..3_000 {
            let mut data = EntityData::new(&format!("Box {i}")).with(&Transform {
                position: Vec3::new(i as f32, (i % 7) as f32, 0.0),
                ..Transform::default()
            });
            if i % 5 != 0 {
                data.set(&Model {
                    asset: Some(reflect::AssetRef { asset, part: None }),
                });
            }
            data.order = area.order_at_end(None);
            area.spawn(data);
        }
        let world = world::World::from_area(&area);
        let on = |cores: usize| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(cores)
                .build()
                .unwrap()
                .install(|| AreaView::of_world(&world, &models, 1.0))
        };
        let one = on(1);
        assert_eq!(one.models.len(), 2_400);
        assert_eq!(one, on(4));
        // In the area's order: the copies go along x.
        assert!(
            one.models
                .windows(2)
                .all(|w| w[0].transform.w_axis.x < w[1].transform.w_axis.x)
        );
    }

    #[test]
    fn the_game_sees_everything_and_picks_nothing() {
        let mut area = area::builtin::starter_area();
        let asset = AssetId::random();
        let mut info = crate::test_support::crate_model();
        info.asset = asset;
        let models = BTreeMap::from([(asset, info)]);
        let boat = area.spawn(
            EntityData::new("Boat")
                .with(&Transform {
                    position: Vec3::new(4.0, 0.0, 0.0),
                    ..Transform::default()
                })
                .with(&Model {
                    asset: Some(reflect::AssetRef { asset, part: None }),
                }),
        );
        area.spawn(EntityData::new("Lens").with(&Camera::default()));
        let world = world::World::from_area(&area);
        let game = AreaView::of_world(&world, &models, 1.0);
        assert_eq!(game.models.len(), 1);
        assert_eq!(game.models[0].pick, 0);
        assert!(game.pickable.is_empty());
        assert_eq!(
            game.models[0].transform.transform_point3(Vec3::ZERO),
            Vec3::new(4.0, 0.0, 0.0)
        );
        assert!(game.sun.is_some());
        assert!(game.cameras.is_empty(), "no markers in the game");
        // The editor's view of the same area: hidden things left out,
        // cameras marked.
        let hidden = HashSet::from([boat]);
        let editor = AreaView::of_shown(&area, &models, &hidden, &HashSet::new());
        assert!(editor.models.is_empty());
        assert_eq!(editor.cameras.len(), 1);
        assert_eq!(editor.markers().count(), 1);
    }

    #[test]
    fn the_editor_and_the_game_light_the_area_with_the_same_sun() {
        // Two suns (a duplicated one, or one from an assembly): both pick
        // the first in tree order, whatever their IDs.
        let mut area = Area::new();
        let low = reflect::EntityId(reflect::Id::from_raw(1).unwrap());
        let high = reflect::EntityId(reflect::Id::from_raw(u64::MAX).unwrap());
        let mut first = EntityData::new("A").with(&Light {
            strength: 7.0,
            ..Light::sun()
        });
        first.order = "a0".into();
        let mut second = EntityData::new("B").with(&Light::sun());
        second.order = "a1".into();
        area.insert(high, first);
        area.insert(low, second);
        assert_eq!(area.depth_first()[0], high);
        let editor = AreaView::of(&area, &BTreeMap::new());
        let game = AreaView::of_world(&world::World::from_area(&area), &BTreeMap::new(), 1.0);
        assert_eq!(editor.sun.unwrap().entity, high);
        assert_eq!(game.sun.unwrap().entity, high);
        assert!(editor.lamps.is_empty() && game.lamps.is_empty());
    }
}
