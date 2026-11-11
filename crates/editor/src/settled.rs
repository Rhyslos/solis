//! Number fields and colour buttons whose change counts once, when it's
//! let go (the review, BUG-20). The editors of a file of their own (a
//! tileset, a clips file, an atlas, an animator, a tile map) make every
//! change an undo step and save it at once, as Edit > Input actions… saves
//! the project's actions (BUG-25), so a field
//! that handed over its value on each frame of a drag would fill the undo
//! history with one step a frame and write the file as often (and, for the
//! actions, have the game's code checked again as often). These show
//! the value so far while the field is dragged or typed in, kept between
//! frames, and hand it over once: when the mouse comes up, Enter is
//! pressed or the field is left.

use egui::Ui;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::RangeInclusive;

/// A number field whose change counts once it's let go (or typed and
/// left), so a drag is one undo step. Returns the value then.
pub(crate) fn settled_drag(
    ui: &mut Ui,
    salt: impl Hash + Debug,
    value: f64,
    range: RangeInclusive<f64>,
    speed: f64,
    suffix: &str,
) -> Option<f64> {
    settled_number(ui, salt, value, |d| {
        d.range(range).speed(speed).suffix(suffix)
    })
    .0
}

/// [`settled_drag`] for a whole number (a count of cells): never shown
/// with a decimal point.
pub(crate) fn settled_whole_drag(
    ui: &mut Ui,
    salt: impl Hash + Debug,
    value: f64,
    range: RangeInclusive<f64>,
    speed: f64,
    suffix: &str,
) -> Option<f64> {
    settled_number(ui, salt, value, |d| {
        d.range(range).speed(speed).suffix(suffix).fixed_decimals(0)
    })
    .0
    .map(f64::round)
    .filter(|v| (v - value).abs() > f64::EPSILON)
}

/// [`settled_drag`] for a field that needs more than a range, a speed and
/// a suffix: `field` sets the number field up (how it reads, say), and the
/// field itself is given back with the value, for a hint on it.
pub(crate) fn settled_number(
    ui: &mut Ui,
    salt: impl Hash + Debug,
    value: f64,
    field: impl for<'a> FnOnce(egui::DragValue<'a>) -> egui::DragValue<'a>,
) -> (Option<f64>, egui::Response) {
    let id = ui.id().with(("settled", salt));
    let mut shown: f64 = ui.data(|d| d.get_temp(id)).unwrap_or(value);
    let response = ui.add(field(egui::DragValue::new(&mut shown)));
    let settled = if response.dragged() || response.has_focus() {
        ui.data_mut(|d| d.insert_temp(id, shown));
        None
    } else {
        ui.data_mut(|d| d.remove::<f64>(id));
        (response.drag_stopped() || response.lost_focus() || response.changed())
            .then_some(shown)
            .filter(|v| (v - value).abs() > f64::EPSILON)
    };
    (settled, response)
}

/// A colour button whose change counts once the mouse is let go, so a
/// drag in its picker is one undo step (and one save), as with
/// [`settled_drag`]. Returns the colour then.
pub(crate) fn settled_colour(
    ui: &mut Ui,
    salt: impl Hash + Debug,
    value: [f32; 3],
) -> Option<[f32; 3]> {
    let id = ui.id().with(("settled colour", salt));
    let now = value.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
    let mut shown: [u8; 3] = ui.data(|d| d.get_temp(id)).unwrap_or(now);
    ui.color_edit_button_srgb(&mut shown);
    if ui.input(|i| i.pointer.any_down()) {
        // Still held: shown as dragged so far, and kept for when it's let
        // go.
        ui.data_mut(|d| d.insert_temp(id, shown));
        None
    } else {
        ui.data_mut(|d| d.remove::<[u8; 3]>(id));
        (shown != now).then(|| shown.map(|v| f32::from(v) / 255.0))
    }
}

#[cfg(test)]
#[path = "settled_tests.rs"]
mod tests;
