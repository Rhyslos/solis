//! Field hints and pickers in approved pictures (stage 1; the mock-up
//! `docs/design/field-hints/`): a Guard written in Lust in Properties,
//! with its slider, tips, group heading and pickers; and its sound
//! picker open, with its search and the sounds under their folder. Drawn
//! for real at 1440 × 900 and cropped to what they show. (Lists wait for
//! Lust, so there's no picture of one.)

use crate::interface_look::InterfaceLook;
use crate::picture_support::{Picture, drawn, editor_window, matches_approved};
use crate::workbench::Workbench;
use egui_kittest::kittest::Queryable;

const SIZE: [f32; 2] = [1440.0, 900.0];

/// Draws `w`, letting `before` act on the window first, and crops
/// Properties.
fn properties_of(
    w: Workbench,
    before: impl FnOnce(&mut egui_kittest::Harness<'static, Workbench>),
) -> Option<Picture> {
    let mut h = editor_window(w, InterfaceLook::default(), SIZE)?;
    for _ in 0..4 {
        h.run();
    }
    before(&mut h);
    for _ in 0..3 {
        h.run();
    }
    Some(drawn(&mut h).cropped([1120, 100], [320, 700]))
}

#[test]
fn a_guard_s_settings_with_hints_and_pickers() {
    let (w, _) = crate::field_hints_tests::with_a_guard();
    let Some(picture) = properties_of(w, |_| {}) else {
        return;
    };
    matches_approved("hints-properties", &picture);
}

#[test]
fn a_sound_picker_open() {
    let (w, _) = crate::field_hints_tests::with_a_guard();
    let Some(picture) = properties_of(w, |h| {
        h.get_by_role_and_label(egui::accesskit::Role::ComboBox, "Alert sound")
            .click();
    }) else {
        return;
    };
    matches_approved("hints-picker", &picture);
}
