//! A picture to work on (the atlas's crops, cutting a picture into
//! cells): the picture filling its space on a dark ground, the wheel
//! zooming about the pointer and the middle button moving it, with lines
//! and boxes drawn over it in its own pixels. Every place that draws on a
//! picture uses this one, so they zoom, move and look the same.

use crate::theme;
use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

/// The closest and furthest a picture zooms.
pub const ZOOM_RANGE: (f32, f32) = (0.25, 64.0);

/// The picture shown, and where: its pixels to the screen and back.
pub struct Board {
    /// The picture's whole space, clicks and drags on it.
    pub response: Response,
    /// Where the picture's top-left pixel is on the screen.
    origin: Pos2,
    /// Screen points a pixel takes.
    zoom: f32,
    /// The picture's size, in pixels.
    size: [u32; 2],
}

impl Board {
    /// The picture's pixel under the screen point `p` (outside it too).
    pub fn to_pixel(&self, p: Pos2) -> [f32; 2] {
        [
            (p.x - self.origin.x) / self.zoom,
            (p.y - self.origin.y) / self.zoom,
        ]
    }

    /// The screen point of the picture's pixel `[x, y]`.
    pub fn to_screen(&self, x: f32, y: f32) -> Pos2 {
        Pos2::new(self.origin.x + x * self.zoom, self.origin.y + y * self.zoom)
    }

    /// The screen box of `rect` (left, top, width, height) in pixels.
    pub fn screen_box(&self, rect: [u32; 4]) -> Rect {
        let [x, y, w, h] = rect.map(|v| v as f32);
        Rect::from_min_max(self.to_screen(x, y), self.to_screen(x + w, y + h))
    }

    /// Faint lines every `cell` pixels across the whole picture, while
    /// the cells are big enough to see.
    pub fn lines(&self, ui: &Ui, cell: [u32; 2]) {
        let cell = cell.map(|g| g.max(1) as f32);
        if cell[0] * self.zoom < 6.0 || cell[1] * self.zoom < 6.0 {
            return;
        }
        let painter = ui.painter_at(self.response.rect);
        let line = Stroke::new(1.0, Color32::from_white_alpha(28));
        let [w, h] = self.size.map(|v| v as f32);
        let mut x = cell[0];
        while x < w {
            painter.line_segment([self.to_screen(x, 0.0), self.to_screen(x, h)], line);
            x += cell[0];
        }
        let mut y = cell[1];
        while y < h {
            painter.line_segment([self.to_screen(0.0, y), self.to_screen(w, y)], line);
            y += cell[1];
        }
    }

    /// `rect` outlined in `colour`, `width` thick, with `label` in its
    /// top-left corner when there's room; nothing when it's out of view.
    pub fn boxed(&self, ui: &Ui, rect: [u32; 4], label: &str, colour: Color32, width: f32) {
        let painter = ui.painter_at(self.response.rect);
        let r = self.screen_box(rect);
        if !painter.clip_rect().intersects(r) {
            return;
        }
        painter.rect_stroke(r, 0.0, Stroke::new(width, colour), egui::StrokeKind::Inside);
        if !label.is_empty() && r.width() > 24.0 {
            painter.text(
                r.left_top() + Vec2::new(3.0, 2.0),
                egui::Align2::LEFT_TOP,
                label,
                egui::FontId::proportional(11.0),
                colour,
            );
        }
    }
}

/// `texture`, a picture `size` pixels big, filling `ui`'s space at
/// `zoom` and moved by `pan` from the middle, both changed by the wheel
/// and the middle button.
pub fn picture_board(
    ui: &mut Ui,
    zoom: &mut f32,
    pan: &mut Vec2,
    texture: &egui::TextureHandle,
    size: [u32; 2],
) -> Board {
    let whole = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    picture_board_in(ui, (zoom, pan), texture, size, whole)
}

/// [`picture_board`] for a picture that is only the part `uv` of its
/// texture (the tileset's, made a whole number of tiles bigger).
pub fn picture_board_in(
    ui: &mut Ui,
    (zoom, pan): (&mut f32, &mut Vec2),
    texture: &egui::TextureHandle,
    size: [u32; 2],
    uv: Rect,
) -> Board {
    let rect = ui.max_rect();
    let response = ui.allocate_rect(rect, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme::backdrop::horizon());
    if response.hovered() {
        let wheel = ui.input(|i| i.smooth_scroll_delta.y);
        if wheel != 0.0 {
            let before = *zoom;
            *zoom = (*zoom * (1.0 + wheel * 0.002)).clamp(ZOOM_RANGE.0, ZOOM_RANGE.1);
            if let Some(at) = response.hover_pos() {
                let from = at - rect.center() - *pan;
                *pan -= from * (*zoom / before - 1.0);
            }
        }
    }
    if response.dragged_by(egui::PointerButton::Middle) {
        *pan += response.drag_delta();
    }
    let shown_size = Vec2::new(size[0] as f32, size[1] as f32) * *zoom;
    let origin = rect.center() + *pan - shown_size / 2.0;
    let shown = Rect::from_min_size(origin, shown_size);
    painter.rect_filled(shown, 0.0, Color32::from_gray(60));
    painter.image(texture.id(), shown, uv, Color32::WHITE);
    Board {
        response,
        origin,
        zoom: *zoom,
        size,
    }
}

/// A zoom that shows a picture `size` big whole in `space`, with a
/// little room round it, in whole steps while it's bigger than 1.
pub fn zoom_to_fit(size: [u32; 2], space: Vec2) -> f32 {
    let fit =
        ((space.x - 32.0) / size[0].max(1) as f32).min((space.y - 32.0) / size[1].max(1) as f32);
    let fit = if fit >= 1.0 { fit.floor() } else { fit };
    fit.clamp(ZOOM_RANGE.0, ZOOM_RANGE.1)
}
