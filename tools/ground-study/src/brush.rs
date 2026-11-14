//! The round sculpting brush: one dab of it, as a frame of a stroke
//! makes, applied to the voxels it reaches.

use crate::voxel::{Dense, SIZE_I, Voxel, split};
use glam::{IVec3, UVec3, Vec3};

/// What a dab does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    /// Raises the ground, filling with this material.
    Add(u8),
    /// Digs the ground away.
    Dig,
    /// Evens out bumps.
    Smooth,
    /// Paints this material on the ground it covers.
    Paint(u8),
}

/// One dab of the round brush.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Brush {
    /// Its middle, in voxels.
    pub center: Vec3,
    /// Its radius, in voxels.
    pub radius: f32,
    /// How much one dab does, 0 to 1.
    pub strength: f32,
    /// What it does.
    pub action: Action,
}

impl Brush {
    /// The voxels it may change: from the first up to (not including)
    /// the second.
    pub fn reach(&self) -> (IVec3, IVec3) {
        let margin = self.radius + 3.0;
        let lo = (self.center - margin).floor().as_ivec3();
        let hi = (self.center + margin).ceil().as_ivec3() + 1;
        (lo, hi)
    }

    /// Applies one dab to `region`; returns the box of voxels it changed
    /// (from, up to), if any.
    pub fn apply(&self, region: &mut Region) -> Option<(IVec3, IVec3)> {
        let before = region.voxels.clone();
        let strength = self.strength.clamp(0.0, 1.0);
        let mut changed: Option<(IVec3, IVec3)> = None;
        for i in 0..region.voxels.len() {
            let p = region.place(i);
            let old = before[i];
            let sphere = (p.as_vec3() - self.center).length() - self.radius;
            let new = match self.action {
                Action::Add(material) => {
                    let d = blend(old.distance(), old.distance().min(sphere), strength);
                    let material = if !old.is_solid() && d < 0.0 {
                        material
                    } else {
                        old.material
                    };
                    Voxel::at(d, material)
                }
                Action::Dig => Voxel::at(
                    blend(old.distance(), old.distance().max(-sphere), strength),
                    old.material,
                ),
                Action::Smooth if sphere < 0.0 => {
                    let mean = region.mean_around(&before, p).unwrap_or(old.distance());
                    Voxel::at(blend(old.distance(), mean, strength), old.material)
                }
                Action::Paint(material) if sphere < 0.0 && old.is_solid() => {
                    Voxel { material, ..old }
                }
                Action::Smooth | Action::Paint(_) => old,
            };
            if new != old {
                region.voxels[i] = new;
                changed = Some(match changed {
                    None => (p, p + 1),
                    Some((lo, hi)) => (lo.min(p), hi.max(p + 1)),
                });
            }
        }
        changed
    }
}

fn blend(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

/// A box of voxels gathered from the chunks a dab reaches.
pub struct Region {
    min: IVec3,
    size: UVec3,
    voxels: Vec<Voxel>,
}

impl Region {
    /// An empty box at `min`, `size` voxels big.
    pub fn new(min: IVec3, size: UVec3) -> Self {
        Self {
            min,
            size,
            voxels: vec![Voxel::AIR; (size.x * size.y * size.z) as usize],
        }
    }

    fn place(&self, i: usize) -> IVec3 {
        let (x, y) = (self.size.x as usize, self.size.y as usize);
        self.min + IVec3::new((i % x) as i32, ((i / x) % y) as i32, (i / (x * y)) as i32)
    }

    fn slot(&self, p: IVec3) -> Option<usize> {
        let q = p - self.min;
        let size = self.size.as_ivec3();
        if q.cmplt(IVec3::ZERO).any() || q.cmpge(size).any() {
            return None;
        }
        Some((q.x + size.x * (q.y + size.y * q.z)) as usize)
    }

    /// Fills every voxel from `read` (given its world place).
    pub fn fill(&mut self, read: impl Fn(IVec3) -> Voxel) {
        for i in 0..self.voxels.len() {
            self.voxels[i] = read(self.place(i));
        }
    }

    fn mean_around(&self, voxels: &[Voxel], p: IVec3) -> Option<f32> {
        let mut sum = 0.0;
        let mut count = 0.0;
        for d in [
            IVec3::ZERO,
            IVec3::X,
            -IVec3::X,
            IVec3::Y,
            -IVec3::Y,
            IVec3::Z,
            -IVec3::Z,
        ] {
            if let Some(i) = self.slot(p + d) {
                sum += voxels[i].distance();
                count += 1.0;
            }
        }
        (count > 0.0).then(|| sum / count)
    }

    /// Writes its voxels that fall in the chunk `key` into `dense`;
    /// returns whether any changed.
    pub fn write_into(&self, key: IVec3, dense: &mut Dense) -> bool {
        let lo = (key * SIZE_I).max(self.min);
        let hi = ((key + 1) * SIZE_I).min(self.min + self.size.as_ivec3());
        let mut changed = false;
        for z in lo.z..hi.z {
            for y in lo.y..hi.y {
                for x in lo.x..hi.x {
                    let p = IVec3::new(x, y, z);
                    let (_, local) = split(p);
                    if let Some(i) = self.slot(p)
                        && dense.get(local) != self.voxels[i]
                    {
                        dense.set(local, self.voxels[i]);
                        changed = true;
                    }
                }
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_floor() -> Region {
        let mut region = Region::new(IVec3::ZERO, UVec3::splat(16));
        region.fill(|p| Voxel::at(p.z as f32 - 8.0 + 0.5, 1));
        region
    }

    #[test]
    fn adding_raises_and_fills_with_its_material() {
        let mut region = flat_floor();
        let brush = Brush {
            center: Vec3::new(8.0, 8.0, 8.0),
            radius: 3.0,
            strength: 1.0,
            action: Action::Add(2),
        };
        assert!(brush.apply(&mut region).is_some());
        let top = region.slot(IVec3::new(8, 8, 10)).unwrap();
        assert!(region.voxels[top].is_solid());
        assert_eq!(region.voxels[top].material, 2);
        let below = region.slot(IVec3::new(8, 8, 2)).unwrap();
        assert_eq!(region.voxels[below].material, 1, "old ground keeps its own");
    }

    #[test]
    fn digging_makes_a_hole_and_nothing_far_changes() {
        let mut region = flat_floor();
        let brush = Brush {
            center: Vec3::new(8.0, 8.0, 7.0),
            radius: 2.0,
            strength: 1.0,
            action: Action::Dig,
        };
        let (lo, hi) = brush.apply(&mut region).unwrap();
        assert!(!region.voxels[region.slot(IVec3::new(8, 8, 6)).unwrap()].is_solid());
        assert!(lo.cmpge(IVec3::splat(3)).all() && hi.cmple(IVec3::splat(13)).all());
    }

    #[test]
    fn a_dab_in_the_air_changes_nothing() {
        let mut region = Region::new(IVec3::ZERO, UVec3::splat(8));
        region.fill(|_| Voxel::AIR);
        let brush = Brush {
            center: Vec3::splat(4.0),
            radius: 1.0,
            strength: 1.0,
            action: Action::Dig,
        };
        assert_eq!(brush.apply(&mut region), None);
    }
}
