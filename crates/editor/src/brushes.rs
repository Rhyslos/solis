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

use area::Area;
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
/// document says a brush may have changed.
#[derive(Debug, Default)]
pub(crate) struct BrushModels {
    /// The document's brush revision these are for.
    seen: Option<u64>,
    /// Each brush's model, by its entity.
    by_entity: IdMap<Arc<BrushModel>>,
    /// The models, by their key.
    by_key: HashMap<u64, Arc<BrushModel>>,
    /// How many models have been made (for the tests: a brush that didn't
    /// change isn't made again).
    made: usize,
}

impl BrushModels {
    /// Brings the models up to date with `area` at the document's brush
    /// `revision`: nothing to do when it's the one already seen. Returns
    /// whether anything was looked at.
    pub(crate) fn refresh(&mut self, area: &Area, revision: u64) -> bool {
        if self.seen == Some(revision) {
            return false;
        }
        self.seen = Some(revision);
        let mut by_entity = IdMap::default();
        let mut by_key: HashMap<u64, Arc<BrushModel>> = HashMap::new();
        for (id, entity) in area.iter() {
            let Some(data) = entity.components.get(Brush::NAME) else {
                continue;
            };
            let key = fingerprint(data);
            let model = match by_key.get(&key).or_else(|| self.by_key.get(&key)) {
                Some(model) => Arc::clone(model),
                None => {
                    self.made += 1;
                    Arc::new(BrushModel::of(&Brush::from_data(data), key))
                }
            };
            by_key.insert(key, Arc::clone(&model));
            by_entity.insert(id, model);
        }
        self.by_entity = by_entity;
        self.by_key = by_key;
        true
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
}

/// A fingerprint of a component's values (FNV-1a over each value's bits
/// and kind), the same for the same values in every run.
fn fingerprint(data: &ComponentData) -> u64 {
    let mut hash = Fnv(0xcbf2_9ce4_8422_2325);
    for (name, value) in &data.fields {
        hash.bytes(name.as_bytes());
        hash.value(value);
    }
    hash.0
}

struct Fnv(u64);

impl Fnv {
    fn word(&mut self, word: u64) {
        self.0 = (self.0 ^ word).wrapping_mul(0x0000_0100_0000_01b3);
    }

    fn bytes(&mut self, bytes: &[u8]) {
        self.word(bytes.len() as u64);
        for chunk in bytes.chunks(8) {
            let mut word = [0u8; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.word(u64::from_le_bytes(word));
        }
    }

    fn value(&mut self, value: &Value) {
        match value {
            Value::Bool(b) => self.word(1 + u64::from(*b)),
            Value::Int(n) => {
                self.word(3);
                self.word(*n as u64);
            }
            Value::Float(v) => {
                self.word(4);
                self.word(v.to_bits());
            }
            Value::Text(t) => {
                self.word(5);
                self.bytes(t.as_bytes());
            }
            Value::List(items) => {
                self.word(6);
                self.word(items.len() as u64);
                for item in items {
                    self.value(item);
                }
            }
            // A brush holds none of these; any other value counts by its
            // kind alone.
            Value::Asset(_) => self.word(7),
            Value::Entity(_) => self.word(8),
        }
    }
}

#[cfg(test)]
#[path = "brushes_scale_tests.rs"]
mod scale_tests;
#[cfg(test)]
#[path = "brushes_tests.rs"]
mod tests;
