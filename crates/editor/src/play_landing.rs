//! Landing what game code spawned and despawned (decision 0035): copies
//! of assemblies made and joined to the area and the world, despawned
//! entities taken out, the running game's components started and stopped
//! with them, and why a spawn made nothing said once a play.

use super::{GameEdit, MAX_SPAWNS_PER_TICK, Playing};
use crate::console::LogLevel;
use crate::document::Document;
use crate::document::FieldsSet;
use crate::game_code::{PlayWorld, sync_entities};
use crate::workbench::Workbench;
use area::assembly::AssemblyError;
use area::{Edit, EntityData};
use reflect::{AssetId, EntityId};
use std::collections::BTreeMap;

/// Why a spawn made nothing.
enum Unmade {
    /// No assembly in the project has the name.
    NoSuchAssembly,
    /// This assembly can't be copied.
    Broken(AssetId, AssemblyError),
    /// The entity it was to be is in the area already.
    IdTaken,
}

/// Where landing a tick's spawns and despawns has got to.
struct Landing {
    /// Why nothing was made, for each name asked for in vain: the first
    /// reason this tick, however many times it was asked.
    unmade: BTreeMap<String, Unmade>,
    /// How many more entities the area has room for.
    room: usize,
    /// Spawns not made: too many in a tick, or no room.
    refused: usize,
    /// Copies made this tick.
    spawned: usize,
    /// Where the next copy goes in the area's tree.
    next_order: String,
    /// The entities copies added.
    added: Vec<EntityId>,
    /// The entities despawned, with everything under them.
    removed: Vec<EntityId>,
    /// The entities despawned themselves.
    removed_roots: Vec<EntityId>,
}

impl Workbench {
    /// Lands what game code spawned and despawned since it was last
    /// landed (`Playing::pending`), in the order it was asked for: copies
    /// of assemblies join the area at the end of its tree and the world
    /// at once, despawned entities leave both, and the running game's
    /// components start and stop with them. Changes to the area that
    /// aren't undo steps (Stop puts the area back; decision 0035). A spawn
    /// that can't be made (no assembly by that name, or one that can't be
    /// copied) says why in the Console, once a play for each name, never
    /// once a frame (the review, BUG-51). The failures of starting the
    /// new components come back.
    pub(super) fn land_edits(&mut self, time: f64, seconds: f32) -> Vec<script::Failure> {
        let edits = match &mut self.open.playing {
            Some(playing) if !playing.pending.is_empty() => std::mem::take(&mut playing.pending),
            _ => return Vec::new(),
        };
        // Each name looked up once, not once a spawn.
        let mut named: std::collections::HashMap<String, Option<(AssetId, String)>> =
            std::collections::HashMap::new();
        let mut landing = Landing {
            unmade: BTreeMap::new(),
            room: area::text::MAX_ENTITIES.saturating_sub(self.open.document.area().len()),
            refused: 0,
            spawned: 0,
            // At the end of the area's tree, one after another.
            next_order: self.open.document.area().order_at_end(None),
            added: Vec::new(),
            removed: Vec::new(),
            removed_roots: Vec::new(),
        };
        // One expander for the tick: each assembly is expanded once, and
        // its size known before a copy is made, so thousands of spawns of
        // one too big to fit cost nothing.
        let assemblies = &self.open.assemblies;
        let lookup = |a| assemblies.get(&a).cloned();
        let mut expander = area::assembly::Expander::new(&lookup);
        for edit in edits {
            match edit {
                GameEdit::Spawn(id, name, place) => {
                    if landing.spawned >= MAX_SPAWNS_PER_TICK || landing.room == 0 {
                        landing.refused += 1;
                        continue;
                    }
                    let found = named
                        .entry(name.clone())
                        .or_insert_with(|| self.assembly_named(&name))
                        .clone();
                    land_spawn(
                        &mut landing,
                        &mut expander,
                        &mut self.open.document,
                        self.open.playing.as_mut(),
                        (id, name, place),
                        found,
                    );
                }
                GameEdit::Despawn(id) => {
                    land_despawn(
                        &mut landing,
                        &mut self.open.document,
                        self.open.playing.as_ref(),
                        id,
                    );
                }
            }
        }
        let Landing {
            unmade,
            refused,
            added,
            removed,
            removed_roots,
            ..
        } = landing;
        let failures = self.start_landed(time, seconds, added, &removed, &removed_roots);
        self.tell_unlanded(refused, unmade);
        failures
    }

    /// The landed entities join the running game: the despawned leave
    /// the world (`removed_roots`, with everything under them in
    /// `removed`), and the components of those `added` start, their
    /// starts' sounds and spawns kept for later. What the starts said is
    /// told; their failures come back.
    fn start_landed(
        &mut self,
        time: f64,
        seconds: f32,
        mut added: Vec<EntityId>,
        removed: &[EntityId],
        removed_roots: &[EntityId],
    ) -> Vec<script::Failure> {
        let mut failures = Vec::new();
        let mut told = Vec::new();
        if let Some(playing) = &mut self.open.playing {
            playing.world.remove_subtrees(removed_roots);
            let gone: std::collections::HashSet<EntityId> = removed.iter().copied().collect();
            added.retain(|id| !gone.contains(id));
            if let Some(game) = &mut playing.game {
                game.retain_on(removed.iter().map(|&id| (id, Vec::new())));
                if !added.is_empty() {
                    let (tick, seed) = (playing.world.clock().ticks, playing.world.seed());
                    let mut host = PlayWorld::new(&mut playing.world, &mut playing.view, time);
                    host.frame_time = f64::from(seconds);
                    failures = sync_entities(
                        game,
                        self.open.document.area(),
                        added,
                        &FieldsSet::new(),
                        &mut host,
                    );
                    host.outside_a_tick();
                    told = std::mem::take(&mut host.said);
                    // Sounds the new components' starts asked for.
                    let sounds = std::mem::take(&mut host.sounds);
                    crate::game_sound::gather(&mut self.open.game_sounds, sounds, tick, seed);
                    // What the new components' starts spawn waits for the
                    // next landing.
                    let more = std::mem::take(&mut host.edits);
                    playing.pending.extend(more);
                }
            }
        }
        for words in told {
            self.log(LogLevel::Info, words);
        }
        failures
    }

    /// Says, once a play, that spawns were refused (`refused` of them:
    /// too many in a tick, or the area full), and why each name in
    /// `unmade` made nothing.
    fn tell_unlanded(&mut self, refused: usize, unmade: BTreeMap<String, Unmade>) {
        if refused > 0
            && let Some(playing) = &mut self.open.playing
            && !playing.told_too_many_spawns
        {
            playing.told_too_many_spawns = true;
            self.log(
                LogLevel::Warning,
                format!(
                    "Game code spawned more than {} copies in one tick, or the area reached \
                     its most entities ({}), so {refused} weren't made (is something spawning \
                     every tick?).",
                    crate::models::grouped(MAX_SPAWNS_PER_TICK),
                    crate::models::grouped(area::text::MAX_ENTITIES)
                ),
            );
        }
        for (name, why) in unmade {
            let Some(playing) = &mut self.open.playing else {
                break;
            };
            // A few names are remembered; past that, nothing more is said.
            if playing.told_unmade.len() < 100 && playing.told_unmade.insert(name.clone()) {
                let why = self.why_unmade(&why);
                self.log(
                    LogLevel::Warning,
                    format!("Game code spawned \"{name}\", but {why}, so nothing was made."),
                );
            }
        }
    }

    /// Why a spawn made nothing, in words for the Console: files by their
    /// names in the Library, where it knows them.
    fn why_unmade(&self, why: &Unmade) -> String {
        let named = |id: AssetId| {
            let listed = self.open.library.iter().find(|item| item.id == Some(id));
            listed.map(|item| format!("\"{}\"", item.name))
        };
        let (asset, why) = match why {
            Unmade::NoSuchAssembly => {
                return "there's no assembly by that name in the project's assets".into();
            }
            Unmade::IdTaken => return "the entity it was to be is already in the area".into(),
            Unmade::Broken(asset, why) => (*asset, why),
        };
        match why {
            AssemblyError::Missing(id) => match named(*id) {
                Some(name) if *id == asset => format!("{name}'s file can't be read"),
                Some(name) => format!("it places {name}, whose file can't be read"),
                None => format!("it places an assembly ({id}) that isn't in the project"),
            },
            AssemblyError::NotAnAssembly(id) if *id == asset => {
                "its file is an area, not an assembly".into()
            }
            AssemblyError::NotAnAssembly(id) => format!(
                "it places {}, which is an area, not an assembly",
                named(*id).unwrap_or_else(|| id.to_string())
            ),
            AssemblyError::Loop(chain) => {
                let chain: Vec<String> = chain
                    .iter()
                    .map(|&id| named(id).unwrap_or_else(|| id.to_string()))
                    .collect();
                format!(
                    "its assemblies place each other in a loop ({})",
                    chain.join(" → ")
                )
            }
            AssemblyError::TooDeep => format!(
                "its assemblies are placed more than {} deep, one inside another",
                area::text::MAX_NESTING
            ),
            AssemblyError::TooBig => format!(
                "a copy would be more than {} entities",
                crate::models::grouped(area::text::MAX_ENTITIES)
            ),
            AssemblyError::Clash(id) => {
                format!("two of its entities would share an ID ({id})")
            }
        }
    }

    /// The assembly game code names `name`: by its file in the assets
    /// (`ships/fighter.solasm`), that without the extension, or just its
    /// name (`fighter`, when only one has it); its asset and name.
    fn assembly_named(&self, name: &str) -> Option<(AssetId, String)> {
        let assemblies = || {
            self.open
                .library
                .iter()
                .filter(|i| i.kind == crate::library::LibraryKind::Assembly)
                .filter_map(|i| i.id.map(|id| (id, i)))
                .filter(|(id, _)| self.open.assemblies.contains_key(id))
        };
        let wanted = name.trim_start_matches('/');
        let by_path = assemblies().find(|(_, i)| {
            i.path.eq_ignore_ascii_case(wanted)
                || i.path
                    .rsplit_once('.')
                    .is_some_and(|(stem, _)| stem.eq_ignore_ascii_case(wanted))
        });
        let found = by_path.or_else(|| {
            let mut named = assemblies().filter(|(_, i)| i.name.eq_ignore_ascii_case(wanted));
            let first = named.next();
            first.filter(|_| named.next().is_none())
        });
        found.map(|(id, i)| (id, i.name.clone()))
    }
}

/// Makes one copy of the assembly game code named `name` (`found`: its
/// asset and name, none when there's no such assembly) as entity `id`,
/// standing at `place`, turned and sized as the assembly made it, after
/// everything else: in the area, then the world. A copy that can't be
/// made notes why; one that doesn't fit is refused.
fn land_spawn(
    landing: &mut Landing,
    expander: &mut area::assembly::Expander<'_>,
    document: &mut Document,
    playing: Option<&mut Playing>,
    (id, name, place): (EntityId, String, glam::Vec3),
    found: Option<(AssetId, String)>,
) {
    let Some((asset, shown)) = found else {
        landing.unmade.entry(name).or_insert(Unmade::NoSuchAssembly);
        return;
    };
    let size = match expander.size_of(asset) {
        Ok(size) => size,
        Err(why) => {
            landing
                .unmade
                .entry(name)
                .or_insert(Unmade::Broken(asset, why));
            return;
        }
    };
    if size > landing.room {
        landing.refused += 1;
        return;
    }
    let data = EntityData::new(&shown);
    let mut entities = match expander.place(asset, id, &data) {
        Ok(entities) => entities,
        Err(why) => {
            landing
                .unmade
                .entry(name)
                .or_insert(Unmade::Broken(asset, why));
            return;
        }
    };
    // The copy stands where it was spawned, turned and sized
    // as the assembly made it, after everything else.
    if let Some((_, root)) = entities.iter_mut().find(|(e, _)| *e == id) {
        let made = root.get::<area::builtin::Transform>().unwrap_or_default();
        root.set(&area::builtin::Transform {
            position: place,
            ..made
        });
        root.parent = None;
        root.order = landing.next_order.clone();
    }
    if entities.len() > landing.room {
        landing.refused += 1;
        return;
    }
    let failed = document.edit_by_game(vec![Edit::Spawn {
        entities: entities.clone(),
    }]);
    if failed > 0 {
        landing.unmade.entry(name).or_insert(Unmade::IdTaken);
        return;
    }
    landing.room -= entities.len();
    landing.spawned += 1;
    landing.next_order = area::order_after(Some(&landing.next_order));
    if let Some(playing) = playing {
        landing.added.extend(playing.world.add_subtree(&entities));
    }
}

/// Takes entity `id`, which game code despawned, out of the area (with
/// everything under it), noting what left for the world and the game.
fn land_despawn(
    landing: &mut Landing,
    document: &mut Document,
    playing: Option<&Playing>,
    id: EntityId,
) {
    if !document.area().contains(id) {
        return;
    }
    let under = match playing {
        Some(playing) => playing.world.subtree(id),
        None => Vec::new(),
    };
    if document.edit_by_game(vec![Edit::Despawn { root: id }]) == 0 {
        landing.removed.extend(under);
        landing.removed_roots.push(id);
    }
}
