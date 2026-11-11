//! The design's tile: a square thumbnail over a name and a kind, as the
//! Library and the Place tab show a file.

use super::{border, focus_ring, words};
use crate::interface_look;
use crate::tokens::{self, size, text};
use egui::{
    Align2, Color32, CornerRadius, Painter, Rect, Response, Sense, Stroke, StrokeKind, Ui,
    WidgetInfo, pos2, vec2,
};

/// What a tile says under its thumbnail.
#[derive(Debug, Clone, Copy)]
pub struct TileText<'a> {
    /// The file's name, cut off with "…" if it is too long.
    pub name: &'a str,
    /// Its kind, in words.
    pub kind: &'a str,
    /// Its kind's colour, for the small dot before the word.
    pub kind_colour: Color32,
}

/// How tall a tile `width` wide is: its padding, the square thumbnail,
/// the name and the kind.
pub fn height(width: f32) -> f32 {
    let thumb = width - 2.0 * size::TILE_PAD;
    size::TILE_PAD + thumb + size::GAP + NAME_H + size::GAP + KIND_H + size::TILE_PAD
}

/// The height of the name's line and of the kind's.
const NAME_H: f32 = 14.0;
const KIND_H: f32 = 13.0;

/// A tile `width` wide. `thumbnail` paints inside the square it is
/// given (a picture, or the kind's icon). `surface` is the colour the
/// tile lies on: a chosen tile's fill is the accent mixed into it.
pub fn tile(
    ui: &mut Ui,
    width: f32,
    said: TileText<'_>,
    chosen: bool,
    surface: Color32,
    thumbnail: impl FnOnce(&Painter, Rect),
) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, height(width)), Sense::click());
    response.widget_info(|| {
        WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), chosen, said.name)
    });
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let accent = interface_look::in_use().accent;
    let painter = ui.painter();
    let round = CornerRadius::same(size::radius::CARD);
    if chosen {
        let fill = tokens::mix(accent.colour(), surface, tokens::share::SELECTED);
        painter.rect_filled(rect, round, fill);
    } else if response.hovered() {
        painter.rect_filled(rect, round, tokens::row_hover());
    }
    let inside = rect.shrink(size::TILE_PAD);
    let thumb = Rect::from_min_size(inside.min, vec2(inside.width(), inside.width()));
    painter.rect_filled(
        thumb,
        CornerRadius::same(size::radius::TAB),
        tokens::hover(),
    );
    thumbnail(&painter.with_clip_rect(thumb), thumb);
    if chosen {
        painter.rect_stroke(
            thumb,
            CornerRadius::same(size::radius::TAB),
            Stroke::new(size::TILE_OUTLINE, accent.colour()),
            StrokeKind::Inside,
        );
    } else {
        border(ui, thumb, size::radius::TAB, tokens::field_border());
    }
    let name = Rect::from_min_size(
        pos2(inside.left(), thumb.bottom() + size::GAP),
        vec2(inside.width(), NAME_H),
    );
    let ink = if chosen {
        tokens::text_emphasis()
    } else {
        tokens::text_tile()
    };
    super::cut_off(ui, name, said.name, text::tile_name(), ink);
    let kind = Rect::from_min_size(
        pos2(inside.left(), name.bottom() + size::GAP),
        vec2(inside.width(), KIND_H),
    );
    let dot = Rect::from_min_size(
        pos2(kind.left(), kind.center().y - size::KIND_DOT / 2.0),
        vec2(size::KIND_DOT, size::KIND_DOT),
    );
    painter.rect_filled(
        dot,
        CornerRadius::same(size::radius::GRIP),
        said.kind_colour,
    );
    let room = Rect::from_min_max(pos2(dot.right() + 5.0, kind.top()), kind.right_bottom());
    words(
        ui,
        room,
        Align2::LEFT_CENTER,
        said.kind,
        text::tile_kind(),
        tokens::text_meta(),
    );
    focus_ring(ui, &response, rect, size::radius::CARD);
    response
}
