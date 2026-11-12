//! File › Project settings › Input (Screen 13; round 2's
//! `13-project-settings-input`): the project's input actions and what sets
//! each off ([decision 0037](../../../../docs/decisions/0037-input-as-named-actions.md)),
//! as a table, with the picked action's bindings in a card beside it.
//!
//! A binding is added by pressing the key or button to bind (Esc calls
//! that off; while listening, Backspace is a key like any other). With
//! one picked, Backspace clears it, as its × does (answer 25). Every
//! change is saved to the project's `input.solinput` at once; a number
//! being dragged or typed is one change, made when it's let go.
//!
//! The ready-made actions share some keys on purpose (the arrows are
//! Move, Pan and MenuMove; Space is Jump and MenuChoose), so a shared key
//! is marked only where one of the actions is the project's own.

use super::input_card::action_card;
use super::input_table::actions_table;
use super::plain;
use crate::console::LogLevel;
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, Entry, UnbuiltId};
use crate::widgets::{self, button::Button, field, segmented::segmented};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Align, Layout, Rect, Ui};
use input::{Action, Bindings};

/// Which inputs the table shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Devices {
    #[default]
    All,
    KeyboardMouse,
    Gamepad,
}

/// The page's own state, for the open project.
#[derive(Debug, Clone, Default)]
pub(crate) struct InputPage {
    /// Waiting for an input to bind: the action's number, which list, and
    /// how many frames it has waited (the click that started it isn't the
    /// input).
    pub(crate) listening: Option<(usize, Slot, u32)>,
    /// The name typed for a new action.
    pub(crate) new_name: String,
    /// The modifiers and pad buttons down when listening started, so
    /// only a new one counts.
    held_before: (egui::Modifiers, Vec<String>),
    /// The action picked in the table.
    pub(crate) picked: Option<usize>,
    /// The binding picked in its card: which list, and its place there.
    pub(crate) picked_binding: Option<(Slot, usize)>,
    /// Which inputs the table shows.
    pub(crate) devices: Devices,
    /// What's typed in the table's search.
    pub(crate) search: String,
}

/// Whether `input` is a gamepad's.
pub(super) fn is_pad(input: &str) -> bool {
    input.starts_with("Pad ")
}

/// Every input the ready-made actions share on purpose, and the clashes
/// to mark: an input on two actions, one of them the project's own. Each
/// clash is the input and the two actions' names.
pub(crate) fn clashes(bindings: &Bindings) -> Vec<(String, String, String)> {
    let ready: Vec<String> = Bindings::default()
        .actions
        .into_iter()
        .map(|a| a.name)
        .collect();
    let all = |a: &Action| -> Vec<String> {
        [&a.inputs, &a.up, &a.down, &a.left, &a.right]
            .into_iter()
            .flatten()
            .cloned()
            .collect()
    };
    let mut found = Vec::new();
    for (n, a) in bindings.actions.iter().enumerate() {
        for b in &bindings.actions[n + 1..] {
            if ready.contains(&a.name) && ready.contains(&b.name) {
                continue;
            }
            let theirs = all(b);
            for input in all(a) {
                if theirs.contains(&input)
                    && !found.iter().any(|(i, x, _)| *i == input && *x == a.name)
                {
                    found.push((input, a.name.clone(), b.name.clone()));
                }
            }
        }
    }
    found
}

impl Workbench {
    /// Opens File › Project settings › Input.
    pub(crate) fn open_input_page(&mut self) {
        if self.open.input_page.is_none() {
            self.open.input_page = Some(InputPage::default());
        }
        self.show_settings(super::Tab::Input);
    }

    /// Draws the Input page: the tools, the table and the picked action's
    /// card.
    pub(super) fn input_page(&mut self, ui: &mut Ui) {
        if self.open.project.is_none() {
            plain(
                ui,
                "Input actions belong to a project: open or make one first.",
                text::ui(),
                tokens::text_meta(),
            );
            return;
        }
        let mut page = self.open.input_page.take().unwrap_or_default();
        let mut bindings = self.open.bindings.clone();
        let pad_down: Vec<String> = self.pad_now.as_ref().map_or(Vec::new(), |p| {
            p.buttons.iter().map(|b| format!("Pad {b}")).collect()
        });
        let mut changed = heard(ui, &mut page, &mut bindings, &pad_down);
        changed |= clear_with_backspace(ui, &mut page, &mut bindings);
        changed |= tools(ui, &mut page, &mut bindings);
        ui.add_space(size::SECTION_GAP);
        let whole = ui.available_rect_before_wrap();
        let card_w = size::PAGE_SIDE_W - size::CARD_GAP * 3.0;
        let table = Rect::from_min_max(
            whole.min,
            egui::pos2(
                whole.right() - card_w - size::PROJECT_CARD_GAP,
                whole.bottom(),
            ),
        );
        let side = Rect::from_min_max(
            egui::pos2(table.right() + size::PROJECT_CARD_GAP, whole.top()),
            whole.max,
        );
        let mut table_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(table)
                .layout(Layout::top_down(Align::Min)),
        );
        egui::ScrollArea::vertical()
            .id_salt("input table")
            .auto_shrink([false, false])
            .show(&mut table_ui, |ui| actions_table(ui, &mut page, &bindings));
        let mut side_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(side)
                .layout(Layout::top_down(Align::Min)),
        );
        side_ui.spacing_mut().item_spacing.y = size::CARD_GAP;
        egui::ScrollArea::vertical()
            .id_salt("input card")
            .auto_shrink([false, false])
            .show(&mut side_ui, |ui| {
                changed |= action_card(ui, &mut page, &mut bindings)
            });
        if changed {
            match bindings.check() {
                Ok(()) => self.change_bindings(bindings),
                Err(why) => self.log(LogLevel::Warning, format!("Couldn't change that: {why}.")),
            }
        }
        self.open.input_page = Some(page);
    }
}

/// Backspace, with a binding picked and nothing listening or typed into,
/// clears it (answer 25). Returns whether it did.
fn clear_with_backspace(ui: &Ui, page: &mut InputPage, bindings: &mut Bindings) -> bool {
    if page.listening.is_some() || ui.ctx().egui_wants_keyboard_input() {
        return false;
    }
    if !ui.input(|i| i.key_pressed(egui::Key::Backspace)) {
        return false;
    }
    let (Some(index), Some((slot, n))) = (page.picked, page.picked_binding) else {
        return false;
    };
    let Some(action) = bindings.actions.get_mut(index) else {
        return false;
    };
    let list = slot.list(action);
    if n >= list.len() {
        return false;
    }
    list.remove(n);
    page.picked_binding = None;
    true
}

/// The row over the table: Contexts (not built), the devices shown, the
/// search and Add action. Returns whether an action was added.
fn tools(ui: &mut Ui, page: &mut InputPage, bindings: &mut Bindings) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = size::CARD_INNER_GAP;
        for entry in unbuilt::filter(vec![Entry::<()>::Unbuilt(UnbuiltId::InputContexts)]) {
            if let Entry::Unbuilt(id) = entry {
                unbuilt::chip(ui, id);
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = size::CARD_INNER_GAP;
            let add = Button::primary("Add action ▾")
                .icon(crate::icons::Icon::Plus)
                .show(ui);
            egui::Popup::menu(&add)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| changed |= add_menu(ui, page, bindings));
            field::search_field(
                ui,
                "Find action or key",
                &mut page.search,
                "Find action or key",
                size::PAGE_SEARCH_W,
            );
            let choices = [
                (Devices::All, "All devices"),
                (Devices::KeyboardMouse, "Keyboard & mouse"),
                (Devices::Gamepad, "Gamepad"),
            ];
            segmented(ui, "Devices shown", &mut page.devices, &choices);
        });
    });
    changed
}

/// Add action's list: the new action's name, then a button or an axis.
fn add_menu(ui: &mut Ui, page: &mut InputPage, bindings: &mut Bindings) -> bool {
    ui.set_min_width(size::SETTING_CONTROL_W);
    field::text_field(
        ui,
        "New action's name",
        &mut page.new_name,
        "Name, such as Jump",
        ui.available_width(),
    );
    let name = page.new_name.trim().to_string();
    let room = bindings.actions.len() < input::MAX_ACTIONS;
    let usable = room && input::valid_action_name(&name) && bindings.get(&name).is_none();
    let why = if room {
        "Letters, digits and _, starting with a letter, and not taken"
    } else {
        "A project has at most 256 actions"
    };
    let mut added = None;
    ui.add_enabled_ui(usable, |ui| {
        if widgets::menu::item(ui, "Add button", "")
            .on_disabled_hover_text(why)
            .clicked()
        {
            added = Some(Action::button(&name, &[]));
        }
        if widgets::menu::item(ui, "Add axis", "")
            .on_disabled_hover_text(why)
            .clicked()
        {
            added = Some(Action::axis(&name, &[], [&[], &[], &[], &[]]));
        }
    });
    let Some(action) = added else {
        return false;
    };
    bindings.actions.push(action);
    page.picked = Some(bindings.actions.len() - 1);
    page.new_name.clear();
    ui.close();
    true
}

/// Which list of an action's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Slot {
    /// A button's inputs, or an axis's whole-axis ones.
    Inputs,
    Up,
    Down,
    Left,
    Right,
}

impl Slot {
    pub(super) fn list(self, action: &mut Action) -> &mut Vec<String> {
        match self {
            Self::Inputs => &mut action.inputs,
            Self::Up => &mut action.up,
            Self::Down => &mut action.down,
            Self::Left => &mut action.left,
            Self::Right => &mut action.right,
        }
    }
}

impl Workbench {
    /// The project's input actions changed (File › Project settings › Input):
    /// kept, saved in the project, and taken by a running game at once
    /// (game code's own rebinding for this play gives way to them).
    pub(crate) fn change_bindings(&mut self, bindings: Bindings) {
        self.open.bindings = bindings;
        if let Some(playing) = &mut self.open.playing {
            playing.bindings = std::sync::Arc::new(self.open.bindings.clone());
        }
        if self.open.project.is_some() {
            self.open
                .outbox
                .push(WorkbenchAction::SaveBindings(self.open.bindings.write()));
        }
    }
}

/// Binds what was pressed while listening; returns whether it bound one.
fn heard(ui: &Ui, window: &mut InputPage, bindings: &mut Bindings, pad_down: &[String]) -> bool {
    let Some((index, slot, waited)) = window.listening else {
        return false;
    };
    if waited < 2 {
        // The click that started listening isn't what's bound.
        window.listening = Some((index, slot, waited + 1));
        if waited == 0 {
            window.held_before = (ui.input(|i| i.modifiers), pad_down.to_vec());
        }
        return false;
    }
    let (events, modifiers) = ui.input(|i| (i.events.clone(), i.modifiers));
    let mut found: Option<String> = None;
    for event in events {
        match event {
            egui::Event::Key {
                key: egui::Key::Escape,
                pressed: true,
                ..
            } => {
                window.listening = None;
                return false;
            }
            egui::Event::Key {
                key,
                pressed: true,
                modifiers: held,
                ..
            } => found = Some(combination(held, crate::play_input::key_name(key))),
            egui::Event::MouseWheel { delta, .. } if delta.y != 0.0 => {
                found = Some(
                    if delta.y > 0.0 {
                        "Mouse WheelUp"
                    } else {
                        "Mouse WheelDown"
                    }
                    .to_string(),
                );
            }
            egui::Event::PointerButton {
                button,
                pressed: true,
                ..
            } => found = Some(crate::play_input::mouse_name(button).to_string()),
            _ => {}
        }
    }
    let before = window.held_before.0;
    for (now, was, name) in [
        (modifiers.shift, before.shift, "Shift"),
        (modifiers.ctrl, before.ctrl, "Ctrl"),
        (modifiers.alt, before.alt, "Alt"),
    ] {
        if now && !was && found.is_none() {
            found = Some(name.to_string());
        }
    }
    if found.is_none() {
        found = pad_down
            .iter()
            .find(|b| !window.held_before.1.contains(b))
            .cloned();
    }
    let Some(input) = found.filter(|i| input::valid_input_name(i)) else {
        return false;
    };
    window.listening = None;
    let Some(action) = bindings.actions.get_mut(index) else {
        return false;
    };
    let list = slot.list(action);
    if list.contains(&input) || list.len() >= input::MAX_INPUTS {
        return false;
    }
    list.push(input);
    true
}

/// A key's name with the modifiers held (`Alt+Left`), as bindings
/// write a combination.
fn combination(held: egui::Modifiers, key: &str) -> String {
    let mut name = String::new();
    for (on, modifier) in [
        (held.ctrl, "Ctrl"),
        (held.alt, "Alt"),
        (held.shift, "Shift"),
    ] {
        if on {
            name.push_str(modifier);
            name.push('+');
        }
    }
    name.push_str(key);
    name
}

#[cfg(test)]
#[path = "input_page_tests.rs"]
mod tests;
