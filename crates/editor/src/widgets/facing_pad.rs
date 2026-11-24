//! The facing pad ([decision 0137](../../../../docs/decisions/0137-the-uniformity-pass.md)):
//! eight arrows round a dot, the picked one lit, wherever a facing is
//! chosen: a Sprite's Facing in Properties, the clips page's preview, an
//! animator's direction parameter.

use crate::theme;
use animation::clips_file::Facing;
use egui::{Color32, Pos2, Rect, Response, Sense, Ui, vec2};

/// The step between arrows.
pub const STEP: f32 = 26.0;

/// The arrow drawn for `facing`.
pub fn arrow(facing: Facing) -> &'static str {
    match facing {
        Facing::Any => "•",
        Facing::North => "↑",
        Facing::NorthEast => "↗",
        Facing::East => "→",
        Facing::SouthEast => "↘",
        Facing::South => "↓",
        Facing::SouthWest => "↙",
        Facing::West => "←",
        Facing::NorthWest => "↖",
    }
}

/// Where `facing`'s arrow sits from the dot, in steps (up the screen is
/// North).
fn offset(facing: Facing) -> Option<[f32; 2]> {
    Some(match facing {
        Facing::North => [0.0, -1.0],
        Facing::NorthEast => [1.0, -1.0],
        Facing::East => [1.0, 0.0],
        Facing::SouthEast => [1.0, 1.0],
        Facing::South => [0.0, 1.0],
        Facing::SouthWest => [-1.0, 1.0],
        Facing::West => [-1.0, 0.0],
        Facing::NorthWest => [-1.0, -1.0],
        Facing::Any => return None,
    })
}

/// The pad round `centre`, with an arrow for each of `ways` and
/// `picked` lit. Gives back the one clicked. `salt` tells this pad's
/// arrows from another's.
pub fn pad_at(
    ui: &mut Ui,
    centre: Pos2,
    ways: &[Facing],
    picked: Option<Facing>,
    salt: impl std::hash::Hash + std::fmt::Debug,
) -> Option<Facing> {
    let painter = ui.painter().clone();
    painter.circle_filled(centre, 6.0, theme::button_border());
    let id = ui.id().with(("facing pad", salt));
    let mut clicked = None;
    for &way in ways {
        let Some([dx, dy]) = offset(way) else {
            continue;
        };
        let rect =
            Rect::from_center_size(centre + vec2(dx, dy) * STEP, vec2(STEP - 4.0, STEP - 4.0));
        let response = ui.interact(rect, id.with(way.name()), Sense::click());
        let lit = picked == Some(way);
        if lit {
            painter.rect_filled(rect, 4.0, theme::kind::SPRITE);
        } else if response.hovered() {
            painter.rect_filled(rect, 4.0, theme::field());
        }
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            arrow(way),
            egui::FontId::proportional(13.0),
            if lit { Color32::WHITE } else { theme::text() },
        );
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, lit, way.name())
        });
        if response.clicked() {
            clicked = Some(way);
        }
    }
    clicked
}

/// [`pad_at`] in a square of its own, at the start of the room left, with
/// all eight ways: a field's control.
pub fn pad(
    ui: &mut Ui,
    picked: Option<Facing>,
    salt: impl std::hash::Hash + std::fmt::Debug,
) -> (Response, Option<Facing>) {
    let (rect, response) = ui.allocate_exact_size(vec2(STEP * 3.0, STEP * 3.0), Sense::hover());
    let ways = Facing::of_directions(8);
    let clicked = pad_at(ui, rect.center(), ways, picked, salt);
    (response, clicked)
}
