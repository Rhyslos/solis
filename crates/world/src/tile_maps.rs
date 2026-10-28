//! Tile maps in the running game (Phase 9, D4): each entity with a
//! `TileMap` component runs its own copy of its map file, which the game
//! changes cell by cell. A map's cells and values are game state: in the
//! checksum (chunk by chunk, worked out again only where they changed) and
//! in saves.

use crate::{Id, World};
use area::builtin::TileMap as TileMapComponent;
use reflect::{AssetId, EntityId};
use std::collections::HashMap;
use std::sync::Arc;
use tiles::{CellValue, MapError, TileMap};

/// The project's tile map files, read, by asset ID.
pub type MapFiles = HashMap<AssetId, Arc<TileMap>>;

/// One entity's map at play: the file it started from (to notice the file
/// changed) and the map as the game has changed it. Kept beside the
/// entity's components, and over edits while playing.
#[derive(Debug, Clone)]
pub(crate) struct MapState {
    from: Arc<TileMap>,
    pub(crate) map: TileMap,
}

impl World {
    /// Gives the world the project's tile map files (again whenever one
    /// changes: a map whose file changed starts again from it).
    pub fn set_tile_maps(&mut self, files: Arc<MapFiles>) {
        if !Arc::ptr_eq(&self.map_files, &files) {
            self.map_files = files;
            self.start_maps();
        }
    }

    /// The project's tile map files.
    pub fn tile_map_files(&self) -> &Arc<MapFiles> {
        &self.map_files
    }

    /// Starts the map of every entity with a TileMap that has none yet, or
    /// one from an older file (each tick, so entities spawned by game code
    /// get theirs).
    pub fn start_maps(&mut self) {
        let mut fresh: Vec<(hecs::Entity, MapState)> = Vec::new();
        for (entity, id, component, state) in self
            .ecs
            .query::<(hecs::Entity, &Id, &TileMapComponent, Option<&MapState>)>()
            .iter()
        {
            let Some(file) = component.map.and_then(|m| self.map_files.get(&m.asset)) else {
                continue;
            };
            if state.is_some_and(|s| Arc::ptr_eq(&s.from, file)) {
                continue;
            }
            let map = match self.saved_maps.get(&id.0) {
                Some(saved) => saved.clone(),
                None => (**file).clone(),
            };
            fresh.push((
                entity,
                MapState {
                    from: Arc::clone(file),
                    map,
                },
            ));
        }
        if fresh.is_empty() {
            return;
        }
        for (entity, state) in fresh {
            if let Ok(id) = self.ecs.get::<&Id>(entity).map(|id| id.0) {
                self.saved_maps.remove(&id);
            }
            let _ = self.ecs.insert_one(entity, state);
        }
        self.touched = true;
    }

    /// `id`'s map as the game has it (just loaded and not started yet: as
    /// the save had it).
    pub fn tile_map(&self, id: EntityId) -> Option<TileMap> {
        let entity = *self.by_id.get(&id)?;
        match self.ecs.get::<&MapState>(entity) {
            Ok(s) => Some(s.map.clone()),
            Err(_) => self.saved_maps.get(&id).cloned(),
        }
    }

    /// Changes `id`'s map with `change`, giving back what it gives.
    fn with_map<T>(
        &mut self,
        id: EntityId,
        change: impl FnOnce(&mut TileMap) -> Result<T, MapError>,
    ) -> Result<T, MapError> {
        let entity = *self.by_id.get(&id).ok_or(MapError::NoSuch)?;
        let mut state = self
            .ecs
            .get::<&mut MapState>(entity)
            .map_err(|_| MapError::NoSuch)?;
        let result = change(&mut state.map);
        drop(state);
        self.touched = true;
        result
    }

    /// Sets a cell of a layer of `id`'s map to the terrain written `code`
    /// (0 empties it). Whether it changed.
    pub fn set_tile(
        &mut self,
        id: EntityId,
        layer: usize,
        cell: [u32; 2],
        code: u8,
    ) -> Result<bool, MapError> {
        self.with_map(id, |m| m.set_code(layer, cell[0], cell[1], code))
    }

    /// Sets a cell's value on `id`'s map. Whether it changed.
    pub fn set_cell_value(
        &mut self,
        id: EntityId,
        value: usize,
        cell: [u32; 2],
        to: CellValue,
    ) -> Result<bool, MapError> {
        self.with_map(id, |m| m.set_value(value, cell[0], cell[1], to))
    }

    /// Every map at play, by entity ID: what saves keep.
    pub fn tile_maps(&self) -> Vec<(EntityId, TileMap)> {
        let mut all: Vec<(EntityId, TileMap)> = self
            .ecs
            .query::<(&Id, &MapState)>()
            .iter()
            .map(|(id, s)| (id.0, s.map.clone()))
            .collect();
        all.extend(
            self.saved_maps
                .iter()
                .filter(|(id, _)| {
                    self.by_id
                        .get(id)
                        .is_some_and(|&e| !self.ecs.satisfies::<&MapState>(e))
                })
                .map(|(id, m)| (*id, m.clone())),
        );
        all.sort_unstable_by_key(|(id, _)| *id);
        all
    }

    /// Gives entities the maps a save had (a load), taken in place of
    /// their files' when each starts.
    pub fn restore_tile_maps(&mut self, maps: Vec<(EntityId, TileMap)>) {
        self.saved_maps.extend(maps);
        let ids: Vec<EntityId> = self.saved_maps.keys().copied().collect();
        for id in ids {
            if let Some(&entity) = self.by_id.get(&id) {
                let _ = self.ecs.remove_one::<MapState>(entity);
            }
        }
        self.start_maps();
        self.touched = true;
    }

    /// Each map's checksum, by entity ID (what the world's checksum covers
    /// of maps); a saved map not started yet counts as it will start.
    pub(crate) fn map_sums(&self) -> Vec<(EntityId, u64)> {
        let mut sums: Vec<(EntityId, u64)> = self
            .ecs
            .query::<(&Id, &MapState)>()
            .iter()
            .map(|(id, s)| (id.0, s.map.checksum()))
            .collect();
        sums.extend(
            self.saved_maps
                .iter()
                .filter(|(id, _)| {
                    self.by_id
                        .get(id)
                        .is_some_and(|&e| !self.ecs.satisfies::<&MapState>(e))
                })
                .map(|(id, m)| (*id, m.checksum())),
        );
        sums.sort_unstable_by_key(|(id, _)| *id);
        sums
    }

    /// Each entity's map, to keep over a rebuild.
    pub(crate) fn map_states(&self) -> Vec<(EntityId, MapState)> {
        self.ecs
            .query::<(&Id, &MapState)>()
            .iter()
            .map(|(id, s)| (id.0, s.clone()))
            .collect()
    }

    /// Puts back maps kept over a rebuild or an update, on entities that
    /// still have a TileMap.
    pub(crate) fn keep_maps(&mut self, states: Vec<(EntityId, MapState)>) {
        for (id, state) in states {
            if let Some(&entity) = self.by_id.get(&id)
                && self.ecs.satisfies::<&TileMapComponent>(entity)
            {
                let _ = self.ecs.insert_one(entity, state);
            }
        }
        self.start_maps();
    }
}

#[cfg(test)]
#[path = "tile_maps_tests.rs"]
mod tests;
