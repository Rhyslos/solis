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
    /// Starting it reads them; a start that fails is a problem, and the
    /// last program that worked stays. While playing, the new code is
    /// swapped into the running game.
    pub fn set_program(&mut self, chunk: Option<Arc<script::Chunk>>) {
        let mut runtime = Runtime::new();
        if let Some(chunk) = &chunk
            && let Err(failure) = runtime.load(chunk, &mut script::NoHost)
        {
            self.log(
                LogLevel::Error,
                format!("The game's code stopped while starting: {failure}"),
            );
            self.code_problems.push(failure.at);
            return;
        }
        for name in std::mem::take(&mut self.game_components) {
            self.document.registry.remove(&name);
        }
        self.sources += 1;
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
        self.program = chunk;
        self.swap_into_game();
    }

    /// Gives the running game (if playing) the current program: running
    /// components get the new code in place and keep their values, new
    /// ones start, and the Console says what changed.
    fn swap_into_game(&mut self) {
        let Some(playing) = &mut self.playing else {
            return;
        };
        let game = playing.game.get_or_insert_with(|| {
            let mut runtime = Runtime::new();
            for kind in &self.rust_components {
                runtime.add_rust(kind.clone());
            }
            runtime
        });
        let time = playing.world.clock().time;
        let mut host = WorldHost::new(&mut playing.world, time);
        let mut failures = Vec::new();
        let mut lines = Vec::new();
        match &self.program {
            None => {
                game.unload();
                lines.push("The game's code is gone, so its components stopped.".to_string());
            }
            Some(chunk) => match game.swap(chunk, &mut host) {
                Ok(swapped) => {
                    lines.push(swap_summary(&swapped));
                    lines.extend(swapped.notes);
                }
                Err(failure) => failures.push(failure),
            },
        }
        failures.extend(sync_all(game, self.document.area(), &mut host));
        let said = std::mem::take(&mut host.said);
        playing.world.settle();
        for line in lines {
            self.log(LogLevel::Info, line);
        }
        for words in said {
            self.log(LogLevel::Info, words);
        }
        for failure in &failures {
            let text = failure_text(self.document.area(), failure);
            self.log(LogLevel::Error, text);
        }
    }

    /// Adds a kind of component written in Rust (a game's own): its
    /// schema joins the registry, so Properties shows it, and play mode
    /// runs it beside the Lust ones, sending and handling the same events.
    pub fn add_rust_component(
        &mut self,
        kind: script::RustKind,
    ) -> Result<(), reflect::RegistryError> {
        self.register_component(kind.schema.clone())?;
        self.rust_components.push(kind);
        Ok(())
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

/// What a swap changed, in one line for the Console.
fn swap_summary(swapped: &script::Swapped) -> String {
    let mut parts = Vec::new();
    for (names, what) in [
        (&swapped.changed, "changed"),
        (&swapped.added, "new"),
        (&swapped.removed, "gone"),
    ] {
        if !names.is_empty() {
            parts.push(format!("{} {what}", names.join(", ")));
        }
    }
    if parts.is_empty() {
        "The running game took the new code; every component keeps its values.".to_string()
    } else {
        format!(
            "The running game took the new code ({}); components keep their values.",
            parts.join("; ")
        )
    }
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
