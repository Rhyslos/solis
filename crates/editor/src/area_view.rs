//! What the viewport draws, worked out from the area each frame: the
//! models, the sun, the lamps and the environment, in the world's terms.

use crate::models::ModelInfo;
use area::Area;
use area::builtin::{Camera, Environment, Light, LightKind, Model, Sprite};
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
    /// Whether it shines only at night (decision 0092).
    pub only_at_night: bool,
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
    /// The box the whole model fits in, in its own space (lowest and
    /// highest corners); none for one of its parts. A brush drawn out on
    /// it starts on its top (Tools 1a, B2).
    pub bounds: Option<(Vec3, Vec3)>,
}

/// What the viewport draws.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AreaView {
    /// The models to draw.
    pub models: Vec<ViewModel>,
    /// The brushes to draw (Tools 1a, B1).
    pub brushes: Vec<crate::brushes::ViewBrush>,
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
    /// The hour it's shown at (decision 0092): the game's time of day, or
    /// the Environment's start while editing.
    pub hour: f64,
    /// How many triangles the models have together.
    pub triangles: usize,
    /// The cameras, where they are (for their markers).
    pub cameras: Vec<(reflect::EntityId, glam::Mat4)>,
    /// The 2D cameras, where they are, each with whether it's the one the
    /// game looks through (the first camera of all): the 2D view frames
    /// what each shows.
    pub screens: Vec<(Vec3, bool)>,
    /// The sprites (Phase 9, D2), with their pick numbers.
    pub sprites: Vec<crate::sprite_sheets::AreaSprite>,
    /// The tile maps (Phase 9, D4): each one's file (in the editor) or its
    /// map as the game has it, and where it is.
    pub tile_maps: Vec<AreaTileMap>,
}

/// A tile map an area or the running game draws.
#[derive(Debug, Clone)]
pub struct AreaTileMap {
    /// Its entity.
    pub entity: reflect::EntityId,
    /// Where it is: its top-left corner at the entity.
    pub transform: glam::Mat4,
    /// Its file (in the editor), whose map is drawn.
    pub file: Option<AssetId>,
    /// Its map as the game has it (while playing).
    pub game: Option<std::sync::Arc<tiles::TileMap>>,
}

/// The same map in the same place: the game's map by its copy (never
/// cell by cell, which would be slow for a big map).
impl PartialEq for AreaTileMap {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity
            && self.transform == other.transform
            && self.file == other.file
            && match (&self.game, &other.game) {
                (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
    }
}

impl AreaView {
    /// The maps to draw, their files' read from `sheets`.
    pub fn view_tile_maps(
        &self,
        sheets: &crate::sprite_sheets::SpriteSheets,
    ) -> Vec<crate::tile_view::ViewTileMap> {
        self.tile_maps
            .iter()
            .filter_map(|m| {
                let map = match (&m.game, m.file) {
                    (Some(game), _) => std::sync::Arc::clone(game),
                    (None, Some(file)) => std::sync::Arc::clone(sheets.tile_map(file)?),
                    (None, None) => return None,
                };
                Some(crate::tile_view::ViewTileMap {
                    entity: m.entity,
                    transform: m.transform,
                    map,
                })
            })
            .collect()
    }
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
        view.hour = f64::from(view.environment.day_start);
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
        for (id, sprite) in area.with::<Sprite>() {
            if hidden.contains(&id) || sprite.sheet.is_none() {
                continue;
            }
            let pick = if locked.contains(&id) {
                0
            } else {
                view.pickable.push(id);
                view.pickable.len() as u32
            };
            view.sprites.push(crate::sprite_sheets::AreaSprite {
                entity: id,
                sprite,
                transform: at(id),
                pick,
                parallax: area
                    .get(id)
                    .and_then(|e| e.get::<area::builtin::Parallax>()),
            });
        }
        for (id, tile_map) in area.with::<area::builtin::TileMap>() {
            if hidden.contains(&id) {
                continue;
            }
            view.tile_maps.push(AreaTileMap {
                entity: id,
                transform: at(id),
                file: tile_map.map.map(|m| m.asset),
                game: None,
            });
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
            hour: world.time_of_day(),
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
        view.sprites = world
            .placed_between::<Sprite>(t)
            .into_iter()
            .filter(|(_, s, _)| s.sheet.is_some())
            .map(
                |(entity, sprite, transform)| crate::sprite_sheets::AreaSprite {
                    entity,
                    sprite,
                    transform,
                    pick: 0,
                    parallax: world.get::<area::builtin::Parallax>(entity),
                },
            )
            .collect();
        view.tile_maps = world
            .placed_between::<area::builtin::TileMap>(t)
            .into_iter()
            .filter_map(|(entity, component, transform)| {
                Some(AreaTileMap {
                    entity,
                    transform,
                    file: component.map.map(|m| m.asset),
                    game: Some(std::sync::Arc::new(world.tile_map(entity)?)),
                })
            })
            .collect();
        view
    }

    /// Adds the area's brushes (`brushes`, made from the same area), placed
    /// by `world`, leaving out `hidden` ones and making `locked` ones
    /// impossible to pick, in the area's order.
    pub(crate) fn add_brushes(
        &mut self,
        area: &Area,
        brushes: &crate::brushes::BrushModels,
        world: &HashMap<reflect::EntityId, glam::Mat4>,
        (hidden, locked): (&HashSet<reflect::EntityId>, &HashSet<reflect::EntityId>),
    ) {
        for (id, _) in area.iter() {
            let Some(model) = brushes.get(id) else {
                continue;
            };
            if hidden.contains(&id) {
                continue;
            }
            let pick = if locked.contains(&id) {
                0
            } else {
                self.pickable.push(id);
                self.pickable.len() as u32
            };
            self.triangles += model.triangles;
            self.brushes.push(crate::brushes::ViewBrush {
                entity: id,
                model: std::sync::Arc::clone(model),
                transform: world.get(&id).copied().unwrap_or_default(),
                pick,
            });
        }
    }

    /// Adds the running `world`'s brushes, each `t` of the way between the
    /// last tick and the next, drawn with `brushes`' models (the editor's,
    /// made from the area the game started from: game code doesn't change
    /// a brush's shape). None can be picked.
    pub(crate) fn add_world_brushes(
        &mut self,
        world: &world::World,
        brushes: &crate::brushes::BrushModels,
        t: f32,
    ) {
        for (id, _, transform) in world.placed_between::<area::builtin::Brush>(t) {
            if let Some(model) = brushes.get(id) {
                self.triangles += model.triangles;
                self.brushes.push(crate::brushes::ViewBrush {
                    entity: id,
                    model: std::sync::Arc::clone(model),
                    transform,
                    pick: 0,
                });
            }
        }
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
            bounds: node.is_none().then_some((info.min, info.max)),
        },
        triangles,
    ))
}

/// `light` placed by `world`: shining along the entity's -Z. Whatever
/// numbers `light` holds (one made in code needn't have been read from a
/// file), what's placed is finite, its inner edge inside its outer one.
fn place(entity: reflect::EntityId, light: &Light, world: glam::Mat4) -> ViewLight {
    let light = light.sanitized();
    // The soft edge is never below zero, so the inner edge is never
    // outside the cone; no bound here comes from the light's own numbers.
    let outer = light.cone.to_radians();
    let inner = (light.cone - light.soft_edge).max(0.0).to_radians();
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
        inner,
        outer,
        shadows: light.shadows,
        only_at_night: light.only_at_night,
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
                bounds: Some((Vec3::ZERO, Vec3::ONE)),
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

    /// Whether a placed light's cone is one the renderer can draw.
    fn drawable(lamp: &ViewLight) -> bool {
        lamp.outer.is_finite()
            && lamp.inner.is_finite()
            && 0.0 <= lamp.inner
            && lamp.inner <= lamp.outer
            && lamp.outer < std::f32::consts::FRAC_PI_2
            && lamp.strength.is_finite()
            && lamp.range.is_none_or(|r| r.is_finite() && r > 0.0)
    }

    #[test]
    fn a_light_holding_any_numbers_is_placed_without_a_crash() {
        // BUG-02: `clamp(0.0, light.cone)` stopped the program when the
        // cone was below zero or not a number.
        let odd = [
            -5.0,
            0.0,
            0.2,
            30.0,
            500.0,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ];
        let id = reflect::EntityId(reflect::Id::from_raw(1).unwrap());
        for cone in odd {
            for soft_edge in odd {
                for other in odd {
                    let light = Light {
                        cone,
                        soft_edge,
                        strength: other,
                        range: other,
                        ..Light::spot()
                    };
                    let lamp = place(id, &light, glam::Mat4::IDENTITY);
                    assert!(drawable(&lamp), "{cone} {soft_edge} {other}: {lamp:?}");
                }
            }
        }
    }

    /// An area file with one spotlight, its cone and soft edge as written.
    fn spot_file(cone: &str, soft_edge: &str) -> String {
        format!(
            "area version 1;\n\nentity 0000000000001 {{\n    name = \"Torch\";\n    \
             Light.kind = \"Spot\";\n    Light.cone = {cone};\n    Light.soft_edge = \
             {soft_edge};\n}}\n"
        )
    }

    #[test]
    fn an_area_file_with_a_bad_cone_loads_and_draws() {
        let registry = area::builtin::registry().unwrap();
        // Numbers the file format reads: each loads, and draws.
        for (cone, soft_edge) in [
            ("-5", "10"),
            ("-5.0", "-3.5"),
            ("0", "0"),
            ("0.0", "10.0"),
            ("30.0", "80.0"),
            ("1000", "5"),
            ("-1.0e30", "1.0e30"),
        ] {
            let text = spot_file(cone, soft_edge);
            let area = area::text::read(&text, &registry).unwrap_or_else(|e| panic!("{text}{e:?}"));
            let editor = AreaView::of(&area, &BTreeMap::new());
            let game = AreaView::of_world(&world::World::from_area(&area), &BTreeMap::new(), 1.0);
            for view in [editor, game] {
                assert_eq!(view.lamps.len(), 1, "{text}");
                assert!(drawable(&view.lamps[0]), "{text}{:?}", view.lamps[0]);
            }
        }
        // What isn't a number at all is refused in words or read as the
        // standard cone, and draws if it's read.
        for odd in ["NaN", "nan", "inf", "-inf", "Infinity", "1e999", "-1e999"] {
            for text in [spot_file(odd, "10.0"), spot_file("30.0", odd)] {
                match area::text::read(&text, &registry) {
                    Ok(area) => {
                        let view = AreaView::of(&area, &BTreeMap::new());
                        assert!(view.lamps.iter().all(drawable), "{text}");
                    }
                    Err(why) => assert!(!why.message.is_empty(), "{text}"),
                }
            }
        }
    }

    #[test]
    fn a_light_changed_to_a_bad_cone_while_playing_is_drawn_safely() {
        // The game's world takes a light's values as they change (edited
        // in Properties while playing, an assembly spawned by game code).
        let mut area = Area::new();
        let id = area.spawn(EntityData::new("Torch").with(&Light::spot()));
        let mut world = world::World::from_area(&area);
        let mut bad = reflect::Component::to_data(&Light::spot());
        bad.set("cone", reflect::Value::Float(-5.0));
        bad.set("soft_edge", reflect::Value::Float(-40.0));
        area.get_mut(id)
            .unwrap()
            .components
            .insert("Light".into(), bad.clone());
        world.update(&area, [(id, true)]);
        let view = AreaView::of_world(&world, &BTreeMap::new(), 1.0);
        assert!(drawable(&view.lamps[0]), "{:?}", view.lamps[0]);
        // And one that arrives while playing.
        let spawned = reflect::EntityId(reflect::Id::from_raw(77).unwrap());
        let mut torch = EntityData::new("Spawned");
        torch.components.insert("Light".into(), bad);
        world.add_subtree(&[(spawned, torch)]);
        let view = AreaView::of_world(&world, &BTreeMap::new(), 1.0);
        assert_eq!(view.lamps.len(), 2);
        assert!(view.lamps.iter().all(drawable), "{:?}", view.lamps);
    }
}
