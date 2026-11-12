//! LIBRARY TOOLS · Picture (decision 0089; the approved additions, round
//! 3, `03-picture-crops.png`): the picture with its crops drawn on it, as
//! the tab's document, and the Crops shelf tab that lists them. Drag on
//! the picture to draw a crop (snapped to the grid), click one to pick
//! it; the list names them. The wheel zooms and the middle button drags
//! the picture, as in the viewport. Crop every tile is on the ribbon.

use crate::atlas_editor::AtlasEditor;
use crate::settled::settled_whole_drag;
use crate::theme;
use egui::{Color32, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};

/// The picture as the tab draws it.
#[derive(Clone, Copy)]
pub(crate) struct AtlasPicture<'a> {
    /// Its texture.
    pub(crate) texture: &'a egui::TextureHandle,
    /// Its size, in pixels.
    pub(crate) size: [u32; 2],
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

/// The picture with its crops, filling `ui`. Delete removes the crop
/// picked while `keys` are the tab's.
pub(crate) fn atlas_tab(
    ui: &mut Ui,
    editor: &mut AtlasEditor,
    picture: Option<&AtlasPicture<'_>>,
    problem: Option<&str>,
    accent: Color32,
    keys: bool,
) -> AtlasOut {
    let mut out = AtlasOut::default();
    if keys && ui.input(|i| i.key_pressed(egui::Key::Delete)) && editor.delete_picked() {
        out.changed = true;
    }
    let full = ui.max_rect();
    let mut canvas = ui.new_child(egui::UiBuilder::new().max_rect(full));
    match (picture, problem) {
        (Some(picture), _) => picture_canvas(&mut canvas, editor, picture, accent, &mut out),
        (None, Some(why)) => {
            crate::panels::empty_note(&mut canvas, why);
        }
        (None, None) => {
            crate::panels::empty_note(&mut canvas, "Reading the picture…");
        }
    }
    ui.advance_cursor_after_rect(full);
    out
}

/// The Crops shelf tab: the grid and every crop as a table (its name,
/// size and place), and the crop picked: its name to change, its size
/// and place, and how game code and a screen name it.
pub(crate) fn crops_tab(
    ui: &mut Ui,
    editor: &mut AtlasEditor,
    picture: Option<&AtlasPicture<'_>>,
) -> AtlasOut {
    use crate::widgets::columns::{Part, Wide, columns};
    let mut out = AtlasOut::default();
    let picked_name = editor
        .picked
        .and_then(|i| editor.atlas.crops.get(i))
        .map(|c| c.name.clone());
    let parts = [
        Part::new("Crops", editor.atlas.crops.len().to_string(), Wide::Rest),
        Part::new(
            picked_name.unwrap_or_else(|| "No crop picked".to_owned()),
            "Crop",
            Wide::Fixed(PICKED_W),
        ),
    ];
    columns(
        ui,
        &parts,
        |_, _| {},
        |ui, index| match index {
            0 => crop_list(ui, editor, &mut out),
            _ => picked_crop(ui, editor, picture, &mut out),
        },
    );
    out
}

/// A crop's row in the list.
const CROP_ROW_H: f32 = 26.0;

/// How wide the picked crop's column is.
const PICKED_W: f32 = 520.0;

/// The grid and the crops, one row each.
fn crop_list(ui: &mut Ui, editor: &mut AtlasEditor, out: &mut AtlasOut) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Grid").color(theme::text_weak()));
        let [w, h] = editor.atlas.grid;
        let a = settled_whole_drag(ui, "grid w", f64::from(w), 1.0..=4096.0, 1.0, " px");
        ui.label("×");
        let b = settled_whole_drag(ui, "grid h", f64::from(h), 1.0..=4096.0, 1.0, " px");
        if a.is_some() || b.is_some() {
            let grid = [a.map_or(w, |v| v as u32), b.map_or(h, |v| v as u32)];
            out.changed |= editor.set_grid(grid);
        }
    });
    if editor.atlas.crops.is_empty() {
        ui.label(
            RichText::new(
                "Drag on the picture to crop out a part of it, or Crop › Every tile on the ribbon.",
            )
            .color(theme::text_weak()),
        );
    }
    // A table: the crop's name, then its size and place in mono, as
    // round 3 draws it.
    let mut picked = editor.picked;
    for (i, crop) in editor.atlas.crops.iter().enumerate() {
        let [x, y, w, h] = crop.rect;
        let (rect, row) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), CROP_ROW_H), Sense::click());
        let chosen = picked == Some(i);
        row.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, chosen, &crop.name)
        });
        let painter = ui.painter();
        if chosen {
            painter.rect_filled(rect, 0.0, crate::interface_look::selected());
        } else if row.hovered() {
            painter.rect_filled(rect, 0.0, crate::tokens::hover());
        }
        let middle = rect.center().y;
        let text = |at: f32, words: String, font| {
            painter.text(
                Pos2::new(at, middle),
                egui::Align2::LEFT_CENTER,
                words,
                font,
                crate::tokens::text_body(),
            );
        };
        text(
            rect.left() + 4.0,
            crop.name.clone(),
            crate::tokens::text::row(),
        );
        let mono = crate::tokens::text::mono_small;
        text(rect.right() - 180.0, format!("{w} \u{d7} {h}"), mono());
        text(rect.right() - 90.0, format!("{x}, {y}"), mono());
        if row.clicked() {
            picked = Some(i);
        }
    }
    if picked != editor.picked {
        editor.pick(picked);
    }
}

/// The crop picked: its name, size and place, and how it is named.
fn picked_crop(
    ui: &mut Ui,
    editor: &mut AtlasEditor,
    picture: Option<&AtlasPicture<'_>>,
    out: &mut AtlasOut,
) {
    let Some(crop) = editor
        .picked
        .and_then(|i| editor.atlas.crops.get(i))
        .cloned()
    else {
        ui.label(
            RichText::new("Click a crop on the picture or in the list.").color(theme::text_weak()),
        );
        return;
    };
    let [x, y, w, h] = crop.rect;
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
    crate::widgets::rows::key_value(ui, "Size", &format!("{w} × {h} px"));
    crate::widgets::rows::key_value(ui, "At", &format!("{x}, {y}"));
    if let Some(p) = picture {
        let side = 96.0_f32;
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
        let size = Vec2::new(p.size[0] as f32, p.size[1] as f32);
        let uv = Rect::from_min_max(
            Pos2::new(x as f32 / size.x, y as f32 / size.y),
            Pos2::new((x + w) as f32 / size.x, (y + h) as f32 / size.y),
        );
        let scale = (side / w.max(h).max(1) as f32).min(side);
        let shown = Rect::from_center_size(rect.center(), Vec2::new(w as f32, h as f32) * scale);
        ui.painter()
            .image(p.texture.id(), shown, uv, Color32::WHITE);
    }
    ui.label(
        RichText::new(
            "A name starts with a letter and holds only letters, digits and _. The picture itself \
             is never changed.",
        )
        .small()
        .color(theme::text_weak()),
    );
    if ui.button("Delete crop").clicked() && editor.delete_picked() {
        out.changed = true;
    }
}

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
