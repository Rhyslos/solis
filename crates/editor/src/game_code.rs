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
    /// Takes in the game's program (the app compiles it on a worker,
    /// starting it there to read its components, and hands it over when
    /// the code has no mistakes; `None` when there's no code). Its
    /// components replace the last program's in the registry; one whose
    /// name is taken (by one of the engine's own components, or a Rust
    /// one) is a problem, and doesn't run. While playing, the running game
    /// takes the new code first: if it can't (its start fails against the
    /// game's world), that's a problem, and the last program stays,
    /// registry and all. Nothing here runs game code unless the game is
    /// playing.
    pub fn set_program(&mut self, chunk: Option<Arc<script::Compiled>>) {
        let schemas = chunk.as_ref().map_or(&[][..], |c| c.schemas.as_slice());
        let left_out: std::collections::BTreeSet<String> = schemas
            .iter()
            .filter(|s| {
                self.document.registry.get(&s.name).is_some()
                    && !self.game_components.contains(&s.name)
            })
            .map(|s| s.name.clone())
            .collect();
        let taken: Vec<CodeProblem> = left_out
            .iter()
            .map(|name| self.name_taken(chunk.as_deref(), name))
            .collect();
        if self.playing.is_some()
            && let Err(failure) = self.swap_into_game(chunk.as_deref(), &left_out)
        {
            let text = format!(
                "The running game couldn't take the new code, so it keeps the last: {failure}"
            );
            self.log(LogLevel::Error, text);
            self.code_problems.push(failure.at);
            return;
        }
        for name in std::mem::take(&mut self.game_components) {
            self.document.registry.remove(&name);
        }
        self.sources += 1;
        for schema in schemas {
            if !left_out.contains(&schema.name)
                && self.document.registry.register(schema.clone()).is_ok()
            {
                self.game_components.insert(schema.name.clone());
            }
        }
        self.code_problems.extend(taken);
        self.left_out = left_out;
        self.program = chunk;
    }

    /// The problem of a component in `chunk` called `name` when the name
    /// is taken already.
    fn name_taken(&self, chunk: Option<&script::Compiled>, name: &str) -> CodeProblem {
        let by = if self.rust_components.iter().any(|k| k.schema.name == name) {
            "a component written in Rust"
        } else {
            "one of the engine's own components"
        };
        let (file, line) = chunk
            .and_then(|c| c.places.get(name).cloned())
            .unwrap_or_else(|| (script::code::ENTRY.to_string(), 0));
        CodeProblem {
            file,
            line,
            column: 0,
            message: format!(
                "{name} is the name of {by}, so the one in your code doesn't run; give it \
                 another name"
            ),
        }
    }

    /// Gives the running game (if playing) `chunk`: running components get
    /// the new code in place and keep their values, new ones start, and
    /// the Console says what changed. The components called one of
    /// `left_out` don't run. On a failure nothing changed.
    fn swap_into_game(
        &mut self,
        chunk: Option<&script::Compiled>,
        left_out: &std::collections::BTreeSet<String>,
    ) -> Result<(), Failure> {
        let Some(playing) = &mut self.playing else {
            return Ok(());
        };
        if playing.game.is_none() {
            playing.game = Some(new_runtime(&self.rust_components)?);
        }
        let Some(game) = &mut playing.game else {
            return Ok(());
        };
        let time = playing.world.clock().time;
        let mut host = WorldHost::new(&mut playing.world, time);
        let mut lines = Vec::new();
        match chunk {
            None => {
                game.unload();
                lines.push("The game's code is gone, so its components stopped.".to_string());
            }
            Some(chunk) => {
                let swapped = game.swap(chunk, &mut host)?;
                lines.push(swap_summary(&swapped));
                lines.extend(swapped.notes);
            }
        }
        game.leave_out(left_out.clone());
        let failures = sync_all(game, self.document.area(), &mut host);
        let said = std::mem::take(&mut host.said);
        playing.world.settle();
        for line in lines {
            self.log(LogLevel::Info, line);
        }
        self.tell_game(said, &failures);
        Ok(())
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

    /// Replaces the Rust components the open project uses (the app gives
    /// each project its own): the last ones leave the registry, these
    /// join it. A name already taken is a problem, and that one is left
    /// out.
    pub fn set_rust_components(&mut self, kinds: Vec<script::RustKind>) {
        for kind in std::mem::take(&mut self.rust_components) {
            self.document.registry.remove(&kind.schema.name);
        }
        self.sources += 1;
        for kind in kinds {
            if let Err(why) = self.add_rust_component(kind) {
                self.log(
                    LogLevel::Warning,
                    format!("A Rust component was left out: {why}."),
                );
            }
        }
    }

    /// The components the game's code has, by name.
    pub fn game_components(&self) -> impl Iterator<Item = &str> {
        self.game_components.iter().map(String::as_str)
    }
}

/// A new runtime with the open project's Rust components.
pub(crate) fn new_runtime(rust: &[script::RustKind]) -> Result<Runtime, Failure> {
    let mut runtime = Runtime::new()?;
    for kind in rust {
        runtime.add_rust(kind.clone());
    }
    Ok(runtime)
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

/// A failure as the Console says it, naming the entity, and whether the
/// component stopped.
pub(crate) fn failure_text(area: &Area, failure: &Failure) -> String {
    match failure.entity.and_then(|id| area.get(id)) {
        Some(entity) if failure.stopped => format!(
            "{} on {} stopped: {}",
            failure.component, entity.name, failure.at
        ),
        Some(entity) => format!("{} on {}: {}", failure.component, entity.name, failure.at),
        None => failure.to_string(),
    }
}

#[cfg(test)]
#[path = "game_code_tests.rs"]
mod tests;
