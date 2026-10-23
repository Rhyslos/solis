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
use area::builtin::{Light, LightKind, Model, Transform};
use area::text::{AreaFile, FileKind};
use area::{Edit, EntityData, MergeKey};
use glam::{Quat, Vec3};
use reflect::AssetId;
use reflect::{AssetRef, Component, ComponentData, EntityId};
use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

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

    /// Makes a component known to the editor, so Properties shows it and
    /// Add component offers it: how game code's components (Lust, Lua,
    /// Rust) join in. Refuses a name already taken or a broken schema.
    pub fn register_component(
        &mut self,
        schema: reflect::ComponentSchema,
    ) -> Result<(), reflect::RegistryError> {
        self.document.registry.register(schema)
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
        let mut model = model;
        model.order = self.document.area().order_at_end(None);
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

    fn spawn_selected(&mut self, mut data: EntityData) -> EntityId {
        let id = EntityId::random();
        data.order = self.document.area().order_at_end(data.parent);
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
            PropertyRequest::AddComponent { entity, component } => {
                let Some(schema) = self.document.registry.get(&component) else {
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
        }
    }

    /// What the selected entity's assembly says about it, for Properties.
    pub(crate) fn assembly_part(&self) -> Option<AssemblyPart> {
        let id = self.document.selection.primary()?;
        let origin = self.document.area().get(id)?.origin.clone()?;
        let lookup = |asset| self.assemblies.get(&asset).cloned();
        let expected = Expander::new(&lookup).expected(self.document.area(), &origin);
        Some(AssemblyPart {
            name: self.asset_name(origin.assembly),
            expected,
            root: origin.path.is_empty(),
        })
    }

    /// An asset's name, from the Library.
    pub(crate) fn asset_name(&self, id: AssetId) -> String {
        self.library
            .iter()
            .find(|item| item.id == id)
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
        let Some(data) = self.document.area().get(entity) else {
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
        self.outbox.push(WorkbenchAction::ApplyToAssembly {
            assembly: origin.assembly,
            path: origin.path,
            component: component.to_string(),
            field: field.to_string(),
            value,
        });
    }

    fn part_of(&self, entity: EntityId) -> Option<AssemblyPart> {
        let origin = self.document.area().get(entity)?.origin.clone()?;
        let lookup = |asset| self.assemblies.get(&asset).cloned();
        let expected = Expander::new(&lookup).expected(self.document.area(), &origin);
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
        let area = self.document.area();
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
        let Some(info) = self.models.get(&asset) else {
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
    /// copy's changes. Not an undo step if nothing was unsaved: the area's
    /// file is the same.
    pub fn set_assembly(&mut self, id: AssetId, file: Arc<AreaFile>) {
        let had = self.assemblies.contains_key(&id);
        let uses = self.document.area().iter().any(|(_, e)| e.origin.is_some());
        if !had || !uses {
            self.assemblies.insert(id, file);
            return;
        }
        // The copies' changes, worked out against the old contents…
        let collapsed = {
            let lookup = |asset| self.assemblies.get(&asset).cloned();
            Expander::new(&lookup).collapse(self.document.area(), FileKind::Area)
        };
        self.assemblies.insert(id, file);
        let Ok(collapsed) = collapsed else { return };
        // …laid over the new contents.
        let rebuilt = {
            let lookup = |asset| self.assemblies.get(&asset).cloned();
            Expander::new(&lookup).expand(&collapsed)
        };
        let name = self.asset_name(id);
        match rebuilt {
            Ok(area) if area != *self.document.area() => {
                if let Err(why) = self
                    .document
                    .rebuild(area, &format!("Update copies of {name}"))
                {
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
                format!("Couldn't update copies of {name}: {why}."),
            ),
        }
    }

    /// The assembly files known, by ID.
    pub fn assemblies(&self) -> &BTreeMap<AssetId, Arc<AreaFile>> {
        &self.assemblies
    }

    /// `file` (an area as read) with its placed assemblies expanded, from
    /// the assemblies known.
    pub fn expand_file(
        &self,
        file: &AreaFile,
    ) -> Result<area::Area, area::assembly::AssemblyError> {
        let lookup = |asset| self.assemblies.get(&asset).cloned();
        Expander::new(&lookup).expand(file)
    }

    /// The open area as a file: placed assemblies as their root plus what
    /// differs from the assembly.
    pub fn area_file(&self) -> Result<AreaFile, area::assembly::AssemblyError> {
        let lookup = |asset| self.assemblies.get(&asset).cloned();
        Expander::new(&lookup).collapse(self.document.area(), FileKind::Area)
    }

    /// Places a copy of assembly `id` where the camera looks, selected.
    pub fn place_assembly(&mut self, id: AssetId) {
        let root = EntityId::random();
        let mut placed = EntityData::new(&self.asset_name(id));
        placed.order = self.document.area().order_at_end(None);
        let entities = {
            let lookup = |asset| self.assemblies.get(&asset).cloned();
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
        // Where the camera looks: the copy's own placement.
        let focus = self.viewport.camera.focus();
        if let Some((_, data)) = entities.iter_mut().find(|(e, _)| *e == root) {
            let mut transform = data.get::<Transform>().unwrap_or_default();
            transform.position = focus;
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
        let Some(original) = self.document.area().get(root).cloned() else {
            return;
        };
        let entities = {
            let lookup = |asset| self.assemblies.get(&asset).cloned();
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
        let selection = self.document.selection.clone();
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
        let area = self.document.area();
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
        let lookup = |asset| self.assemblies.get(&asset).cloned();
        Expander::new(&lookup)
            .collapse(&part, FileKind::Assembly { root })
            .ok()
    }

    /// Carries out what the user asked for in the Area panel.
    pub(crate) fn apply_area_request(&mut self, request: AreaRequest) {
        match request {
            AreaRequest::Select(id) => self.select(id),
            AreaRequest::Toggle(id) => self.document.selection.toggle(id),
            AreaRequest::SelectRange(id) => self.select_range(id),
            AreaRequest::Delete(id) => self.delete(id),
            AreaRequest::Reparent { entity, onto } => {
                // Dragging one of several selected moves them all.
                let moving = if self.document.selection.contains(entity) {
                    self.selected_roots()
                } else {
                    vec![entity]
                };
                self.reparent(&moving, onto);
            }
            AreaRequest::Rename { entity, name } => {
                self.command("Rename", vec![Edit::Rename { entity, name }], None, None);
            }
        }
    }

    /// Selects every entity in the Area panel's order from the one shown
    /// in Properties to `id` (Shift+click).
    pub fn select_range(&mut self, id: EntityId) {
        let Some(anchor) = self.document.selection.primary() else {
            self.select(id);
            return;
        };
        let order = self.document.area().depth_first();
        let (Some(a), Some(b)) = (
            order.iter().position(|&e| e == anchor),
            order.iter().position(|&e| e == id),
        ) else {
            self.select(id);
            return;
        };
        let (from, to) = (a.min(b), a.max(b));
        self.document
            .selection
            .select_all(order[from..=to].iter().copied(), anchor);
    }

    /// The selected entities that aren't below another selected one: the
    /// ones to move, copy or delete (their children come with them).
    pub(crate) fn selected_roots(&self) -> Vec<EntityId> {
        let area = self.document.area();
        let selection = &self.document.selection;
        let order = area.depth_first();
        order
            .into_iter()
            .filter(|&id| selection.contains(id))
            .filter(|&id| {
                // No selected entity above it.
                let mut at = area.get(id).and_then(|e| e.parent);
                let mut steps = 0;
                while let Some(p) = at {
                    if selection.contains(p) || steps > area.len() {
                        return false;
                    }
                    steps += 1;
                    at = area.get(p).and_then(|e| e.parent);
                }
                true
            })
            .collect()
    }

    /// Moves `ids` under `parent` (or to the top level), at the end of its
    /// children, keeping where each is in the world. Ones that would go
    /// under themselves are left where they are.
    pub fn reparent(&mut self, ids: &[EntityId], parent: Option<EntityId>) {
        let area = self.document.area();
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
        let area = self.document.area();
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
                let edits = many.iter().map(|&root| Edit::Despawn { root }).collect();
                if self.command(&label, edits, None, None) {
                    self.log(LogLevel::Info, format!("{label}."));
                }
            }
        }
    }

    /// Points the camera at the selected entities, so they fill the view
    /// (F). Models count with their whole size, other entities as a
    /// metre-wide box around them.
    pub fn frame_selection(&mut self) {
        let area = self.document.area();
        let mut bounds: Option<(Vec3, Vec3)> = None;
        let mut add = |lo: Vec3, hi: Vec3| {
            bounds = Some(bounds.map_or((lo, hi), |(a, b)| (a.min(lo), b.max(hi))));
        };
        for id in self.document.selection.iter() {
            let world = area.world_matrix(id);
            let model = area
                .get(id)
                .and_then(|e| e.get::<Model>())
                .and_then(|m| m.asset)
                .and_then(|r| self.models.get(&r.asset));
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
