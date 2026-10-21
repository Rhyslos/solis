//! The models imported into the viewport. Until entities arrive in Phase 2,
//! an imported model sits in the viewport where its file places it, listed
//! in the Scene panel, and isn't saved.

use glam::Vec3;

/// Which imported model, as the app numbers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModelId(pub u64);

/// What the editor knows about an imported model (the app holds the model
/// itself, on the GPU).
#[derive(Debug, Clone, PartialEq)]
pub struct ModelInfo {
    /// Its number.
    pub id: ModelId,
    /// A name to show, such as the file's name.
    pub name: String,
    /// How many triangles it has.
    pub triangles: usize,
    /// The lowest corner of the box it fits in, in metres.
    pub min: Vec3,
    /// The highest corner.
    pub max: Vec3,
}

impl ModelInfo {
    /// Its size along each axis, for the console: "2.0 × 6.0 × 2.2 m".
    pub fn size_text(&self) -> String {
        let s = self.max - self.min;
        let fmt = |v: f32| {
            if v >= 100.0 {
                format!("{v:.0}")
            } else if v >= 1.0 {
                format!("{v:.1}")
            } else {
                format!("{v:.2}")
            }
        };
        format!("{} × {} × {} m", fmt(s.x), fmt(s.y), fmt(s.z))
    }
}

/// Whole numbers with thousands separated, for counts: "12,345".
pub fn grouped(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_are_grouped_in_thousands() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(1000), "1,000");
        assert_eq!(grouped(1_234_567), "1,234,567");
    }

    #[test]
    fn sizes_read_naturally() {
        let info = ModelInfo {
            id: ModelId(1),
            name: "Crate".into(),
            triangles: 12,
            min: Vec3::new(-1.0, -0.05, 0.0),
            max: Vec3::new(1.0, 0.05, 250.0),
        };
        assert_eq!(info.size_text(), "2.0 × 0.10 × 250 m");
    }
}
