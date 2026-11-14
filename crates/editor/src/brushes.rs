//! Brushes as the viewport draws them (Tools 1a, B1): each brush's mesh
//! cut into flat triangles once, made again only when the brush changes,
//! never every frame.
//!
//! The document says when any brush may have changed
//! ([`crate::document::Document::brushes`]); only then are the area's
//! brushes looked at again, each by a fingerprint of its values, and only
//! a brush whose values are new is cut into triangles. Brushes with the
//! same values (a hundred copies of one box) share one model, which the
//! renderer uploads once and draws in one go.

use area::builtin::Brush;
use glam::{Mat4, Vec3};
use reflect::{Component, ComponentData, EntityId, IdMap, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// A brush's mesh ready to draw: flat triangles, one mesh of the plain
/// material (each face's own material arrives with B5).
#[derive(Debug)]
pub struct BrushModel {
    /// A fingerprint of the brush's values: brushes with the same one look
    /// the same, and share this model.
    pub key: u64,
    /// The triangles, as a model the renderer uploads.
    pub model: assets::Model,
    /// How many triangles it has.
    pub triangles: usize,
    /// Whether the brush's mesh couldn't be read, so it is drawn as
    /// nothing (the Problems list names it, Tools 1a B2).
    pub damaged: bool,
}

impl BrushModel {
    /// `brush` cut into flat triangles.
    fn of(brush: &Brush, key: u64) -> Self {
        let flat = mesh::flat_triangles(&brush.mesh);
        let vertices: Vec<assets::Vertex> = flat
            .positions
            .iter()
            .zip(&flat.normals)
            .map(|(p, n)| assets::Vertex {
                position: p.to_array(),
                normal: n.to_array(),
                uv: [0.0, 0.0],
            })
            .collect();
        let bounds =
            assets::Bounds::around(flat.positions.iter().copied()).unwrap_or(assets::Bounds {
                min: Vec3::ZERO,
                max: Vec3::ZERO,
            });
        let triangles = flat.len();
        let meshes = if triangles == 0 {
            Vec::new()
        } else {
            vec![assets::Mesh {
                vertices,
                indices: flat.indices,
                material: None,
                lods: Vec::new(),
            }]
        };
        let nodes = vec![assets::Node {
            name: Brush::NAME.into(),
            parent: None,
            local: Mat4::IDENTITY,
            world: Mat4::IDENTITY,
            meshes: (0..meshes.len()).collect(),
        }];
        Self {
            key,
            model: assets::Model {
                name: Brush::NAME.into(),
                meshes,
                nodes,
                materials: Vec::new(),
                textures: Vec::new(),
                lamps: Vec::new(),
                bounds,
                warnings: Vec::new(),
            },
            triangles,
            damaged: brush.damaged,
        }
    }
}

/// A brush to draw, where it is.
#[derive(Debug, Clone)]
pub struct ViewBrush {
    /// Its entity.
    pub entity: EntityId,
    /// Its triangles.
    pub model: Arc<BrushModel>,
    /// Where its entity is in the world.
    pub transform: Mat4,
    /// Its number for picking, as [`crate::area_view::ViewModel::pick`].
    pub pick: u32,
}

/// The same brush in the same place: its model by its key, never
/// triangle by triangle.
impl PartialEq for ViewBrush {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity
            && self.model.key == other.model.key
            && self.transform == other.transform
            && self.pick == other.pick
    }
}

/// Every brush in the open area, cut into triangles, kept until the
/// document says a brush may have changed, and then made again only for
/// the brushes it names.
#[derive(Debug, Default)]
pub(crate) struct BrushModels {
    /// The document's brush revision these are for.
    seen: Option<u64>,
    /// Each brush's model, by its entity.
    by_entity: IdMap<Arc<BrushModel>>,
    /// The models, by their key, with how many brushes use each.
    by_key: HashMap<u64, (Arc<BrushModel>, usize)>,
    /// How many models have been made (for the tests: a brush that didn't
    /// change isn't made again).
    made: usize,
    /// How many brushes' values have been looked at (for the tests: a
    /// change to one brush looks at that one alone).
    looked: usize,
}

impl BrushModels {
    /// Brings the models up to date with `document`'s area: nothing to do
    /// when its brush revision is the one already seen; otherwise only the
    /// brushes it says may have changed are looked at again (all of them
    /// after an undo, say), each by its fingerprint. Returns whether
    /// anything was looked at.
    pub(crate) fn refresh(&mut self, document: &mut crate::document::Document) -> bool {
        if self.seen == Some(document.brushes()) {
            return false;
        }
        let first = self.seen.is_none();
        self.seen = Some(document.brushes());
        let changes = document.take_brush_changes();
        let area = document.area();
        if changes.all || first {
            // Every brush again, keeping the models made so far to hand:
            // an undo brings back values whose model is already made.
            let spare: HashMap<u64, Arc<BrushModel>> = self
                .by_key
                .drain()
                .map(|(key, (model, _))| (key, model))
                .collect();
            self.by_entity.clear();
            for (id, entity) in area.iter() {
                if let Some(data) = entity.components.get(Brush::NAME) {
                    self.hold(id, data, &spare);
                }
            }
            return true;
        }
        if changes.deleted {
            let gone: Vec<EntityId> = self
                .by_entity
                .keys()
                .copied()
                .filter(|id| !area.contains(*id))
                .collect();
            for id in gone {
                self.let_go(id);
            }
        }
        for id in changes.entities {
            match area.get(id).and_then(|e| e.components.get(Brush::NAME)) {
                Some(data) => self.hold(id, data, &HashMap::new()),
                None => self.let_go(id),
            }
        }
        true
    }

    /// Gives `id` the model of its brush `data`, made only if no brush
    /// (nor `spare`) has those values already.
    fn hold(&mut self, id: EntityId, data: &ComponentData, spare: &HashMap<u64, Arc<BrushModel>>) {
        let key = fingerprint(data);
        self.looked += 1;
        if self.by_entity.get(&id).is_some_and(|m| m.key == key) {
            return;
        }
        self.let_go(id);
        let made = &mut self.made;
        let (model, users) = self.by_key.entry(key).or_insert_with(|| {
            let model = spare.get(&key).cloned().unwrap_or_else(|| {
                *made += 1;
                Arc::new(BrushModel::of(&Brush::from_data(data), key))
            });
            (model, 0)
        });
        *users += 1;
        self.by_entity.insert(id, Arc::clone(model));
    }

    /// Forgets `id`'s brush, and its model once no brush uses it.
    fn let_go(&mut self, id: EntityId) {
        let Some(model) = self.by_entity.remove(&id) else {
            return;
        };
        if let Some((_, users)) = self.by_key.get_mut(&model.key) {
            *users -= 1;
            if *users == 0 {
                self.by_key.remove(&model.key);
            }
        }
    }

    /// The model of the brush on `entity`, if it has one.
    pub(crate) fn get(&self, entity: EntityId) -> Option<&Arc<BrushModel>> {
        self.by_entity.get(&entity)
    }

    /// Every brush's model, by its entity, in no particular order.
    #[cfg(test)]
    pub(crate) fn iter(&self) -> impl Iterator<Item = (EntityId, &Arc<BrushModel>)> {
        self.by_entity.iter().map(|(id, m)| (*id, m))
    }

    /// How many different models there are.
    #[cfg(test)]
    pub(crate) fn models(&self) -> usize {
        self.by_key.len()
    }

    /// How many models have been made since the start.
    #[cfg(test)]
    pub(crate) fn made(&self) -> usize {
        self.made
    }

    /// The brushes whose mesh couldn't be read, in no particular order:
    /// looked at again only when a brush may have changed.
    pub(crate) fn damaged(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.by_entity
            .iter()
            .filter(|(_, model)| model.damaged)
            .map(|(id, _)| *id)
    }

    /// How many brushes' values have been looked at since the start.
    #[cfg(test)]
    pub(crate) fn looked(&self) -> usize {
        self.looked
    }
}

/// A fingerprint of a component's values: every value's kind and bits
/// through the standard library's hasher, the same for the same values
/// (it's never saved, so it only has to agree within one run). Its mixing
/// makes two different brushes sharing one practically impossible; a
/// weaker hash (FNV a word at a time) gave boxes of slightly different
/// sizes the same one.
fn fingerprint(data: &ComponentData) -> u64 {
    use std::hash::Hasher;
    let mut hash = std::hash::DefaultHasher::new();
    for (name, value) in &data.fields {
        hash.write(name.as_bytes());
        hash.write_u8(0xff);
        add_value(&mut hash, value);
    }
    hash.finish()
}

fn add_value(hash: &mut impl std::hash::Hasher, value: &Value) {
    match value {
        Value::Bool(b) => hash.write_u8(1 + u8::from(*b)),
        Value::Int(n) => {
            hash.write_u8(3);
            hash.write_i64(*n);
        }
        Value::Float(v) => {
            hash.write_u8(4);
            hash.write_u64(v.to_bits());
        }
        Value::Text(t) => {
            hash.write_u8(5);
            hash.write_usize(t.len());
            hash.write(t.as_bytes());
        }
        Value::List(items) => {
            hash.write_u8(6);
            hash.write_usize(items.len());
            for item in items {
                add_value(hash, item);
            }
        }
        // A brush holds none of these; any other value counts by its kind
        // alone.
        Value::Asset(_) => hash.write_u8(7),
        Value::Entity(_) => hash.write_u8(8),
    }
}

#[cfg(test)]
#[path = "brushes_scale_tests.rs"]
mod scale_tests;
#[cfg(test)]
#[path = "brushes_tests.rs"]
mod tests;
