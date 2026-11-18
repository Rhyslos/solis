//! Landing what game code spawned and despawned (decision 0035): copies
//! of prefabs made and joined to the level and the world, despawned
//! entities taken out, the running game's components started and stopped
//! with them, and why a spawn made nothing said once a play.

use super::{GameEdit, MAX_SPAWNS_PER_TICK, Playing};
use crate::console::LogLevel;
use crate::document::Document;
use crate::document::FieldsSet;
use crate::game_code::{PlayWorld, sync_entities};
use crate::workbench::Workbench;
use level::prefab::PrefabError;
use level::{Edit, EntityData};
use reflect::{AssetId, EntityId};
use std::collections::BTreeMap;

/// Why a spawn made nothing.
enum Unmade {
    /// No prefab in the project has the name.
    NoSuchPrefab,
    /// This prefab can't be copied.
    Broken(AssetId, PrefabError),
    /// The entity it was to be is in the level already.
    IdTaken,
}

/// Where landing a tick's spawns and despawns has got to.
struct Landing {
    /// Why nothing was made, for each name asked for in vain: the first
    /// reason this tick, however many times it was asked.
    unmade: BTreeMap<String, Unmade>,
    /// How many more entities the level has room for.
    room: usize,
    /// Spawns not made: too many in a tick, or no room.
    refused: usize,
    /// Copies made this tick.
    spawned: usize,
    /// Where the next copy goes in the level's tree.
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
    /// of prefabs join the level at the end of its tree and the world
    /// at once, despawned entities leave both, and the running game's
    /// components start and stop with them. Changes to the level that
    /// aren't undo steps (Stop puts the level back; decision 0035). A spawn
    /// that can't be made (no prefab by that name, or one that can't be
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
            room: level::text::MAX_ENTITIES.saturating_sub(self.open.document.level().len()),
            refused: 0,
            spawned: 0,
            // At the end of the level's tree, one after another.
            next_order: self.open.document.level().order_at_end(None),
            added: Vec::new(),
            removed: Vec::new(),
            removed_roots: Vec::new(),
        };
        // One expander for the tick: each prefab is expanded once, and
        // its size known before a copy is made, so thousands of spawns of
        // one too big to fit cost nothing.
        let prefabs = &self.open.prefabs;
        let lookup = |a| prefabs.get(&a).cloned();
        let mut expander = level::prefab::Expander::new(&lookup);
        for edit in edits {
            match edit {
                GameEdit::Spawn(id, name, place) => {
                    if landing.spawned >= MAX_SPAWNS_PER_TICK || landing.room == 0 {
                        landing.refused += 1;
                        continue;
                    }
                    let found = named
                        .entry(name.clone())
                        .or_insert_with(|| self.prefab_named(&name))
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
                        self.open.document.level(),
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
    /// too many in a tick, or the level full), and why each name in
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
                    "Game code spawned more than {} copies in one tick, or the level reached \
                     its most entities ({}), so {refused} weren't made (is something spawning \
                     every tick?).",
                    crate::models::grouped(MAX_SPAWNS_PER_TICK),
                    crate::models::grouped(level::text::MAX_ENTITIES)
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
            Unmade::NoSuchPrefab => {
                return "there's no prefab by that name in the project's assets".into();
            }
            Unmade::IdTaken => return "the entity it was to be is already in the level".into(),
            Unmade::Broken(asset, why) => (*asset, why),
        };
        match why {
            PrefabError::Missing(id) => match named(*id) {
                Some(name) if *id == asset => format!("{name}'s file can't be read"),
                Some(name) => format!("it places {name}, whose file can't be read"),
                None => format!("it places a prefab ({id}) that isn't in the project"),
            },
            PrefabError::NotAnPrefab(id) if *id == asset => {
                "its file is a level, not a prefab".into()
            }
            PrefabError::NotAnPrefab(id) => format!(
                "it places {}, which is a level, not a prefab",
                named(*id).unwrap_or_else(|| id.to_string())
            ),
            PrefabError::Loop(chain) => {
                let chain: Vec<String> = chain
                    .iter()
                    .map(|&id| named(id).unwrap_or_else(|| id.to_string()))
                    .collect();
                format!(
                    "its prefabs place each other in a loop ({})",
                    chain.join(" → ")
                )
            }
            PrefabError::TooDeep => format!(
                "its prefabs are placed more than {} deep, one inside another",
                level::text::MAX_NESTING
            ),
            PrefabError::TooBig => format!(
                "a copy would be more than {} entities",
                crate::models::grouped(level::text::MAX_ENTITIES)
            ),
            PrefabError::Clash(id) => {
                format!("two of its entities would share an ID ({id})")
            }
        }
    }

    /// The prefab game code names `name`: by its file in the assets
    /// (`ships/fighter.prefab.lst`), that without its ending, or just its
    /// name (`fighter`, when only one has it); its asset and name.
    fn prefab_named(&self, name: &str) -> Option<(AssetId, String)> {
        let prefabs = || {
            self.open
                .library
                .iter()
                .filter(|i| i.kind == crate::library::LibraryKind::Prefab)
                .filter_map(|i| i.id.map(|id| (id, i)))
                .filter(|(id, _)| self.open.prefabs.contains_key(id))
        };
        let wanted = name.trim_start_matches('/');
        let by_path = prefabs().find(|(_, i)| {
            i.path.eq_ignore_ascii_case(wanted)
                || reflect::data_files::path_stem(&i.path).eq_ignore_ascii_case(wanted)
        });
        let found = by_path.or_else(|| {
            let mut named = prefabs().filter(|(_, i)| i.name.eq_ignore_ascii_case(wanted));
            let first = named.next();
            first.filter(|_| named.next().is_none())
        });
        found.map(|(id, i)| (id, i.name.clone()))
    }
}

/// Makes one copy of the prefab game code named `name` (`found`: its
/// asset and name, none when there's no such prefab) as entity `id`,
/// standing at `place`, turned and sized as the prefab made it, after
/// everything else: in the level, then the world. A copy that can't be
/// made notes why; one that doesn't fit is refused.
fn land_spawn(
    landing: &mut Landing,
    expander: &mut level::prefab::Expander<'_>,
    document: &mut Document,
    playing: Option<&mut Playing>,
    (id, name, place): (EntityId, String, glam::Vec3),
    found: Option<(AssetId, String)>,
) {
    let Some((asset, shown)) = found else {
        landing.unmade.entry(name).or_insert(Unmade::NoSuchPrefab);
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
    // as the prefab made it, after everything else.
    if let Some((_, root)) = entities.iter_mut().find(|(e, _)| *e == id) {
        let made = root.get::<level::builtin::Transform>().unwrap_or_default();
        root.set(&level::builtin::Transform {
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
    landing.next_order = level::order_after(Some(&landing.next_order));
    if let Some(playing) = playing {
        landing.added.extend(playing.world.add_subtree(&entities));
    }
}

/// Takes entity `id`, which game code despawned, out of the level (with
/// everything under it), noting what left for the world and the game.
fn land_despawn(
    landing: &mut Landing,
    document: &mut Document,
    playing: Option<&Playing>,
    id: EntityId,
) {
    if !document.level().contains(id) {
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
