//! Commands: everything the editor does to the area, each turned into
//! reversible edits and recorded as one undo step
//! ([decision 0022](../../../docs/decisions/0022-undo-as-reversible-edits.md)).
//! Nothing else in the editor changes the area.

use crate::console::LogLevel;
use crate::document::Selection;
use crate::models::{FileLight, FileLightKind, ModelInfo};
use crate::properties::PropertyRequest;
use crate::workbench::Workbench;
use area::builtin::{Light, LightKind, Model, Transform};
use area::{Edit, EntityData, MergeKey};
use glam::{Quat, Vec3};
use reflect::{AssetRef, Component, ComponentData, EntityId};
use std::hash::{DefaultHasher, Hash, Hasher};

/// How far above the camera's focus point a new point light goes, in
/// metres.
pub(crate) const POINT_LIGHT_HEIGHT: f32 = 2.0;

/// The edits that change `entity`'s component from `before` (`None` if it
/// has none yet) to `after`: only the fields that differ.
fn component_edits<T: Component>(
    entity: EntityId,
    before: Option<&ComponentData>,
    after: &T,
) -> Vec<Edit> {
    let data = after.to_data();
    let Some(before) = before else {
        return vec![Edit::AddComponent {
            entity,
            component: T::NAME.to_string(),
            data,
        }];
    };
    data.fields
        .into_iter()
        .filter(|(field, value)| before.get(field) != Some(value))
        .map(|(field, value)| Edit::SetField {
            entity,
            component: T::NAME.to_string(),
            field,
            value: Some(value),
        })
        .collect()
}

/// A merge key for changes to one thing during one gesture.
fn merge_key(target: impl Hash, gesture: u64) -> MergeKey {
    let mut hasher = DefaultHasher::new();
    target.hash(&mut hasher);
    MergeKey {
        target: hasher.finish(),
        gesture,
    }
}

impl Workbench {
    /// Changes the area as one undo step, or says in the console why it
    /// couldn't. Returns whether it worked.
    fn command(
        &mut self,
        label: &str,
        edits: Vec<Edit>,
        merge: Option<MergeKey>,
        select: Option<Selection>,
    ) -> bool {
        match self.document.edit(label, edits, merge, select) {
            Ok(()) => true,
            Err(why) => {
                self.log(LogLevel::Error, format!("Couldn't {label}: {why}."));
                false
            }
        }
    }

    /// Undoes the last change, if there is one.
    pub fn undo(&mut self) {
        match self.document.undo() {
            Some(Ok(label)) => self.log(LogLevel::Info, format!("Undid {label}.")),
            Some(Err(why)) => self.log(LogLevel::Error, format!("Couldn't undo: {why}.")),
            None => {}
        }
    }

    /// Redoes the last change undone, if there is one.
    pub fn redo(&mut self) {
        match self.document.redo() {
            Some(Ok(label)) => self.log(LogLevel::Info, format!("Redid {label}.")),
            Some(Err(why)) => self.log(LogLevel::Error, format!("Couldn't redo: {why}.")),
            None => {}
        }
    }

    /// Selects only `id` (as clicking it in the Area panel does); nothing
    /// if it isn't in the area. Not an undo step.
    pub fn select(&mut self, id: EntityId) {
        if self.document.area().contains(id) {
            self.document.selection.select_only(id);
        } else {
            self.document.selection.clear();
        }
    }

    /// Adds an imported model to the area as an entity (with any lights its
    /// file carries as entities under it; a sun in the file sets the area's
    /// sun), as one undo step. Frames the camera on it, and says so in the
    /// console with its size and any `warnings` (parts of the file that
    /// were left out).
    pub fn add_model(
        &mut self,
        info: ModelInfo,
        warnings: &[String],
        seconds: f32,
        lights: &[FileLight],
    ) -> EntityId {
        self.log(
            LogLevel::Info,
            format!(
                "Imported {}: {} triangles, {}, in {seconds:.1} s.",
                info.name,
                crate::models::grouped(info.triangles),
                info.size_text()
            ),
        );
        for warning in warnings {
            self.log(LogLevel::Warning, format!("{}: {warning}", info.name));
        }
        self.viewport.camera.frame(info.min, info.max);
        let id = EntityId::random();
        let model = EntityData::new(&info.name)
            .with(&Transform::default())
            .with(&Model {
                asset: Some(AssetRef {
                    asset: info.asset,
                    part: None,
                }),
            });
        let mut entities = vec![(id, model)];
        let mut edits = Vec::new();
        for (i, light) in lights.iter().enumerate() {
            match file_light(i, light) {
                FromFile::Lamp(mut data) => {
                    data.parent = Some(id);
                    entities.push((EntityId::random(), *data));
                }
                FromFile::Sun(light, rotation) => {
                    edits.extend(self.sun_edits(&light, rotation));
                    self.log(
                        LogLevel::Info,
                        "The file's sun now lights the area; change it under Sun.".to_string(),
                    );
                }
            }
        }
        let lamps = entities.len() - 1;
        if lamps > 0 {
            let s = if lamps == 1 { "" } else { "s" };
            self.log(
                LogLevel::Info,
                format!("The file brought {lamps} light{s} with it."),
            );
        }
        edits.insert(0, Edit::Spawn { entities });
        let label = format!("Import {}", info.name);
        self.models.insert(info.asset, info);
        self.command(&label, edits, None, None);
        id
    }

    /// The edits that make the area's sun (adding one if there's none)
    /// shine like `light`, turned by `rotation`.
    fn sun_edits(&self, light: &Light, rotation: Quat) -> Vec<Edit> {
        let area = self.document.area();
        let sun = area
            .with::<Light>()
            .find(|(_, l)| l.kind == LightKind::Sun)
            .map(|(id, _)| id);
        let transform = |old: Option<Transform>| Transform {
            rotation,
            ..old.unwrap_or_default()
        };
        let Some(id) = sun else {
            let data = EntityData::new("Sun").with(&transform(None)).with(light);
            return vec![Edit::Spawn {
                entities: vec![(EntityId::random(), data)],
            }];
        };
        let Some(entity) = area.get(id) else {
            return Vec::new();
        };
        let old = entity.get::<Light>().unwrap_or_else(Light::sun);
        let mut edits = component_edits(
            id,
            entity.components.get(Light::NAME),
            &Light {
                color: light.color,
                strength: light.strength,
                ..old
            },
        );
        edits.extend(component_edits(
            id,
            entity.components.get(Transform::NAME),
            &transform(entity.get::<Transform>()),
        ));
        edits
    }

    /// Adds an entity with only a transform, at the point the camera looks
    /// at, and selects it.
    pub fn add_entity(&mut self) -> EntityId {
        let focus = self.viewport.camera.focus();
        self.spawn_selected(EntityData::new("Entity").with(&Transform {
            position: focus,
            ..Transform::default()
        }))
    }

    /// Adds a point light above where the camera looks, and selects it.
    pub fn add_point_light(&mut self) -> EntityId {
        let n = self.count_lights(LightKind::Point) + 1;
        let position = self.viewport.camera.focus() + Vec3::Z * POINT_LIGHT_HEIGHT;
        self.spawn_selected(
            EntityData::new(&format!("Point light {n}"))
                .with(&Transform {
                    position,
                    ..Transform::default()
                })
                .with(&Light::point()),
        )
    }

    /// Adds a spotlight at the camera, shining where it looks, and selects
    /// it.
    pub fn add_spotlight(&mut self) -> EntityId {
        let n = self.count_lights(LightKind::Spot) + 1;
        let camera = &self.viewport.camera;
        self.spawn_selected(
            EntityData::new(&format!("Spotlight {n}"))
                .with(&Transform {
                    position: camera.position,
                    rotation: area::builtin::aim(camera.forward()),
                    ..Transform::default()
                })
                .with(&Light::spot()),
        )
    }

    fn count_lights(&self, kind: LightKind) -> usize {
        self.document
            .area()
            .with::<Light>()
            .filter(|(_, l)| l.kind == kind)
            .count()
    }

    fn spawn_selected(&mut self, data: EntityData) -> EntityId {
        let id = EntityId::random();
        let label = format!("Add {}", data.name);
        let mut selected = Selection::default();
        selected.select_only(id);
        self.command(
            &label,
            vec![Edit::Spawn {
                entities: vec![(id, data)],
            }],
            None,
            Some(selected),
        );
        id
    }

    /// Carries out what the user asked for in Properties. Changes to one
    /// field during one `gesture` (a drag, say) are one undo step.
    pub(crate) fn apply_property(&mut self, request: PropertyRequest, gesture: u64) {
        match request {
            PropertyRequest::SetField {
                entity,
                component,
                field,
                value,
            } => {
                let label = format!("Change {component}.{field}");
                let merge = merge_key((entity, &component, &field), gesture);
                self.command(
                    &label,
                    vec![Edit::SetField {
                        entity,
                        component,
                        field,
                        value: Some(value),
                    }],
                    Some(merge),
                    None,
                );
            }
            PropertyRequest::Rename { entity, name } => {
                let merge = merge_key((entity, "name"), gesture);
                self.command(
                    "Rename",
                    vec![Edit::Rename { entity, name }],
                    Some(merge),
                    None,
                );
            }
            PropertyRequest::RemoveComponent { entity, component } => {
                let label = format!("Remove {component}");
                self.command(
                    &label,
                    vec![Edit::RemoveComponent { entity, component }],
                    None,
                    None,
                );
            }
            PropertyRequest::Delete(entity) => self.delete(entity),
            PropertyRequest::MoveHere(entity) => self.move_here(entity),
        }
    }

    /// Deletes `id` and everything below it.
    pub fn delete(&mut self, id: EntityId) {
        let area = self.document.area();
        let Some(entity) = area.get(id) else {
            return;
        };
        let label = format!("Delete {}", entity.name);
        let extra = match area.tree(id).len() - 1 {
            0 => String::new(),
            1 => " and 1 entity under it".into(),
            n => format!(" and {n} entities under it"),
        };
        if self.command(&label, vec![Edit::Despawn { root: id }], None, None) {
            self.log(LogLevel::Info, format!("{label}{extra}."));
        }
    }

    /// Moves `id` to where the camera looks: a point light above that
    /// point, a spotlight to the camera, shining the same way, anything
    /// else to the point itself.
    pub fn move_here(&mut self, id: EntityId) {
        let camera = self.viewport.camera;
        let Some(entity) = self.document.area().get(id) else {
            return;
        };
        let mut transform = entity.get::<Transform>().unwrap_or_default();
        match entity.get::<Light>().map(|l| l.kind) {
            Some(LightKind::Spot) => {
                transform.position = camera.position;
                transform.rotation = area::builtin::aim(camera.forward());
            }
            Some(LightKind::Point) => {
                transform.position = camera.focus() + Vec3::Z * POINT_LIGHT_HEIGHT;
            }
            _ => transform.position = camera.focus(),
        }
        let edits = component_edits(id, entity.components.get(Transform::NAME), &transform);
        self.command("Move here", edits, None, None);
    }
}

/// A light from a model's file, as the area takes it.
enum FromFile {
    /// A lamp: an entity to put under the model.
    Lamp(Box<EntityData>),
    /// A sun: how it shines, and which way.
    Sun(Light, Quat),
}

/// The `i`th light from a model's file, made safe: bad numbers become
/// usable ones.
fn file_light(i: usize, light: &FileLight) -> FromFile {
    let color = light.color.map(crate::models::linear_to_srgb);
    let rotation = area::builtin::aim(light.direction);
    let strength = if light.strength.is_finite() {
        light.strength.max(0.0)
    } else {
        0.0
    };
    if light.kind == FileLightKind::Sun {
        return FromFile::Sun(
            Light {
                color,
                strength,
                ..Light::sun()
            },
            rotation,
        );
    }
    let (kind, cone, soft_edge) = match light.kind {
        FileLightKind::Spot { inner, outer } => {
            let outer = outer.to_degrees().clamp(1.0, 89.0);
            let inner = inner.to_degrees().clamp(0.0, outer);
            (LightKind::Spot, outer, outer - inner)
        }
        _ => (LightKind::Point, 30.0, 10.0),
    };
    let name = if light.name.trim().is_empty() {
        format!("Light {}", i + 1)
    } else {
        light.name.clone()
    };
    let position = if light.position.is_finite() {
        light.position
    } else {
        Vec3::ZERO
    };
    let mut data = EntityData::new(&name)
        .with(&Transform {
            position,
            rotation,
            ..Transform::default()
        })
        .with(&Light {
            kind,
            color,
            strength,
            range: light
                .range
                .filter(|r| r.is_finite() && *r > 0.0)
                .unwrap_or(0.0),
            cone,
            soft_edge,
            ..Light::point()
        });
    data.order = format!("a{i:04}");
    FromFile::Lamp(Box::new(data))
}
