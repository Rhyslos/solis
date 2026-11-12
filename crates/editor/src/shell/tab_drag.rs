//! Dragging a document tab (the project lead's answers 25 and 31; round
//! 3, `12-drag-tab-split` and `12-drag-tab-merge`): dropped on the left
//! or right half of the center, the document stands alone in a group
//! there and the rest go beside it; dropped on another group's tab
//! strip, the groups join into one. Esc while dragging puts the tab
//! back. What is dropped is kept in the tab's `Arrangement`.

use super::{RibbonTab, Shell};
use crate::panel::Panel;
use crate::tokens::{self, size};
use egui::{CornerRadius, Id, Rect, Stroke, StrokeKind, Ui};

/// Where a dragged tab would land if let go now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Landing {
    /// Alone in a group on the left half (`false`) or the right.
    Half(bool),
    /// Into the group whose strip this is: the groups join.
    Strip,
}

fn key() -> Id {
    Id::new("dragged document tab")
}

/// A drag began on `panel`'s tab.
pub(crate) fn start(ui: &Ui, panel: Panel) {
    ui.data_mut(|d| d.insert_temp(key(), panel));
}

/// The document whose tab is being dragged, if one is.
pub(crate) fn dragged(ui: &Ui) -> Option<Panel> {
    ui.data(|d| d.get_temp::<Panel>(key()))
}

/// Where a tab let go at `at` lands, over `center` with these groups'
/// `strips` (the group holding the tab's own strip is `own`).
pub(crate) fn landing(
    at: egui::Pos2,
    center: Rect,
    strips: &[Rect],
    own: usize,
) -> Option<Landing> {
    if strips
        .iter()
        .enumerate()
        .any(|(index, strip)| index != own && strip.contains(at))
    {
        return Some(Landing::Strip);
    }
    if !center.contains(at) || strips.iter().any(|strip| strip.contains(at)) {
        return None;
    }
    Some(Landing::Half(at.x >= center.center().x))
}

/// Follows a tab being dragged over the center: shows where it would
/// land, and when it is let go, lands it; Esc puts it back. `groups`
/// are the documents of each group, drawn in `strips`.
pub(crate) fn follow(
    ui: &Ui,
    center: Rect,
    strips: &[Rect],
    groups: &[Vec<Panel>],
    shell: &mut Shell,
    tab: RibbonTab,
) {
    let Some(panel) = dragged(ui) else {
        return;
    };
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        ui.data_mut(|d| d.remove::<Panel>(key()));
        return;
    }
    let own = groups
        .iter()
        .position(|docs| docs.contains(&panel))
        .unwrap_or(0);
    let at = ui.input(|i| i.pointer.latest_pos());
    let landing = at.and_then(|at| landing(at, center, strips, own));
    let accent = crate::interface_look::in_use().accent;
    match landing {
        Some(Landing::Half(right)) => {
            let half = if right {
                center.with_min_x(center.center().x)
            } else {
                center.with_max_x(center.center().x)
            };
            target(ui, half.shrink(size::GAP), accent);
        }
        Some(Landing::Strip) => {
            if let Some(strip) = strips
                .iter()
                .enumerate()
                .find_map(|(i, s)| (i != own && at.is_some_and(|at| s.contains(at))).then_some(*s))
            {
                target(ui, strip, accent);
            }
        }
        None => {}
    }
    if ui.input(|i| i.pointer.any_released()) {
        ui.data_mut(|d| d.remove::<Panel>(key()));
        let all: usize = groups.iter().map(Vec::len).sum();
        let arrangement = shell.arrangement_mut(tab);
        match landing {
            Some(Landing::Half(right)) if all >= 2 => {
                arrangement.alone = Some((panel, right));
                arrangement.merged = false;
                arrangement.front = [None, None];
            }
            Some(Landing::Strip) => {
                arrangement.alone = None;
                arrangement.merged = true;
                arrangement.front = [Some(panel), None];
            }
            _ => {}
        }
    }
}

/// The place a tab would land: the accent, faint, with its outline.
fn target(ui: &Ui, rect: Rect, accent: tokens::Accent) {
    let fill = tokens::mix(accent.colour(), tokens::viewport(), tokens::share::SELECTED);
    let round = CornerRadius::same(size::radius::CARD);
    ui.painter()
        .rect_filled(rect, round, fill.gamma_multiply(0.6));
    ui.painter().rect_stroke(
        rect,
        round,
        Stroke::new(1.0, accent.colors().fill),
        StrokeKind::Inside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};

    #[test]
    fn a_tab_lands_on_a_half_or_on_another_group_s_strip() {
        let center = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 400.0));
        let strips = [
            Rect::from_min_size(pos2(0.0, 0.0), vec2(500.0, 30.0)),
            Rect::from_min_size(pos2(501.0, 0.0), vec2(499.0, 30.0)),
        ];
        assert_eq!(
            landing(pos2(100.0, 200.0), center, &strips, 0),
            Some(Landing::Half(false))
        );
        assert_eq!(
            landing(pos2(900.0, 200.0), center, &strips, 0),
            Some(Landing::Half(true))
        );
        assert_eq!(
            landing(pos2(700.0, 10.0), center, &strips, 0),
            Some(Landing::Strip)
        );
        // Its own strip, or outside the center: nowhere.
        assert_eq!(landing(pos2(100.0, 10.0), center, &strips, 0), None);
        assert_eq!(landing(pos2(100.0, 900.0), center, &strips, 0), None);
    }
}
