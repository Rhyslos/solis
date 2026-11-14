//! Which size of chunk is drawn where: a layout of chunk columns, each
//! a square of the ground at one level, tiling the world without gaps or
//! overlaps. Two ways of choosing them are compared:
//!
//! - [`Layout::rings`]: nested rings of fixed-size chunks round the
//!   camera, each level a square of `width` columns with the next level
//!   down in its middle (a clipmap made of chunks).
//! - [`Layout::octree`]: the world split like a quadtree, a column split
//!   in four wherever the camera is nearer than `split` times its width,
//!   then split further until neighbours differ by at most one level.
//!
//! Columns stand the world's whole height: levels differ across the
//! ground, never up and down it.

use super::chunk_metres;
use glam::{IVec2, Vec2, Vec3};

/// One column of chunks: its level, and its key in that level's columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Column {
    /// Its level of detail.
    pub level: u32,
    /// Where it is, in columns of its level.
    pub key: IVec2,
}

impl Column {
    fn order(&self) -> (u32, i32, i32) {
        (self.level, self.key.y, self.key.x)
    }
}

impl PartialOrd for Column {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Column {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.order().cmp(&other.order())
    }
}

/// The world's columns, and the level drawn over each level-0 column.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// The world's size in level-0 columns (32 m each).
    across: IVec2,
    levels: Vec<u8>,
}

impl Layout {
    /// Every column at `level` (the study's same-size grounds).
    pub fn uniform(across: IVec2, level: u32) -> Self {
        Self {
            across,
            levels: vec![level as u8; (across.x * across.y).max(0) as usize],
        }
    }

    /// Nested rings round `camera` (metres): level `top` covers the
    /// world, and each level below it a square `width` of its columns
    /// across (even), in the middle of the one above, its edges on the
    /// level above's column edges.
    pub fn rings(across: IVec2, camera: Vec2, top: u32, width: i32) -> Self {
        let mut out = Self::uniform(across, top);
        let width = width.max(2) & !1;
        // The square of the level above, in its own columns.
        let mut outer: Option<(IVec2, IVec2)> = None;
        for level in (0..top).rev() {
            let size = chunk_metres(level) as f32;
            let middle = (camera / size).floor().as_ivec2();
            let mut lo = middle - width / 2;
            lo = lo.div_euclid(IVec2::splat(2)) * 2;
            if let Some((outer_lo, outer_hi)) = outer {
                // Inside the level above, a column of it in from its edge
                // so that no column has a neighbour two levels away.
                let min = outer_lo * 2 + 2;
                let max = outer_hi * 2 - 2 - width;
                lo = lo.max(min).min(max.max(min));
            }
            let hi = lo + width;
            out.fill(level, lo, hi);
            outer = Some((lo, hi));
        }
        out
    }

    /// The world split as a quadtree from columns of level `top`: a
    /// column splits while `camera` (metres, its height too) is nearer
    /// than `split` times its width, then columns split until every
    /// neighbour is within a level. Returns the layout and how many
    /// columns split only to keep neighbours within a level.
    pub fn octree(across: IVec2, camera: Vec3, top: u32, split: f32) -> (Self, usize) {
        let mut out = Self::uniform(across, top);
        let top_across = Self::columns_across(across, top);
        let mut work: Vec<Column> = (0..top_across.y)
            .flat_map(|y| (0..top_across.x).map(move |x| IVec2::new(x, y)))
            .map(|key| Column { level: top, key })
            .collect();
        while let Some(column) = work.pop() {
            let size = chunk_metres(column.level) as f32;
            if column.level == 0 || out.distance(column, camera) >= split * size {
                continue;
            }
            let level = column.level - 1;
            out.fill(level, column.key * 2, column.key * 2 + 2);
            for d in [IVec2::ZERO, IVec2::X, IVec2::Y, IVec2::ONE] {
                work.push(Column {
                    level,
                    key: column.key * 2 + d,
                });
            }
        }
        let forced = out.balance();
        (out, forced)
    }

    /// Splits any column with a neighbour two or more levels finer,
    /// again and again until there are none; returns how many split.
    fn balance(&mut self) -> usize {
        let mut forced = 0;
        loop {
            let mut split = Vec::new();
            for column in self.columns() {
                if column.level >= 2 && self.finest_round(column) + 1 < column.level {
                    split.push(column);
                }
            }
            if split.is_empty() {
                return forced;
            }
            forced += split.len();
            for c in split {
                self.fill(c.level - 1, c.key * 2, c.key * 2 + 2);
            }
        }
    }

    /// The finest level touching `column` from outside, corners too.
    fn finest_round(&self, column: Column) -> u32 {
        let lo = column.key << column.level;
        let hi = lo + (1 << column.level);
        let mut finest = u32::MAX;
        for x in lo.x - 1..=hi.x {
            for y in [lo.y - 1, hi.y] {
                finest = finest.min(self.level_of(IVec2::new(x, y)).unwrap_or(u32::MAX));
            }
        }
        for y in lo.y..hi.y {
            for x in [lo.x - 1, hi.x] {
                finest = finest.min(self.level_of(IVec2::new(x, y)).unwrap_or(u32::MAX));
            }
        }
        finest
    }

    /// The distance from `camera` to the nearest point of `column`'s
    /// square (at the camera's height above the ground's floor).
    fn distance(&self, column: Column, camera: Vec3) -> f32 {
        let size = chunk_metres(column.level) as f32;
        let lo = column.key.as_vec2() * size;
        let flat = camera.truncate().clamp(lo, lo + size) - camera.truncate();
        flat.extend(camera.z).length()
    }

    fn columns_across(across: IVec2, level: u32) -> IVec2 {
        (across + (1 << level) - 1) >> level
    }

    /// Sets the columns of `level` from `lo` up to (not including) `hi`
    /// (in that level's columns) to be drawn at `level`.
    fn fill(&mut self, level: u32, lo: IVec2, hi: IVec2) {
        let lo = (lo << level).max(IVec2::ZERO);
        let hi = (hi << level).min(self.across);
        for y in lo.y..hi.y {
            for x in lo.x..hi.x {
                self.levels[(x + self.across.x * y) as usize] = level as u8;
            }
        }
    }

    /// The world's size in level-0 columns.
    pub fn across(&self) -> IVec2 {
        self.across
    }

    /// The level drawn over level-0 column `cell`, if it's in the world.
    pub fn level_of(&self, cell: IVec2) -> Option<u32> {
        if cell.cmplt(IVec2::ZERO).any() || cell.cmpge(self.across).any() {
            return None;
        }
        Some(u32::from(
            self.levels[(cell.x + self.across.x * cell.y) as usize],
        ))
    }

    /// The level drawn at `point` (metres), the nearest column's if it's
    /// past the world's edge.
    pub fn level_at(&self, point: Vec2) -> u32 {
        let cell = (point / chunk_metres(0) as f32).floor().as_ivec2();
        self.level_of(cell.clamp(IVec2::ZERO, self.across - 1))
            .unwrap_or(0)
    }

    /// The coarsest level of the columns whose squares (edges included)
    /// hold `point`, a place in whole metres: on a border between
    /// columns, the coarser side's.
    pub fn coarsest_at(&self, point: IVec2) -> u32 {
        let size = chunk_metres(0);
        let mut coarsest = 0;
        for dy in [0, -1] {
            for dx in [0, -1] {
                let d = IVec2::new(dx, dy);
                let on_edge = point.rem_euclid(IVec2::splat(size)).cmpeq(IVec2::ZERO);
                if (dx < 0 && !on_edge.x) || (dy < 0 && !on_edge.y) {
                    continue;
                }
                let cell = point.div_euclid(IVec2::splat(size)) + d;
                let cell = cell.clamp(IVec2::ZERO, self.across - 1);
                coarsest = coarsest.max(self.level_of(cell).unwrap_or(0));
            }
        }
        coarsest
    }

    /// Every column, in a fixed order (level, then y, then x).
    pub fn columns(&self) -> Vec<Column> {
        let mut out = Vec::new();
        for y in 0..self.across.y {
            for x in 0..self.across.x {
                let cell = IVec2::new(x, y);
                let Some(level) = self.level_of(cell) else {
                    continue;
                };
                let key = cell >> level;
                if key << level == cell {
                    out.push(Column { level, key });
                }
            }
        }
        out.sort();
        out
    }

    /// The most two touching columns' levels differ by, corners too.
    pub fn worst_step(&self) -> u32 {
        let mut worst = 0;
        for y in 0..self.across.y {
            for x in 0..self.across.x {
                let here = self.level_of(IVec2::new(x, y)).unwrap_or(0);
                for d in [IVec2::X, IVec2::Y, IVec2::ONE, IVec2::new(1, -1)] {
                    if let Some(there) = self.level_of(IVec2::new(x, y) + d) {
                        worst = worst.max(here.abs_diff(there));
                    }
                }
            }
        }
        worst
    }
}

/// The columns in `after` that weren't in `before`: what must be made
/// when the camera moves from one to the other.
pub fn new_columns(before: &Layout, after: &Layout) -> Vec<Column> {
    let old: std::collections::HashSet<Column> = before.columns().into_iter().collect();
    after
        .columns()
        .into_iter()
        .filter(|c| !old.contains(c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn covers_once(layout: &Layout) {
        let mut seen = vec![0; (layout.across.x * layout.across.y) as usize];
        for c in layout.columns() {
            let lo = c.key << c.level;
            for y in lo.y..lo.y + (1 << c.level) {
                for x in lo.x..lo.x + (1 << c.level) {
                    if x < layout.across.x && y < layout.across.y {
                        seen[(x + layout.across.x * y) as usize] += 1;
                    }
                }
            }
        }
        assert!(seen.iter().all(|n| *n == 1), "every place drawn once");
    }

    #[test]
    fn rings_tile_the_world_and_step_one_level_at_a_time() {
        let across = IVec2::splat(640);
        for camera in [Vec2::splat(10_240.0), Vec2::new(100.0, 20_000.0)] {
            let layout = Layout::rings(across, camera, 7, 8);
            covers_once(&layout);
            assert_eq!(layout.worst_step(), 1);
            assert_eq!(layout.level_at(camera), 0);
        }
    }

    #[test]
    fn the_octree_tiles_the_world_and_is_balanced() {
        let across = IVec2::splat(640);
        let (layout, _) = Layout::octree(across, Vec3::new(10_240.0, 9_000.0, 60.0), 7, 2.5);
        covers_once(&layout);
        assert!(layout.worst_step() <= 1);
        assert_eq!(layout.level_at(Vec2::new(10_240.0, 9_000.0)), 0);
    }

    #[test]
    fn a_border_point_reads_the_coarser_side() {
        let layout = Layout::rings(IVec2::splat(64), Vec2::splat(1024.0), 3, 8);
        // Level 0's square is 256 m round the camera; its edge is level 1's.
        let inside = layout.columns().iter().any(|c| c.level == 0);
        assert!(inside);
        let lo = (0..64)
            .find(|x| layout.level_of(IVec2::new(*x, 32)) == Some(0))
            .unwrap_or(0);
        let edge = IVec2::new(lo * 32, 32 * 32 + 5);
        assert_eq!(layout.coarsest_at(edge), 1);
        assert_eq!(layout.coarsest_at(edge + IVec2::X), 0);
    }

    #[test]
    fn moving_the_camera_a_little_changes_few_columns() {
        let across = IVec2::splat(640);
        let a = Layout::rings(across, Vec2::splat(10_000.0), 7, 8);
        let b = Layout::rings(across, Vec2::splat(10_100.0), 7, 8);
        let changed = new_columns(&a, &b).len();
        assert!(changed > 0 && changed < 80, "{changed}");
        assert!(new_columns(&a, &a).is_empty());
    }
}
