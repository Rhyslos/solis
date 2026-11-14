//! One voxel, and one chunk of them laid out in full.
//!
//! A voxel holds how far its point is from the ground's surface (negative
//! inside the ground, positive in the air) and which material it is. The
//! distance is kept in 1/64ths of a voxel in one signed byte, so it only
//! means something within two voxels of the surface; farther away it
//! stays at its end value, which is all any of the looks need to know.

use glam::{IVec3, UVec3};

/// Voxels along each side of a chunk.
pub const SIZE: usize = 32;
/// [`SIZE`] as a signed number, for chunk keys and world places.
pub const SIZE_I: i32 = SIZE as i32;
/// Voxels in a chunk.
pub const VOLUME: usize = SIZE * SIZE * SIZE;
/// Distance steps in one voxel's width.
pub const STEPS: f32 = 64.0;

/// One voxel: its distance from the surface and its material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Voxel {
    /// Distance from the surface in 1/64ths of a voxel, negative inside.
    pub distance: i8,
    /// Which of the ground's materials it is (an index into its palette).
    pub material: u8,
}

impl Voxel {
    /// Air, far from any surface.
    pub const AIR: Self = Self {
        distance: i8::MAX,
        material: 0,
    };

    /// Deep inside the ground, of `material`.
    pub const fn solid(material: u8) -> Self {
        Self {
            distance: i8::MIN,
            material,
        }
    }

    /// A voxel `distance` voxels from the surface (negative inside).
    /// Far air carries no material, so equal ground is always kept as
    /// equal voxels.
    pub fn at(distance: f32, material: u8) -> Self {
        let distance = quantise(distance);
        if distance == i8::MAX {
            return Self::AIR;
        }
        Self { distance, material }
    }

    /// Whether it is inside the ground.
    pub fn is_solid(self) -> bool {
        self.distance < 0
    }

    /// Its distance in voxels.
    pub fn distance(self) -> f32 {
        f32::from(self.distance) / STEPS
    }

    /// Whether its distance is at an end of its range, so it says only
    /// "air" or "solid" (what the sparse and octree stores fold away).
    pub fn is_far(self) -> bool {
        self.distance == i8::MAX || self.distance == i8::MIN
    }

    /// The two bytes a file or a store keeps.
    pub fn to_bits(self) -> u16 {
        u16::from(self.distance as u8) | (u16::from(self.material) << 8)
    }

    /// Back from [`Self::to_bits`].
    pub fn from_bits(bits: u16) -> Self {
        Self {
            distance: (bits & 0xff) as u8 as i8,
            material: (bits >> 8) as u8,
        }
    }
}

/// A distance in voxels as steps, held to the byte's range.
pub fn quantise(distance: f32) -> i8 {
    if distance.is_nan() {
        return i8::MAX;
    }
    (distance * STEPS)
        .round()
        .clamp(f32::from(i8::MIN), f32::from(i8::MAX)) as i8
}

/// A chunk's voxels in full, x fastest, then y, then z.
#[derive(Clone, PartialEq, Eq)]
pub struct Dense {
    voxels: Box<[Voxel]>,
}

impl Dense {
    /// A chunk of `voxel` all through.
    pub fn filled(voxel: Voxel) -> Self {
        Self {
            voxels: vec![voxel; VOLUME].into_boxed_slice(),
        }
    }

    /// The voxel at `p` (each part below [`SIZE`]).
    pub fn get(&self, p: UVec3) -> Voxel {
        self.voxels[index(p)]
    }

    /// Sets the voxel at `p`.
    pub fn set(&mut self, p: UVec3, voxel: Voxel) {
        self.voxels[index(p)] = voxel;
    }

    /// Every voxel, in order.
    pub fn voxels(&self) -> &[Voxel] {
        &self.voxels
    }

    /// Every voxel, in order, to change.
    pub fn voxels_mut(&mut self) -> &mut [Voxel] {
        &mut self.voxels
    }

    /// The one voxel it is made of, if it's the same all through.
    pub fn uniform(&self) -> Option<Voxel> {
        let first = self.voxels[0];
        self.voxels.iter().all(|v| *v == first).then_some(first)
    }
}

/// Where `p` is in a chunk's list.
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
            for material in [0, 1, 255] {
                let v = Voxel { distance, material };
                assert_eq!(Voxel::from_bits(v.to_bits()), v);
            }
        }
    }

    #[test]
    fn distances_are_held_to_two_voxels() {
        assert_eq!(quantise(10.0), i8::MAX);
        assert_eq!(quantise(-10.0), i8::MIN);
        assert_eq!(quantise(f32::NAN), i8::MAX);
        assert!((Voxel::at(0.5, 0).distance() - 0.5).abs() < 1.0 / STEPS);
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
