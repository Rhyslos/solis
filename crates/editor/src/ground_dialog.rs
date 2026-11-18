//! The New ground dialog's fields (Tools 1b, G1): its size, its height
//! and its starting shape. Sculpting comes in G2.

use crate::workbench_ground::{HEIGHTS, NewGroundForm, SIZES};
use egui::Ui;

/// Draws the form's fields.
pub(crate) fn form(ui: &mut Ui, form: &mut NewGroundForm) {
    ui.label(
        "A ground made of voxels, one-metre cubes, so it can later have caves and overhangs. \
         A level has one.",
    );
    egui::Grid::new("new ground fields")
        .num_columns(2)
        .spacing([16.0, 8.0])
        .show(ui, |ui| {
            ui.label("Size");
            ui.horizontal(|ui| {
                for side in SIZES {
                    let words = format!("{0} × {0} km", side / 1024);
                    ui.radio_value(&mut form.side, side, words);
                }
            });
            ui.end_row();
            ui.label("Height");
            ui.horizontal(|ui| {
                for height in HEIGHTS {
                    ui.radio_value(&mut form.height, height, format!("{height} m"));
                }
            });
            ui.end_row();
            ui.label("Shape");
            ui.horizontal(|ui| {
                ui.radio_value(&mut form.hills, true, "Gentle hills");
                ui.radio_value(&mut form.hills, false, "Flat");
            });
            ui.end_row();
            ui.label("Surface at");
            ui.add(
                egui::DragValue::new(&mut form.surface)
                    .range(1.0..=form.height as f32 - 1.0)
                    .speed(0.5)
                    .suffix(" m"),
            );
            ui.end_row();
        });
    let side = form.side as f32 / 1024.0;
    ui.label(
        egui::RichText::new(format!(
            "{side} × {side} km, {} m high. Sculpting comes next.",
            form.height
        ))
        .color(crate::tokens::text_meta()),
    );
}
