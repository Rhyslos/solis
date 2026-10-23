//! What the editor knows about imported models (the app holds the models
//! themselves, on the GPU), and lights a model's file brings with it.

use glam::Vec3;
use reflect::AssetId;

/// What the editor knows about an imported model.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelInfo {
    /// The model asset.
    pub asset: AssetId,
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

/// What kind of light a model's file carries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileLightKind {
    /// A sun: it sets the area's sun.
    Sun,
    /// A point light.
    Point,
    /// A spotlight, its cone in radians: full strength inside `inner`,
    /// nothing past `outer`.
    Spot {
        /// Where the fade starts.
        inner: f32,
        /// Where the light ends.
        outer: f32,
    },
}

/// A light a model's file carries, placed, in Blender's units.
#[derive(Debug, Clone, PartialEq)]
pub struct FileLight {
    /// Its name in the file.
    pub name: String,
    /// Sun, point or spot.
    pub kind: FileLightKind,
    /// Where it is.
    pub position: Vec3,
    /// The way it shines.
    pub direction: Vec3,
    /// Its colour, linear 0 to 1.
    pub color: [f32; 3],
    /// Watts, or W/m² for a sun.
    pub strength: f32,
    /// How far it reaches, if the file says.
    pub range: Option<f32>,
}

/// One linear colour channel as sRGB, 0 to 1.
pub(crate) fn linear_to_srgb(c: f32) -> f32 {
    let c = if c.is_finite() {
        c.clamp(0.0, 1.0)
    } else {
        1.0
    };
    if c <= 0.003_130_8 {
        c * 12.92
    } else if c >= 1.0 {
        // The formula would leave white a hair short of 1.
        1.0
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
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
            asset: AssetId::random(),
            name: "Crate".into(),
            triangles: 12,
            min: Vec3::new(-1.0, -0.05, 0.0),
            max: Vec3::new(1.0, 0.05, 250.0),
        };
        assert_eq!(info.size_text(), "2.0 × 0.10 × 250 m");
    }

    #[test]
    fn linear_colours_become_srgb() {
        assert_eq!(linear_to_srgb(0.0), 0.0);
        assert!((linear_to_srgb(1.0) - 1.0).abs() < 1e-6);
        assert!((linear_to_srgb(0.214) - 0.5).abs() < 0.01);
        assert_eq!(linear_to_srgb(f32::NAN), 1.0);
    }
}
