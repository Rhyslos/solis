//! A new ground's starting shape: flat at a chosen height, or gently
//! rolling (the study's noise, a few metres up and down). Sculpting comes
//! in G2; Open worlds' recipes (decision 0117) will make shapes the same
//! way, chunk by chunk, on workers.

use crate::ground::Ground;
use crate::noise;
use crate::store::{Chunk, Fixed, origin};
use crate::voxel::{SIZE, STEPS, VOLUME, Voxel};
use glam::{IVec3, UVec3, Vec2};

/// How far the gentle shape goes up and down from its height, in metres.
pub const GENTLE_RISE: f32 = 6.0;
/// How wide the gentle shape's hills are, in metres.
pub const GENTLE_WIDTH: f32 = 96.0;
/// The ground kind a new ground is made of (the palette's first).
pub const FIRST_KIND: u8 = 0;

/// A new ground's starting shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Start {
    /// Flat, its surface `height` metres above the ground's floor.
    Flat {
        /// Metres above the floor.
        height: f32,
    },
    /// Gently rolling around `height`, the hills chosen by `seed`.
    Gentle {
        /// Metres above the floor, on average.
        height: f32,
        /// Which hills.
        seed: u32,
    },
}

impl Start {
    /// The surface's height at the column `(x, y)`, in metres.
    pub fn height_at(self, x: f32, y: f32) -> f32 {
        match self {
            Self::Flat { height } => height,
            Self::Gentle { height, seed } => {
                let n = noise::layered2(seed, Vec2::new(x, y) / GENTLE_WIDTH, 3);
                height + n * 2.0 * GENTLE_RISE
            }
        }
    }

    /// How far the surface can be from its height, in metres.
    fn reach(self) -> f32 {
        match self {
            Self::Flat { .. } => 0.0,
            Self::Gentle { .. } => 2.0 * GENTLE_RISE,
        }
    }

    /// The middle height.
    fn height(self) -> f32 {
        match self {
            Self::Flat { height } | Self::Gentle { height, .. } => height,
        }
    }

    /// The chunk at `key` of a ground in this shape.
    pub fn chunk(self, key: IVec3) -> Chunk {
        let low = origin(key).z as f32;
        let high = low + SIZE as f32;
        // Two voxels' room: farther from the surface, distances are at
        // their ends anyway.
        if low > self.height() + self.reach() + 3.0 {
            return Chunk::Uniform(Voxel::AIR);
        }
        if high < self.height() - self.reach() - 3.0 {
            return Chunk::Uniform(Voxel::solid(FIRST_KIND));
        }
        let first = origin(key);
        let mut columns = [[0f32; SIZE]; SIZE];
        for (y, row) in columns.iter_mut().enumerate() {
            for (x, h) in row.iter_mut().enumerate() {
                *h = self.height_at((first.x + x as i32) as f32, (first.y + y as i32) as f32);
            }
        }
        let mut voxels = vec![Voxel::AIR; VOLUME];
        for (i, v) in voxels.iter_mut().enumerate() {
            let p = crate::voxel::place(i);
            let h = columns[p.y as usize][p.x as usize];
            let above = (first.z + p.z as i32) as f32 - h;
            let steps = (above * STEPS as f32).round() as i32;
            *v = Voxel::at_steps(steps, FIRST_KIND);
        }
        Chunk::keep(&voxels)
    }

    /// A whole ground `extent` chunks big in this shape.
    pub fn ground(self, extent: UVec3) -> Ground<Fixed> {
        Ground::make(extent, |key| self.chunk(key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    #[test]
    fn a_flat_ground_keeps_sky_and_rock_as_one_voxel_each() {
        let ground = Start::Flat { height: 40.5 }.ground(UVec3::new(2, 2, 4));
        let store = ground.store();
        let mixed: Vec<IVec3> = crate::store::keys(store.extent())
            .filter(|k| !ground.chunk(*k).is_some_and(Chunk::is_uniform))
            .collect();
        assert_eq!(mixed.len(), 4, "only the layer the surface crosses");
        assert!(mixed.iter().all(|k| k.z == 1));
        assert_eq!(
            ground.chunk(IVec3::new(0, 0, 3)),
            Some(&Chunk::Uniform(Voxel::AIR))
        );
    }

    #[test]
    fn a_gentle_ground_stays_near_its_height_and_is_the_same_each_time() {
        let start = Start::Gentle {
            height: 50.0,
            seed: 3,
        };
        for i in 0..200 {
            let (x, y) = (i as f32 * 7.3, i as f32 * 3.1);
            let h = start.height_at(x, y);
            assert!((h - 50.0).abs() <= 2.0 * GENTLE_RISE, "{h}");
            assert_eq!(h.to_bits(), start.height_at(x, y).to_bits());
        }
    }
}
