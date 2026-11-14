//! Smooth random numbers over space (value noise), for the study's test
//! grounds: the same place always gives the same number, so every run
//! measures the same ground.

use glam::{Vec2, Vec3};

fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

fn lattice(seed: u32, x: i32, y: i32, z: i32) -> f32 {
    let h = hash(seed ^ hash(x as u32 ^ hash(y as u32 ^ hash(z as u32))));
    (h & 0xffff) as f32 / 32767.5 - 1.0
}

fn smooth(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Noise in -1 to 1 at `p`, changing over about one unit.
pub fn noise3(seed: u32, p: Vec3) -> f32 {
    let base = p.floor();
    let f = p - base;
    let (x, y, z) = (base.x as i32, base.y as i32, base.z as i32);
    let (u, v, w) = (smooth(f.x), smooth(f.y), smooth(f.z));
    let corner = |dx: i32, dy: i32, dz: i32| lattice(seed, x + dx, y + dy, z + dz);
    let near = lerp(
        lerp(corner(0, 0, 0), corner(1, 0, 0), u),
        lerp(corner(0, 1, 0), corner(1, 1, 0), u),
        v,
    );
    let far = lerp(
        lerp(corner(0, 0, 1), corner(1, 0, 1), u),
        lerp(corner(0, 1, 1), corner(1, 1, 1), u),
        v,
    );
    lerp(near, far, w)
}

/// Noise in -1 to 1 at `p` on a plane.
pub fn noise2(seed: u32, p: Vec2) -> f32 {
    noise3(seed, p.extend(0.5))
}

/// `octaves` layers of [`noise2`], each twice as fine and half as strong.
pub fn layered2(seed: u32, p: Vec2, octaves: u32) -> f32 {
    let (mut sum, mut scale, mut weight) = (0.0, 1.0, 0.5);
    for octave in 0..octaves {
        sum += noise2(seed.wrapping_add(octave), p * scale) * weight;
        scale *= 2.0;
        weight *= 0.5;
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_the_same_each_time_and_in_range() {
        for i in 0..1000 {
            let p = Vec3::new(i as f32 * 0.37, i as f32 * -0.11, i as f32 * 0.05);
            let n = noise3(7, p);
            assert_eq!(n, noise3(7, p));
            assert!((-1.0..=1.0).contains(&n));
        }
    }
}
