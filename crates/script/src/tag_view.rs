//! Questions about tags, answered for one tick (stage 1, tags): whether an
//! entity carries a tag (`hasTag`), every entity of a tag within a
//! distance, nearest first (`World:within`), and the nearest one
//! (`World:nearest`).
//!
//! A [`TagView`] is made once a tick, from where the tagged entities are
//! as the tick starts ([decision 0033](../../../docs/decisions/0033-game-code-sees-the-start-of-the-tick.md)),
//! and shared read-only by every core: every core gets the same answers,
//! in the same order. Ties in distance go by entity ID.
//!
//! Each tag's members are kept in ID order, and, when there are enough of
//! them, sorted into a uniform grid over the ground (X and Y) by a
//! counting sort: a question looks only in the cells its distance reaches.
//! The cell is sized from the area the tag's members cover, about two to a
//! cell, and the grid never has more than four cells for each member, so
//! it costs little to make however they're spread. A distance whose
//! square of cells would cover more cells than the tag has members looks
//! through the tag's own list instead, which is then quicker. Both ways
//! measure the same and sort the same, so they always agree.
//!
//! In a 2D project distance is measured flat, across the ground, whatever
//! the layers (the project lead's choice); in 3D (or a project of both) it
//! is measured in 3D. The grid lies on the ground either way.

use glam::{Vec2, Vec3};
use reflect::{EntityId, TagId};
use std::collections::HashMap;

/// Below this many members a tag is looked through whole: quicker than a
/// grid.
const GRID_FROM: usize = 32;

/// The most cells a tag's grid has for each member.
const CELLS_PER_MEMBER: usize = 4;

/// The smallest a cell may be, in metres.
const MIN_CELL: f32 = 0.25;

/// The tags of a tick: who carries which, and where each tag's members
/// are.
#[derive(Debug, Default, PartialEq)]
pub struct TagView {
    /// Each tag by its name (as game code says it: `Enemy`).
    by_name: HashMap<String, TagId>,
    /// The tags each tagged entity carries.
    carried: reflect::IdMap<Vec<TagId>>,
    /// Each tag's members.
    members: HashMap<TagId, Members>,
    /// Whether distance is measured across the ground only (a 2D project).
    flat: bool,
}

/// One tag's members, in ID order, with their places, and the grid when
/// there are enough of them.
#[derive(Debug, Default, PartialEq)]
struct Members {
    ids: Vec<EntityId>,
    places: Vec<Vec3>,
    grid: Option<Grid>,
}

/// A uniform grid over the ground: each cell's members (by their number
/// in [`Members`]), in ID order.
#[derive(Debug, PartialEq)]
struct Grid {
    origin: Vec2,
    cell: f32,
    cols: i64,
    rows: i64,
    /// Where each cell's run starts in `entries` (one more than cells).
    starts: Vec<u32>,
    entries: Vec<u32>,
}

impl TagView {
    /// The view of a tick: the project's tags (`(ID, name)`), and every
    /// tagged entity in ID order with where it is and what it carries (as
    /// `world::World::tagged` gives them). Tags not in the project's list
    /// are left out; so are places that aren't numbers, from distances.
    /// `flat`: a 2D project, measuring across the ground.
    pub fn new(
        tags: &[(TagId, String)],
        tagged: &[(EntityId, Vec3, Vec<TagId>)],
        flat: bool,
    ) -> Self {
        let mut view = Self {
            by_name: tags.iter().map(|(id, name)| (name.clone(), *id)).collect(),
            flat,
            ..Self::default()
        };
        let known: std::collections::HashSet<TagId> = tags.iter().map(|(id, _)| *id).collect();
        for (entity, place, carried) in tagged {
            let carried: Vec<TagId> = carried.iter().copied().filter(|t| known.contains(t)).collect();
            if carried.is_empty() {
                continue;
            }
            if place.is_finite() {
                for tag in &carried {
                    let members = view.members.entry(*tag).or_default();
                    members.ids.push(*entity);
                    members.places.push(*place);
                }
            }
            view.carried.insert(*entity, carried);
        }
        for members in view.members.values_mut() {
            members.sort_by_id();
            members.grid = Grid::new(&members.places);
        }
        view
    }

    /// The tag game code calls `name` (`Enemy`, from `Tag.Enemy`).
    pub fn tag(&self, name: &str) -> Option<TagId> {
        self.by_name.get(name).copied()
    }

    /// Whether `entity` carried the tag called `name` as the tick started.
    pub fn has(&self, entity: EntityId, name: &str) -> bool {
        let (Some(tag), Some(carried)) = (self.tag(name), self.carried.get(&entity)) else {
            return false;
        };
        carried.contains(&tag)
    }

    /// Every entity carrying the tag `name` within `radius` of `place`,
    /// nearest first, ties by ID. None for a radius below zero or a place
    /// or radius that isn't a number.
    pub fn within(&self, name: &str, place: Vec3, radius: f32) -> Vec<EntityId> {
        let Some(members) = self.tag(name).and_then(|t| self.members.get(&t)) else {
            return Vec::new();
        };
        if !place.is_finite() || !radius.is_finite() || radius < 0.0 {
            return Vec::new();
        }
        let reach = radius * radius;
        let mut found: Vec<(f32, EntityId)> = Vec::new();
        let mut look = |i: usize| {
            let d = self.distance_squared(members.places[i], place);
            if d <= reach {
                found.push((d, members.ids[i]));
            }
        };
        match members.grid.as_ref().and_then(|g| g.cells_near(place, radius, members.ids.len())) {
            Some((grid, x, y)) => {
                for row in y.0..=y.1 {
                    for col in x.0..=x.1 {
                        grid.each_in(col, row, &mut look);
                    }
                }
            }
            None => (0..members.ids.len()).for_each(look),
        }
        found.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        found.into_iter().map(|(_, id)| id).collect()
    }

    /// The entity carrying the tag `name` nearest to `place` (ties by ID),
    /// or none when none does or the place isn't a number.
    pub fn nearest(&self, name: &str, place: Vec3) -> Option<EntityId> {
        let members = self.tag(name).and_then(|t| self.members.get(&t))?;
        if !place.is_finite() {
            return None;
        }
        let mut best: Option<(f32, EntityId)> = None;
        let look = |i: usize, best: &mut Option<(f32, EntityId)>| {
            let d = self.distance_squared(members.places[i], place);
            let this = (d, members.ids[i]);
            if best.is_none_or(|b| d.total_cmp(&b.0).then(this.1.cmp(&b.1)).is_lt()) {
                *best = Some(this);
            }
        };
        match &members.grid {
            Some(grid) => {
                let (cx, cy, first, last) = grid.rings(place);
                for k in first..=last {
                    grid.ring(cx, cy, k, &mut |col, row| {
                        grid.each_in(col, row, &mut |i| look(i, &mut best));
                    });
                    // Nothing in the next ring is nearer than this (a ring
                    // spare for rounding).
                    let clear = (k - 1).max(0) as f32 * grid.cell;
                    if k >= 1 && best.is_some_and(|b| b.0 <= clear * clear) {
                        break;
                    }
                }
            }
            None => (0..members.ids.len()).for_each(|i| look(i, &mut best)),
        }
        best.map(|(_, id)| id)
    }

    /// How far apart two places are, squared: across the ground in a 2D
    /// project, in 3D otherwise.
    fn distance_squared(&self, a: Vec3, b: Vec3) -> f32 {
        if self.flat {
            a.truncate().distance_squared(b.truncate())
        } else {
            a.distance_squared(b)
        }
    }
}

impl Members {
    /// Puts the members in ID order (they arrive in it from the world; a
    /// caller's list in another order still gives the same answers).
    fn sort_by_id(&mut self) {
        if self.ids.is_sorted() {
            return;
        }
        let mut both: Vec<(EntityId, Vec3)> =
            self.ids.iter().copied().zip(self.places.iter().copied()).collect();
        both.sort_unstable_by_key(|(id, _)| *id);
        (self.ids, self.places) = both.into_iter().unzip();
    }
}

impl Grid {
    /// The grid of `places` (in ID order), or none for too few.
    fn new(places: &[Vec3]) -> Option<Self> {
        if places.len() < GRID_FROM {
            return None;
        }
        let ground = places.iter().map(|p| p.truncate());
        let (low, high) = ground.fold((Vec2::INFINITY, Vec2::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(p), hi.max(p))
        });
        let span = (high - low).max(Vec2::splat(MIN_CELL));
        let n = places.len();
        // About two members to a cell, over the area they cover.
        let mut cell = (span.x * span.y * 2.0 / n as f32).sqrt().max(MIN_CELL);
        let most = (n * CELLS_PER_MEMBER) as f64;
        let count = |cell: f32| {
            let cols = (f64::from(span.x) / f64::from(cell)).floor() + 1.0;
            let rows = (f64::from(span.y) / f64::from(cell)).floor() + 1.0;
            (cols, rows)
        };
        while {
            let (cols, rows) = count(cell);
            cols * rows > most
        } {
            cell *= 2.0;
        }
        let (cols, rows) = count(cell);
        let (cols, rows) = (cols as i64, rows as i64);
        let mut grid = Self {
            origin: low,
            cell,
            cols,
            rows,
            starts: vec![0; (cols * rows) as usize + 1],
            entries: vec![0; n],
        };
        // A counting sort: members in ID order stay in it in each cell.
        let cells: Vec<usize> = places.iter().map(|p| grid.index_of(p.truncate())).collect();
        for &c in &cells {
            grid.starts[c + 1] += 1;
        }
        for c in 1..grid.starts.len() {
            grid.starts[c] += grid.starts[c - 1];
        }
        let mut next = grid.starts.clone();
        for (i, &c) in cells.iter().enumerate() {
            grid.entries[next[c] as usize] = i as u32;
            next[c] += 1;
        }
        Some(grid)
    }

    /// The column and row a place on the ground falls in, unbounded (a
    /// place off the grid gives one off it), kept within a range no
    /// question overflows.
    fn cell_of(&self, ground: Vec2) -> (i64, i64) {
        const FAR: f64 = (1u64 << 40) as f64;
        let at = |v: f32, o: f32| (f64::from(v - o) / f64::from(self.cell)).floor().clamp(-FAR, FAR) as i64;
        (at(ground.x, self.origin.x), at(ground.y, self.origin.y))
    }

    /// The cell a member's place is kept in.
    fn index_of(&self, ground: Vec2) -> usize {
        let (col, row) = self.cell_of(ground);
        let (col, row) = (col.clamp(0, self.cols - 1), row.clamp(0, self.rows - 1));
        (row * self.cols + col) as usize
    }

    /// Calls `look` with each member in the cell at `col`, `row`.
    fn each_in(&self, col: i64, row: i64, look: &mut impl FnMut(usize)) {
        let c = (row * self.cols + col) as usize;
        let (start, end) = (self.starts[c] as usize, self.starts[c + 1] as usize);
        for &i in &self.entries[start..end] {
            look(i as usize);
        }
    }

    /// The columns and rows (each from, to) a question of `radius` about
    /// `place` looks in, a cell more each way for rounding; none when
    /// looking through all `members` is quicker.
    fn cells_near(
        &self,
        place: Vec3,
        radius: f32,
        members: usize,
    ) -> Option<(&Self, (i64, i64), (i64, i64))> {
        let ground = place.truncate();
        let (x0, y0) = self.cell_of(ground - Vec2::splat(radius));
        let (x1, y1) = self.cell_of(ground + Vec2::splat(radius));
        let (x0, y0) = ((x0 - 1).max(0), (y0 - 1).max(0));
        let (x1, y1) = ((x1 + 1).min(self.cols - 1), (y1 + 1).min(self.rows - 1));
        if x0 > x1 || y0 > y1 {
            // Wholly off the grid: nothing's there.
            return Some((self, (0, -1), (0, -1)));
        }
        let cells = (x1 - x0 + 1) as f64 * (y1 - y0 + 1) as f64;
        (cells <= members as f64).then_some((self, (x0, x1), (y0, y1)))
    }

    /// The cell `place` is in, and the first and last rings round it that
    /// hold any of the grid's cells (counting diagonals as one).
    fn rings(&self, place: Vec3) -> (i64, i64, i64, i64) {
        let (cx, cy) = self.cell_of(place.truncate());
        let outside = |v: i64, n: i64| (-v).max(v - (n - 1)).max(0);
        let first = outside(cx, self.cols).max(outside(cy, self.rows));
        let last = cx
            .max(self.cols - 1 - cx)
            .max(cy)
            .max(self.rows - 1 - cy)
            .max(first);
        (cx, cy, first, last)
    }

    /// Visits the cells of the grid at ring `k` round `cx`, `cy` (those
    /// `k` cells away, counting diagonals as one).
    fn ring(&self, cx: i64, cy: i64, k: i64, visit: &mut impl FnMut(i64, i64)) {
        let (x0, x1) = ((cx - k).max(0), (cx + k).min(self.cols - 1));
        if k == 0 {
            if (0..self.cols).contains(&cx) && (0..self.rows).contains(&cy) {
                visit(cx, cy);
            }
            return;
        }
        for row in [cy - k, cy + k] {
            if (0..self.rows).contains(&row) {
                for col in x0..=x1 {
                    visit(col, row);
                }
            }
        }
        let (y0, y1) = ((cy - k + 1).max(0), (cy + k - 1).min(self.rows - 1));
        for col in [cx - k, cx + k] {
            if (0..self.cols).contains(&col) {
                for row in y0..=y1 {
                    visit(col, row);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "tag_view_tests.rs"]
mod tests;
