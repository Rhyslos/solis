//! Commands: everything the editor does to the area, each turned into
//! reversible edits and recorded as one undo step
//! ([decision 0022](../../../docs/decisions/0022-undo-as-reversible-edits.md)).
//! Nothing else in the editor changes the area.

use crate::area_panel::AreaRequest;
use crate::console::LogLevel;
use crate::document::Selection;
use crate::models::{FileLight, FileLightKind, ModelInfo};
use crate::properties::AssemblyPart;
use crate::properties::PropertyRequest;
use crate::workbench::{Workbench, WorkbenchAction};
use area::assembly::Expander;
use area::builtin::{Brush, Light, LightKind, Model, Transform};
use area::text::{AreaFile, FileKind};
use area::{Edit, EntityData, MergeKey};
use glam::{Quat, Vec3};
use reflect::AssetId;
use reflect::{AssetRef, Component, ComponentData, EntityId};
use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

/// How high above the ground a 2D camera added from the 2D view goes, in
/// metres: it sees everything below it, so things stacked in layers (each
/// a little higher) are all in view.
const TWO_D_CAMERA_HEIGHT: f32 = 100.0;

/// How far above the camera's focus point a new point light goes, in
/// metres.
pub(crate) const POINT_LIGHT_HEIGHT: f32 = 2.0;

/// The edits that change `entity`'s component from `before` (`None` if it
/// has none yet) to `after`: only the fields that differ.
pub(crate) fn component_edits<T: Component>(
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
pub(crate) fn merge_key(target: impl Hash, gesture: u64) -> MergeKey {
    let mut hasher = DefaultHasher::new();
    target.hash(&mut hasher);
    MergeKey {
        target: hasher.finish(),
        gesture,
    }
}

/// What the Console adds when a delete leaves parts of a deleted copy
/// behind (parts moved out from under it, which `edits` make ordinary
/// entities): nothing when it leaves none.
fn left_behind(edits: &[Edit]) -> String {
    let left = edits
        .iter()
        .filter(|e| matches!(e, Edit::SetOrigin { .. }))
        .count();
    match left {
        0 => String::new(),
        1 => " A part moved out of the copy stays, as an ordinary entity.".into(),
        n => format!(" {n} parts moved out of the copy stay, as ordinary entities."),
    }
}

impl Workbench {
    /// Changes the area as one undo step, or says in the console why it
    /// couldn't. Returns whether it worked.
    pub(crate) fn command(
        &mut self,
        label: &str,
        edits: Vec<Edit>,
        merge: Option<MergeKey>,
        select: Option<Selection>,
    ) -> bool {
        match self.open.document.edit(label, edits, merge, select) {
            Ok(()) => true,
            Err(why) => {
                self.log(LogLevel::Error, format!("Couldn't {label}: {why}."));
                false
            }
        }
    }

    /// Undoes the last change, if there is one.
    pub fn undo(&mut self) {
        match self.open.document.undo() {
            Some(Ok(label)) => self.log(LogLevel::Info, format!("Undid {label}.")),
            Some(Err(why)) => self.log(LogLevel::Error, format!("Couldn't undo: {why}.")),
            None => {}
        }
    }

    /// Redoes the last change undone, if there is one.
    pub fn redo(&mut self) {
        match self.open.document.redo() {
            Some(Ok(label)) => self.log(LogLevel::Info, format!("Redid {label}.")),
            Some(Err(why)) => self.log(LogLevel::Error, format!("Couldn't redo: {why}.")),
            None => {}
        }
    }

    /// Makes a component known to the editor, so Properties shows it and
    /// Add component offers it: how game code's components (Lust, Lua,
    /// Rust) join in. Refuses a name already taken or a broken schema.
    pub fn register_component(
        &mut self,
        schema: reflect::ComponentSchema,
    ) -> Result<(), reflect::RegistryError> {
        self.sources += 1;
        self.open.document.registry.register(schema)
    }

    /// Selects only `id` (as clicking it in the Area panel does); nothing
    /// if it isn't in the area. Not an undo step.
    pub fn select(&mut self, id: EntityId) {
        if self.open.document.area().contains(id) {
            self.open.document.selection.select_only(id);
        } else {
            self.open.document.selection.clear();
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
        // Dropped somewhere: placed there (and selected); otherwise at the
        // origin, with the camera framing it.
        let target = self.take_place_target(info.asset);
        if target.is_none() {
            self.viewport.camera.frame(info.min, info.max);
        }
        let id = EntityId::random();
        let model = EntityData::new(&info.name)
            .with(&Transform {
                position: target.map_or(Vec3::ZERO, |(at, _)| at),
                ..Transform::default()
            })
            .with(&Model {
                asset: Some(AssetRef {
                    asset: info.asset,
                    part: None,
                }),
            });
        let mut model = model;
        model.parent = target.and_then(|(_, parent)| parent);
        model.order = self.open.document.area().order_at_end(model.parent);
        let mut entities = vec![(id, model)];
        let mut edits = Vec::new();
        let mut order = None;
        for (i, light) in lights.iter().enumerate() {
            match file_light(i, light) {
                FromFile::Lamp(mut data) => {
                    data.parent = Some(id);
                    let next = area::order_after(order.as_deref());
                    data.order.clone_from(&next);
                    order = Some(next);
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
        self.open.models.insert(info.asset, info);
        self.sources += 1;
        let select = target.map(|_| {
            let mut s = Selection::default();
            s.select_only(id);
            s
        });
        self.command(&label, edits, None, select);
        id
    }

    /// The edits that make the area's sun (adding one if there's none)
    /// shine like `light`, turned by `rotation`.
    fn sun_edits(&self, light: &Light, rotation: Quat) -> Vec<Edit> {
        let area = self.open.document.area();
        let sun = area.first_in_tree(
            area.with::<Light>()
                .filter(|(_, l)| l.kind == LightKind::Sun)
                .map(|(id, _)| id),
        );
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
                    position: camera.viewpoint(),
                    rotation: area::builtin::aim(camera.forward()),
                    ..Transform::default()
                })
                .with(&Light::spot()),
        )
    }

    /// Adds a camera where the editor's camera is, looking the same way,
    /// and selects it. From the 2D view it's a 2D camera, over the view's
    /// middle on a whole pixel.
    pub fn add_camera(&mut self) -> EntityId {
        let n = self
            .open
            .document
            .area()
            .with::<area::builtin::Camera>()
            .count()
            + 1;
        let camera = &self.viewport.camera;
        if camera.kind == crate::camera::ViewKind::TwoD {
            let ppu = self.open.pixels.pixels_per_unit.max(1) as f32;
            let on_pixel = |v: f32| (v * ppu).round() / ppu;
            return self.spawn_selected(
                EntityData::new(&format!("Camera {n}"))
                    .with(&Transform {
                        position: glam::Vec3::new(
                            on_pixel(camera.position.x),
                            on_pixel(camera.position.y),
                            TWO_D_CAMERA_HEIGHT,
                        ),
                        ..Transform::default()
                    })
                    .with(&area::builtin::Camera {
                        view: area::builtin::CameraView::TwoD,
                        ..area::builtin::Camera::default()
                    }),
            );
        }
        // A camera entity looks along its -Z with +Y up.
        let rotation = glam::Quat::from_mat3(&glam::Mat3::from_cols(
            camera.right(),
            camera.up(),
            -camera.forward(),
        ))
        .normalize();
        self.spawn_selected(
            EntityData::new(&format!("Camera {n}"))
                .with(&Transform {
                    position: camera.viewpoint(),
                    rotation,
                    ..Transform::default()
                })
                .with(&area::builtin::Camera {
                    fov: camera.fov_y.to_degrees(),
                    ..area::builtin::Camera::default()
                }),
        )
    }

    fn count_lights(&self, kind: LightKind) -> usize {
        self.open
            .document
            .area()
            .with::<Light>()
            .filter(|(_, l)| l.kind == kind)
            .count()
    }

    pub(crate) fn spawn_selected(&mut self, mut data: EntityData) -> EntityId {
        let id = EntityId::random();
        data.order = self.open.document.area().order_at_end(data.parent);
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
                if let Some(playing) = &mut self.open.playing {
                    playing.note_yours(entity, &component, &field);
                }
                let label = format!("Change {component}.{field}");
                let merge = merge_key((entity, &component, &field), gesture);
                // A brush's Made as makes its mesh again too (B2).
                let made_as = (component == Brush::NAME && field == "shape")
                    .then(|| self.made_as_edits(entity, &value))
                    .flatten();
                let edits = made_as.unwrap_or_else(|| {
                    vec![Edit::SetField {
                        entity,
                        component,
                        field,
                        value: Some(value),
                    }]
                });
                self.command(&label, edits, Some(merge), None);
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
            PropertyRequest::AddComponent { entity, component } => {
                let Some(schema) = self.open.document.registry.get(&component) else {
                    return;
                };
                let data = ComponentData::defaults(schema);
                let label = format!("Add {component}");
                self.command(
                    &label,
                    vec![Edit::AddComponent {
                        entity,
                        component,
                        data,
                    }],
                    None,
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
            PropertyRequest::RevertField {
                entity,
                component,
                field,
            } => self.revert_field(entity, &component, &field),
            PropertyRequest::ApplyField {
                entity,
                component,
                field,
            } => self.apply_field(entity, &component, &field),
            PropertyRequest::Unpack(root) => self.unpack(root),
            // Asked of the Library, so a component's file is treated as a
            // double-click on it there is: Lust opens in its program, a
            // .lua file is shown in its folder (never run), and a file
            // named outside the assets isn't asked for at all.
            PropertyRequest::OpenCode(file) if self.open.project.is_some() => {
                let mut actions = Vec::new();
                self.library_request(
                    crate::library::LibraryRequest::Open(crate::library::LibraryKind::Code, file),
                    &mut actions,
                );
                self.open.outbox.extend(actions);
            }
            PropertyRequest::OpenGuide(chapter) => self
                .open
                .outbox
                .push(WorkbenchAction::OpenGuide(Some(chapter.to_string()))),
            PropertyRequest::OpenCode(file) => self.log(
                LogLevel::Warning,
                format!("{file} is in the ready-made code: open a project to see it."),
            ),
            PropertyRequest::SetFieldMany {
                entities,
                component,
                field,
                value,
                part,
            } => self.set_field_many(&entities, &component, &field, &value, part, gesture),
            PropertyRequest::AddComponentMany {
                entities,
                component,
            } => self.add_component_many(&entities, &component),
            PropertyRequest::RemoveComponentMany {
                entities,
                component,
            } => self.remove_component_many(&entities, &component),
            PropertyRequest::CopyComponents { entity, only } => {
                self.copy_components(entity, only.as_deref());
            }
            PropertyRequest::PasteComponents { entities, only } => {
                self.paste_components(&entities, only.as_deref());
            }
            PropertyRequest::ResetComponent {
                entities,
                component,
            } => self.reset_component(&entities, &component),
            PropertyRequest::HintsOff => self.hints = false,
        }
    }

    /// What the selected entity's assembly says about it, for Properties.
    pub(crate) fn assembly_part(&self) -> Option<AssemblyPart> {
        let key = (
            self.open.document.selection.primary(),
            self.open.document.revision(),
            self.sources,
        );
        if let Some((kept, part)) = &*self.open.assembly_part_kept.borrow()
            && *kept == key
        {
            return part.clone();
        }
        let part = self.work_out_assembly_part();
        *self.open.assembly_part_kept.borrow_mut() = Some((key, part.clone()));
        part
    }

    fn work_out_assembly_part(&self) -> Option<AssemblyPart> {
        let id = self.open.document.selection.primary()?;
        let origin = self.open.document.area().get(id)?.origin.clone()?;
        let lookup = |asset| self.open.assemblies.get(&asset).cloned();
        let expected = Expander::new(&lookup).expected(self.open.document.area(), &origin);
        Some(AssemblyPart {
            name: self.asset_name(origin.assembly),
            expected,
            root: origin.path.is_empty(),
        })
    }

    /// An asset's name, from the Library.
    pub(crate) fn asset_name(&self, id: AssetId) -> String {
        self.open
            .library
            .iter()
            .find(|item| item.id == Some(id))
            .map_or_else(|| "its assembly".to_string(), |item| item.name.clone())
    }

    /// Puts a field of a placed assembly's entity back to the assembly's
    /// value.
    pub fn revert_field(&mut self, entity: EntityId, component: &str, field: &str) {
        let Some(part) = self.part_of(entity) else {
            return;
        };
        let Some(expected) = part.expected else {
            return;
        };
        let label = format!("Revert {component}.{field}");
        let edit = match expected.components.get(component) {
            Some(data) => Edit::SetField {
                entity,
                component: component.to_string(),
                field: field.to_string(),
                value: data.get(field).cloned(),
            },
            None => return,
        };
        self.command(&label, vec![edit], None, None);
    }

    /// Asks the app to make a field's value its assembly's, for every copy.
    pub fn apply_field(&mut self, entity: EntityId, component: &str, field: &str) {
        let Some(data) = self.open.document.area().get(entity) else {
            return;
        };
        let (Some(origin), Some(value)) = (
            data.origin.clone(),
            data.components
                .get(component)
                .and_then(|c| c.get(field))
                .cloned(),
        ) else {
            return;
        };
        self.open.outbox.push(WorkbenchAction::ApplyToAssembly {
            assembly: origin.assembly,
            path: origin.path,
            component: component.to_string(),
            field: field.to_string(),
            value,
        });
    }

    fn part_of(&self, entity: EntityId) -> Option<AssemblyPart> {
        let origin = self.open.document.area().get(entity)?.origin.clone()?;
        let lookup = |asset| self.open.assemblies.get(&asset).cloned();
        let expected = Expander::new(&lookup).expected(self.open.document.area(), &origin);
        Some(AssemblyPart {
            name: self.asset_name(origin.assembly),
            expected,
            root: origin.path.is_empty(),
        })
    }

    /// Turns the placed copy at `root` into plain entities that no longer
    /// follow the assembly; or splits a model at `root` into one entity per
    /// part of it (the file's nodes), each drawing its own part.
    pub fn unpack(&mut self, root: EntityId) {
        let area = self.open.document.area();
        let whole_model = area
            .get(root)
            .filter(|e| e.origin.is_none())
            .and_then(|e| e.get::<Model>())
            .and_then(|m| m.asset)
            .filter(|r| r.part.is_none());
        if let Some(model) = whole_model {
            self.unpack_model(root, model.asset);
            return;
        }
        let edits: Vec<Edit> = area
            .iter()
            .filter(|(_, e)| e.origin.as_ref().is_some_and(|o| o.instance == root))
            .map(|(entity, _)| Edit::SetOrigin {
                entity,
                origin: None,
            })
            .collect();
        let Some(name) = area.get(root).map(|e| e.name.clone()) else {
            return;
        };
        if !edits.is_empty() {
            self.command(&format!("Unpack {name}"), edits, None, None);
        }
    }

    /// One entity per part of the model `asset` drawn by `root`, under it,
    /// placed as in the file; `root` keeps its place and stops drawing the
    /// whole.
    fn unpack_model(&mut self, root: EntityId, asset: AssetId) {
        let Some(info) = self.open.models.get(&asset) else {
            return;
        };
        if info.nodes.is_empty() {
            return;
        }
        let ids: Vec<EntityId> = info.nodes.iter().map(|_| EntityId::random()).collect();
        let mut order = None;
        let mut entities = Vec::new();
        for (i, node) in info.nodes.iter().enumerate() {
            let (scale, rotation, position) = node.local.to_scale_rotation_translation();
            let mut data = EntityData::new(&node.name).with(&Transform {
                position,
                rotation: rotation.normalize(),
                scale,
            });
            if node.has_meshes {
                data.set(&Model {
                    asset: Some(AssetRef {
                        asset,
                        part: Some(node.part),
                    }),
                });
            }
            data.parent = Some(
                node.parent
                    .and_then(|p| ids.get(p).copied())
                    .unwrap_or(root),
            );
            let next = area::order_after(order.as_deref());
            data.order.clone_from(&next);
            order = Some(next);
            entities.push((ids[i], data));
        }
        let name = info.name.clone();
        let mut edits = vec![Edit::RemoveComponent {
            entity: root,
            component: Model::NAME.to_string(),
        }];
        // Parents come before their children (nodes are stored that way),
        // each part spawned as its own little tree.
        for (id, data) in entities {
            edits.push(Edit::Spawn {
                entities: vec![(id, data)],
            });
        }
        self.command(&format!("Unpack {name}"), edits, None, None);
    }

    /// Makes the assembly `id`'s contents known (read from its file),
    /// rebuilding every copy in the area from it while keeping each
    /// copy's changes. Nothing to save if nothing was unsaved: the area's
    /// file is the same. While the game plays it isn't an undo step: the
    /// rebuilt area has the game's entities in it, and Stop rebuilds the
    /// copies of the area from before Play itself.
    pub fn set_assembly(&mut self, id: AssetId, file: Arc<AreaFile>) {
        let had = self.open.assemblies.contains_key(&id);
        let uses = self
            .open
            .document
            .area()
            .iter()
            .any(|(_, e)| e.origin.is_some());
        if !had || !uses {
            self.open.assemblies.insert(id, file);
            self.sources += 1;
            return;
        }
        let name = self.asset_name(id);
        // The copies' changes, worked out against the old contents…
        let collapsed = {
            let lookup = |asset| self.open.assemblies.get(&asset).cloned();
            Expander::new(&lookup).collapse(self.open.document.area(), FileKind::Area)
        };
        let collapsed = match collapsed {
            Ok(collapsed) => collapsed,
            Err(why) => {
                // The copies can't be told apart from their old contents:
                // the new file is used from now on, but nothing is
                // rebuilt, so nothing is lost.
                self.open.assemblies.insert(id, file);
                self.sources += 1;
                self.log(
                    LogLevel::Error,
                    format!("Couldn't update copies of {name}: {why}."),
                );
                return;
            }
        };
        // …laid over the new contents. If they can't be (an assembly it
        // now places isn't read yet), the old file stays in use, so saving
        // never writes the copies against contents they weren't built from.
        let (rebuilt, mended) = {
            let lookup = |asset| {
                if asset == id {
                    Some(file.clone())
                } else {
                    self.open.assemblies.get(&asset).cloned()
                }
            };
            let mut expander = Expander::new(&lookup);
            let rebuilt = expander.expand(&collapsed);
            (rebuilt, expander.mended().to_vec())
        };
        if rebuilt.is_ok() {
            self.open.assemblies.insert(id, file);
            self.sources += 1;
            self.log_mended(&mended);
        }
        let name = self.asset_name(id);
        match rebuilt {
            Ok(area) if area != *self.open.document.area() => {
                let updated = if self.open.playing.is_some() {
                    // Not one of the user's edits, so not a step for Keep
                    // to make again; the game takes it in as a change.
                    self.open.document.replace_by_game(&area);
                    self.open.document.touch();
                    Ok(())
                } else {
                    self.open
                        .document
                        .rebuild(area, &format!("Update copies of {name}"))
                };
                if let Err(why) = updated {
                    self.log(
                        LogLevel::Error,
                        format!("Couldn't update copies of {name}: {why}."),
                    );
                } else {
                    self.log(LogLevel::Info, format!("Updated the copies of {name}."));
                }
            }
            Ok(_) => {}
            Err(why) => self.log(
                LogLevel::Error,
                format!(
                    "Couldn't update copies of {name}: {why}. They keep the version they \
                     were made from."
                ),
            ),
        }
    }

    /// The assembly files known, by ID.
    pub fn assemblies(&self) -> &BTreeMap<AssetId, Arc<AreaFile>> {
        &self.open.assemblies
    }

    /// `file` (an area as read) with its placed assemblies expanded, from
    /// the assemblies known. Entities whose parents led round in a loop (a
    /// damaged file) come out at the top level, and the Console says
    /// which.
    pub fn expand_file(
        &mut self,
        file: &AreaFile,
    ) -> Result<area::Area, area::assembly::AssemblyError> {
        let (area, mended) = {
            let lookup = |asset| self.open.assemblies.get(&asset).cloned();
            let mut expander = Expander::new(&lookup);
            let area = expander.expand(file);
            (area, expander.mended().to_vec())
        };
        if area.is_ok() {
            self.log_mended(&mended);
        }
        area
    }

    /// Says in the Console which entities were taken out of a loop of
    /// parents: the first few each, then how many more, so a file full of
    /// them doesn't fill it.
    fn log_mended(&mut self, mended: &[area::Mended]) {
        const EACH: usize = 5;
        for one in mended.iter().take(EACH) {
            self.log(LogLevel::Warning, format!("{one}."));
        }
        if let Some(more) = mended.len().checked_sub(EACH).filter(|&more| more > 0) {
            self.log(
                LogLevel::Warning,
                format!(
                    "{more} more entities were inside themselves; each is now at the top level."
                ),
            );
        }
    }

    /// The open area as a file: placed assemblies as their root plus what
    /// differs from the assembly.
    pub fn area_file(&self) -> Result<AreaFile, area::assembly::AssemblyError> {
        let lookup = |asset| self.open.assemblies.get(&asset).cloned();
        Expander::new(&lookup).collapse(self.open.document.area(), FileKind::Area)
    }

    /// Places a copy of assembly `id` where the camera looks, selected.
    pub fn place_assembly(&mut self, id: AssetId) {
        let root = EntityId::random();
        let target = self.take_place_target(id);
        let mut placed = EntityData::new(&self.asset_name(id));
        placed.parent = target.and_then(|(_, parent)| parent);
        placed.order = self.open.document.area().order_at_end(placed.parent);
        let entities = {
            let lookup = |asset| self.open.assemblies.get(&asset).cloned();
            Expander::new(&lookup).place(id, root, &placed)
        };
        let mut entities = match entities {
            Ok(e) => e,
            Err(why) => {
                self.log(
                    LogLevel::Error,
                    format!("Couldn't place {}: {why}.", placed.name),
                );
                return;
            }
        };
        // Where it was dropped, or where the camera looks: the copy's own
        // placement.
        let at = target.map_or_else(|| self.viewport.camera.focus(), |(at, _)| at);
        if let Some((_, data)) = entities.iter_mut().find(|(e, _)| *e == root) {
            let mut transform = data.get::<Transform>().unwrap_or_default();
            transform.position = at;
            data.set(&transform);
        }
        let mut selected = Selection::default();
        selected.select_only(root);
        let label = format!("Place {}", placed.name);
        self.command(&label, vec![Edit::Spawn { entities }], None, Some(selected));
    }

    /// Turns `root` and everything under it into a copy of assembly `id`
    /// (just made from them), keeping the root's ID, name and place.
    pub fn make_copy(&mut self, root: EntityId, id: AssetId) {
        let Some(original) = self.open.document.area().get(root).cloned() else {
            return;
        };
        let entities = {
            let lookup = |asset| self.open.assemblies.get(&asset).cloned();
            Expander::new(&lookup).place(id, root, &original)
        };
        let entities = match entities {
            Ok(e) => e,
            Err(why) => {
                self.log(
                    LogLevel::Error,
                    format!("Couldn't make the assembly: {why}."),
                );
                return;
            }
        };
        let label = format!("Create assembly {}", self.asset_name(id));
        let selection = self.open.document.selection.clone();
        self.command(
            &label,
            vec![Edit::Despawn { root }, Edit::Spawn { entities }],
            None,
            Some(selection),
        );
    }

    /// `root` and everything under it as an assembly file (for Create
    /// assembly from selection): the root at the top, nested copies as
    /// copies.
    pub fn assembly_file(&self, root: EntityId) -> Option<AreaFile> {
        let area = self.open.document.area();
        let tree = area.tree(root);
        let inside: std::collections::HashSet<EntityId> = tree.iter().copied().collect();
        let mut part = area::Area::new();
        for id in tree {
            let mut data = area.get(id)?.clone();
            if id == root {
                data.parent = None;
            }
            // Copies placed inside the tree stay copies; a copy at the root
            // (and parts whose copy is outside) become the assembly's own.
            if data
                .origin
                .as_ref()
                .is_some_and(|o| o.instance == root || !inside.contains(&o.instance))
            {
                data.origin = None;
            }
            part.insert(id, data);
        }
        let lookup = |asset| self.open.assemblies.get(&asset).cloned();
        Expander::new(&lookup)
            .collapse(&part, FileKind::Assembly { root })
            .ok()
    }

    /// Carries out what the user asked for in the Area panel.
    pub(crate) fn apply_area_request(&mut self, request: AreaRequest) {
        match request {
            AreaRequest::Select(id) => self.select(id),
            AreaRequest::Toggle(id) => self.open.document.selection.toggle(id),
            AreaRequest::SelectRange(id) => self.select_range(id),
            AreaRequest::Reparent { entity, onto } => {
                // Dragging one of several selected moves them all.
                let moving = if self.open.document.selection.contains(entity) {
                    self.selected_roots()
                } else {
                    vec![entity]
                };
                self.reparent(&moving, onto);
            }
            AreaRequest::Rename { entity, name } => {
                self.command("Rename", vec![Edit::Rename { entity, name }], None, None);
            }
            // Menu commands need the window (to start renaming), so the
            // workbench runs them where it has it.
            AreaRequest::Command(_) => {}
            AreaRequest::PlaceUnder { asset, parent } => {
                let at = self
                    .open
                    .document
                    .area()
                    .world_matrix(parent)
                    .w_axis
                    .truncate();
                self.place_dropped(asset, at, Some(parent));
            }
        }
    }

    /// Selects every entity in the Area panel's order from the one shown
    /// in Properties to `id` (Shift+click).
    pub fn select_range(&mut self, id: EntityId) {
        let Some(anchor) = self.open.document.selection.primary() else {
            self.select(id);
            return;
        };
        let order = self.open.document.area().depth_first();
        let (Some(a), Some(b)) = (
            order.iter().position(|&e| e == anchor),
            order.iter().position(|&e| e == id),
        ) else {
            self.select(id);
            return;
        };
        let (from, to) = (a.min(b), a.max(b));
        self.open
            .document
            .selection
            .select_all(order[from..=to].iter().copied(), anchor);
    }

    /// The selected entities that aren't below another selected one: the
    /// ones to move, copy or delete (their children come with them).
    pub(crate) fn selected_roots(&self) -> Vec<EntityId> {
        let area = self.open.document.area();
        let selection = &self.open.document.selection;
        let order = area.depth_first();
        order
            .into_iter()
            .filter(|&id| selection.contains(id))
            .filter(|&id| {
                // No selected entity above it (one in a loop of parents
                // is above itself).
                let mut above = area.ancestors(id);
                let under_another = above.by_ref().any(|(p, _)| selection.contains(p));
                !under_another && !above.looped()
            })
            .collect()
    }

    /// Moves `ids` under `parent` (or to the top level), at the end of its
    /// children, keeping where each is in the world. Ones that would go
    /// under themselves are left where they are.
    pub fn reparent(&mut self, ids: &[EntityId], parent: Option<EntityId>) {
        let area = self.open.document.area();
        let parent_world = parent.map_or(glam::Mat4::IDENTITY, |p| area.world_matrix(p));
        let inverse = parent_world.inverse();
        let mut edits = Vec::new();
        let mut order = area.order_at_end(parent);
        for &id in ids {
            let Some(entity) = area.get(id) else { continue };
            if entity.parent == parent || parent.is_some_and(|p| area.tree(id).contains(&p)) {
                continue;
            }
            edits.push(Edit::Reparent {
                entity: id,
                parent,
                order: order.clone(),
            });
            order = area::order_after(Some(&order));
            if let Some(transform) = entity.get::<Transform>() {
                let local = inverse * area.world_matrix(id);
                if local.is_finite() && inverse.is_finite() {
                    let (scale, rotation, position) = local.to_scale_rotation_translation();
                    let kept = Transform {
                        position,
                        rotation: rotation.normalize(),
                        scale,
                    };
                    if kept != transform {
                        edits.extend(component_edits(
                            id,
                            entity.components.get(Transform::NAME),
                            &kept,
                        ));
                    }
                }
            }
        }
        if edits.is_empty() {
            return;
        }
        let label = match parent.and_then(|p| area.get(p)) {
            Some(p) => format!("Move under {}", p.name),
            None => "Move to the top level".to_string(),
        };
        self.command(&label, edits, None, None);
    }

    /// Copies the selected entities (with everything below them) and
    /// selects the copies, which sit beside the originals (Ctrl+D).
    pub fn duplicate_selection(&mut self) {
        let roots = self.selected_roots();
        if roots.is_empty() {
            return;
        }
        let area = self.open.document.area();
        let mut edits = Vec::new();
        let mut copies = Vec::new();
        let mut orders: std::collections::HashMap<Option<EntityId>, String> = Default::default();
        for root in roots {
            let ids = area.tree(root);
            // Parts of placed assemblies get the IDs a new copy of their
            // assembly would (so they stay copies); everything else gets a
            // new random one. Parts whose copy's root stays behind become
            // plain.
            let inside: std::collections::HashSet<EntityId> = ids.iter().copied().collect();
            let part_of = |id: EntityId| {
                area.get(id)
                    .and_then(|e| e.origin.as_ref())
                    .filter(|o| o.instance != id && inside.contains(&o.instance))
            };
            let mut new_ids: std::collections::HashMap<EntityId, EntityId> = ids
                .iter()
                .filter(|&&id| part_of(id).is_none())
                .map(|&id| (id, EntityId::random()))
                .collect();
            for &id in &ids {
                if let Some(o) = part_of(id) {
                    let new_instance = new_ids.get(&o.instance).copied().unwrap_or(o.instance);
                    new_ids.insert(id, area::assembly::derive(new_instance, &o.path));
                }
            }
            let mut entities = Vec::new();
            for id in &ids {
                let Some(original) = area.get(*id) else {
                    continue;
                };
                let mut copy = original.clone();
                if *id == root {
                    let parent = original.parent;
                    let order = orders
                        .entry(parent)
                        .or_insert_with(|| area.order_at_end(parent));
                    copy.order.clone_from(order);
                    *order = area::order_after(Some(order));
                } else {
                    copy.parent = original
                        .parent
                        .map(|p| new_ids.get(&p).copied().unwrap_or(p));
                }
                // Still part of a copied assembly, or plain if its copy's
                // root stays behind.
                copy.origin = original
                    .origin
                    .as_ref()
                    .filter(|o| inside.contains(&o.instance))
                    .map(|o| area::Origin {
                        instance: new_ids[&o.instance],
                        ..o.clone()
                    });
                // References inside the copied tree point at the copies.
                for data in copy.components.values_mut() {
                    for value in data.fields.values_mut() {
                        if let reflect::Value::Entity(Some(target)) = value
                            && let Some(&moved) = new_ids.get(target)
                        {
                            *target = moved;
                        }
                    }
                }
                entities.push((new_ids[id], copy));
            }
            copies.push(new_ids[&root]);
            edits.push(Edit::Spawn { entities });
        }
        let Some(&primary) = copies.last() else {
            return;
        };
        let mut selected = Selection::default();
        selected.select_all(copies.iter().copied(), primary);
        let label = if copies.len() == 1 {
            "Duplicate".to_string()
        } else {
            format!("Duplicate {} entities", copies.len())
        };
        self.command(&label, edits, None, Some(selected));
    }

    /// Deletes the selected entities and everything below them, as one step
    /// (Delete).
    pub fn delete_selection(&mut self) {
        let roots = self.selected_roots();
        match roots.as_slice() {
            [] => {}
            [one] => self.delete(*one),
            many => {
                let label = format!("Delete {} entities", many.len());
                let edits = area::assembly::despawn(self.open.document.area(), many);
                let left = left_behind(&edits);
                if self.command(&label, edits, None, None) {
                    self.log(LogLevel::Info, format!("{label}.{left}"));
                }
            }
        }
    }

    /// Points the camera at the selected entities, so they fill the view
    /// (F). Models count with their whole size, other entities as a
    /// metre-wide box around them.
    pub fn frame_selection(&mut self) {
        let area = self.open.document.area();
        let mut bounds: Option<(Vec3, Vec3)> = None;
        let mut add = |lo: Vec3, hi: Vec3| {
            bounds = Some(bounds.map_or((lo, hi), |(a, b)| (a.min(lo), b.max(hi))));
        };
        for id in self.open.document.selection.iter() {
            let world = area.world_matrix(id);
            let model = area
                .get(id)
                .and_then(|e| e.get::<Model>())
                .and_then(|m| m.asset)
                // A part of an unpacked model sits where its node is, and
                // the whole model's box would be in the wrong place: it's
                // framed as a point, as its outline is drawn.
                .filter(|r| r.part.is_none())
                .and_then(|r| self.open.models.get(&r.asset));
            match model {
                Some(info) => {
                    for i in 0..8 {
                        let corner = Vec3::new(
                            if i & 1 == 0 { info.min.x } else { info.max.x },
                            if i & 2 == 0 { info.min.y } else { info.max.y },
                            if i & 4 == 0 { info.min.z } else { info.max.z },
                        );
                        let p = world.transform_point3(corner);
                        add(p, p);
                    }
                }
                None => {
                    let p = world.transform_point3(Vec3::ZERO);
                    add(p - Vec3::splat(0.5), p + Vec3::splat(0.5));
                }
            }
        }
        if let Some((min, max)) = bounds {
            self.viewport.camera.frame(min, max);
        }
    }

    /// Deletes `id` and everything below it.
    pub fn delete(&mut self, id: EntityId) {
        let area = self.open.document.area();
        let Some(entity) = area.get(id) else {
            return;
        };
        let label = format!("Delete {}", entity.name);
        let extra = match area.tree(id).len() - 1 {
            0 => String::new(),
            1 => " and 1 entity under it".into(),
            n => format!(" and {n} entities under it"),
        };
        let edits = area::assembly::despawn(area, &[id]);
        let left = left_behind(&edits);
        if self.command(&label, edits, None, None) {
            self.log(LogLevel::Info, format!("{label}{extra}.{left}"));
        }
    }

    /// Moves `id` to where the camera looks: a point light above that
    /// point, a spotlight to the camera, shining the same way, anything
    /// else to the point itself.
    pub fn move_here(&mut self, id: EntityId) {
        let camera = self.viewport.camera;
        let Some(entity) = self.open.document.area().get(id) else {
            return;
        };
        let mut transform = entity.get::<Transform>().unwrap_or_default();
        match entity.get::<Light>().map(|l| l.kind) {
            Some(LightKind::Spot) => {
                transform.position = camera.viewpoint();
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
            // An angle that isn't a number (a damaged import cache) takes
            // the importer's standard: as a bound of `clamp` it would
            // stop the program. `outer` is a number in the cone's range
            // before it bounds `inner`.
            let number = |v: f32, standard: f32| if v.is_finite() { v } else { standard };
            let (narrow, wide) = Light::CONE_RANGE;
            let outer = number(outer, std::f32::consts::FRAC_PI_4)
                .to_degrees()
                .clamp(narrow, wide);
            let inner = number(inner, 0.0).to_degrees().clamp(0.0, outer);
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
    let data = EntityData::new(&name)
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
    FromFile::Lamp(Box::new(data))
}
