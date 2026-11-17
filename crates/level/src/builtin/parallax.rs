//! A parallax layer ([decision 0094](../../../../docs/decisions/0094-parallax-layers.md)):
//! turns the entity's Sprite into a layer that moves with the camera at
//! its own speed (far hills slower, leaves in front faster), repeated
//! side by side if wanted, drawn behind everything or in front of it.

use reflect::{Component, ComponentData, ComponentSchema, Fields, Value};

use super::field;

/// How a sprite moves as a parallax layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parallax {
    /// How far it moves across with the camera: 0 stays still on the
    /// screen, 1 moves with the world, above 1 faster.
    pub scroll_x: f32,
    /// How far it moves up and down with the camera.
    pub scroll_y: f32,
    /// Drawn again and again side by side, at its sheet's width.
    pub repeat_x: bool,
    /// Drawn again and again above and below, at its sheet's height.
    pub repeat_y: bool,
    /// Drawn over everything instead of behind it.
    pub in_front: bool,
}

impl Default for Parallax {
    fn default() -> Self {
        Self {
            scroll_x: 0.5,
            scroll_y: 0.5,
            repeat_x: true,
            repeat_y: false,
            in_front: false,
        }
    }
}

impl Parallax {
    /// The fastest a layer scrolls: four times the world.
    pub const MAX_SCROLL: f32 = 4.0;

    /// How far a layer is drawn from where it's placed once the camera
    /// has `moved` from where it started: the whole layer goes along by
    /// `(1 − scroll) × moved`, across and up apart, so on the screen it
    /// moves `scroll` times as far as the world. Nothing while the camera
    /// is at its start.
    pub fn offset(&self, moved: [f32; 2]) -> [f32; 2] {
        let s = self.sanitized();
        [(1.0 - s.scroll_x) * moved[0], (1.0 - s.scroll_y) * moved[1]]
    }

    /// Its speeds within 0 to [`Self::MAX_SCROLL`] (a broken one: 1, with
    /// the world).
    pub fn sanitized(&self) -> Self {
        let speed = |v: f32| {
            if v.is_finite() {
                v.clamp(0.0, Self::MAX_SCROLL)
            } else {
                1.0
            }
        };
        Self {
            scroll_x: speed(self.scroll_x),
            scroll_y: speed(self.scroll_y),
            ..*self
        }
    }
}

impl Component for Parallax {
    const NAME: &'static str = "Parallax";

    fn schema() -> ComponentSchema {
        let d = Self::default();
        let scroll = (0.0, f64::from(Self::MAX_SCROLL));
        ComponentSchema {
            name: Self::NAME.into(),
            help: "Makes the entity's Sprite a parallax layer: it moves with the camera at its \
                   own speed, so far hills drift slowly and leaves in front rush past, and a \
                   flat 2D level feels deep. It's exactly where you put it while the game's \
                   camera is where it starts."
                .into(),
            fields: vec![
                field::float(
                    "scroll_x",
                    "Scroll across",
                    d.scroll_x,
                    field::hint(
                        scroll,
                        0.01,
                        "×",
                        "How far it moves across with the camera: 0 stays still on the screen \
                         (the sky), 0.5 half as far (far hills), 1 with the world, 2 twice as \
                         far (something close in front).",
                    ),
                ),
                field::float(
                    "scroll_y",
                    "Scroll up",
                    d.scroll_y,
                    field::hint(
                        scroll,
                        0.01,
                        "×",
                        "How far it moves up and down with the camera, the same way.",
                    ),
                ),
                field::boolean(
                    "repeat_x",
                    "Repeat across",
                    d.repeat_x,
                    "Drawn again and again side by side, so it never runs out (a sky, a line \
                     of hills). Draw its picture so its left and right edges meet.",
                ),
                field::boolean(
                    "repeat_y",
                    "Repeat up",
                    d.repeat_y,
                    "Drawn again and again above and below.",
                ),
                field::boolean(
                    "in_front",
                    "In front",
                    d.in_front,
                    "Drawn over everything instead of behind it: leaves, posts or fog close \
                     to the camera. Give it a scroll above 1.",
                ),
            ],
        }
    }

    fn to_data(&self) -> ComponentData {
        let mut data = ComponentData::default();
        data.set("scroll_x", Value::Float(f64::from(self.scroll_x)));
        data.set("scroll_y", Value::Float(f64::from(self.scroll_y)));
        data.set("repeat_x", Value::Bool(self.repeat_x));
        data.set("repeat_y", Value::Bool(self.repeat_y));
        data.set("in_front", Value::Bool(self.in_front));
        data
    }

    fn from_data(data: &ComponentData) -> Self {
        let d = Self::default();
        let f = Fields(data);
        Self {
            scroll_x: f.float("scroll_x", d.scroll_x),
            scroll_y: f.float("scroll_y", d.scroll_y),
            repeat_x: f.bool("repeat_x", d.repeat_x),
            repeat_y: f.bool("repeat_y", d.repeat_y),
            in_front: f.bool("in_front", d.in_front),
        }
        .sanitized()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_layer_goes_along_with_the_camera_by_what_it_doesn_t_scroll() {
        let p = Parallax::default();
        assert_eq!(p.offset([0.0, 0.0]), [0.0, 0.0], "at the start, as placed");
        // The camera 8 m right and 2 m up: a half-speed layer goes 4 m
        // right and 1 m up, so on the screen it moved half as far.
        assert_eq!(p.offset([8.0, 2.0]), [4.0, 1.0]);
        let sky = Parallax {
            scroll_x: 0.0,
            scroll_y: 0.0,
            ..p
        };
        assert_eq!(
            sky.offset([-30.0, 7.0]),
            [-30.0, 7.0],
            "stays on the screen"
        );
        let world = Parallax {
            scroll_x: 1.0,
            scroll_y: 1.0,
            ..p
        };
        assert_eq!(world.offset([-30.0, 7.0]), [0.0, 0.0]);
        let near = Parallax {
            scroll_x: 2.0,
            scroll_y: 1.0,
            ..p
        };
        assert_eq!(
            near.offset([8.0, 2.0]),
            [-8.0, 0.0],
            "twice as far the other way"
        );
    }

    #[test]
    fn damaged_speeds_are_made_sensible() {
        let mut data = Parallax::default().to_data();
        data.set("scroll_x", Value::Float(f64::NAN));
        data.set("scroll_y", Value::Float(99.0));
        let read = Parallax::from_data(&data);
        assert_eq!(read.scroll_x, 0.5, "a broken speed read: the default");
        assert_eq!(read.scroll_y, Parallax::MAX_SCROLL);
        data.set("scroll_y", Value::Float(-2.0));
        assert_eq!(Parallax::from_data(&data).scroll_y, 0.0);
        let p = Parallax {
            scroll_x: f32::INFINITY,
            ..Parallax::default()
        };
        // A broken speed in memory moves with the world.
        assert_eq!(p.sanitized().scroll_x, 1.0);
        assert!(p.offset([5.0, 5.0]).iter().all(|v| v.is_finite()));
    }
}
