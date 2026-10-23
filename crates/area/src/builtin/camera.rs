//! A camera: what the game shows. It looks along the entity's -Z, with
//! the entity's +Y up, the way lights shine along -Z.

use reflect::{Component, ComponentData, ComponentSchema, Fields, Value};

use super::field;

/// A camera at the entity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// How wide it sees, top to bottom, in degrees.
    pub fov: f32,
    /// The nearest it sees, in metres.
    pub near: f32,
    /// The farthest it sees, in metres.
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            fov: 50.0,
            near: 0.1,
            far: 2000.0,
        }
    }
}

impl Camera {
    /// The narrowest and widest field of view, in degrees.
    pub const FOV_RANGE: (f32, f32) = (5.0, 150.0);

    /// The same camera with anything unusable put back in range: a field
    /// of view in [`Self::FOV_RANGE`], a near plane above zero, and a far plane
    /// beyond the near one.
    #[must_use]
    pub fn sanitized(self) -> Self {
        let d = Self::default();
        let fov = if self.fov.is_finite() {
            self.fov.clamp(Self::FOV_RANGE.0, Self::FOV_RANGE.1)
        } else {
            d.fov
        };
        let near = if self.near.is_finite() && self.near > 0.0 {
            self.near.clamp(0.001, 1000.0)
        } else {
            d.near
        };
        let far = if self.far.is_finite() && self.far > near {
            self.far.min(1.0e7)
        } else {
            (near * 10_000.0).max(d.far)
        };
        Self { fov, near, far }
    }

    /// Its projection for an image `aspect` times as wide as it's tall,
    /// with depth reversed (1 at near, 0 at far), as the editor's camera
    /// and the renderer have it.
    pub fn projection(&self, aspect: f32) -> glam::Mat4 {
        let c = self.sanitized();
        let aspect = if aspect.is_finite() && aspect > 0.0 {
            aspect
        } else {
            1.0
        };
        let y_scale = 1.0 / (c.fov.to_radians() * 0.5).tan();
        let range = c.far - c.near;
        glam::Mat4::from_cols(
            glam::Vec4::new(y_scale / aspect, 0.0, 0.0, 0.0),
            glam::Vec4::new(0.0, y_scale, 0.0, 0.0),
            glam::Vec4::new(0.0, 0.0, c.near / range, -1.0),
            glam::Vec4::new(0.0, 0.0, c.far * c.near / range, 0.0),
        )
    }
}

impl Component for Camera {
    const NAME: &'static str = "Camera";

    fn schema() -> ComponentSchema {
        let d = Self::default();
        ComponentSchema {
            name: Self::NAME.into(),
            help: "What the game shows: it looks along the entity's -Z. Press Play to see \
                   through the first camera in the area."
                .into(),
            fields: vec![
                field::float(
                    "fov",
                    "Field of view",
                    d.fov,
                    field::hint(
                        (f64::from(Self::FOV_RANGE.0), f64::from(Self::FOV_RANGE.1)),
                        0.5,
                        "°",
                        "How wide it sees, top to bottom. 50° is like a normal photo; more \
                         sees more but stretches the edges.",
                    ),
                ),
                field::float(
                    "near",
                    "Near",
                    d.near,
                    field::hint(
                        (0.001, 1000.0),
                        0.01,
                        " m",
                        "The nearest it sees. Too small makes far surfaces flicker.",
                    ),
                ),
                field::float(
                    "far",
                    "Far",
                    d.far,
                    field::hint(
                        (1.0, 10_000_000.0),
                        10.0,
                        " m",
                        "The farthest it sees; anything beyond isn't drawn.",
                    ),
                ),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("fov", Value::Float(f64::from(self.fov)));
        data.set("near", Value::Float(f64::from(self.near)));
        data.set("far", Value::Float(f64::from(self.far)));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        let d = Self::default();
        let f = Fields(data);
        Self {
            fov: f.float("fov", d.fov),
            near: f.float("near", d.near),
            far: f.float("far", d.far),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unusable_values_are_put_back_in_range() {
        let wild = Camera {
            fov: f32::NAN,
            near: -1.0,
            far: 0.0,
        }
        .sanitized();
        assert_eq!(wild.fov, 50.0);
        assert!(wild.near > 0.0 && wild.far > wild.near);
        let tight = Camera {
            fov: 500.0,
            near: 5.0,
            far: 4.0,
        }
        .sanitized();
        assert_eq!(tight.fov, Camera::FOV_RANGE.1);
        assert!(tight.far > tight.near);
        assert!(Camera::default().projection(0.0).is_finite());
        // Reversed depth: 1 at near, 0 at far.
        let p = Camera::default().projection(1.0);
        let depth = |z: f32| {
            let v = p * glam::Vec4::new(0.0, 0.0, -z, 1.0);
            v.z / v.w
        };
        assert!((depth(0.1) - 1.0).abs() < 1e-4);
        assert!(depth(2000.0).abs() < 1e-4);
        assert!(wild.projection(f32::INFINITY).is_finite());
    }
}
