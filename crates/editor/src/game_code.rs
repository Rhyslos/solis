//! Game code in the editor: the program's components in the registry (so
//! Properties shows them and Add component lists them), and the program
//! running in play mode against the game's world
//! ([decision 0028](../../../docs/decisions/0028-lust-runs-as-luau-while-editing.md)).

use crate::console::LogLevel;
use crate::workbench::Workbench;
use area::Area;
use area::builtin::Transform;
use glam::{Quat, Vec3};
use reflect::EntityId;
use script::{CodeProblem, Failure, Host, Runtime};
use std::sync::Arc;

impl Workbench {
    /// Takes in the game's program (the app compiles it on a worker, and
    /// hands it over when the code has no mistakes; `None` when there's no
    /// code). Its components replace the last program's in the registry.
    /// Starting it reads them; a start that fails is a problem, and leaves
    /// no components.
    pub fn set_program(&mut self, chunk: Option<Arc<script::Chunk>>) {
        for name in std::mem::take(&mut self.game_components) {
            self.document.registry.remove(&name);
        }
        self.sources += 1;
        self.program = None;
        let Some(chunk) = chunk else {
            return;
        };
        let mut runtime = Runtime::new();
        if let Err(failure) = runtime.load(&chunk, &mut script::NoHost) {
            self.log(
                LogLevel::Error,
                format!("The game's code stopped while starting: {failure}"),
            );
            self.code_problems.push(failure.at);
            return;
        }
        for schema in runtime.schemas() {
            match self.document.registry.register(schema.clone()) {
                Ok(()) => {
                    self.game_components.insert(schema.name.clone());
                }
                Err(why) => self.code_problems.push(CodeProblem {
                    file: script::code::ENTRY.to_string(),
                    line: 0,
                    column: 0,
                    message: match why {
                        reflect::RegistryError::Duplicate(name) => format!(
                            "{name} is the name of one of the engine's own components; \
                             give the component in your code another name"
                        ),
                        other => other.to_string(),
                    },
                }),
            }
        }
        self.program = Some(chunk);
    }

    /// The components the game's code has, by name.
    pub fn game_components(&self) -> impl Iterator<Item = &str> {
        self.game_components.iter().map(String::as_str)
    }
}

/// The game's world, as game code reaches it while a tick runs.
pub(crate) struct WorldHost<'a> {
    pub(crate) world: &'a mut world::World,
    /// The game's time at this tick.
    pub(crate) time: f64,
    /// What game code said, for the Console.
    pub(crate) said: Vec<String>,
}

impl<'a> WorldHost<'a> {
    pub(crate) fn new(world: &'a mut world::World, time: f64) -> Self {
        Self {
            world,
            time,
            said: Vec::new(),
        }
    }

    fn transform(&self, id: EntityId) -> Transform {
        self.world.get::<Transform>(id).unwrap_or_default()
    }
}

impl Host for WorldHost<'_> {
    fn say(&mut self, words: &str) {
        self.said.push(words.to_string());
    }

    fn time(&self) -> f64 {
        self.time
    }

    fn find(&self, name: &str) -> Option<EntityId> {
        self.world.find(name)
    }

    fn position(&self, which: EntityId) -> Option<Vec3> {
        self.world.global(which).map(|m| m.w_axis.truncate())
    }

    fn move_to(&mut self, which: EntityId, place: Vec3) {
        let mut t = self.transform(which);
        // A transform is measured from the parent.
        t.position = self
            .world
            .parent_global(which)
            .inverse()
            .transform_point3(place);
        if t.position.is_finite() {
            self.world.set_transform(which, t);
        }
    }

    fn rotation(&self, which: EntityId) -> Option<Quat> {
        self.world
            .global(which)
            .map(|m| m.to_scale_rotation_translation().1.normalize())
    }

    fn turn_to(&mut self, which: EntityId, facing: Quat) {
        let mut t = self.transform(which);
        let parent = self
            .world
            .parent_global(which)
            .to_scale_rotation_translation()
            .1;
        t.rotation = (parent.inverse() * facing).normalize();
        if t.rotation.is_finite() {
            self.world.set_transform(which, t);
        }
    }

    fn name(&self, which: EntityId) -> Option<String> {
        self.world.get::<world::Name>(which).map(|n| n.0)
    }
}

/// Makes `id`'s game components in `area` run (or gives them their
/// changed fields), and stops those it no longer has.
pub(crate) fn sync_entity(
    runtime: &mut Runtime,
    area: &Area,
    id: EntityId,
    host: &mut dyn Host,
) -> Vec<Failure> {
    let mut failures = Vec::new();
    match area.get(id) {
        Some(entity) => {
            for (name, data) in &entity.components {
                if runtime.has_component(name)
                    && let Err(failure) = runtime.sync(id, name, data, host)
                {
                    failures.push(failure);
                }
            }
            runtime.retain(|e, name| e != id || entity.components.contains_key(name));
        }
        None => runtime.retain(|e, _| e != id),
    }
    failures
}

/// [`sync_entity`] for every entity in `area`, stopping components of
/// entities that are gone.
pub(crate) fn sync_all(runtime: &mut Runtime, area: &Area, host: &mut dyn Host) -> Vec<Failure> {
    let mut failures = Vec::new();
    for (id, entity) in area.iter() {
        for (name, data) in &entity.components {
            if runtime.has_component(name)
                && let Err(failure) = runtime.sync(id, name, data, host)
            {
                failures.push(failure);
            }
        }
    }
    runtime.retain(|id, name| {
        area.get(id)
            .is_some_and(|e| e.components.contains_key(name))
    });
    failures
}

/// A failure as the Console says it, naming the entity.
pub(crate) fn failure_text(area: &Area, failure: &Failure) -> String {
    match failure.entity.and_then(|id| area.get(id)) {
        Some(entity) => format!(
            "{} on {} stopped: {}",
            failure.component, entity.name, failure.at
        ),
        None => failure.to_string(),
    }
}

#[cfg(test)]
#[path = "game_code_tests.rs"]
mod tests;
