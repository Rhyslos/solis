//! The Atlas Crop tab (decision 0089): a picture with its crops drawn on
//! it. Drag on the picture to draw a crop (snapped to the grid), click one
//! to pick it; the list beside names them. The wheel zooms and the middle
//! button drags the picture, as in the viewport.

use crate::atlas_editor::AtlasEditor;
use crate::settled::settled_whole_drag;
use crate::theme;
use egui::{Color32, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};

/// The picture as the tab draws it.
pub(crate) struct AtlasPicture<'a> {
    /// Its texture.
    pub(crate) texture: &'a egui::TextureHandle,
    /// Its size, in pixels.
    pub(crate) size: [u32; 2],
    /// Its pixels (straight RGBA, rows from the top), to tell an empty
    /// tile from a drawn one.
    pub(crate) rgba: &'a [u8],
}

/// What the tab asks of the workbench.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct AtlasOut {
    /// The crops changed: save them.
    pub(crate) changed: bool,
    /// Something to say in the Console.
    pub(crate) said: Option<String>,
}

/// Whether any pixel of `rect` in a picture `size` big is not see-through.
pub(crate) fn filled(rgba: &[u8], size: [u32; 2], rect: [u32; 4]) -> bool {
    let [x, y, w, h] = rect;
    (y..(y + h).min(size[1])).any(|row| {
        (x..(x + w).min(size[0])).any(|column| {
            let at = ((row as usize * size[0] as usize) + column as usize) * 4 + 3;
            rgba.get(at).is_some_and(|&a| a > 0)
        })
    })
}

/// Draws the tab: the list on the left, the picture on the right. Delete
/// takes the picked crop out while the keys are the tab's (`keys`: the
/// pointer rests over it and nothing is being typed).
pub(crate) fn atlas_tab(
    ui: &mut Ui,
    editor: &mut AtlasEditor,
    picture: Option<AtlasPicture<'_>>,
    problem: Option<&str>,
    accent: Color32,
    keys: bool,
) -> AtlasOut {
    let mut out = AtlasOut::default();
    if keys && ui.input(|i| i.key_pressed(egui::Key::Delete)) && editor.delete_picked() {
        out.changed = true;
    }
    let full = ui.max_rect();
    let list_width = 230.0_f32.min(full.width() * 0.4);
    let list = Rect::from_min_size(full.min, Vec2::new(list_width, full.height()));
    let canvas = Rect::from_min_max(Pos2::new(list.max.x + 1.0, full.min.y), full.max);
    let mut left = ui.new_child(egui::UiBuilder::new().max_rect(list.shrink(8.0)));
    crop_list(&mut left, editor, picture.as_ref(), &mut out);
    ui.painter()
        .vline(list.max.x, full.y_range(), Stroke::new(1.0, theme::line()));
    let mut right = ui.new_child(egui::UiBuilder::new().max_rect(canvas));
    match (&picture, problem) {
        (Some(picture), _) => picture_canvas(&mut right, editor, picture, accent, &mut out),
        (None, Some(why)) => {
            crate::panels::empty_note(&mut right, why);
        }
        (None, None) => {
            crate::panels::empty_note(&mut right, "Reading the picture…");
        }
    }
    ui.advance_cursor_after_rect(full);
    out
}

/// The grid, Crop every tile, and the crops by name.
fn crop_list(
    ui: &mut Ui,
    editor: &mut AtlasEditor,
    picture: Option<&AtlasPicture<'_>>,
    out: &mut AtlasOut,
) {
    ui.label(RichText::new(&editor.atlas.picture).strong());
    ui.horizontal(|ui| {
        ui.label("Grid");
        // Each counts once it's let go: a drag is one undo step and one
        // save.
        let [w, h] = editor.atlas.grid;
        let a = settled_whole_drag(ui, "grid w", f64::from(w), 1.0..=4096.0, 1.0, " px");
        ui.label("×");
        let b = settled_whole_drag(ui, "grid h", f64::from(h), 1.0..=4096.0, 1.0, " px");
        if a.is_some() || b.is_some() {
            let grid = [a.map_or(w, |v| v as u32), b.map_or(h, |v| v as u32)];
            out.changed |= editor.set_grid(grid);
        }
    });
    if let Some(picture) = picture
        && ui
            .button("Crop every tile")
            .on_hover_text(
                "A crop for every grid cell with something drawn in it, named by its column and \
                 row.",
            )
            .clicked()
    {
        let (rgba, size) = (picture.rgba, picture.size);
        let added = editor.crop_every_tile(size, &|r| filled(rgba, size, r));
        out.changed |= added > 0;
        out.said = Some(format!("Atlas Crop: added {added} crops."));
    }
    ui.separator();
    if editor.atlas.crops.is_empty() {
        ui.label(
            RichText::new("Drag on the picture to crop out a part of it.")
                .color(theme::text_weak()),
        );
    }
    let mut picked = editor.picked;
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .max_height((ui.available_height() - 110.0).max(60.0))
        .show(ui, |ui| {
            for (i, crop) in editor.atlas.crops.iter().enumerate() {
                let [x, y, w, h] = crop.rect;
                let row = ui.selectable_label(
                    picked == Some(i),
                    format!("{}   {w}×{h} at {x}, {y}", crop.name),
                );
                if row.clicked() {
                    picked = Some(i);
                }
            }
        });
    if picked != editor.picked {
        editor.pick(picked);
    }
    if editor.picked.is_some() {
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Name");
            let field = ui.text_edit_singleline(&mut editor.naming);
            if field.lost_focus() || ui.button("Rename").clicked() {
                match editor.rename() {
                    Ok(changed) => out.changed |= changed,
                    Err(why) => out.said = Some(why),
                }
            }
        });
        if ui.button("Delete crop").clicked() && editor.delete_picked() {
            out.changed = true;
        }
    }
}

/// The picture, its grid and its crops; drawing and picking.
fn picture_canvas(
    ui: &mut Ui,
    editor: &mut AtlasEditor,
    picture: &AtlasPicture<'_>,
    accent: Color32,
    out: &mut AtlasOut,
) {
    let rect = ui.max_rect();
    let response = ui.allocate_rect(rect, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme::backdrop::horizon());
    // Wheel zooms about the pointer; middle-drag moves.
    if response.hovered() {
        let wheel = ui.input(|i| i.smooth_scroll_delta.y);
        if wheel != 0.0 {
            let before = editor.zoom;
            editor.zoom = (editor.zoom * (1.0 + wheel * 0.002)).clamp(0.25, 64.0);
            if let Some(at) = response.hover_pos() {
                let from = at - rect.center() - editor.pan;
                editor.pan -= from * (editor.zoom / before - 1.0);
            }
        }
    }
    if response.dragged_by(egui::PointerButton::Middle) {
        editor.pan += response.drag_delta();
    }
    let size = Vec2::new(picture.size[0] as f32, picture.size[1] as f32) * editor.zoom;
    let origin = rect.center() + editor.pan - size / 2.0;
    let shown = Rect::from_min_size(origin, size);
    let to_pixel = |p: Pos2| {
        [
            (p.x - origin.x) / editor.zoom,
            (p.y - origin.y) / editor.zoom,
        ]
    };
    let to_screen =
        |x: f32, y: f32| Pos2::new(origin.x + x * editor.zoom, origin.y + y * editor.zoom);
    painter.rect_filled(shown, 0.0, Color32::from_gray(60));
    painter.image(
        picture.texture.id(),
        shown,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
    // The grid, while its cells are big enough to see.
    let grid = editor.atlas.grid.map(|g| g.max(1) as f32);
    if grid[0] * editor.zoom >= 6.0 && grid[1] * editor.zoom >= 6.0 {
        let line = Stroke::new(1.0, Color32::from_white_alpha(28));
        let mut x = grid[0];
        while x < picture.size[0] as f32 {
            painter.line_segment(
                [to_screen(x, 0.0), to_screen(x, picture.size[1] as f32)],
                line,
            );
            x += grid[0];
        }
        let mut y = grid[1];
        while y < picture.size[1] as f32 {
            painter.line_segment(
                [to_screen(0.0, y), to_screen(picture.size[0] as f32, y)],
                line,
            );
            y += grid[1];
        }
    }
    // The crops, the picked one in the accent.
    for (i, crop) in editor.atlas.crops.iter().enumerate() {
        let [x, y, w, h] = crop.rect.map(|v| v as f32);
        let r = Rect::from_min_max(to_screen(x, y), to_screen(x + w, y + h));
        let picked = editor.picked == Some(i);
        let colour = if picked {
            accent
        } else {
            Color32::from_rgb(255, 216, 77)
        };
        painter.rect_stroke(
            r,
            0.0,
            Stroke::new(if picked { 2.0 } else { 1.0 }, colour),
            egui::StrokeKind::Inside,
        );
        if r.width() > 24.0 {
            painter.text(
                r.left_top() + Vec2::new(3.0, 2.0),
                egui::Align2::LEFT_TOP,
                &crop.name,
                egui::FontId::proportional(11.0),
                colour,
            );
        }
    }
    // Drawing a box: from where the drag started to the pointer.
    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(at) = response.interact_pointer_pos()
    {
        editor.drawing = Some(to_pixel(at));
    }
    if let (Some(start), Some(now)) = (editor.drawing, response.interact_pointer_pos())
        && let Some([x, y, w, h]) = editor.snapped(start, to_pixel(now), picture.size)
    {
        let [x, y, w, h] = [x, y, w, h].map(|v| v as f32);
        painter.rect_stroke(
            Rect::from_min_max(to_screen(x, y), to_screen(x + w, y + h)),
            0.0,
            Stroke::new(2.0, accent),
            egui::StrokeKind::Inside,
        );
    }
    if response.drag_stopped_by(egui::PointerButton::Primary) {
        if let (Some(start), Some(end)) = (editor.drawing.take(), response.interact_pointer_pos())
            && let Some(rect) = editor.snapped(start, to_pixel(end), picture.size)
        {
            out.changed |= editor.add(rect);
        }
    } else if response.clicked()
        && let Some(at) = response.interact_pointer_pos()
    {
        let crop = editor.crop_at(to_pixel(at));
        editor.pick(crop);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tile_with_any_pixel_drawn_is_filled() {
        // 4 × 2, one pixel drawn at (3, 1).
        let mut rgba = vec![0u8; 4 * 2 * 4];
        rgba[(4 + 3) * 4 + 3] = 255;
        assert!(filled(&rgba, [4, 2], [2, 0, 2, 2]));
        assert!(!filled(&rgba, [4, 2], [0, 0, 2, 2]));
        assert!(!filled(&rgba, [4, 2], [8, 8, 2, 2]), "outside");
        assert!(filled(&rgba, [4, 2], [3, 1, 99, 99]), "cut to the picture");
    }
}
