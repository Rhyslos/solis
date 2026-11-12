//! Placing (spec 236-287; the approved additions, round 2,
//! `06-placing-a.png`): the Placement column of Home's Place shelf tab,
//! and what the viewport shows while a file is dragged over it. Bringing
//! the Place tab to the front is the placing mode (the project lead's
//! answer 20): the ribbon takes its build form and the toolbox goes.
//!
//! What lands where: a file dropped on the viewport lands on the surface
//! under the pointer and, with **Align to Grid**, its X and Y on the
//! ribbon's grid step while Grid snapping is on. **Free** never snaps;
//! nor does a drop with Ctrl held (answer 25).

use crate::tokens::{self, size, text};
use crate::transforming::Snapping;
use crate::unbuilt::{self, Entry, UnbuiltId};
use crate::widgets;
use egui::{Align2, CornerRadius, Rect, Stroke, Ui, pos2, vec2};
use glam::Vec3;

/// How what's placed lands: on the grid or free.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Align {
    Grid,
    Free,
}

/// The Placement column: Align to (Surface unfinished, Grid, Free), then
/// the design's rows that aren't built yet.
pub(crate) fn placement(ui: &mut Ui, snapping: &mut Snapping) {
    egui::Frame::new()
        .inner_margin(egui::Margin::same(size::PANEL_PAD as i8))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = size::ROW_GAP * 2.0;
            ui.horizontal_wrapped(|ui| {
                let (name, _) = ui.allocate_exact_size(
                    vec2(size::FIELD_LABEL_W, size::field_h()),
                    egui::Sense::hover(),
                );
                widgets::words(
                    ui,
                    name,
                    Align2::LEFT_CENTER,
                    "Align to",
                    text::row(),
                    tokens::text_label(),
                );
                let mut align = if snapping.place_free {
                    Align::Free
                } else {
                    Align::Grid
                };
                if widgets::segmented::segmented(
                    ui,
                    "Align to",
                    &mut align,
                    &[(Align::Grid, "Grid"), (Align::Free, "Free")],
                ) {
                    snapping.place_free = align == Align::Free;
                }
                // The design's third choice, Surface, isn't built: it
                // follows the two, and wraps under them when narrow.
                for entry in
                    unbuilt::filter(vec![Entry::<()>::Unbuilt(UnbuiltId::PlaceAlignSurface)])
                {
                    if let Entry::Unbuilt(part) = entry {
                        unbuilt::chip(ui, part);
                    }
                }
            });
            let rows = vec![
                Entry::<()>::Unbuilt(UnbuiltId::PlaceRotation),
                Entry::Unbuilt(UnbuiltId::PlaceRandomYaw),
                Entry::Unbuilt(UnbuiltId::PlaceScale),
                Entry::Unbuilt(UnbuiltId::PlacePhysics),
            ];
            for entry in unbuilt::filter(rows) {
                if let Entry::Unbuilt(part) = entry {
                    unbuilt::field_row(ui, part);
                }
            }
        });
}

/// Where a file dropped at `at` lands: on the grid's step in X and Y
/// (Solis is Z up) when the ribbon's Grid is on, Align is Grid and Ctrl
/// isn't held; otherwise just where it was dropped.
pub(crate) fn landing(at: Vec3, snapping: &Snapping, ctrl: bool) -> Vec3 {
    if !snaps(snapping, ctrl) {
        return at;
    }
    let step = snapping.grid_step;
    Vec3::new(
        (at.x / step).round() * step,
        (at.y / step).round() * step,
        at.z,
    )
}

/// Whether a drop now snaps to the grid.
fn snaps(snapping: &Snapping, ctrl: bool) -> bool {
    snapping.grid && !snapping.place_free && !ctrl && snapping.grid_step > 0.0
}

/// What a drop snaps to, in a word, for the label and the status bar.
pub(crate) fn snap_word(snapping: &Snapping, ctrl: bool) -> &'static str {
    if snaps(snapping, ctrl) {
        "grid"
    } else {
        "free"
    }
}

/// Half a footprint's side, in metres, where there's no grid step to
/// draw it at. Solis's own.
pub(crate) const FOOTPRINT_HALF: f32 = 0.5;

/// The footprint under what's dragged (spec 246): the square on the
/// ground around `spot`, `half` a side out each way, as the camera sees
/// it, in the accent at 16% with an accent outline at 80%.
pub(crate) fn footprint(
    ui: &Ui,
    view: Rect,
    seen: &crate::camera::Camera,
    (spot, half): (Vec3, f32),
) {
    let size = glam::Vec2::new(view.width(), view.height());
    let corners: Option<Vec<egui::Pos2>> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
        .into_iter()
        .map(|(x, y)| {
            seen.project(spot + Vec3::new(x * half, y * half, 0.0), size)
                .map(|p| view.min + vec2(p.x, p.y))
        })
        .collect();
    let Some(corners) = corners else {
        return;
    };
    let accent = crate::interface_look::accent_fill();
    let fill = accent.gamma_multiply(tokens::share::SELECTED);
    ui.painter_at(view).add(egui::Shape::convex_polygon(
        corners,
        fill,
        Stroke::new(
            size::LINE,
            accent.gamma_multiply(tokens::share::GHOST_OUTLINE),
        ),
    ));
}

/// The label beside the pointer while a file is dragged over the view
/// (spec 253): its name, and where it would land and what it snaps to.
pub(crate) fn drag_label(ui: &Ui, pointer: egui::Pos2, name: &str, at: Option<Vec3>, snap: &str) {
    let painter = ui.painter();
    let where_words = match at {
        Some(at) => format!("x {:.2}  y {:.2} \u{b7} {snap}", at.x, at.y),
        None => format!("\u{b7} {snap}"),
    };
    let name_galley =
        painter.layout_no_wrap(name.to_owned(), text::card_name(), tokens::text_emphasis());
    let where_galley = painter.layout_no_wrap(where_words, text::mono_small(), tokens::text_meta());
    let width = name_galley.size().x.max(where_galley.size().x) + 2.0 * size::CARD_PAD;
    let height =
        name_galley.size().y + where_galley.size().y + size::ROW_GAP + 2.0 * size::CARD_PAD;
    let corner = pointer + vec2(size::PLACE_LABEL_OFFSET, -height / 2.0);
    let rect = Rect::from_min_size(corner, vec2(width, height));
    painter.rect(
        rect,
        CornerRadius::same(size::radius::MENU),
        tokens::label_box_fill(),
        Stroke::new(size::LINE, tokens::strong_line()),
        egui::StrokeKind::Inside,
    );
    let inside = rect.shrink(size::CARD_PAD);
    let below = inside.top() + name_galley.size().y + size::ROW_GAP;
    painter.galley(inside.min, name_galley, tokens::text_emphasis());
    painter.galley(
        pos2(inside.left(), below),
        where_galley,
        tokens::text_meta(),
    );
}

/// The hint strip at the view's bottom middle while a file is dragged
/// (spec 254): Release places, Ctrl places free, Esc cancels; the keys
/// not built yet are unfinished parts.
pub(crate) fn hint_strip(ui: &mut Ui, view: Rect) {
    let width = size::PLACE_HINT_W.min(view.width() - 2.0 * size::PILL_MARGIN);
    let rect = Rect::from_center_size(
        pos2(
            view.center().x,
            view.bottom() - size::PILL_MARGIN - size::PLACE_HINT_H / 2.0,
        ),
        vec2(width, size::PLACE_HINT_H),
    );
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(size::radius::CARD),
        tokens::label_box_fill(),
    );
    let mut strip = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(vec2(size::PILL_PAD, 0.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    strip.spacing_mut().item_spacing.x = size::GAP;
    for (key, words) in [("Release", "place"), ("Ctrl", "free, no snap")] {
        widgets::keycap::keycap(&mut strip, key, widgets::keycap::KeycapState::Bound);
        strip.label(
            egui::RichText::new(words)
                .font(text::ui())
                .color(tokens::text_body()),
        );
    }
    let later = vec![
        Entry::<()>::Unbuilt(UnbuiltId::PlaceKeepPlacing),
        Entry::Unbuilt(UnbuiltId::PlaceRotateGhost),
    ];
    for entry in unbuilt::filter(later) {
        if let Entry::Unbuilt(part) = entry {
            unbuilt::chip(&mut strip, part);
        }
    }
    widgets::keycap::keycap(&mut strip, "Esc", widgets::keycap::KeycapState::Bound);
    strip.label(
        egui::RichText::new("cancel")
            .font(text::ui())
            .color(tokens::text_body()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapping(grid: bool, free: bool) -> Snapping {
        Snapping {
            grid,
            grid_step: 0.25,
            place_free: free,
            ..Snapping::default()
        }
    }

    #[test]
    fn a_drop_lands_on_the_grid_only_when_it_snaps() {
        let at = Vec3::new(18.3, -6.41, 0.7);
        let snapped = landing(at, &snapping(true, false), false);
        assert_eq!(snapped, Vec3::new(18.25, -6.5, 0.7), "Z is left alone");
        assert_eq!(landing(at, &snapping(true, true), false), at, "Free");
        assert_eq!(landing(at, &snapping(true, false), true), at, "Ctrl");
        assert_eq!(landing(at, &snapping(false, false), false), at, "Grid off");
        assert_eq!(snap_word(&snapping(true, false), false), "grid");
        assert_eq!(snap_word(&snapping(true, false), true), "free");
    }

    #[test]
    fn a_zero_step_never_snaps() {
        let mut odd = snapping(true, false);
        odd.grid_step = 0.0;
        let at = Vec3::new(1.3, 2.7, 0.0);
        assert_eq!(landing(at, &odd, false), at);
    }
}
