//! One voxel, and where voxels sit in chunks.
//!
//! A voxel is a one-metre cube's corner. It holds how far that point is
//! from the ground's surface (negative inside the ground, positive in the
//! air) and which ground kind it is (an index into the ground's palette).
//! The distance is kept in 1/64ths of a voxel in one signed byte, so it
//! only means something within two voxels of the surface; farther away it
//! stays at its end value, which is all the looks need (decision 0116).

use glam::{IVec3, UVec3};

/// Voxels along each side of a chunk.
pub const SIZE: usize = 32;
/// [`SIZE`] as a signed number, for chunk keys and world places.
pub const SIZE_I: i32 = SIZE as i32;
/// Voxels in a chunk.
pub const VOLUME: usize = SIZE * SIZE * SIZE;
/// Distance steps in one voxel's width.
pub const STEPS: i32 = 64;

/// One voxel: its distance from the surface and its ground kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Voxel {
    /// Distance from the surface in 1/64ths of a voxel, negative inside.
    pub distance: i8,
    /// Which of the ground's kinds it is (an index into its palette).
    pub kind: u8,
}

impl Voxel {
    /// Air, far from any surface. Far air carries no kind, so equal air
    /// is always kept as equal voxels.
    pub const AIR: Self = Self {
        distance: i8::MAX,
        kind: 0,
    };

    /// Deep inside the ground, of `kind`.
    pub const fn solid(kind: u8) -> Self {
        Self {
            distance: i8::MIN,
            kind,
        }
    }

    /// A voxel `steps` 1/64ths of a voxel from the surface (negative
    /// inside), held to the byte's range.
    pub fn at_steps(steps: i32, kind: u8) -> Self {
        let distance = steps.clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8;
        if distance == i8::MAX {
            return Self::AIR;
        }
        Self { distance, kind }
    }

    /// Whether it is inside the ground.
    pub fn is_solid(self) -> bool {
        self.distance < 0
    }

    /// Its distance in voxels.
    pub fn distance(self) -> f32 {
        f32::from(self.distance) / STEPS as f32
    }

    /// The two bytes a file keeps: the distance, then the kind.
    pub fn to_bits(self) -> u16 {
        u16::from(self.distance as u8) | (u16::from(self.kind) << 8)
    }

    /// Back from [`Self::to_bits`].
    pub fn from_bits(bits: u16) -> Self {
        Self {
            distance: (bits & 0xff) as u8 as i8,
            kind: (bits >> 8) as u8,
        }
    }
}

/// Where `p` is in a chunk's list: x fastest, then y, then z.
pub fn index(p: UVec3) -> usize {
    p.x as usize + SIZE * (p.y as usize + SIZE * p.z as usize)
}

/// The place in a chunk at list index `i`.
pub fn place(i: usize) -> UVec3 {
    UVec3::new(
        (i % SIZE) as u32,
        ((i / SIZE) % SIZE) as u32,
        (i / (SIZE * SIZE)) as u32,
    )
}

/// The chunk a world voxel is in, and where in it.
pub fn split(world: IVec3) -> (IVec3, UVec3) {
    let key = world.div_euclid(IVec3::splat(SIZE_I));
    let local = world.rem_euclid(IVec3::splat(SIZE_I)).as_uvec3();
    (key, local)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_voxel_survives_its_two_bytes() {
        for distance in [i8::MIN, -1, 0, 1, i8::MAX] {
            for kind in [0, 1, 255] {
                let v = Voxel { distance, kind };
                assert_eq!(Voxel::from_bits(v.to_bits()), v);
            }
        }
    }

    #[test]
    fn distances_are_held_to_two_voxels_and_far_air_has_no_kind() {
        assert_eq!(Voxel::at_steps(10_000, 3), Voxel::AIR);
        assert_eq!(Voxel::at_steps(-10_000, 3), Voxel::solid(3));
        assert_eq!(Voxel::at_steps(32, 1).distance(), 0.5);
    }

    #[test]
    fn places_split_into_chunks_below_zero_too() {
        assert_eq!(
            split(IVec3::new(-1, 0, 33)),
            (IVec3::new(-1, 0, 1), UVec3::new(31, 0, 1))
        );
        for i in [0, 1, 1000, VOLUME - 1] {
            assert_eq!(index(place(i)), i);
        }
    }
}
