//! How models are shaded in the viewport: the standard look, the stylized
//! (cel-shaded) look, or unlit. Chosen in the viewport's look menu (bottom
//! left) and remembered between sessions. Until materials become assets
//! of their own (Phase 2), the choice applies to every model at once
//! ([decision 0013](../../../docs/decisions/0013-looks-for-the-whole-view-first.md)).

use serde::{Deserialize, Serialize};

/// The ways models can be shaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LookKind {
    /// Realistic: metal and roughness, highlights, soft sky light.
    #[default]
    Standard,
    /// Cel-shaded: flat bands of light, crisp highlights, outlines.
    Stylized,
    /// Colours and textures only, without light.
    Unlit,
}

impl LookKind {
    /// Every look, in menu order.
    pub const ALL: [Self; 3] = [Self::Standard, Self::Stylized, Self::Unlit];

    /// The look's name, as the menu shows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::Stylized => "Stylized",
            Self::Unlit => "Unlit",
        }
    }

    /// What the look is for, when pointed at in the menu.
    pub fn hint(self) -> &'static str {
        match self {
            Self::Standard => "Realistic light: metal, roughness and soft light from the sky",
            Self::Stylized => "Cel-shaded: flat bands of light, crisp highlights and outlines",
            Self::Unlit => "Colours and textures as painted, without light",
        }
    }
}

/// The look and the stylized look's settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Look {
    /// Which look.
    pub kind: LookKind,
    /// The stylized look's bands of light, 2 to 4.
    pub bands: u8,
    /// Whether the stylized look draws outlines.
    pub outlines: bool,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            kind: LookKind::Standard,
            bands: 3,
            outlines: true,
        }
    }
}

impl Look {
    /// The fewest and most bands of light the menu offers.
    pub const BANDS: std::ops::RangeInclusive<u8> = 2..=4;

    /// Outline width in points (multiplied by the screen's scale).
    pub const OUTLINE_POINTS: f32 = 1.5;

    /// The look with anything out of range (from a damaged save) put back
    /// in range.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            bands: self.bands.clamp(*Self::BANDS.start(), *Self::BANDS.end()),
            ..self
        }
    }

    /// The outline width in pixels for a screen at `pixels_per_point`, or
    /// 0 when no outlines are drawn.
    pub fn outline_px(self, pixels_per_point: f32) -> f32 {
        if self.kind == LookKind::Stylized && self.outlines && pixels_per_point.is_finite() {
            Self::OUTLINE_POINTS * pixels_per_point.clamp(0.5, 4.0)
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standard_look_comes_first() {
        assert_eq!(Look::default().kind, LookKind::Standard);
        assert_eq!(LookKind::ALL[0], LookKind::Standard);
    }

    #[test]
    fn outlines_belong_to_the_stylized_look_and_follow_screen_scale() {
        let stylized = Look {
            kind: LookKind::Stylized,
            ..Look::default()
        };
        assert_eq!(stylized.outline_px(2.0), 3.0);
        assert_eq!(Look::default().outline_px(2.0), 0.0);
        let off = Look {
            outlines: false,
            ..stylized
        };
        assert_eq!(off.outline_px(1.0), 0.0);
        assert!(stylized.outline_px(f32::NAN) == 0.0);
    }

    #[test]
    fn a_damaged_save_is_put_back_in_range() {
        let look: Look = ron::from_str("(bands: 200)").unwrap();
        assert_eq!(look.sanitized().bands, 4);
        let look: Look = ron::from_str("(kind: Stylized)").unwrap();
        assert_eq!(look.bands, 3, "missing settings take their defaults");
        assert!(ron::from_str::<Look>("(kind: Shiny)").is_err());
    }
}
