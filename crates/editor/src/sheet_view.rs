//! A sprite sheet in the Library and Properties (Phase 9, D2): its frames
//! drawn crisply (each of its pixels a whole number of screen pixels when
//! it fits), and what Solis read from the Aseprite file: tags, slices,
//! layers and the palette.

use crate::sprite_sheets::SheetState;
use crate::theme;
use assets::sprite_sheet::SpriteSheet;
use egui::{Color32, Rect, RichText, Ui};

/// Draws frame `frame` of `sheet` (whose atlas is `atlas`) fitted into
/// `rect`, as big as fits in whole steps when it can, centred.
pub(crate) fn draw_frame(
    painter: &egui::Painter,
    rect: Rect,
    sheet: &SpriteSheet,
    atlas: &egui::TextureHandle,
    frame: usize,
) {
    let Some(f) = sheet.frames.get(frame) else {
        return;
    };
    let [cw, ch] = sheet.size.map(|v| v as f32);
    let ppp = painter.ctx().pixels_per_point();
    // Screen pixels per sheet pixel: a whole number when it fits.
    let fits = (rect.width() * ppp / cw).min(rect.height() * ppp / ch);
    let scale = if fits >= 1.0 { fits.floor() } else { fits } / ppp;
    let canvas = Rect::from_center_size(rect.center(), egui::vec2(cw * scale, ch * scale));
    // On whole screen pixels, so nothing blurs.
    let canvas = Rect::from_min_size(
        egui::pos2(
            (canvas.min.x * ppp).round() / ppp,
            (canvas.min.y * ppp).round() / ppp,
        ),
        canvas.size(),
    );
    let [ax, ay, w, h] = f.rect.map(|v| v as f32);
    if w == 0.0 {
        return;
    }
    let [aw, ah] = sheet.atlas_size.map(|v| v as f32);
    let picture = Rect::from_min_size(
        canvas.min + egui::vec2(f.offset[0] as f32, f.offset[1] as f32) * scale,
        egui::vec2(w, h) * scale,
    );
    let uv = Rect::from_min_max(
        egui::pos2(ax / aw, ay / ah),
        egui::pos2((ax + w) / aw, (ay + h) / ah),
    );
    painter.image(atlas.id(), picture, uv, Color32::WHITE);
}

/// The frame Properties shows of the sheet at `path` (clicked in its
/// strip), kept by egui for the window's life.
fn shown_frame(ui: &Ui, path: &str) -> egui::Id {
    ui.id().with(("sheet frame", path))
}

/// Properties for the Aseprite file at `path`.
pub(crate) fn sheet_properties(
    ui: &mut Ui,
    path: &str,
    state: Option<&SheetState>,
    atlas: Option<&egui::TextureHandle>,
) {
    let name = path.rsplit_once('/').map_or(path, |(_, n)| n);
    ui.horizontal(|ui| {
        ui.label(RichText::new("▦").color(theme::kind::SPRITE));
        ui.label(RichText::new(name).strong());
    });
    let sheet = match state {
        None | Some(SheetState::Reading) => {
            ui.label(RichText::new("Reading…").color(theme::TEXT_WEAK));
            return;
        }
        Some(SheetState::Failed(why)) => {
            ui.label(RichText::new(format!("It couldn't be read: {why}.")).color(theme::ERROR));
            return;
        }
        Some(SheetState::Read(sheet)) => sheet,
    };
    let id = shown_frame(ui, path);
    let mut frame: usize = ui.data(|d| d.get_temp(id)).unwrap_or(0);
    if frame >= sheet.frames.len() {
        frame = 0;
    }
    // The picture, big.
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 120.0),
        egui::Sense::hover(),
    );
    ui.painter().rect_filled(rect, 3.0, theme::FIELD);
    if let Some(atlas) = atlas {
        draw_frame(ui.painter(), rect.shrink(4.0), sheet, atlas, frame);
    }
    // The strip of frames: click one to see it.
    egui::ScrollArea::horizontal()
        .id_salt(("strip", path))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                for i in 0..sheet.frames.len() {
                    let (cell, response) =
                        ui.allocate_exact_size(egui::vec2(24.0, 26.0), egui::Sense::click());
                    let painter = ui.painter();
                    painter.rect_filled(cell, 2.0, theme::FIELD);
                    if let Some(atlas) = atlas {
                        draw_frame(painter, cell.shrink(2.0), sheet, atlas, i);
                    }
                    let stroke = if i == frame {
                        egui::Stroke::new(1.5, theme::kind::SPRITE)
                    } else {
                        egui::Stroke::new(1.0, theme::BUTTON_BORDER)
                    };
                    painter.rect_stroke(cell, 2.0, stroke, egui::StrokeKind::Inside);
                    response.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::Button,
                            true,
                            i == frame,
                            format!("Frame {}", i + 1),
                        )
                    });
                    if response.clicked() {
                        frame = i;
                    }
                }
            });
        });
    ui.data_mut(|d| d.insert_temp(id, frame));
    let f = &sheet.frames[frame];
    let tag = sheet
        .tags
        .iter()
        .find(|t| (t.from as usize..=t.to as usize).contains(&frame))
        .map_or(String::new(), |t| format!(" · tag {}", t.name));
    let events = if f.events.is_empty() {
        String::new()
    } else {
        format!(" · events: {}", f.events.join(", "))
    };
    ui.label(
        RichText::new(format!(
            "Frame {} of {} · {} ms{tag}{events}",
            frame + 1,
            sheet.frames.len(),
            f.duration_ms
        ))
        .size(11.0)
        .color(theme::TEXT_WEAK),
    );
    ui.add_space(4.0);
    let row = |ui: &mut Ui, k: &str, v: String| {
        crate::inspector::row(ui, k, "", |ui| ui.label(v));
    };
    row(
        ui,
        "Size",
        format!("{} × {} pixels", sheet.size[0], sheet.size[1]),
    );
    row(ui, "Frames", sheet.frames.len().to_string());
    let layers: Vec<&str> = sheet
        .layers
        .iter()
        .filter(|l| !l.group)
        .map(|l| l.name.as_str())
        .collect();
    if !layers.is_empty() {
        row(ui, "Layers", layers.join(", "));
    }
    ui.add_space(6.0);
    if sheet.tags.is_empty() {
        ui.label(
            RichText::new("No tags: in Aseprite, tag runs of frames to make clips.")
                .size(11.0)
                .color(theme::TEXT_WEAK),
        );
    } else {
        ui.label(
            RichText::new("Tags: the clips it can play")
                .size(11.0)
                .color(theme::TEXT_WEAK),
        );
        egui::Grid::new(("tags", path))
            .num_columns(3)
            .spacing([10.0, 2.0])
            .show(ui, |ui| {
                for tag in &sheet.tags {
                    ui.label(&tag.name);
                    ui.label(
                        RichText::new(crate::library::frames_text(
                            (tag.to - tag.from + 1) as usize,
                        ))
                        .color(theme::TEXT_WEAK),
                    );
                    let times = match tag.repeats {
                        0 => "loops".to_owned(),
                        1 => "once".to_owned(),
                        n => format!("×{n}"),
                    };
                    ui.label(
                        RichText::new(format!("{}, {times}", tag.direction.name()))
                            .color(theme::TEXT_WEAK),
                    );
                    ui.end_row();
                }
            });
    }
    ui.add_space(4.0);
    if !sheet.slices.is_empty() {
        let slices: Vec<String> = sheet
            .slices
            .iter()
            .map(|s| {
                let what = match s.keys.first() {
                    Some(k) if k.pivot.is_some() => " (pivot)",
                    Some(k) if k.center.is_some() => " (9-slice)",
                    _ => "",
                };
                format!("{}{what}", s.name)
            })
            .collect();
        ui.label(
            RichText::new(format!("Slices: {}", slices.join(" · ")))
                .size(11.0)
                .color(theme::TEXT_WEAK),
        );
    }
    if !sheet.palette.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!("Palette: {} colours", sheet.palette.len()))
                    .size(11.0)
                    .color(theme::TEXT_WEAK),
            );
            for c in sheet.palette.iter().take(64) {
                let (swatch, _) =
                    ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(swatch, 0.0, Color32::from_rgb(c[0], c[1], c[2]));
            }
        });
    }
    for note in &sheet.notes {
        ui.label(RichText::new(note).size(11.0).color(theme::WARNING));
    }
    ui.add_space(6.0);
    ui.label(
        RichText::new(
            "Saving it in Aseprite updates it here at once. Drag it into the viewport to place it \
             as a sprite.",
        )
        .size(11.0)
        .color(theme::TEXT_WEAK),
    );
}
