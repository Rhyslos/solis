//! 2D physics in the running game (decision 0071): entities with a
//! `Collider2D` join `physics::flat` (characters walked by game code, or
//! solid and still), and each tile map's solid cells become walls, built
//! again chunk by chunk only where cells changed. Sizes on the component
//! are pixels, turned into metres with the project's 2D settings.

use crate::tile_maps::MapState;
use crate::{Id, World};
use area::builtin::{Collider2D, Shape2D, TileMap as TileMapComponent};
use area::pixels::PixelSettings;
use glam::{Vec2, Vec3};
use physics::flat::{FlatCollider, FlatShape};
use reflect::{AssetId, EntityId};
use std::collections::HashMap;
use std::sync::Arc;
use tiles::walls::{Rect, Solid};

/// One chunk's walls: the chunk, and each rectangle's lowest and highest
/// corner in metres.
type ChunkWalls = ([u32; 2], Vec<(Vec2, Vec2)>);

/// Which terrains of each tileset are solid, by the tileset's asset ID.
pub type SolidSets = HashMap<AssetId, Solid>;

/// What a map's walls were last built from, to build again only what
/// changed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WallsBuilt {
    /// Where the map's top-left corner was, and a cell's size, metres.
    origin: Vec2,
    cell: [f32; 2],
    /// Which codes were solid.
    solid: Solid,
    /// Each chunk's revision when its walls were built, in rows.
    revisions: Vec<u64>,
    chunks: [u32; 2],
}

impl World {
    /// Gives the world the project's 2D settings (pixels per metre and
    /// the tile size): `Collider2D`'s pixels and tile maps' cells are
    /// measured with them.
    pub fn set_pixels(&mut self, pixels: PixelSettings) {
        if self.pixels != pixels {
            self.pixels = pixels;
            self.walls_built.clear();
            self.join_all();
        }
    }

    /// The project's 2D settings, as given.
    pub fn pixels(&self) -> &PixelSettings {
        &self.pixels
    }

    /// Gives the world which terrains of each tileset are solid (again
    /// whenever a tileset changes): their cells become walls.
    pub fn set_solid_sets(&mut self, solid: Arc<SolidSets>) {
        self.solid_sets = solid;
    }

    /// 2D physics, to read.
    pub fn flat(&self) -> &physics::flat::Flat {
        &self.flat
    }

    /// A `Collider2D` in metres.
    pub(crate) fn flat_collider(&self, c: &Collider2D) -> FlatCollider {
        let px = self.pixels.pixel_size();
        let size = |v: f32| if v.is_finite() { v.clamp(0.1, 4096.0) } else { 1.0 } * px;
        let off = |v: f32| if v.is_finite() { v.clamp(-4096.0, 4096.0) } else { 0.0 } * px;
        FlatCollider {
            shape: match c.shape {
                Shape2D::Circle => FlatShape::Circle(size(c.radius)),
                Shape2D::Box => FlatShape::Box(Vec2::new(size(c.width), size(c.height))),
            },
            center: Vec2::new(off(c.across), off(c.up)),
        }
    }

    /// Makes `id`'s part in 2D physics match its `Collider2D` and where
    /// it is now.
    pub(crate) fn flat_join(&mut self, id: EntityId) {
        let Some(&entity) = self.by_id.get(&id) else {
            self.flat.remove(id);
            return;
        };
        let Ok(c) = self.ecs.get::<&Collider2D>(entity).map(|c| *c) else {
            self.flat.remove(id);
            return;
        };
        let place = self.global(id).map_or(Vec3::ZERO, |g| g.w_axis.truncate());
        let collider = self.flat_collider(&c);
        self.flat.add(id, place.truncate(), collider, c.character);
        self.touched = true;
    }

    /// Builds each tile map's walls again where its cells, its place, its
    /// tileset's solid terrains or the tile size changed, and takes away
    /// the walls of maps that are gone.
    pub(crate) fn update_walls(&mut self) {
        let [cw, ch] = self.pixels.tile_size();
        let mut seen: Vec<EntityId> = Vec::new();
        let mut work: Vec<(EntityId, Vec2, Solid)> = Vec::new();
        for (id, _, global) in self
            .ecs
            .query::<(&Id, &TileMapComponent, &crate::Global)>()
            .iter()
        {
            let Some(solid) = self.map_solid(id.0) else {
                continue;
            };
            seen.push(id.0);
            work.push((id.0, global.0.w_axis.truncate().truncate(), solid));
        }
        work.sort_unstable_by_key(|(id, ..)| *id);
        for (id, origin, solid) in work {
            self.build_map_walls(id, origin, [cw, ch], solid);
        }
        // Maps gone, or no longer with solid terrains.
        let gone: Vec<EntityId> = self
            .walls_built
            .keys()
            .filter(|id| !seen.contains(id))
            .copied()
            .collect();
        for id in gone {
            self.walls_built.remove(&id);
            self.flat.remove_walls(id);
            self.touched = true;
        }
    }

    /// The solid codes of map `id`'s tileset, if it has a running map
    /// whose tileset has any.
    fn map_solid(&self, id: EntityId) -> Option<Solid> {
        let entity = *self.by_id.get(&id)?;
        let state = self.ecs.get::<&MapState>(entity).ok()?;
        let solid = *self.solid_sets.get(&state.map.tileset?)?;
        solid.iter().any(|s| *s).then_some(solid)
    }

    fn build_map_walls(&mut self, id: EntityId, origin: Vec2, cell: [f32; 2], solid: Solid) {
        let Some(&entity) = self.by_id.get(&id) else {
            return;
        };
        // The running map, read in place (never copied: maps are big).
        let Ok(state) = self.ecs.get::<&MapState>(entity) else {
            return;
        };
        let map = &state.map;
        let chunks = map.chunks();
        let revisions: Vec<u64> = (0..chunks[1])
            .flat_map(|y| (0..chunks[0]).map(move |x| [x, y]))
            .map(|c| map.chunk_revision(c))
            .collect();
        let old = self.walls_built.get(&id);
        let same_frame = old.is_some_and(|o| {
            o.origin == origin && o.cell == cell && o.solid == solid && o.chunks == chunks
        });
        let mut changed: Vec<[u32; 2]> = Vec::new();
        for y in 0..chunks[1] {
            for x in 0..chunks[0] {
                let i = (y * chunks[0] + x) as usize;
                let fresh =
                    same_frame && old.is_some_and(|o| o.revisions.get(i) == revisions.get(i));
                if !fresh {
                    changed.push([x, y]);
                }
            }
        }
        if changed.is_empty() {
            return;
        }
        let built: Vec<ChunkWalls> = changed
            .iter()
            .map(|&chunk| {
                let rects = tiles::walls::chunk_walls(map, &solid, chunk)
                    .into_iter()
                    .map(|r| corners(origin, cell, r))
                    .collect();
                (chunk, rects)
            })
            .collect();
        drop(state);
        if !same_frame {
            // Everything again: chunks that no longer exist go too.
            self.flat.remove_walls(id);
        }
        for (chunk, rects) in built {
            self.flat.set_walls(id, chunk, &rects);
        }
        self.walls_built.insert(
            id,
            WallsBuilt {
                origin,
                cell,
                solid,
                revisions,
                chunks,
            },
        );
        self.touched = true;
    }

    /// Moves 2D physics on by one tick: walls catch up with the maps,
    /// characters take the moves game code asked for in order, and their
    /// entities follow them on the ground (keeping their layer).
    pub(crate) fn step_flat(&mut self, moved: &[EntityId], walks: &[(EntityId, Vec2)]) {
        for id in moved {
            if self.flat.state().contains(*id)
                && let Some(g) = self.global(*id)
            {
                self.flat.set_place(*id, g.w_axis.truncate().truncate());
            }
        }
        self.update_walls();
        // Each character's move this tick, for game code's `grounded()`
        // and `velocity()`; none asked, they all stood still.
        let seconds = self.tick_time() as f32;
        self.flat.move_characters(walks, seconds);
        if walks.is_empty() {
            return;
        }
        self.following = true;
        for (id, place) in self.flat.moved() {
            self.change_transform(id, |parent, mut t| {
                // On the ground where physics put it, at the layer it was.
                let layer = parent.transform_point3(t.position).z;
                let at = Vec3::new(place.x, place.y, layer);
                t.position = if parent == glam::Mat4::IDENTITY {
                    at
                } else {
                    parent.inverse().transform_point3(at)
                };
                t.position.is_finite().then_some(t)
            });
        }
        self.following = false;
    }
}

/// A rectangle of cells as its lowest and highest corner in the world:
/// the map's top-left corner at `origin`, cells running right along X and
/// down along −Y.
fn corners(origin: Vec2, cell: [f32; 2], r: Rect) -> (Vec2, Vec2) {
    let lo = Vec2::new(
        origin.x + r.x as f32 * cell[0],
        origin.y - (r.y + r.h) as f32 * cell[1],
    );
    let hi = Vec2::new(
        origin.x + (r.x + r.w) as f32 * cell[0],
        origin.y - r.y as f32 * cell[1],
    );
    (lo, hi)
}

#[cfg(test)]
#[path = "flat_physics_tests.rs"]
mod tests;
