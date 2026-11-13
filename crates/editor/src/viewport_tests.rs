//! The viewport's own tests: the cursor readout, the tools' keys, the
//! view handed to the renderer, overlays as saved, and no renderer.

use super::*;
use crate::theme::backdrop::rgb;

#[test]
fn the_cursor_readout_is_in_the_game_s_pixels_y_counting_down() {
    let pixels = area::pixels::PixelSettings {
        pixels_per_unit: 16,
        ..Default::default()
    };
    assert_eq!(
        cursor_readout(glam::Vec3::new(12.0, -4.0, 0.0), pixels, Some([12, 4])),
        "x 192 · y 64 px · cell 12, 4"
    );
    assert_eq!(
        cursor_readout(glam::Vec3::new(0.5, 0.25, 0.0), pixels, None),
        "x 8 · y -4 px"
    );
    // A nonsense density counts as one pixel a metre.
    let none = area::pixels::PixelSettings {
        pixels_per_unit: 0,
        ..Default::default()
    };
    assert_eq!(
        cursor_readout(glam::Vec3::new(3.0, 0.0, 0.0), none, None),
        "x 3 · y 0 px"
    );
}

#[test]
fn every_tool_has_its_own_key() {
    let mut keys: Vec<_> = Tool::ALL.iter().map(|t| t.key()).collect();
    keys.sort_by_key(|k| k.name());
    keys.dedup();
    assert_eq!(keys.len(), Tool::ALL.len());
    assert_eq!(Tool::Move.hint(), "Move (W)");
    assert_eq!(Tool::Transform.key(), egui::Key::T);
}

#[test]
fn the_view_carries_the_size_and_backdrop() {
    let view = Viewport::default().view([800, 600], 1.0);
    assert_eq!(view.size_px, [800, 600]);
    assert_eq!(view.sky_top, rgb(theme::backdrop::sky_top()));
    assert!(view.view_projection.is_finite());
}

#[test]
fn every_overlay_shows_by_default() {
    let o = Overlays::default();
    assert!(o.tools && o.snapping && o.stats);
}

#[test]
fn old_saved_overlays_fill_in_new_switches() {
    let o: Overlays = ron::from_str("(stats: false)").unwrap();
    assert!(!o.stats);
    assert!(o.tools && o.snapping);
}

#[test]
fn no_renderer_explains_itself() {
    let mut r = NoRenderer {
        reason: "no GPU".into(),
    };
    assert_eq!(
        r.render(&Viewport::default().view([1, 1], 1.0)),
        Err("no GPU".to_string())
    );
}
