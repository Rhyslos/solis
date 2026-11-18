//! Smooth random numbers over space (value noise): the same place and
//! seed give the same number on every PC.
//!
//! The randomness is integer hashing, and the blending is plain `f32`
//! arithmetic written out step by step: Rust never fuses a multiply and
//! an add on its own (that would round differently on some processors),
//! and nothing here asks it to. So a ground made from noise is the same
//! everywhere, which game code's edits and replays rely on.
//!
//! It lives in the `ground` crate for now: the ground's starting shape is
//! its only user, and the recipes of the Open worlds phase (decision 0117)
//! will be the next. Materials' and lighting's noise runs on the graphics
//! card, in shaders. It needs nothing but `glam`, so moving it to a
//! layer-0 crate when a second crate needs it on the processor is a file
//! moved, not a change.

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
    let inner = t * 6.0 - 15.0;
    let inner = t * inner + 10.0;
    t * t * t * inner
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    let step = (b - a) * t;
    a + step
}

/// Noise from -1 to 1 at `p`, changing over about one unit.
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

/// Noise from -1 to 1 at `p` on a plane.
pub fn noise2(seed: u32, p: Vec2) -> f32 {
    noise3(seed, p.extend(0.5))
}

/// `octaves` layers of [`noise2`], each twice as fine and half as strong
/// (from -1 to 1).
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

    const PINNED: [u32; 3] = [1_021_125_984, 3_189_806_303, 1_045_740_970];

    /// Fixed numbers: if these change, grounds made from noise change
    /// shape on every computer, so a change here is a breaking one.
    #[test]
    fn noise_gives_the_numbers_it_always_gave() {
        let got: Vec<u32> = [(1, 0.25, 0.5), (2, 10.5, -3.25), (99, -7.0, 123.75)]
            .iter()
            .map(|&(seed, x, y)| noise2(seed, Vec2::new(x, y)).to_bits())
            .collect();
        let again: Vec<u32> = [(1, 0.25, 0.5), (2, 10.5, -3.25), (99, -7.0, 123.75)]
            .iter()
            .map(|&(seed, x, y)| noise2(seed, Vec2::new(x, y)).to_bits())
            .collect();
        assert_eq!(got, again);
        assert_eq!(hash(0), 0);
        assert_eq!(got, PINNED);
    }
}
