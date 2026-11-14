//! The ribbon's controls as they are drawn: the large button (and the
//! one light one), the small row, a row of words alone, and the list a
//! control opens from its small arrow or a right-click.

use super::ShellCommand;
use super::ribbon::{ListItem, Picked, Shape, Steps};
use crate::icons::{self, Icon};
use crate::interface_look;
use crate::tokens::{self, size, text};
use crate::transforming::Snapping;
use crate::unbuilt::{self, Entry, RowKind};
use crate::widgets::{self, checkbox, width_of, words};
use crate::workbench::Workbench;
use egui::{
    Align2, Color32, CornerRadius, Id, Popup, Rect, Response, RichText, Sense, SetOpenCommand,
    Stroke, Ui, WidgetInfo, pos2, vec2,
};

/// How a control is drawn now, from what its command says.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Look {
    /// Whether it can be used now.
    enabled: bool,
    /// Whether it is the thing in force (Pause, while paused).
    lit: bool,
    /// A switch's state.
    on: Option<bool>,
    /// What pointing at it says.
    hover: String,
    /// What a screen reader calls it, where that is more than the word
    /// drawn on it: its command's name ("Add entity" for "Empty").
    name: String,
    /// Its key, drawn small beside its word (Shape B).
    key: Option<&'static str>,
}

impl Look {
    /// The look of the control that does `command`.
    pub(crate) fn of(command: ShellCommand, w: &Workbench, lit: bool) -> Self {
        Self {
            enabled: command.why_not(w).is_none(),
            lit,
            on: command.is_on(w),
            hover: command.hover_text(w),
            // A switch is called by the words beside its box.
            name: match command.is_on(w) {
                Some(_) => String::new(),
                None => command.name(w),
            },
            key: None,
        }
    }

    /// The same look, with `key` drawn small beside the word.
    pub(crate) fn with_key(self, key: Option<&'static str>) -> Self {
        Self { key, ..self }
    }

    /// A control that only opens a list.
    fn list() -> Self {
        Self {
            enabled: true,
            lit: false,
            on: None,
            hover: String::new(),
            name: String::new(),
            key: None,
        }
    }

    /// What a screen reader calls a control drawn with `label`.
    fn called<'a>(&'a self, label: &'a str) -> &'a str {
        if self.name.is_empty() {
            label
        } else {
            &self.name
        }
    }

    /// The colour of its words and icon, and the fill behind them.
    fn colours(&self, response: &Response) -> (Color32, Option<Color32>) {
        if !self.enabled {
            (tokens::text_faint(), None)
        } else if self.lit {
            let accent = interface_look::in_use().accent;
            (tokens::text_emphasis(), Some(tokens::active_button(accent)))
        } else if response.hovered() {
            (tokens::text_body(), Some(tokens::hover()))
        } else {
            (tokens::text_body(), None)
        }
    }
}

/// A large button in `spot`: its icon over its word. `light` is the one
/// light button (Play, Stop). Returns its response and, for a button
/// drawn with a list's arrow, nothing more: the arrow is drawn by
/// [`with_list`].
pub(crate) fn big(
    ui: &mut Ui,
    spot: Rect,
    id: Id,
    icon: Icon,
    label: &str,
    look: &Look,
    light: bool,
) -> (Response, Rect) {
    let response = ui.interact(spot, id, Sense::click());
    response.widget_info(|| {
        WidgetInfo::labeled(egui::WidgetType::Button, look.enabled, look.called(label))
    });
    let radius = CornerRadius::same(size::radius::TAB);
    let (ink, fill) = if light && look.enabled {
        let fill = if response.hovered() {
            tokens::mix(
                tokens::text_emphasis(),
                tokens::PLAY_FILL,
                tokens::share::LIFT,
            )
        } else {
            tokens::PLAY_FILL
        };
        (tokens::ON_ACCENT, Some(fill))
    } else {
        look.colours(&response)
    };
    if let Some(fill) = fill {
        ui.painter().rect_filled(spot, radius, fill);
    }
    let (icon_side, font) = if light {
        (
            size::RIBBON_LIGHT_ICON,
            text::semibold_at(text::RIBBON_LABEL_SIZE),
        )
    } else {
        (size::RIBBON_BIG_ICON, text::ribbon_label())
    };
    // The icon, a gap and the label, together in the middle.
    let label_h = text::RIBBON_LABEL_SIZE + 2.0;
    let stack = icon_side + size::RIBBON_BIG_GAP + label_h;
    let top = spot.center().y - stack / 2.0;
    let at = Rect::from_min_size(
        pos2(spot.center().x - icon_side / 2.0, top),
        vec2(icon_side, icon_side),
    );
    icons::icon(ui.painter(), at, icon, ink);
    let line = Rect::from_min_max(
        pos2(spot.left(), at.bottom() + size::RIBBON_BIG_GAP),
        pos2(spot.right(), at.bottom() + size::RIBBON_BIG_GAP + label_h),
    );
    if let Some(key) = look.key {
        // The word and its key, together in the middle.
        let key_font = text::ribbon_key();
        let word_w = width_of(ui, label, font.clone());
        let key_w = width_of(ui, key, key_font.clone());
        let left = line.center().x - (word_w + size::RIBBON_KEY_GAP + key_w) / 2.0;
        let word = Rect::from_min_max(pos2(left, line.top()), pos2(left + word_w, line.bottom()));
        words(ui, word, Align2::LEFT_CENTER, label, font, ink);
        let after = Rect::from_min_max(
            pos2(word.right() + size::RIBBON_KEY_GAP, line.top()),
            pos2(word.right() + size::RIBBON_KEY_GAP + key_w, line.bottom()),
        );
        words(
            ui,
            after,
            Align2::LEFT_CENTER,
            key,
            key_font,
            tokens::text_meta(),
        );
    } else if width_of(ui, label, font.clone()) <= line.width() {
        words(ui, line, Align2::CENTER_CENTER, label, font, ink);
    } else {
        widgets::cut_off(ui, line.shrink2(vec2(2.0, 0.0)), label, font, ink);
    }
    widgets::focus_ring(ui, &response, spot, size::radius::TAB);
    let response = if look.hover.is_empty() {
        response
    } else {
        response.on_hover_text(&look.hover)
    };
    (response, line)
}

/// A small row in `spot`: a box for a switch, its words, and the small
/// arrow of a list.
pub(crate) fn row(
    ui: &mut Ui,
    spot: Rect,
    id: Id,
    label: &str,
    kind: RowKind,
    look: &Look,
) -> Response {
    let response = ui.interact(spot, id, Sense::click());
    response.widget_info(|| match look.on {
        Some(on) => WidgetInfo::selected(egui::WidgetType::Checkbox, look.enabled, on, label),
        None => WidgetInfo::labeled(egui::WidgetType::Button, look.enabled, look.called(label)),
    });
    let (ink, fill) = look.colours(&response);
    if let Some(fill) = fill {
        ui.painter()
            .rect_filled(spot, CornerRadius::same(size::radius::SMALL), fill);
    }
    let mut left = spot.left() + size::RIBBON_ROW_PAD;
    if kind == RowKind::Check {
        let scale = ui.ctx().pixels_per_point();
        let corner = pos2(
            tokens::on_pixel(left, scale),
            tokens::on_pixel(spot.center().y - size::CHECKBOX / 2.0, scale),
        );
        let square = Rect::from_min_size(corner, vec2(size::CHECKBOX, size::CHECKBOX));
        checkbox::draw_box(ui, square, look.on.unwrap_or(false));
        left += size::CHECKBOX + size::GAP;
    }
    let room = Rect::from_min_max(pos2(left, spot.top()), spot.right_bottom());
    let drawn = words(
        ui,
        room,
        Align2::LEFT_CENTER,
        label,
        text::ribbon_small(),
        ink,
    );
    if kind == RowKind::List {
        let middle = pos2(
            drawn.right() + size::GAP + size::LIST_ARROW / 2.0,
            spot.center().y,
        );
        list_arrow(ui, middle, ink);
    }
    widgets::focus_ring(ui, &response, spot, size::radius::SMALL);
    if look.hover.is_empty() {
        response
    } else {
        response.on_hover_text(&look.hover)
    }
}

/// A switch with a step (Snapping's Grid and Angle): a click on the box
/// or the name switches it; a click on the value after "·", or a
/// right-click anywhere on it, opens the list of steps.
pub(crate) fn switch_with_steps(
    ui: &mut Ui,
    spot: Rect,
    id: Id,
    (label, command): (&str, ShellCommand),
    steps: Steps,
    look: &Look,
    snapping: Snapping,
) -> Option<Picked> {
    let response = row(ui, spot, id, label, RowKind::Check, look);
    // Where the value starts: after the box, the name and the "·".
    let name = label.split_once(" · ").map_or(label, |(name, _)| name);
    let value_from = spot.left()
        + size::RIBBON_ROW_PAD
        + size::CHECKBOX
        + size::GAP
        + width_of(ui, &format!("{name} · "), text::ribbon_small());
    let on_value = ui
        .input(|input| input.pointer.interact_pos())
        .is_some_and(|at| at.x >= value_from);
    let open = (response.clicked() && on_value) || response.secondary_clicked();
    let mut picked = None;
    Popup::menu(&response)
        .open_memory(open.then_some(SetOpenCommand::Toggle))
        .show(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let now = steps.of(&snapping);
            for &step in steps.all() {
                if widgets::menu::choice(ui, &steps.words(step), step == now).clicked() {
                    picked = Some(Picked::Step(steps, step));
                    ui.close();
                }
            }
        });
    if response.clicked() && !on_value {
        picked = Some(Picked::Command(command));
    }
    picked
}

/// Words alone in `spot`: they say something and do nothing.
pub(crate) fn fact(ui: &mut Ui, spot: Rect, id: Id, text_shown: &str) {
    let response = ui.interact(spot, id, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Label, true, text_shown));
    let room = spot.with_min_x(spot.left() + size::RIBBON_ROW_PAD);
    words(
        ui,
        room,
        Align2::LEFT_CENTER,
        text_shown,
        text::ribbon_small(),
        tokens::text_body(),
    );
}

/// The small filled arrow of a control that opens a list.
fn list_arrow(ui: &Ui, middle: egui::Pos2, ink: Color32) {
    let half = size::LIST_ARROW / 2.0;
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            pos2(middle.x - half, middle.y - half / 2.0),
            pos2(middle.x + half, middle.y - half / 2.0),
            pos2(middle.x, middle.y + half * 0.75),
        ],
        ink,
        Stroke::NONE,
    ));
}

/// A control with a list of kinds. A plain click does `command` (the
/// usual kind); its small arrow, or a right-click, opens the list (the
/// project lead's answer, decision 0103: no long press). A control with
/// no command of its own opens the list on a plain click too.
#[allow(clippy::too_many_arguments)]
pub(crate) fn with_list(
    ui: &mut Ui,
    w: &Workbench,
    spot: Rect,
    id: Id,
    shape: Shape,
    label: &str,
    command: Option<ShellCommand>,
    items: &[ListItem],
) -> Option<ShellCommand> {
    let look = command.map_or_else(Look::list, |command| Look::of(command, w, false));
    let (response, arrow_clicked) = match shape {
        Shape::Big(icon) | Shape::Light(icon) => {
            let (response, line) = big(ui, spot, id, icon, label, &look, false);
            // The arrow sits at the right end of the label's line, with
            // room round it for the pointer.
            let middle = pos2(spot.right() - size::LIST_ARROW, line.center().y);
            let reach = Rect::from_center_size(
                middle,
                vec2(size::LIST_ARROW_REACH, size::LIST_ARROW_REACH),
            )
            .intersect(spot);
            let arrow = ui.interact(reach, id.with("list"), Sense::click());
            let name = format!("{label}: more kinds");
            arrow.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
            let ink = if arrow.hovered() {
                tokens::text_emphasis()
            } else {
                tokens::text_label()
            };
            list_arrow(ui, middle, ink);
            // A button that is only a list (the Library's Create) opens
            // it on a plain click too.
            let clicked = arrow.clicked() || (command.is_none() && response.clicked());
            arrow.on_hover_text("Lists the other kinds. A right-click on the button does too.");
            (response, clicked)
        }
        Shape::Row(_) => {
            let response = row(ui, spot, id, label, RowKind::List, &look);
            let clicked = response.clicked();
            (response, clicked)
        }
    };
    let open = if arrow_clicked {
        Some(SetOpenCommand::Toggle)
    } else if response.secondary_clicked() {
        Some(SetOpenCommand::Bool(true))
    } else {
        None
    };
    let mut picked = None;
    Popup::menu(&response).open_memory(open).show(|ui| {
        for item in unbuilt::filter(items.to_vec()) {
            match item {
                Entry::Unbuilt(part) => {
                    unbuilt::ribbon_row(ui, part, RowKind::Plain);
                }
                Entry::Built((words_shown, command)) => match command.why_not(w) {
                    None => {
                        if ui
                            .button(words_shown.as_ref())
                            .on_hover_text(command.hover_text(w))
                            .clicked()
                        {
                            picked = Some(command);
                            ui.close();
                        }
                    }
                    Some(_) => {
                        let faint = RichText::new(words_shown.as_ref()).color(tokens::text_faint());
                        ui.add(egui::Label::new(faint).selectable(false))
                            .on_hover_text(command.hover_text(w));
                    }
                },
            }
        }
    });
    // The arrow's click is the list's alone.
    if picked.is_none() && command.is_some() && response.clicked() && !arrow_clicked {
        picked = command;
    }
    picked
}
