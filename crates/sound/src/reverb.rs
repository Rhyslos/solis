//! Reverb zones (Phase 7, S6; decision 0056, question 6): where the
//! listener is decides the echo. A zone is a box or a ball with a soft
//! edge and a preset (a small room, a hall, a cave, a sewer, outdoors);
//! inside it, every sound placed in the world gets that reverb, and
//! across the soft edge it fades in and out. Where zones overlap the
//! smaller one wins (a cupboard inside a hall sounds like a cupboard).
//!
//! Plain data and sums: the app gives each zone with the listener in its
//! own space ([`ZoneHere`]), [`mix`] says how much of up to
//! [`MAX_HEARD`] zones' reverbs is heard, and the device sets its reverb
//! sends to match.

/// How many zones' reverbs can be heard at once (one reverb send each).
pub const MAX_HEARD: usize = 4;
/// The largest a zone may be, in metres (a box's side, a ball's radius).
pub const MAX_SIZE_M: f32 = 100_000.0;

/// How a zone sounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Preset {
    /// A small room: short and close.
    SmallRoom,
    /// A hall: long and smooth.
    #[default]
    Hall,
    /// A cave: long, dark and full of echoes.
    Cave,
    /// A sewer: hard and ringing.
    Sewer,
    /// Outdoors: barely there, a little air.
    Outdoors,
}

impl Preset {
    /// Every preset, in the order Properties lists them.
    pub const ALL: [Self; 5] = [
        Self::SmallRoom,
        Self::Hall,
        Self::Cave,
        Self::Sewer,
        Self::Outdoors,
    ];

    /// As saved and shown.
    pub fn name(self) -> &'static str {
        match self {
            Self::SmallRoom => "Small room",
            Self::Hall => "Hall",
            Self::Cave => "Cave",
            Self::Sewer => "Sewer",
            Self::Outdoors => "Outdoors",
        }
    }

    /// The preset by its name (`None` for one it doesn't know).
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.name() == name)
    }

    /// Its reverb: how long the echoes last (feedback, 0 to 1) and how
    /// dull they get (damping, 0 bright to 1 dull).
    pub fn settings(self) -> (f64, f64) {
        match self {
            Self::SmallRoom => (0.55, 0.55),
            Self::Hall => (0.85, 0.3),
            Self::Cave => (0.93, 0.15),
            Self::Sewer => (0.88, 0.05),
            Self::Outdoors => (0.35, 0.8),
        }
    }
}

/// A zone's shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// A box of this size, centred on its entity.
    Box([f32; 3]),
    /// A ball of this radius, centred on its entity.
    Ball(f32),
}

/// A zone, with the listener given in the zone's own space (its entity's
/// place and turn taken away).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneHere {
    /// Its shape.
    pub shape: Shape,
    /// How far in from its edge the reverb is full, in metres.
    pub soft_m: f32,
    /// How it sounds.
    pub preset: Preset,
    /// How strong its reverb is, 0 to 1.
    pub strength: f32,
    /// Where the listener is, in the zone's space.
    pub listener: [f32; 3],
}

impl ZoneHere {
    /// How far inside the zone the listener is, in metres (negative
    /// outside).
    fn depth(&self) -> f32 {
        let p = self.listener;
        match self.shape {
            Shape::Box(size) => {
                // Inside: the distance to the nearest face; outside, how
                // far out (only its sign matters).
                let mut inside = f32::INFINITY;
                let mut out = 0.0_f32;
                for axis in 0..3 {
                    let half = (size[axis] * 0.5).max(0.0);
                    let d = half - p[axis].abs();
                    inside = inside.min(d);
                    out = out.max(-d);
                }
                if out > 0.0 { -out } else { inside }
            }
            Shape::Ball(radius) => {
                radius.max(0.0) - (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt()
            }
        }
    }

    /// How much of the zone the listener is in, 0 outside to 1 past the
    /// soft edge.
    pub fn weight(&self) -> f32 {
        let depth = self.depth();
        if !depth.is_finite() || depth < 0.0 {
            return 0.0;
        }
        if self.soft_m <= 0.0 || !self.soft_m.is_finite() {
            return 1.0;
        }
        (depth / self.soft_m).clamp(0.0, 1.0)
    }

    /// How big the zone is, to let the smaller win.
    fn volume(&self) -> f32 {
        match self.shape {
            Shape::Box([x, y, z]) => (x * y * z).abs(),
            Shape::Ball(r) => 4.19 * r.abs().powi(3),
        }
    }
}

/// How much of each zone's reverb is heard, from 0 to 1, the most heard
/// first, at most [`MAX_HEARD`]: smaller zones first take what share
/// they cover, and larger ones get what's left (so a small room inside a
/// hall sounds like the room, and walking out blends into the hall).
/// Gives back each zone's place in `zones` and how much is heard.
pub fn mix(zones: &[ZoneHere]) -> Vec<(usize, f32)> {
    let mut order: Vec<usize> = (0..zones.len())
        .filter(|i| zones[*i].weight() > 0.0)
        .collect();
    order.sort_by(|a, b| {
        zones[*a]
            .volume()
            .total_cmp(&zones[*b].volume())
            .then(a.cmp(b))
    });
    let mut left = 1.0_f32;
    let mut heard = Vec::new();
    for i in order {
        if left <= 0.0 || heard.len() == MAX_HEARD {
            break;
        }
        let share = zones[i].weight() * left;
        left -= share;
        let strength = if zones[i].strength.is_finite() {
            zones[i].strength.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if share * strength > 0.0 {
            heard.push((i, share * strength));
        }
    }
    heard
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone(shape: Shape, at: [f32; 3]) -> ZoneHere {
        ZoneHere {
            shape,
            soft_m: 2.0,
            preset: Preset::Hall,
            strength: 1.0,
            listener: at,
        }
    }

    #[test]
    fn inside_full_outside_none_and_the_edge_fades() {
        let hall = Shape::Box([20.0, 20.0, 6.0]);
        assert_eq!(zone(hall, [0.0, 0.0, 0.0]).weight(), 1.0);
        assert_eq!(zone(hall, [20.0, 0.0, 0.0]).weight(), 0.0);
        // 1 m in from the wall, with a 2 m soft edge: half.
        assert!((zone(hall, [9.0, 0.0, 0.0]).weight() - 0.5).abs() < 1e-5);
        let ball = Shape::Ball(5.0);
        assert_eq!(zone(ball, [0.0, 0.0, 0.0]).weight(), 1.0);
        assert!((zone(ball, [0.0, 4.5, 0.0]).weight() - 0.25).abs() < 1e-5);
        assert_eq!(zone(ball, [0.0, 6.0, 0.0]).weight(), 0.0);
        // Moving across the edge, it changes smoothly, never jumping.
        let mut last = 0.0;
        for step in 0..=120 {
            let x = 12.0 - step as f32 * 0.05;
            let w = zone(hall, [x, 0.0, 0.0]).weight();
            assert!((w - last).abs() <= 0.03, "a jump at {x}");
            last = w;
        }
        assert_eq!(last, 1.0);
    }

    #[test]
    fn no_soft_edge_is_a_hard_edge_and_odd_numbers_are_outside() {
        let mut hard = zone(Shape::Ball(5.0), [0.0, 4.9, 0.0]);
        hard.soft_m = 0.0;
        assert_eq!(hard.weight(), 1.0);
        assert_eq!(zone(Shape::Ball(5.0), [f32::NAN, 0.0, 0.0]).weight(), 0.0);
        assert_eq!(zone(Shape::Box([-3.0, 2.0, 2.0]), [0.0; 3]).weight(), 0.0);
    }

    #[test]
    fn the_smaller_zone_wins_where_they_overlap() {
        let mut cupboard = zone(Shape::Box([2.0, 2.0, 2.0]), [0.0; 3]);
        cupboard.preset = Preset::SmallRoom;
        cupboard.soft_m = 0.5;
        let hall = zone(Shape::Box([40.0, 40.0, 10.0]), [0.0; 3]);
        let heard = mix(&[hall, cupboard]);
        assert_eq!(heard, [(1, 1.0)], "deep in the cupboard: only it");
        // In the cupboard's doorway (half its soft edge): half each.
        cupboard.listener = [0.75, 0.0, 0.0];
        let heard = mix(&[hall, cupboard]);
        assert_eq!(heard.len(), 2);
        assert_eq!(heard[0].0, 1);
        assert!((heard[0].1 - 0.5).abs() < 1e-5 && (heard[1].1 - 0.5).abs() < 1e-5);
        // Strength scales what's heard.
        let mut faint = hall;
        faint.strength = 0.25;
        assert_eq!(mix(&[faint]), [(0, 0.25)]);
        assert!(mix(&[]).is_empty());
    }

    #[test]
    fn at_most_four_are_heard() {
        let zones: Vec<ZoneHere> = (1..=8)
            .map(|i| {
                let mut z = zone(Shape::Ball(i as f32 * 10.0), [0.0; 3]);
                // Each lets some through to the next.
                z.soft_m = i as f32 * 20.0;
                z
            })
            .collect();
        let heard = mix(&zones);
        assert_eq!(heard.len(), MAX_HEARD);
        assert_eq!(heard[0].0, 0, "the smallest first");
        assert!(heard.iter().map(|(_, s)| s).sum::<f32>() <= 1.0 + 1e-5);
    }

    #[test]
    fn the_component_s_presets_are_these() {
        let names: Vec<&str> = Preset::ALL.iter().map(|p| p.name()).collect();
        assert_eq!(names, area::builtin::ReverbZone::PRESETS);
    }

    #[test]
    fn presets_have_names_and_settings() {
        for preset in Preset::ALL {
            assert_eq!(Preset::named(preset.name()), Some(preset));
            let (feedback, damping) = preset.settings();
            assert!((0.0..1.0).contains(&feedback) && (0.0..=1.0).contains(&damping));
        }
        assert_eq!(Preset::named("Bathroom"), None);
    }
}
