//! Projectiles: thousands of small, fast things that aren't entities
//! (lasers, bolts, tracers), each with a place, a speed along its
//! direction, a time left and a colour. They're moved in jobs every tick
//! and expire when their time is up. Hitting things waits for Phase 6's
//! physics.

use glam::Vec3;

/// The most projectiles alive at once; more aren't fired.
pub const MAX_PROJECTILES: usize = 1_000_000;

/// The longest a projectile lives, in seconds.
pub const MAX_LIFE: f32 = 60.0;

/// The fastest a projectile flies, in metres a second.
pub const MAX_SPEED: f32 = 100_000.0;

/// The brightest a projectile's colour may be (each part), so a typo
/// can't blind the picture.
pub const MAX_GLOW: f32 = 100.0;

/// What game code asks for when it fires.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shot {
    /// Where it starts, in metres.
    pub from: Vec3,
    /// Which way it flies (any length but zero).
    pub direction: Vec3,
    /// How fast, in metres a second.
    pub speed: f32,
    /// How long it lives, in seconds.
    pub life: f32,
    /// Its colour, linear RGB; above 1 glows.
    pub colour: [f32; 3],
}

/// A projectile in flight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projectile {
    /// Where it is, in metres.
    pub position: Vec3,
    /// Its speed and direction, metres a second.
    pub velocity: Vec3,
    /// Seconds it has left.
    pub left: f32,
    /// Its colour, linear RGB.
    pub colour: [f32; 3],
}

/// Every projectile in flight, in the order they were fired.
#[derive(Debug, Clone, Default)]
pub struct Projectiles {
    flying: Vec<Projectile>,
    /// Shots not fired because [`MAX_PROJECTILES`] were already flying.
    refused: u64,
}

impl Projectiles {
    /// Fires `shot`. Returns whether it flies: not when it isn't sensible
    /// numbers (no direction, no time to live) or when
    /// [`MAX_PROJECTILES`] already are (those are counted, see
    /// [`Self::refused`]). Speeds, lives and colours past their limits
    /// are brought within them.
    pub fn fire(&mut self, shot: Shot) -> bool {
        let Some(direction) = shot.direction.try_normalize() else {
            return false;
        };
        let sensible = shot.from.is_finite()
            && shot.speed.is_finite()
            && shot.life.is_finite()
            && shot.life > 0.0
            && shot.colour.iter().all(|c| c.is_finite());
        if !sensible {
            return false;
        }
        if self.flying.len() >= MAX_PROJECTILES {
            self.refused += 1;
            return false;
        }
        self.flying.push(Projectile {
            position: shot.from,
            velocity: direction * shot.speed.clamp(0.0, MAX_SPEED),
            left: shot.life.min(MAX_LIFE),
            colour: shot.colour.map(|c| c.clamp(0.0, MAX_GLOW)),
        });
        true
    }

    /// Puts back the projectiles a save game held (decision 0038), in
    /// their order, within the limits [`Self::fire`] keeps (a save's
    /// own always are); those that aren't sensible numbers, or past
    /// [`MAX_PROJECTILES`], are left out.
    pub fn restore(&mut self, flying: Vec<Projectile>) {
        self.flying = flying
            .into_iter()
            .filter(|p| {
                p.position.is_finite()
                    && p.velocity.is_finite()
                    && p.left.is_finite()
                    && p.colour.iter().all(|c| c.is_finite())
            })
            .map(|p| Projectile {
                velocity: p.velocity.clamp_length_max(MAX_SPEED),
                left: p.left.min(MAX_LIFE),
                colour: p.colour.map(|c| c.clamp(0.0, MAX_GLOW)),
                ..p
            })
            .take(MAX_PROJECTILES)
            .collect();
    }

    /// Moves every projectile on by `dt` seconds, in jobs, and lets go of
    /// those whose time is up. The same on any number of cores.
    pub fn tick(&mut self, dt: f32) {
        use rayon::prelude::*;
        if !(dt.is_finite() && dt > 0.0) {
            return;
        }
        self.flying.par_iter_mut().for_each(|p| {
            p.position += p.velocity * dt;
            p.left -= dt;
        });
        // Within a tenth of a millisecond is up: adding up ticks leaves a
        // little over.
        self.flying.retain(|p| p.left > 1e-4);
    }

    /// The projectiles in flight.
    pub fn flying(&self) -> &[Projectile] {
        &self.flying
    }

    /// How many are in flight.
    pub fn len(&self) -> usize {
        self.flying.len()
    }

    /// Whether none are.
    pub fn is_empty(&self) -> bool {
        self.flying.is_empty()
    }

    /// How many shots weren't fired because too many were flying.
    pub fn refused(&self) -> u64 {
        self.refused
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot(life: f32) -> Shot {
        Shot {
            from: Vec3::ZERO,
            direction: Vec3::new(0.0, 2.0, 0.0),
            speed: 10.0,
            life,
            colour: [4.0, 0.2, 0.1],
        }
    }

    #[test]
    fn projectiles_fly_their_way_and_expire_on_time() {
        let mut all = Projectiles::default();
        assert!(all.fire(shot(0.5)));
        assert!(all.fire(shot(1.0)));
        for _ in 0..30 {
            all.tick(1.0 / 60.0);
        }
        // Half a second at 10 m/s along +Y (the direction's length doesn't
        // matter): the first has just gone, the second is 5 m on.
        assert_eq!(all.len(), 1);
        let p = all.flying()[0];
        assert!(
            (p.position - Vec3::new(0.0, 5.0, 0.0)).length() < 1e-3,
            "{p:?}"
        );
        for _ in 0..30 {
            all.tick(1.0 / 60.0);
        }
        assert!(all.is_empty());
    }

    #[test]
    fn nonsense_isn_t_fired_and_limits_hold() {
        let mut all = Projectiles::default();
        assert!(!all.fire(Shot {
            direction: Vec3::ZERO,
            ..shot(1.0)
        }));
        assert!(!all.fire(Shot {
            from: Vec3::NAN,
            ..shot(1.0)
        }));
        assert!(!all.fire(shot(0.0)));
        assert!(!all.fire(shot(f32::INFINITY)));
        assert!(all.fire(Shot {
            speed: 1e12,
            colour: [1e9, -5.0, 0.5],
            ..shot(1e6)
        }));
        let p = all.flying()[0];
        assert_eq!(p.velocity.length(), MAX_SPEED);
        assert_eq!(p.left, MAX_LIFE);
        assert_eq!(p.colour, [MAX_GLOW, 0.0, 0.5]);
        // Silly ticks change nothing.
        all.tick(f32::NAN);
        all.tick(-1.0);
        assert_eq!(all.flying()[0], p);
    }

    #[test]
    fn the_cap_holds_and_counts_what_it_refused() {
        let mut all = Projectiles::default();
        for _ in 0..MAX_PROJECTILES {
            all.fire(shot(1.0));
        }
        assert!(!all.fire(shot(1.0)));
        assert!(!all.fire(shot(1.0)));
        assert_eq!(all.len(), MAX_PROJECTILES);
        assert_eq!(all.refused(), 2);
    }

    #[test]
    fn the_same_on_one_core_or_four() {
        let run = |threads| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap()
                .install(|| {
                    let mut all = Projectiles::default();
                    for i in 0..10_000 {
                        all.fire(Shot {
                            direction: Vec3::new((i as f32).sin(), (i as f32).cos(), 0.1),
                            life: 0.2 + (i % 50) as f32 * 0.02,
                            ..shot(1.0)
                        });
                    }
                    for _ in 0..40 {
                        all.tick(1.0 / 60.0);
                    }
                    all.flying().to_vec()
                })
        };
        let (one, four) = (run(1), run(4));
        assert!(!one.is_empty());
        assert_eq!(one, four);
    }

    #[test]
    fn projectiles_from_a_save_keep_fire_s_limits() {
        let mut all = Projectiles::default();
        all.restore(vec![
            Projectile {
                position: Vec3::ZERO,
                velocity: Vec3::new(f32::MAX, 0.0, 0.0),
                left: 3.0e38,
                colour: [1.0e30, -5.0, 0.5],
            },
            Projectile {
                position: Vec3::NAN,
                velocity: Vec3::X,
                left: 1.0,
                colour: [1.0; 3],
            },
        ]);
        assert_eq!(all.flying().len(), 1, "the NaN one left out");
        let p = all.flying()[0];
        assert!(p.velocity.length() <= MAX_SPEED * 1.0001);
        assert_eq!(p.left, MAX_LIFE);
        assert_eq!(p.colour, [MAX_GLOW, 0.0, 0.5]);
        all.tick(1.0);
        assert!(all.flying()[0].position.is_finite());
    }
}
