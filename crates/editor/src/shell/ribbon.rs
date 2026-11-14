//! The ribbon (specification, "Ribbon"): groups of large buttons and
//! small rows, each group over its label, with the Play group at the far
//! right. A tab's ribbon is a table (`ribbons/`); this file lays a table
//! out and draws it.
//!
//! Every width is known before anything is drawn, so the groups are
//! placed in one pass: left to right, the Play group pushed to the right
//! edge while there is room, and the whole band scrolling sideways when
//! there isn't.

use super::ShellCommand;
use super::controls::{self, Look};
use crate::icons::Icon;
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, Entry, RowKind};
use crate::widgets::width_of;
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Align2, Rect, Sense, Ui, UiBuilder, pos2, vec2};
use std::borrow::Cow;

/// The shape a control has on the ribbon, built or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape {
    /// A large button: an icon over a word.
    Big(Icon),
    /// The one light button: Play, and Stop while the game runs.
    Light(Icon),
    /// A small row.
    Row(RowKind),
}

/// One thing in a control's list of kinds: its words there and what it
/// does.
pub(crate) type ListItem = Entry<(Cow<'static, str>, ShellCommand)>;

/// One thing in a list: its words there and what it does.
pub(crate) fn item(words: impl Into<Cow<'static, str>>, command: ShellCommand) -> ListItem {
    Entry::Built((words.into(), command))
}

/// A control that works.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Control {
    /// Does a command when clicked. `lit` draws it as the thing in
    /// force (Pause, while paused); `keyed` shows its command's key small
    /// beside its word (Shape B).
    Do {
        label: &'static str,
        command: ShellCommand,
        lit: bool,
        keyed: bool,
    },
    /// A switch: a row with a box, ticked while it is on. With `steps`,
    /// the value after its "·" opens the list of steps (spec 172:
    /// "clicking the value edits the step"), as a right-click does.
    Switch {
        label: String,
        command: ShellCommand,
        steps: Option<Steps>,
    },
    /// Opens a list from its small arrow or a right-click. With a
    /// command of its own, a plain click does that; without one, a
    /// click opens the list too.
    List {
        label: Cow<'static, str>,
        command: Option<ShellCommand>,
        items: Vec<ListItem>,
    },
    /// Words that say something and do nothing.
    Fact(String),
}

/// One place on the ribbon: its shape, and what is there. A list of one
/// thing, so the unfinished parts' filter takes a hidden one out whole.
pub(crate) type Slot = (Shape, Vec<Entry<Control>>);

/// A group of the ribbon.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Group {
    /// The words under it. A group of three stacked rows has none: they
    /// don't fit under three rows.
    pub(crate) label: Option<&'static str>,
    pub(crate) slots: Vec<Slot>,
}

/// A large button that does `command`.
pub(crate) fn big(icon: Icon, label: &'static str, command: ShellCommand) -> Slot {
    let control = Control::Do {
        label,
        command,
        lit: false,
        keyed: false,
    };
    (Shape::Big(icon), vec![Entry::Built(control)])
}

/// A large button that does `command`, its key small beside its word,
/// lit while what it opened is in use (Home › Create › Shape).
pub(crate) fn big_keyed(icon: Icon, label: &'static str, command: ShellCommand, lit: bool) -> Slot {
    let control = Control::Do {
        label,
        command,
        lit,
        keyed: true,
    };
    (Shape::Big(icon), vec![Entry::Built(control)])
}

/// A large button that makes the usual kind on a click, and lists the
/// others from its small arrow or a right-click.
pub(crate) fn big_list(
    icon: Icon,
    label: &'static str,
    command: ShellCommand,
    items: Vec<ListItem>,
) -> Slot {
    let control = Control::List {
        label: label.into(),
        command: Some(command),
        items,
    };
    (Shape::Big(icon), vec![Entry::Built(control)])
}

/// A small row that does `command`.
pub(crate) fn row(label: &'static str, command: ShellCommand) -> Slot {
    let control = Control::Do {
        label,
        command,
        lit: false,
        keyed: false,
    };
    (Shape::Row(RowKind::Plain), vec![Entry::Built(control)])
}

/// A small row with a box to tick.
pub(crate) fn switch(label: impl Into<String>, command: ShellCommand) -> Slot {
    let control = Control::Switch {
        label: label.into(),
        command,
        steps: None,
    };
    (Shape::Row(RowKind::Check), vec![Entry::Built(control)])
}

/// A small row with a box to tick and a step to choose (Snapping).
pub(crate) fn switch_with_steps(
    label: impl Into<String>,
    command: ShellCommand,
    steps: Steps,
) -> Slot {
    let control = Control::Switch {
        label: label.into(),
        command,
        steps: Some(steps),
    };
    (Shape::Row(RowKind::Check), vec![Entry::Built(control)])
}

/// Which snapping step a switch's list chooses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Steps {
    /// The grid's, in metres.
    Grid,
    /// Turning's, in degrees.
    Angle,
}

impl Steps {
    /// The steps to choose from.
    pub(crate) fn all(self) -> &'static [f32] {
        match self {
            Self::Grid => &crate::transforming::GRID_STEPS,
            Self::Angle => &crate::transforming::ANGLE_STEPS,
        }
    }

    /// A step as the list says it: "0.25 m", "15°".
    pub(crate) fn words(self, step: f32) -> String {
        match self {
            Self::Grid => format!("{step} m"),
            Self::Angle => format!("{step}°"),
        }
    }

    /// The step in force.
    pub(crate) fn of(self, snapping: &crate::transforming::Snapping) -> f32 {
        match self {
            Self::Grid => snapping.grid_step,
            Self::Angle => snapping.angle_step,
        }
    }

    /// Puts `step` in force.
    pub(crate) fn set(self, snapping: &mut crate::transforming::Snapping, step: f32) {
        match self {
            Self::Grid => snapping.grid_step = step,
            Self::Angle => snapping.angle_step = step,
        }
    }
}

/// What was chosen on the ribbon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Picked {
    /// A control's command.
    Command(ShellCommand),
    /// A snapping step.
    Step(Steps, f32),
}

/// A small row that opens a list.
pub(crate) fn row_list(label: impl Into<Cow<'static, str>>, items: Vec<ListItem>) -> Slot {
    let control = Control::List {
        label: label.into(),
        command: None,
        items,
    };
    (Shape::Row(RowKind::List), vec![Entry::Built(control)])
}

/// A small row of words alone.
pub(crate) fn fact(words: impl Into<String>) -> Slot {
    (
        Shape::Row(RowKind::Plain),
        vec![Entry::Built(Control::Fact(words.into()))],
    )
}

/// A place whose control isn't built: `part` is `Entry::Unbuilt(..)`.
pub(crate) fn unbuilt_at(shape: Shape, part: Entry<Control>) -> Slot {
    (shape, vec![part])
}

/// `groups` as they are to be drawn: with unfinished parts hidden, each
/// unbuilt place taken out, and then each group left with nothing.
pub(crate) fn filtered(groups: Vec<Group>) -> Vec<Group> {
    let hide = unbuilt::hidden();
    groups
        .into_iter()
        .map(|group| Group {
            label: group.label,
            slots: unbuilt::filter_groups(group.slots),
        })
        .filter(|group| !(hide && group.slots.is_empty()))
        .collect()
}

/// A column of a group: one large button, or small rows stacked.
#[derive(Debug, Clone, PartialEq)]
struct Column {
    width: f32,
    slots: Vec<Slot>,
}

/// How wide a row is.
fn row_width(ui: &Ui, kind: RowKind, entry: &Entry<Control>) -> f32 {
    let label = match entry {
        Entry::Unbuilt(id) => return unbuilt::ribbon_row_width(ui, *id, kind),
        Entry::Built(Control::Do { label, .. }) => label,
        Entry::Built(Control::List { label, .. }) => label.as_ref(),
        Entry::Built(Control::Switch { label, .. }) => label.as_str(),
        Entry::Built(Control::Fact(words)) => words.as_str(),
    };
    let extra = match kind {
        RowKind::Plain => 0.0,
        RowKind::Check => size::CHECKBOX + size::GAP,
        RowKind::List => size::LIST_ARROW + size::GAP,
    };
    2.0 * size::RIBBON_ROW_PAD + extra + width_of(ui, label, text::ribbon_small())
}

/// A group's columns: each large button its own, and rows stacked two
/// high over a label, three high without one.
fn columns(ui: &Ui, group: &Group) -> Vec<Column> {
    let stack = if group.label.is_some() { 2 } else { 3 };
    let mut columns: Vec<Column> = Vec::new();
    let mut stacking = false;
    for slot in &group.slots {
        let (shape, entries) = slot;
        let Some(entry) = entries.first() else {
            continue;
        };
        match shape {
            Shape::Row(kind) => {
                let width = row_width(ui, *kind, entry);
                match columns.last_mut() {
                    Some(column) if stacking && column.slots.len() < stack => {
                        column.width = column.width.max(width);
                        column.slots.push(slot.clone());
                    }
                    _ => columns.push(Column {
                        width,
                        slots: vec![slot.clone()],
                    }),
                }
                stacking = true;
            }
            Shape::Big(_) | Shape::Light(_) => {
                // A working button whose word is longer than the button
                // ("Eyedropper") is widened to show it whole.
                let (word, key) = match entry {
                    Entry::Built(Control::Do {
                        label,
                        command,
                        keyed,
                        ..
                    }) => (Some(*label), keyed.then(|| command.keys())),
                    Entry::Built(Control::List { label, .. }) => (Some(label.as_ref()), None),
                    _ => (None, None),
                };
                let key_w = key.map_or(0.0, |k| {
                    size::RIBBON_KEY_GAP + width_of(ui, k, text::ribbon_key())
                });
                let words_w = word.map_or(0.0, |w| {
                    width_of(ui, w, text::ribbon_label()) + key_w + 2.0 * size::RIBBON_BIG_WORD_PAD
                });
                columns.push(Column {
                    width: size::RIBBON_BIG[0].max(words_w),
                    slots: vec![slot.clone()],
                });
                stacking = false;
            }
        }
    }
    columns
}

/// How wide a group is, its line included.
fn group_width(columns: &[Column]) -> f32 {
    let gaps = size::RIBBON_GAP * columns.len().saturating_sub(1) as f32;
    let inside: f32 = columns.iter().map(|c| c.width).sum();
    2.0 * size::RIBBON_GROUP_PAD[1] + inside + gaps + size::LINE
}

/// Where the last frame's ribbon put each group, for the tests that
/// measure it: the group's label and its rectangle.
#[cfg(test)]
pub(crate) fn groups_drawn(ctx: &egui::Context) -> Vec<(Option<&'static str>, Rect)> {
    ctx.data(|data| data.get_temp(egui::Id::new("ribbon groups drawn")))
        .unwrap_or_default()
}

/// Draws `groups` and, at the far right, `play`, in `band`. Returns what
/// was chosen.
pub(crate) fn ribbon(ui: &mut Ui, w: &Workbench, groups: &[Group], play: &Group) -> Option<Picked> {
    let band = ui.max_rect();
    ui.painter().rect_filled(band, 0.0, tokens::ribbon());
    let laid: Vec<(&Group, Vec<Column>)> = groups.iter().map(|g| (g, columns(ui, g))).collect();
    let play_columns = columns(ui, play);
    let play_w = if play.slots.is_empty() {
        0.0
    } else {
        group_width(&play_columns)
    };
    let groups_w: f32 = laid.iter().map(|(_, c)| group_width(c)).sum();
    let width = (groups_w + play_w).max(band.width());
    wheel_scrolls_sideways(ui, band);
    let mut picked = None;
    egui::ScrollArea::horizontal()
        .id_salt("ribbon")
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            let (whole, _) = ui.allocate_exact_size(vec2(width, band.height()), Sense::hover());
            let mut left = whole.left();
            #[cfg(test)]
            let mut drawn = Vec::new();
            for (n, (group, columns)) in laid.iter().enumerate() {
                let rect = Rect::from_min_size(
                    pos2(left, whole.top()),
                    vec2(group_width(columns), whole.height()),
                );
                tokens::vline(
                    ui.painter(),
                    rect.right() - size::LINE,
                    rect.y_range(),
                    tokens::line(),
                );
                picked = picked.or(draw_group(ui, w, rect, group, columns, n));
                #[cfg(test)]
                drawn.push((group.label, rect));
                left = rect.right();
            }
            if play_w > 0.0 {
                // Pushed to the right edge by whatever room is left.
                let rect = Rect::from_min_max(
                    pos2(whole.right() - play_w, whole.top()),
                    whole.right_bottom(),
                );
                tokens::vline(ui.painter(), rect.left(), rect.y_range(), tokens::line());
                let inner = rect.with_min_x(rect.left() + size::LINE);
                picked = picked.or(draw_group(ui, w, inner, play, &play_columns, usize::MAX));
                #[cfg(test)]
                drawn.push((play.label, rect));
            }
            #[cfg(test)]
            ui.ctx()
                .data_mut(|data| data.insert_temp(egui::Id::new("ribbon groups drawn"), drawn));
        });
    tokens::hline(
        ui.painter(),
        band.x_range(),
        band.bottom() - size::LINE,
        tokens::line(),
    );
    picked
}

/// The wheel over the ribbon scrolls it sideways (the project lead's
/// answer, decision 0103): an up-and-down turn of the wheel is taken as
/// a sideways one while the pointer is over the band.
fn wheel_scrolls_sideways(ui: &Ui, band: Rect) {
    let over = ui
        .input(|input| input.pointer.hover_pos())
        .is_some_and(|at| band.contains(at));
    if over {
        ui.input_mut(|input| {
            let turned = input.smooth_scroll_delta;
            if turned.x == 0.0 && turned.y != 0.0 {
                input.smooth_scroll_delta = vec2(turned.y, 0.0);
            }
        });
    }
}

/// Draws one group in `rect`: its columns in the middle of the room over
/// its label, and the label.
fn draw_group(
    ui: &mut Ui,
    w: &Workbench,
    rect: Rect,
    group: &Group,
    columns: &[Column],
    n: usize,
) -> Option<Picked> {
    let [pad_top, pad_side, pad_foot] = size::RIBBON_GROUP_PAD;
    let label_h = if group.label.is_some() {
        size::RIBBON_GROUP_LABEL_H
    } else {
        0.0
    };
    let room = Rect::from_min_max(
        pos2(rect.left() + pad_side, rect.top() + pad_top),
        pos2(
            rect.right() - pad_side - size::LINE,
            rect.bottom() - size::LINE - pad_foot - label_h,
        ),
    );
    if let Some(label) = group.label {
        let job = text::caps(label, text::GROUP_LABEL, tokens::text_meta());
        let galley = ui.painter().layout_job(job);
        let line = Rect::from_min_max(
            room.left_bottom(),
            pos2(room.right(), room.bottom() + label_h),
        );
        let at = Align2::CENTER_CENTER
            .align_size_within_rect(galley.size(), line)
            .min;
        ui.painter().galley(at, galley, tokens::text_meta());
    }
    let row_h = if group.label.is_some() {
        size::RIBBON_ROW_H
    } else {
        size::RIBBON_ROW_H_TIGHT
    };
    let mut picked = None;
    let mut left = room.left();
    for (c, column) in columns.iter().enumerate() {
        let stack = column.slots.len() as f32;
        let height = match column.slots.first() {
            Some((Shape::Row(_), _)) => stack * row_h + (stack - 1.0) * size::RIBBON_ROW_GAP,
            _ => size::RIBBON_BIG[1],
        };
        let mut top = room.center().y - height / 2.0;
        for (r, (shape, entries)) in column.slots.iter().enumerate() {
            let Some(entry) = entries.first() else {
                continue;
            };
            let slot_h = match shape {
                Shape::Row(_) => row_h,
                Shape::Big(_) | Shape::Light(_) => size::RIBBON_BIG[1],
            };
            let spot = Rect::from_min_size(pos2(left, top), vec2(column.width, slot_h));
            let id = ui.id().with(("ribbon", n, c, r));
            picked = picked.or(draw_slot(ui, w, spot, id, *shape, entry));
            top += slot_h + size::RIBBON_ROW_GAP;
        }
        left += column.width + size::RIBBON_GAP;
    }
    picked
}

/// Draws what is at one place.
fn draw_slot(
    ui: &mut Ui,
    w: &Workbench,
    spot: Rect,
    id: egui::Id,
    shape: Shape,
    entry: &Entry<Control>,
) -> Option<Picked> {
    let control = match entry {
        Entry::Unbuilt(part) => {
            let mut place = ui.new_child(UiBuilder::new().max_rect(spot));
            match shape {
                Shape::Big(icon) | Shape::Light(icon) => {
                    unbuilt::ribbon_big(&mut place, *part, icon)
                }
                Shape::Row(kind) => unbuilt::ribbon_row(&mut place, *part, kind),
            };
            return None;
        }
        Entry::Built(control) => control,
    };
    match (control, shape) {
        (Control::Fact(words), _) => {
            controls::fact(ui, spot, id, words);
            None
        }
        (
            Control::Switch {
                label,
                command,
                steps,
            },
            _,
        ) => {
            let look = Look::of(*command, w, false);
            match steps {
                None => controls::row(ui, spot, id, label, RowKind::Check, &look)
                    .clicked()
                    .then_some(Picked::Command(*command)),
                Some(steps) => controls::switch_with_steps(
                    ui,
                    spot,
                    id,
                    (label, *command),
                    *steps,
                    &look,
                    w.viewport.snapping,
                ),
            }
        }
        (
            Control::Do {
                label,
                command,
                lit,
                keyed,
            },
            _,
        ) => {
            let look = Look::of(*command, w, *lit).with_key(keyed.then(|| command.keys()));
            let response = match shape {
                Shape::Big(icon) => controls::big(ui, spot, id, icon, label, &look, false).0,
                Shape::Light(icon) => controls::big(ui, spot, id, icon, label, &look, true).0,
                Shape::Row(kind) => controls::row(ui, spot, id, label, kind, &look),
            };
            response.clicked().then_some(Picked::Command(*command))
        }
        (
            Control::List {
                label,
                command,
                items,
            },
            _,
        ) => {
            controls::with_list(ui, w, spot, id, shape, label, *command, items).map(Picked::Command)
        }
    }
}

impl Workbench {
    /// The ribbon of the tab in front, in its own band under the tab
    /// bar; or, collapsed, dropped down over the center after a click
    /// on a tab, until something on it is chosen or the pointer leaves.
    pub(crate) fn ribbon_band(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        if self.shell.tab == super::RibbonTab::File {
            return;
        }
        let groups = filtered(super::ribbons::of(self.shell.tab, self));
        // LIBRARY TOOLS ribbons have no Play group, as the design's
        // Mesh ribbon (round 3).
        let empty = Group {
            label: None,
            slots: Vec::new(),
        };
        let play = if super::RibbonTab::CONTEXT.contains(&self.shell.tab) {
            empty
        } else {
            filtered(vec![super::play_group::group(self)])
                .pop()
                .unwrap_or(empty)
        };
        let mut picked = None;
        if self.shell.ribbon_in_band() {
            egui::Panel::top("ribbon")
                .frame(egui::Frame::new())
                .exact_size(size::RIBBON_H)
                .show_separator_line(false)
                .show(ui, |ui| picked = ribbon(ui, self, &groups, &play));
        } else if self.shell.dropped {
            let screen = ui.ctx().content_rect();
            let band = Rect::from_min_size(
                pos2(screen.left(), screen.top() + size::TAB_BAR_H),
                vec2(screen.width(), size::RIBBON_H),
            );
            egui::Area::new(egui::Id::new("ribbon dropped"))
                .order(egui::Order::Middle)
                .fixed_pos(band.min)
                .show(ui.ctx(), |ui| {
                    ui.set_clip_rect(band);
                    let mut inside = ui.new_child(UiBuilder::new().max_rect(band));
                    picked = ribbon(&mut inside, self, &groups, &play);
                    ui.allocate_rect(band, Sense::hover());
                });
            // Away again once the pointer is below it, unless a list
            // opened from it is still up.
            let below = ui
                .input(|input| input.pointer.hover_pos())
                .is_some_and(|at| at.y > band.bottom());
            if below && !egui::Popup::is_any_open(ui.ctx()) {
                self.shell.dropped = false;
            }
        }
        match picked {
            Some(Picked::Command(command)) => {
                self.shell.dropped = false;
                command.run(self, ui.ctx(), actions);
            }
            Some(Picked::Step(steps, step)) => {
                self.shell.dropped = false;
                steps.set(&mut self.viewport.snapping, step);
            }
            None => {}
        }
    }
}
