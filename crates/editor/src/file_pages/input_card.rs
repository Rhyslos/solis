//! Project settings › Input's card: the picked action's name and type,
//! its bindings with their devices and ×, + Add binding, the whole axis,
//! the speeds; Device settings (not built) and the ready-made actions'
//! links.

use super::input_page::{InputPage, Slot, is_pad};
use super::{plain, sentence};
use crate::settled::settled_number;
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, Entry, UnbuiltId};
use crate::widgets::{self, button::Button, checkbox, keycap, segmented::segmented};
use egui::{Align, Layout, Sense, Ui, vec2};
use input::{Action, ActionKind, Bindings};

/// The picked action's card, the device settings (not built) and the
/// ready-made actions' links. Returns whether anything changed.
pub(super) fn action_card(ui: &mut Ui, page: &mut InputPage, bindings: &mut Bindings) -> bool {
    let mut changed = false;
    if bindings.actions.is_empty() {
        sentence(ui, "No actions yet: Add action makes one.");
    } else {
        let index = page.picked.unwrap_or(0).min(bindings.actions.len() - 1);
        page.picked = Some(index);
        let mut remove = false;
        widgets::card::frame(ui).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = size::CARD_INNER_GAP;
            changed |= card_head(ui, &mut bindings.actions[index]);
            changed |= binding_lists(ui, page, index, &mut bindings.actions[index]);
            for entry in unbuilt::filter(vec![
                Entry::<()>::Unbuilt(UnbuiltId::InputTrigger),
                Entry::Unbuilt(UnbuiltId::InputBuffer),
            ]) {
                if let Entry::Unbuilt(id) = entry {
                    unbuilt::field_row(ui, id);
                }
            }
            changed |= speeds(ui, index, &mut bindings.actions[index]);
            remove = Button::plain("Remove the action").show(ui).clicked();
        });
        if remove {
            bindings.actions.remove(index);
            page.picked = None;
            page.picked_binding = None;
            page.listening = None;
            changed = true;
        }
    }
    let device = unbuilt::filter(vec![
        Entry::<()>::Unbuilt(UnbuiltId::InputDeadZone),
        Entry::Unbuilt(UnbuiltId::InputInvertY),
    ]);
    if !device.is_empty() {
        widgets::card::card(ui, "Device settings", |ui| {
            for entry in device {
                if let Entry::Unbuilt(id) = entry {
                    unbuilt::field_row(ui, id);
                }
            }
        });
    }
    changed |= ready_made(ui, bindings);
    changed
}

/// The card's first lines: the name, Button or Axis, and how game code
/// names it.
fn card_head(ui: &mut Ui, action: &mut Action) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        plain(ui, &action.name, text::heading(), tokens::text_primary());
        let mut kind = action.kind;
        let choices = [(ActionKind::Button, "Button"), (ActionKind::Axis, "Axis")];
        if segmented(ui, "Type", &mut kind, &choices) {
            action.kind = kind;
            changed = true;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            plain(
                ui,
                &format!("Action.{}", action.name),
                text::mono_small(),
                tokens::text_meta(),
            );
        });
    });
    changed
}

/// The action's bindings, a list for a button or one per way for an
/// axis, each binding a row with ×, then + to listen for another.
fn binding_lists(ui: &mut Ui, page: &mut InputPage, index: usize, action: &mut Action) -> bool {
    let mut changed = false;
    plain(ui, "Bindings", text::row(), tokens::text_meta());
    let slots: &[(Slot, &str)] = match action.kind {
        ActionKind::Button => &[(Slot::Inputs, "")],
        ActionKind::Axis => &[
            (Slot::Up, "Up"),
            (Slot::Down, "Down"),
            (Slot::Left, "Left"),
            (Slot::Right, "Right"),
        ],
    };
    for &(slot, way) in slots {
        if !way.is_empty() {
            plain(ui, way, text::row(), tokens::text_label());
        }
        let mut remove = None;
        for (n, input) in slot.list(action).iter().enumerate() {
            let picked = page.picked_binding == Some((slot, n));
            let (row, cross) = binding_row(ui, input, picked);
            if cross.clicked() {
                remove = Some(n);
            } else if row.clicked() {
                page.picked_binding = Some((slot, n));
            }
        }
        if let Some(n) = remove {
            slot.list(action).remove(n);
            page.picked_binding = None;
            changed = true;
        }
        let listening = page
            .listening
            .is_some_and(|(i, s, _)| i == index && s == slot);
        if listening {
            keycap::keycap(
                ui,
                "Press a key, mouse button or gamepad button… (Esc calls it off)",
                keycap::KeycapState::Listening,
            );
        } else {
            let add = if way.is_empty() {
                "+ Add binding".to_string()
            } else {
                format!("+ {way}")
            };
            if widgets::button::dashed(ui, &add)
                .on_hover_text("Press it next")
                .clicked()
            {
                page.listening = Some((index, slot, 0));
            }
        }
    }
    if action.kind == ActionKind::Axis {
        plain(ui, "Whole axis", text::row(), tokens::text_label());
        for whole in [
            input::MOUSE_MOTION,
            input::MOUSE_WHEEL,
            "Pad LeftStick",
            "Pad RightStick",
        ] {
            let mut on = action.inputs.iter().any(|i| i == whole);
            if checkbox::checkbox(ui, &mut on, whole).changed() {
                if on {
                    action.inputs.push(whole.to_string());
                } else {
                    action.inputs.retain(|i| i != whole);
                }
                changed = true;
            }
        }
    }
    ui.horizontal(|ui| {
        keycap::keycap(ui, "Backspace", keycap::KeycapState::Bound);
        plain(
            ui,
            "clears the picked binding",
            text::row(),
            tokens::text_meta(),
        );
    });
    changed
}

/// A binding: its device, its keycap and ×. Returns the row's response
/// and the ×'s.
fn binding_row(ui: &mut Ui, input: &str, picked: bool) -> (egui::Response, egui::Response) {
    let width = ui.available_width();
    let (rect, row) = ui.allocate_exact_size(
        vec2(width, size::KEYCAP + size::CARD_INNER_GAP),
        Sense::click(),
    );
    row.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Button,
            true,
            picked,
            format!("Binding {input}"),
        )
    });
    let edge = if picked {
        crate::interface_look::accent_fill()
    } else {
        tokens::card_border()
    };
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(size::radius::CONTROL),
        tokens::field(),
    );
    widgets::border(ui, rect, size::radius::CONTROL, edge);
    let device = if is_pad(input) {
        "Gamepad"
    } else if input.starts_with("Mouse") {
        "Mouse"
    } else {
        "Keyboard"
    };
    let inside = rect.shrink2(vec2(size::FIELD_PAD, 0.0));
    let mut line = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inside)
            .layout(Layout::left_to_right(Align::Center)),
    );
    line.spacing_mut().item_spacing.x = size::CARD_INNER_GAP;
    plain(&mut line, device, text::row(), tokens::text_label());
    keycap::keycap(
        &mut line,
        input.trim_start_matches("Pad "),
        keycap::KeycapState::Bound,
    );
    let cross = line
        .with_layout(Layout::right_to_left(Align::Center), |ui| {
            widgets::button::icon(ui, crate::icons::Icon::Close, &format!("Unbind {input}"))
        })
        .inner;
    (row, cross)
}

/// Mouse speed and Stick and key speed, for an axis that goes with the
/// mouse. Each counts once it's let go, so a drag is one save.
fn speeds(ui: &mut Ui, index: usize, action: &mut Action) -> bool {
    if action.kind != ActionKind::Axis || !action.is_motion() {
        return false;
    }
    let mut changed = false;
    ui.horizontal(|ui| {
        plain(ui, "Mouse speed", text::row(), tokens::text_label());
        let (speed, field) = settled_number(ui, ("mouse speed", index), f64::from(action.mouse_speed), |d| {
            d.range(0.0..=100.0).speed(0.01)
        });
        field.on_hover_text("How far a pixel of mouse movement (or a notch of the wheel) goes: degrees for Look, steps for Zoom.");
        if let Some(speed) = speed {
            action.mouse_speed = speed as f32;
            changed = true;
        }
    });
    ui.horizontal(|ui| {
        plain(ui, "Stick and key speed", text::row(), tokens::text_label());
        let (speed, field) =
            settled_number(ui, ("pad speed", index), f64::from(action.pad_speed), |d| {
                d.range(0.0..=10_000.0).speed(1.0)
            });
        field.on_hover_text("How far a stick or key held all the way goes in a second.");
        if let Some(speed) = speed {
            action.pad_speed = speed as f32;
            changed = true;
        }
    });
    changed
}

/// The links under the cards: the ready-made actions a project lacks,
/// and putting them all back.
fn ready_made(ui: &mut Ui, bindings: &mut Bindings) -> bool {
    let mut changed = false;
    let missing = Bindings::default()
        .actions
        .iter()
        .filter(|a| bindings.get(&a.name).is_none())
        .count();
    if missing > 0
        && widgets::button::link(
            ui,
            &format!("Add the {missing} ready-made actions it's missing"),
        )
        .on_hover_text(
            "Projects made before the cameras or menus lack some of Pan, Turn, Zoom, Drag, \
                 MenuMove, MenuChoose, MenuBack and Pause: this adds them with their usual keys, \
                 and changes nothing else",
        )
        .clicked()
    {
        bindings.add_missing();
        changed = true;
    }
    let restore = widgets::button::link(ui, "Restore the ready-made actions").on_hover_text(
        "Move, Look, Jump, Sprint, Interact, Pan, Turn, Zoom, Drag and the menu actions with their \
         usual keys; other actions stay",
    );
    if restore.clicked() {
        for ready in Bindings::default().actions {
            let room = bindings.actions.len() < input::MAX_ACTIONS;
            match bindings.actions.iter_mut().find(|a| a.name == ready.name) {
                Some(mine) => *mine = ready,
                None if room => bindings.actions.push(ready),
                None => {}
            }
        }
        changed = true;
    }
    changed
}
