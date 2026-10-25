//! Assemblies placed in areas ([decision 0020](../../../docs/decisions/0020-scene-files-in-lust-style.md)):
//! a placed assembly is written as its root entity plus overrides, never a
//! copy, and expanded into ordinary entities while it's edited.
//!
//! Each entity inside a placed copy gets an ID worked out from the copy's
//! root and the entity's ID inside the assembly ([`derive()`]), so the same
//! file always expands to the same IDs: undo, selection and references to
//! them keep working when the assembly changes, and overrides find their
//! entity again.
//!
//! Expanding counts entities before making any, refuses assemblies that
//! place each other in a loop (naming the chain), and stops at
//! [`MAX_NESTING`] levels, so no file can make the editor explode.

use crate::document::{Area, EntityData, Origin};
use crate::text::{AreaFile, FileKind, MAX_ENTITIES, MAX_NESTING, Override};
use reflect::{AssetId, EntityId, Value};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

/// The ID of the entity reached by `path` inside the copy whose root is
/// `instance` (the root itself for an empty path).
pub fn derive(instance: EntityId, path: &[EntityId]) -> EntityId {
    let mut id = instance.0.raw();
    for step in path {
        let mut z = id ^ step.0.raw().rotate_left(29) ^ 0xA55E_3B1E_D000_0001;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        id = z ^ (z >> 31);
    }
    // Zero isn't an ID; nudging it keeps the result fixed for this path.
    EntityId(reflect::Id::from_raw(id).unwrap_or(instance.0))
}

/// Why placed assemblies couldn't be expanded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssemblyError {
    /// The assembly isn't in the project (or couldn't be read).
    Missing(AssetId),
    /// The file is an area, not an assembly.
    NotAnAssembly(AssetId),
    /// Assemblies place each other in a loop: this chain.
    Loop(Vec<AssetId>),
    /// Placed inside each other more than [`MAX_NESTING`] deep.
    TooDeep,
    /// Expanding would make more than [`MAX_ENTITIES`] entities.
    TooBig,
    /// Two entities would share an ID.
    Clash(EntityId),
}

impl fmt::Display for AssemblyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(id) => write!(f, "the assembly {id} isn't in the project"),
            Self::NotAnAssembly(id) => write!(f, "{id} is an area, not an assembly"),
            Self::Loop(chain) => {
                let names: Vec<String> = chain.iter().map(ToString::to_string).collect();
                write!(
                    f,
                    "assemblies place each other in a loop: {}",
                    names.join(" → ")
                )
            }
            Self::TooDeep => write!(f, "assemblies are placed more than {MAX_NESTING} deep"),
            Self::TooBig => write!(f, "it would make more than {MAX_ENTITIES} entities"),
            Self::Clash(id) => write!(f, "two entities would share the ID {id}"),
        }
    }
}

impl std::error::Error for AssemblyError {}

/// Finds assembly files by ID (the app reads them from the project).
pub type Lookup<'a> = &'a dyn Fn(AssetId) -> Option<Arc<AreaFile>>;

/// Expands and collapses placed assemblies, remembering each assembly's
/// expanded contents while it lives.
pub struct Expander<'a> {
    lookup: Lookup<'a>,
    expanded: HashMap<AssetId, (Arc<Area>, EntityId)>,
    sizes: HashMap<AssetId, usize>,
}

impl fmt::Debug for Expander<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Expander({} assemblies)", self.expanded.len())
    }
}

/// The way to `id` inside an expanded assembly whose root is `root`.
fn path_in(area: &Area, root: EntityId, id: EntityId) -> Vec<EntityId> {
    if id == root {
        return Vec::new();
    }
    match area.get(id).and_then(|e| e.origin.as_ref()) {
        Some(o) => std::iter::once(o.instance)
            .chain(o.path.iter().copied())
            .collect(),
        None => vec![id],
    }
}

/// Points every entity reference in `value` that `map` knows at its new ID.
fn remap(value: &mut Value, map: &HashMap<EntityId, EntityId>) {
    match value {
        Value::Entity(Some(id)) => {
            if let Some(new) = map.get(id) {
                *id = *new;
            }
        }
        Value::List(items) => items.iter_mut().for_each(|v| remap(v, map)),
        _ => {}
    }
}

impl<'a> Expander<'a> {
    /// An expander reading assemblies through `lookup`.
    pub fn new(lookup: Lookup<'a>) -> Self {
        Self {
            lookup,
            expanded: HashMap::new(),
            sizes: HashMap::new(),
        }
    }

    /// The file of assembly `id`.
    fn file(&self, id: AssetId) -> Result<(Arc<AreaFile>, EntityId), AssemblyError> {
        let file = (self.lookup)(id).ok_or(AssemblyError::Missing(id))?;
        match file.kind {
            FileKind::Assembly { root } => Ok((file, root)),
            FileKind::Area => Err(AssemblyError::NotAnAssembly(id)),
        }
    }

    /// How many entities assembly `id` expands to, counted without making
    /// any.
    fn size(&mut self, id: AssetId, chain: &mut Vec<AssetId>) -> Result<usize, AssemblyError> {
        if let Some(&n) = self.sizes.get(&id) {
            return Ok(n);
        }
        if chain.contains(&id) {
            chain.push(id);
            return Err(AssemblyError::Loop(chain.clone()));
        }
        if chain.len() >= MAX_NESTING {
            return Err(AssemblyError::TooDeep);
        }
        let (file, _) = self.file(id)?;
        chain.push(id);
        let n = self.count(&file, chain)?;
        chain.pop();
        self.sizes.insert(id, n);
        Ok(n)
    }

    fn count(&mut self, file: &AreaFile, chain: &mut Vec<AssetId>) -> Result<usize, AssemblyError> {
        let mut total = 0usize;
        for (_, entity) in file.area.iter() {
            let n = match &entity.origin {
                Some(o) if o.path.is_empty() => self.size(o.assembly, chain)?,
                _ => 1,
            };
            total = total.checked_add(n).ok_or(AssemblyError::TooBig)?;
            if total > MAX_ENTITIES {
                return Err(AssemblyError::TooBig);
            }
        }
        Ok(total)
    }

    /// Assembly `id`'s contents with its own placed assemblies expanded,
    /// and its root.
    pub fn assembly(&mut self, id: AssetId) -> Result<(Arc<Area>, EntityId), AssemblyError> {
        self.assembly_in(id, &mut Vec::new())
    }

    fn assembly_in(
        &mut self,
        id: AssetId,
        chain: &mut Vec<AssetId>,
    ) -> Result<(Arc<Area>, EntityId), AssemblyError> {
        if let Some(found) = self.expanded.get(&id) {
            return Ok(found.clone());
        }
        self.size(id, &mut chain.clone())?;
        let (file, root) = self.file(id)?;
        chain.push(id);
        let area = self.expand_in(&file, chain)?;
        chain.pop();
        let found = (Arc::new(area), root);
        self.expanded.insert(id, found.clone());
        Ok(found)
    }

    /// Every placed assembly in `file` expanded into ordinary entities,
    /// with its overrides.
    pub fn expand(&mut self, file: &AreaFile) -> Result<Area, AssemblyError> {
        self.count(file, &mut Vec::new())?;
        self.expand_in(file, &mut Vec::new())
    }

    fn expand_in(
        &mut self,
        file: &AreaFile,
        chain: &mut Vec<AssetId>,
    ) -> Result<Area, AssemblyError> {
        let mut area = Area::new();
        for (id, entity) in file.area.iter() {
            let entities = match &entity.origin {
                Some(o) if o.path.is_empty() => {
                    let overrides: HashMap<Vec<EntityId>, Override> = file
                        .overrides
                        .range((id, Vec::new())..)
                        .take_while(|((root, _), _)| *root == id)
                        .map(|((_, path), o)| (path.clone(), o.clone()))
                        .collect();
                    self.place_in(o.assembly, id, entity, &overrides, chain)?
                }
                _ => vec![(
                    id,
                    EntityData {
                        origin: None,
                        ..entity.clone()
                    },
                )],
            };
            for (new, data) in entities {
                if area.insert(new, data).is_some() {
                    return Err(AssemblyError::Clash(new));
                }
            }
        }
        Ok(area)
    }

    /// The entities of a copy of `assembly` whose root is `root`: `placed`
    /// gives the root's name, parent and order; `overrides` this copy's
    /// changes, by path (kept even when their part is gone from the
    /// assembly).
    fn place_in(
        &mut self,
        assembly: AssetId,
        root: EntityId,
        placed: &EntityData,
        overrides: &HashMap<Vec<EntityId>, Override>,
        chain: &mut Vec<AssetId>,
    ) -> Result<Vec<(EntityId, EntityData)>, AssemblyError> {
        let (contents, inner_root) = self.assembly_in(assembly, chain)?;
        let order = contents.depth_first();
        let paths: HashMap<EntityId, Vec<EntityId>> = order
            .iter()
            .map(|&id| (id, path_in(&contents, inner_root, id)))
            .collect();
        let ids: HashMap<EntityId, EntityId> = paths
            .iter()
            .map(|(&id, path)| (id, derive(root, path)))
            .collect();
        let mut out = Vec::with_capacity(order.len());
        let mut skipped: HashSet<EntityId> = HashSet::new();
        let mut found: HashSet<Vec<EntityId>> = HashSet::new();
        for id in order {
            let Some(original) = contents.get(id) else {
                continue;
            };
            let path = &paths[&id];
            found.insert(path.clone());
            // A removed part takes everything under it.
            if original.parent.is_some_and(|p| skipped.contains(&p)) {
                skipped.insert(id);
                continue;
            }
            let changes = overrides.get(path);
            if changes.is_some_and(|c| c.removed) {
                skipped.insert(id);
                continue;
            }
            let mut data = original.clone();
            if path.is_empty() {
                data.name.clone_from(&placed.name);
                data.parent = placed.parent;
                data.order.clone_from(&placed.order);
            } else {
                data.parent = original.parent.and_then(|p| ids.get(&p).copied());
            }
            for component in data.components.values_mut() {
                for value in component.fields.values_mut() {
                    remap(value, &ids);
                }
            }
            if let Some(changes) = changes {
                apply(&mut data, changes);
            }
            data.origin = Some(Origin {
                assembly,
                instance: root,
                path: path.clone(),
            });
            out.push((ids[&id], data));
        }
        // Overrides whose part is gone from the assembly are kept as
        // entities of their own, so nothing is lost; the editor warns.
        for (path, changes) in overrides {
            if path.is_empty() || found.contains(path) || changes.removed {
                continue;
            }
            let mut data = EntityData::new(changes.name.as_deref().unwrap_or("Missing part"));
            data.parent = Some(changes.parent.unwrap_or(root));
            if let Some(order) = &changes.order {
                data.order.clone_from(order);
            }
            data.components.clone_from(&changes.components);
            data.origin = Some(Origin {
                assembly,
                instance: root,
                path: path.clone(),
            });
            out.push((derive(root, path), data));
        }
        Ok(out)
    }

    /// A new copy of `assembly` with its root at `root`, named, parented
    /// and ordered as `placed` says, with no changes.
    pub fn place(
        &mut self,
        assembly: AssetId,
        root: EntityId,
        placed: &EntityData,
    ) -> Result<Vec<(EntityId, EntityData)>, AssemblyError> {
        self.size(assembly, &mut Vec::new())?;
        self.place_in(assembly, root, placed, &HashMap::new(), &mut Vec::new())
    }

    /// What the assembly says the entity at `origin` should be, before its
    /// copy's changes (for showing changes in bold, and reverting them).
    /// `None` for a part that's gone from the assembly.
    pub fn expected(&mut self, area: &Area, origin: &Origin) -> Option<EntityData> {
        let placed = area.get(origin.instance)?;
        let copy = self.place(origin.assembly, origin.instance, placed).ok()?;
        copy.into_iter()
            .find(|(_, e)| e.origin.as_ref().is_some_and(|o| o.path == origin.path))
            .map(|(_, e)| e)
    }

    /// `area` as a file: placed assemblies become their root plus what
    /// differs from the assembly; entities that belong to no placed
    /// assembly stay as they are.
    pub fn collapse(&mut self, area: &Area, kind: FileKind) -> Result<AreaFile, AssemblyError> {
        let mut file = AreaFile {
            kind,
            area: Area::new(),
            overrides: Default::default(),
        };
        let is_root = |id: EntityId| {
            area.get(id)
                .and_then(|e| e.origin.as_ref())
                .is_some_and(|o| o.path.is_empty() && o.instance == id)
        };
        let mut roots = Vec::new();
        for (id, entity) in area.iter() {
            match &entity.origin {
                Some(o) if o.path.is_empty() && o.instance == id => {
                    let mut placed = EntityData::new(&entity.name);
                    placed.parent = entity.parent;
                    placed.order.clone_from(&entity.order);
                    placed.origin = Some(o.clone());
                    file.area.insert(id, placed);
                    roots.push((id, o.assembly));
                }
                Some(o) if is_root(o.instance) => {} // part of a placed copy
                _ => {
                    file.area.insert(
                        id,
                        EntityData {
                            origin: None,
                            ..entity.clone()
                        },
                    );
                }
            }
        }
        for (root, assembly) in roots {
            let placed = file.area.get(root).cloned().unwrap_or_default();
            let expected = self.place(assembly, root, &placed)?;
            let mut known: HashSet<Vec<EntityId>> = HashSet::new();
            let present: HashSet<EntityId> = expected
                .iter()
                .map(|(id, _)| *id)
                .filter(|id| area.contains(*id))
                .collect();
            for (id, wanted) in &expected {
                let path = wanted
                    .origin
                    .as_ref()
                    .map(|o| o.path.clone())
                    .unwrap_or_default();
                known.insert(path.clone());
                let changes = match area.get(*id) {
                    // Gone: removed, unless its parent went too.
                    None => {
                        let parent_gone = wanted.parent.is_some_and(|p| {
                            expected.iter().any(|(e, _)| *e == p) && !present.contains(&p)
                        });
                        if parent_gone {
                            continue;
                        }
                        Override {
                            removed: true,
                            ..Override::default()
                        }
                    }
                    Some(actual) => difference(wanted, actual, path.is_empty()),
                };
                if !changes.is_empty() {
                    file.overrides.insert((root, path), changes);
                }
            }
            // Parts whose place in the assembly is gone are written whole.
            for (_, entity) in area.iter() {
                let Some(o) = &entity.origin else { continue };
                if o.instance != root || o.path.is_empty() || known.contains(&o.path) {
                    continue;
                }
                file.overrides.insert(
                    (root, o.path.clone()),
                    Override {
                        name: Some(entity.name.clone()),
                        parent: entity.parent,
                        order: Some(entity.order.clone()),
                        components: entity.components.clone(),
                        ..Override::default()
                    },
                );
            }
        }
        Ok(file)
    }
}

/// Sets `component.field` of the entity at `path` inside assembly `file`
/// (Apply to assembly): on the entity itself, or, for an entity inside an
/// assembly placed in this one, as that copy's override.
pub fn set_in_file(
    file: &mut AreaFile,
    path: &[EntityId],
    component: &str,
    field: &str,
    value: Value,
) -> bool {
    let target = match (file.kind, path) {
        (FileKind::Assembly { root }, []) => vec![root],
        (_, path) => path.to_vec(),
    };
    match target.as_slice() {
        [] => false,
        [id] => match file.area.get_mut(*id) {
            Some(entity) if entity.origin.is_none() => {
                entity
                    .components
                    .entry(component.to_string())
                    .or_default()
                    .fields
                    .insert(field.to_string(), value);
                true
            }
            // The root of a copy placed in the assembly: its own override.
            Some(_) => {
                set_override(file, *id, Vec::new(), component, field, value);
                true
            }
            None => false,
        },
        [placed, inside @ ..] => {
            if !file.area.contains(*placed) {
                return false;
            }
            set_override(file, *placed, inside.to_vec(), component, field, value);
            true
        }
    }
}

fn set_override(
    file: &mut AreaFile,
    root: EntityId,
    path: Vec<EntityId>,
    component: &str,
    field: &str,
    value: Value,
) {
    file.overrides
        .entry((root, path))
        .or_default()
        .components
        .entry(component.to_string())
        .or_default()
        .fields
        .insert(field.to_string(), value);
}

/// Makes `data` as `changes` say.
fn apply(data: &mut EntityData, changes: &Override) {
    if let Some(name) = &changes.name {
        data.name.clone_from(name);
    }
    if let Some(parent) = changes.parent {
        data.parent = Some(parent);
    }
    if let Some(order) = &changes.order {
        data.order.clone_from(order);
    }
    for component in &changes.remove {
        data.components.remove(component);
    }
    for (component, fields) in &changes.components {
        let target = data.components.entry(component.clone()).or_default();
        for (field, value) in &fields.fields {
            target.fields.insert(field.clone(), value.clone());
        }
    }
}

/// What changed from `wanted` (the assembly's) to `actual` (the area's).
/// The root's name, parent and order belong to the area, not the
/// assembly, so they're never overrides.
fn difference(wanted: &EntityData, actual: &EntityData, root: bool) -> Override {
    let mut changes = Override::default();
    if !root {
        if actual.name != wanted.name {
            changes.name = Some(actual.name.clone());
        }
        if actual.parent != wanted.parent {
            changes.parent = actual.parent;
        }
        if actual.order != wanted.order {
            changes.order = Some(actual.order.clone());
        }
    }
    for component in wanted.components.keys() {
        if !actual.components.contains_key(component) {
            changes.remove.push(component.clone());
        }
    }
    for (component, data) in &actual.components {
        let before = wanted.components.get(component);
        let mut fields = reflect::ComponentData::default();
        for (field, value) in &data.fields {
            if before.and_then(|b| b.fields.get(field)) != Some(value) {
                fields.fields.insert(field.clone(), value.clone());
            }
        }
        // A component added in this copy is kept even with no fields.
        if !fields.fields.is_empty() || before.is_none() {
            changes.components.insert(component.clone(), fields);
        }
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::{Light, Transform};
    use crate::text;

    fn id(n: u64) -> EntityId {
        EntityId(reflect::Id::from_raw(n).unwrap())
    }

    fn asset(n: u64) -> AssetId {
        AssetId(reflect::Id::from_raw(n).unwrap())
    }

    /// A lamp post: a post with a lamp under it.
    fn lamp_post() -> AreaFile {
        let mut area = Area::new();
        area.insert(
            id(1),
            EntityData::new("Lamp post").with(&Transform::default()),
        );
        let mut lamp = EntityData::new("Lamp")
            .with(&Light::point())
            .with(&Transform {
                position: glam::Vec3::Z * 3.0,
                ..Transform::default()
            });
        lamp.parent = Some(id(1));
        area.insert(id(2), lamp);
        AreaFile {
            kind: FileKind::Assembly { root: id(1) },
            area,
            overrides: Default::default(),
        }
    }

    /// A street: two lamp posts, the second with a brighter lamp.
    fn street() -> AreaFile {
        let mut area = Area::new();
        area.insert(id(10), EntityData::new("Street"));
        for (n, x) in [(11, 0.0f32), (12, 5.0)] {
            let mut post = EntityData::new("Post");
            post.parent = Some(id(10));
            post.origin = Some(Origin {
                assembly: asset(100),
                instance: id(n),
                path: Vec::new(),
            });
            area.insert(id(n), post);
            let _ = x;
        }
        let mut overrides = std::collections::BTreeMap::new();
        let mut light = reflect::ComponentData::default();
        light.fields.insert("strength".into(), Value::Float(5000.0));
        overrides.insert(
            (id(12), vec![id(2)]),
            Override {
                components: [("Light".to_string(), light)].into(),
                ..Override::default()
            },
        );
        AreaFile {
            kind: FileKind::Assembly { root: id(10) },
            area,
            overrides,
        }
    }

    fn library(files: Vec<(AssetId, AreaFile)>) -> impl Fn(AssetId) -> Option<Arc<AreaFile>> {
        let map: HashMap<AssetId, Arc<AreaFile>> =
            files.into_iter().map(|(a, f)| (a, Arc::new(f))).collect();
        move |id| map.get(&id).cloned()
    }

    /// An area with one street placed at `root`.
    fn town(root: EntityId) -> AreaFile {
        let mut area = Area::new();
        let mut placed = EntityData::new("Main street");
        placed.origin = Some(Origin {
            assembly: asset(200),
            instance: root,
            path: Vec::new(),
        });
        area.insert(root, placed);
        AreaFile::plain(area)
    }

    fn strength(area: &Area, entity: EntityId) -> f32 {
        area.get(entity).unwrap().get::<Light>().unwrap().strength
    }

    #[test]
    fn nested_assemblies_expand_with_overrides_innermost_first() {
        let lib = library(vec![(asset(100), lamp_post()), (asset(200), street())]);
        let mut expander = Expander::new(&lib);
        let root = id(50);
        let area = expander.expand(&town(root)).unwrap();
        // Street, two posts, two lamps.
        assert_eq!(area.len(), 5);
        let first_lamp = derive(root, &[id(11), id(2)]);
        let second_lamp = derive(root, &[id(12), id(2)]);
        assert_eq!(strength(&area, first_lamp), 1000.0, "the assembly's value");
        assert_eq!(
            strength(&area, second_lamp),
            5000.0,
            "the street's override"
        );
        assert_eq!(area.get(root).unwrap().name, "Main street");
        assert_eq!(
            area.get(second_lamp).unwrap().parent,
            Some(derive(root, &[id(12)]))
        );
        // The same file gives the same IDs every time.
        let again = Expander::new(&lib).expand(&town(root)).unwrap();
        assert_eq!(again, area);
    }

    #[test]
    fn a_copy_collapses_to_its_overrides_and_expands_back() {
        let lib = library(vec![(asset(100), lamp_post()), (asset(200), street())]);
        let root = id(50);
        let mut area = Expander::new(&lib).expand(&town(root)).unwrap();
        let lamp = derive(root, &[id(11), id(2)]);
        area.get_mut(lamp).unwrap().set(&Light {
            strength: 42.0,
            ..Light::point()
        });
        area.get_mut(lamp).unwrap().name = "Bright lamp".into();
        let gone = derive(root, &[id(12)]);
        area.remove_tree(gone);
        // Something the user added under the copy is plain.
        let mut extra = EntityData::new("Bench");
        extra.parent = Some(root);
        area.insert(id(77), extra);
        let file = Expander::new(&lib).collapse(&area, FileKind::Area).unwrap();
        assert_eq!(file.area.len(), 2, "the placed root and the bench");
        let changes = &file.overrides[&(root, vec![id(11), id(2)])];
        assert_eq!(changes.name.as_deref(), Some("Bright lamp"));
        assert_eq!(
            changes.components["Light"].fields.len(),
            1,
            "only the changed field"
        );
        assert!(file.overrides[&(root, vec![id(12)])].removed);
        assert!(
            !file.overrides.contains_key(&(root, vec![id(12), id(2)])),
            "its children went with it"
        );
        // Written, read and expanded, it's the same area.
        let registry = crate::builtin::registry().unwrap();
        let text = text::write_file(&file, &registry, &|_| None);
        assert!(text.contains(&format!("entity {root}/{}/{} {{", id(11), id(2))));
        let read = text::read_file(&text, &registry).unwrap();
        assert_eq!(read, file);
        assert_eq!(Expander::new(&lib).expand(&read).unwrap(), area);
    }

    #[test]
    fn overrides_survive_edits_to_the_assembly() {
        let root = id(50);
        let lib = library(vec![(asset(100), lamp_post()), (asset(200), street())]);
        let mut area = Expander::new(&lib).expand(&town(root)).unwrap();
        let lamp = derive(root, &[id(11), id(2)]);
        area.get_mut(lamp).unwrap().set(&Light {
            strength: 42.0,
            ..Light::point()
        });
        let file = Expander::new(&lib).collapse(&area, FileKind::Area).unwrap();
        // The lamp post gets a new colour and a bench in the assembly.
        let mut post = lamp_post();
        post.area.get_mut(id(2)).unwrap().set(&Light {
            color: [1.0, 0.5, 0.0],
            ..Light::point()
        });
        let mut bench = EntityData::new("Bench");
        bench.parent = Some(id(1));
        post.area.insert(id(3), bench);
        let lib = library(vec![(asset(100), post), (asset(200), street())]);
        let area = Expander::new(&lib).expand(&file).unwrap();
        let light = area.get(lamp).unwrap().get::<Light>().unwrap();
        assert_eq!(light.strength, 42.0, "the copy's change stays");
        assert_eq!(
            light.color,
            [1.0, 0.5, 0.0],
            "the assembly's change arrives"
        );
        assert!(
            area.contains(derive(root, &[id(11), id(3)])),
            "and its new bench"
        );
    }

    #[test]
    fn a_change_to_a_part_gone_from_the_assembly_is_kept() {
        let root = id(50);
        let lib = library(vec![(asset(100), lamp_post())]);
        let mut file = town(root);
        file.area
            .get_mut(root)
            .unwrap()
            .origin
            .as_mut()
            .unwrap()
            .assembly = asset(100);
        let mut light = reflect::ComponentData::default();
        light.fields.insert("strength".into(), Value::Float(9.0));
        file.overrides.insert(
            (root, vec![id(99)]),
            Override {
                name: Some("Old sign".into()),
                components: [("Light".to_string(), light)].into(),
                ..Override::default()
            },
        );
        let area = Expander::new(&lib).expand(&file).unwrap();
        let kept = area.get(derive(root, &[id(99)])).unwrap();
        assert_eq!(kept.name, "Old sign");
        assert_eq!(kept.parent, Some(root));
        let again = Expander::new(&lib).collapse(&area, FileKind::Area).unwrap();
        assert_eq!(
            again.overrides[&(root, vec![id(99)])].name.as_deref(),
            Some("Old sign")
        );
    }

    #[test]
    fn loops_depth_and_size_are_refused() {
        // An assembly that places itself.
        let mut selfish = lamp_post();
        let mut inner = EntityData::new("Me again");
        inner.parent = Some(id(1));
        inner.origin = Some(Origin {
            assembly: asset(100),
            instance: id(5),
            path: Vec::new(),
        });
        selfish.area.insert(id(5), inner);
        let lib = library(vec![(asset(100), selfish)]);
        let err = Expander::new(&lib).assembly(asset(100)).unwrap_err();
        assert_eq!(err, AssemblyError::Loop(vec![asset(100), asset(100)]));
        assert!(err.to_string().contains("loop"));
        // A chain deeper than allowed.
        let mut files = Vec::new();
        for level in 0..=MAX_NESTING as u64 + 1 {
            let mut file = lamp_post();
            let mut next = EntityData::new("Deeper");
            next.parent = Some(id(1));
            next.origin = Some(Origin {
                assembly: asset(300 + level + 1),
                instance: id(5),
                path: Vec::new(),
            });
            file.area.insert(id(5), next);
            files.push((asset(300 + level), file));
        }
        files.push((asset(300 + MAX_NESTING as u64 + 2), lamp_post()));
        let lib = library(files);
        assert_eq!(
            Expander::new(&lib).assembly(asset(300)).unwrap_err(),
            AssemblyError::TooDeep
        );
        // Ten copies of ten copies… of a post: counted, refused, never made.
        let mut files = vec![(asset(400), lamp_post())];
        for level in 0..8u64 {
            let mut area = Area::new();
            area.insert(id(1), EntityData::new("Crowd"));
            for n in 0..10u64 {
                let mut copy = EntityData::new("Copy");
                copy.parent = Some(id(1));
                copy.origin = Some(Origin {
                    assembly: asset(400 + level),
                    instance: id(100 + n),
                    path: Vec::new(),
                });
                area.insert(id(100 + n), copy);
            }
            files.push((
                asset(401 + level),
                AreaFile {
                    kind: FileKind::Assembly { root: id(1) },
                    area,
                    overrides: Default::default(),
                },
            ));
        }
        let lib = library(files);
        assert_eq!(
            Expander::new(&lib).assembly(asset(408)).unwrap_err(),
            AssemblyError::TooBig
        );
        assert_eq!(
            Expander::new(&lib).assembly(asset(999)).unwrap_err(),
            AssemblyError::Missing(asset(999))
        );
    }

    #[test]
    fn expected_values_come_from_the_assembly() {
        let root = id(50);
        let lib = library(vec![(asset(100), lamp_post()), (asset(200), street())]);
        let area = Expander::new(&lib).expand(&town(root)).unwrap();
        let lamp = derive(root, &[id(12), id(2)]);
        let origin = area.get(lamp).unwrap().origin.clone().unwrap();
        let wanted = Expander::new(&lib).expected(&area, &origin).unwrap();
        // The street's own override is part of what the street assembly
        // says; only this copy's changes differ from it.
        assert_eq!(wanted.get::<Light>().unwrap().strength, 5000.0);
    }

    #[test]
    fn derived_ids_differ_by_copy_and_path() {
        let a = derive(id(1), &[id(2)]);
        assert_ne!(a, derive(id(3), &[id(2)]));
        assert_ne!(a, derive(id(1), &[id(4)]));
        assert_ne!(
            derive(id(1), &[id(2), id(3)]),
            derive(id(1), &[id(3), id(2)])
        );
        assert_eq!(derive(id(1), &[]), id(1));
    }
}
