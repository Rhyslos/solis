//! The 2D view's bottom-right pills (Phase 9, D1; board 08): from the
//! right, the zoom, the tile grid, what moves snap to and the X and Y
//! locks.
//!
//! In a narrow view they fit as the tool bar does (`viewport_tool_bar.rs`):
//! first their words shorten to the board's ("Tiles ▾", "200% ▾",
//! "16 px"), then they wrap onto rows going up the view, so none ever
//! runs off its left edge (`fit`).

use super::Viewport;
use super::pills::{drop_down, locks_pill, locks_width, mono_pill, sentence_case};
use crate::tokens::{size, text};
use crate::transforming::PixelSnap;
use crate::widgets::{self, menu, pill::pill};
use egui::{Align, Layout, Rect, Ui, UiBuilder};

/// The pills, right to left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pill {
    Zoom,
    Grid,
    Snap,
    Locks,
}

const PILLS: [Pill; 4] = [Pill::Zoom, Pill::Grid, Pill::Snap, Pill::Locks];

/// How the pills fit a view: with their full words or the board's short
/// ones, and how many of them (from the right) go on each row, the
/// bottom row first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Fitted {
    /// The board's short words.
    pub(crate) short: bool,
    /// How many pills each row holds, the bottom row first.
    pub(crate) rows: Vec<usize>,
}

/// How pills of the widths `full` (or, where those don't fit on one row,
/// `short`), laid from the right with `gap` between them, fit `room`
/// points across: on one row with the full words if they fit, else with
/// the short ones, else the short ones on as many rows as they need, each
/// row holding as many as fit (and at least one).
pub(crate) fn fit(full: &[f32], short: &[f32], room: f32, gap: f32) -> Fitted {
    let across =
        |widths: &[f32]| widths.iter().sum::<f32>() + gap * widths.len().saturating_sub(1) as f32;
    if across(full) <= room {
        return Fitted {
            short: false,
            rows: vec![full.len()],
        };
    }
    let mut rows = Vec::new();
    let mut row = 0;
    let mut used = 0.0;
    for &width in short {
        let wanted = if row == 0 { width } else { used + gap + width };
        if row > 0 && wanted > room {
            rows.push(row);
            row = 1;
            used = width;
        } else {
            row += 1;
            used = wanted;
        }
    }
    if row > 0 {
        rows.push(row);
    }
    Fitted { short: true, rows }
}

impl Viewport {
    /// The 2D view's bottom-right pills in view `rect`, fitted to its
    /// width. Returns where they are.
    pub(super) fn pixel_pills(&mut self, ui: &mut Ui, rect: Rect) -> Rect {
        let inner = rect.shrink(size::PILL_MARGIN);
        let width =
            |words: &str| widgets::width_of(ui, words, text::ribbon_small()) + 2.0 * size::PILL_PAD;
        let measure = |short: bool| -> Vec<f32> {
            PILLS
                .iter()
                .map(|&p| match p {
                    Pill::Locks => locks_width(ui, 2),
                    Pill::Grid => {
                        let words = self.words(p, short);
                        widgets::width_of(ui, &words, text::mono_small()) + 2.0 * size::PILL_PAD
                    }
                    _ => width(&self.words(p, short)),
                })
                .collect()
        };
        let fitted = fit(
            &measure(false),
            &measure(true),
            inner.width(),
            size::PILL_GAP,
        );
        let mut covered = Rect::NOTHING;
        let mut next = 0;
        for (n, &count) in fitted.rows.iter().enumerate() {
            let lift = n as f32 * (size::PILL_H + size::PILL_GAP);
            let mut row = ui.new_child(
                UiBuilder::new()
                    .id_salt(("viewport pills", "pixel pills", n))
                    .max_rect(inner.with_max_y(inner.bottom() - lift))
                    .layout(Layout::right_to_left(Align::Max)),
            );
            row.spacing_mut().item_spacing.x = size::PILL_GAP;
            for &p in &PILLS[next..(next + count).min(PILLS.len())] {
                self.pixel_pill(&mut row, p, fitted.short);
            }
            next += count;
            covered = covered.union(row.min_rect());
        }
        covered
    }

    /// What pill `p` says, in full or in the board's short words.
    fn words(&self, p: Pill, short: bool) -> String {
        let zoom = format!("{:.0}%", self.camera.zoom * 100.0);
        let [w, h] = self.pixels.tile;
        let tile = if w == h {
            format!("{w} px")
        } else {
            format!("{w} × {h} px")
        };
        let snap = self.snapping.pixel_snap;
        match (p, short) {
            (Pill::Zoom, false) => format!("Zoom {zoom} ▾"),
            (Pill::Zoom, true) => format!("{zoom} ▾"),
            (Pill::Grid, false) => format!("Grid {tile}"),
            (Pill::Grid, true) => tile,
            (Pill::Snap, false) => format!("Snap: {} ▾", sentence_case(snap.name())),
            (Pill::Snap, true) => match snap {
                PixelSnap::Off => "No snap ▾".to_owned(),
                PixelSnap::Pixels => "Pixels ▾".to_owned(),
                PixelSnap::Tiles => "Tiles ▾".to_owned(),
            },
            (Pill::Locks, _) => String::new(),
        }
    }

    /// Draws pill `p` in `row`.
    fn pixel_pill(&mut self, row: &mut Ui, p: Pill, short: bool) {
        let words = self.words(p, short);
        match p {
            Pill::Zoom => {
                let percent = |z: f32| format!("{:.0}%", z * 100.0);
                let response = pill(row, &words).on_hover_text(
                    "Screen pixels to each of the game's: whole steps keep pixels sharp. The \
                     wheel steps through them",
                );
                let zoom = &mut self.camera.zoom;
                drop_down(&response, |ui| {
                    for &step in crate::camera::ZOOM_STEPS.iter().rev() {
                        if menu::choice(ui, &percent(step), *zoom == step).clicked() {
                            *zoom = step;
                            ui.close();
                        }
                    }
                });
            }
            Pill::Grid => {
                mono_pill(row, &words)
                    .on_hover_text("The tile size: File › Project settings changes it");
            }
            Pill::Snap => {
                let response = pill(row, &words).on_hover_text(
                    "What moves land on: whole pixels (so nothing rests between pixels), whole \
                     tiles, or anywhere. Hold Ctrl while dragging to flip it",
                );
                let snapping = &mut self.snapping;
                drop_down(&response, |ui| {
                    for mode in PixelSnap::ALL {
                        let words = sentence_case(mode.name());
                        if menu::choice(ui, &words, snapping.pixel_snap == mode).clicked() {
                            snapping.pixel_snap = mode;
                            ui.close();
                        }
                    }
                });
            }
            Pill::Locks => locks_pill(row, &mut self.snapping, 2),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_enough_keeps_one_row_of_full_words() {
        let fitted = fit(
            &[80.0, 60.0, 120.0, 70.0],
            &[50.0, 40.0, 60.0, 70.0],
            400.0,
            6.0,
        );
        assert_eq!(
            fitted,
            Fitted {
                short: false,
                rows: vec![4]
            }
        );
        // Exactly the room: still one row.
        assert!(!fit(&[100.0, 100.0], &[50.0, 50.0], 206.0, 6.0).short);
    }

    #[test]
    fn narrower_shortens_the_words_then_wraps_rows_upward() {
        let full = [80.0, 60.0, 120.0, 70.0];
        let short = [50.0, 40.0, 60.0, 70.0];
        // The short words fit on one row (238 points).
        assert_eq!(fit(&full, &short, 250.0, 6.0).rows, vec![4]);
        assert!(fit(&full, &short, 250.0, 6.0).short);
        // Two rows: the zoom, grid and snap at the bottom, the locks above.
        assert_eq!(fit(&full, &short, 170.0, 6.0).rows, vec![3, 1]);
        // One a row where even two don't fit.
        assert_eq!(fit(&full, &short, 60.0, 6.0).rows, vec![1, 1, 1, 1]);
    }

    #[test]
    fn no_room_still_places_every_pill_once() {
        for room in [0.0, -5.0, f32::NAN] {
            let rows = fit(&[10.0; 4], &[10.0; 4], room, 6.0).rows;
            assert_eq!(rows.iter().sum::<usize>(), 4, "{room}");
            assert!(rows.iter().all(|&n| n >= 1));
        }
        assert_eq!(fit(&[], &[], 100.0, 6.0).rows, vec![0]);
    }
}
