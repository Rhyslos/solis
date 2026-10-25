//! Edit > Input actions…: the project's input actions and what sets each
//! off ([decision 0037](../../../docs/decisions/0037-input-as-named-actions.md)),
//! changed by pressing the key or button to bind. Every change is saved
//! to the project's `input.solinput` at once.

use crate::console::LogLevel;
use crate::theme;
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{RichText, Ui};
use input::{Action, ActionKind, Bindings};

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
    fn list(self, action: &mut Action) -> &mut Vec<String> {
        match self {
            Self::Inputs => &mut action.inputs,
            Self::Up => &mut action.up,
            Self::Down => &mut action.down,
            Self::Left => &mut action.left,
            Self::Right => &mut action.right,
        }
    }
}

/// The window's own state.
#[derive(Debug, Clone, Default)]
pub(crate) struct InputWindow {
    /// Waiting for an input to bind: the action's number, which list, and
    /// how many frames it has waited (the click that started it isn't the
    /// input).
    pub(crate) listening: Option<(usize, Slot, u32)>,
    /// The name typed for a new action.
    pub(crate) new_name: String,
    /// The modifiers and pad buttons down when listening started, so
    /// only a new one counts.
    held_before: (egui::Modifiers, Vec<String>),
}

/// What happened to the bindings this frame.
enum Change {
    None,
    Changed,
}

impl Workbench {
    /// Opens Edit > Input actions….
    pub(crate) fn open_input_window(&mut self) {
        if self.input_window.is_none() {
            self.input_window = Some(InputWindow::default());
        }
    }

    /// Draws the Input actions window, if it's open.
    pub(crate) fn input_actions_window(&mut self, ctx: &egui::Context) {
        let Some(mut window) = self.input_window.take() else {
            return;
        };
        let mut open = true;
        let mut bindings = self.bindings.clone();
        let pad_down: Vec<String> = self.pad_now.as_ref().map_or(Vec::new(), |p| {
            p.buttons.iter().map(|b| format!("Pad {b}")).collect()
        });
        let mut change = Change::None;
        egui::Window::new("Input actions")
            .open(&mut open)
            .default_width(460.0)
            .default_height(520.0)
            .vscroll(true)
            .show(ctx, |ui| {
                if heard(ui, &mut window, &mut bindings, &pad_down) {
                    change = Change::Changed;
                }
                if actions_ui(ui, &mut window, &mut bindings, &pad_down) {
                    change = Change::Changed;
                }
            });
        if let Change::Changed = change {
            match bindings.check() {
                Ok(()) => {
                    self.bindings = bindings;
                    if self.project.is_some() {
                        self.outbox
                            .push(WorkbenchAction::SaveBindings(self.bindings.write()));
                    }
                }
                Err(why) => self.log(LogLevel::Warning, format!("Couldn't change that: {why}.")),
            }
        }
        if open {
            self.input_window = Some(window);
        }
    }
}

/// Binds what was pressed while listening; returns whether it bound one.
fn heard(ui: &Ui, window: &mut InputWindow, bindings: &mut Bindings, pad_down: &[String]) -> bool {
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
                key, pressed: true, ..
            } => found = Some(crate::play_input::key_name(key).to_string()),
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

/// A row of an action's inputs, each with × to remove, and + to listen
/// for another. Returns whether one was removed.
fn chips(ui: &mut Ui, window: &mut InputWindow, action: &mut Action, index: usize, slot: Slot) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        let list = slot.list(action);
        let mut remove = None;
        for (n, input) in list.iter().enumerate() {
            ui.label(RichText::new(input).monospace());
            if ui
                .small_button("×")
                .on_hover_text(format!("Unbind {input}"))
                .clicked()
            {
                remove = Some(n);
            }
            ui.add_space(6.0);
        }
        if let Some(n) = remove {
            list.remove(n);
            changed = true;
        }
        let listening = window.listening.is_some_and(|(i, s, _)| i == index && s == slot);
        if listening {
            ui.label(
                RichText::new("Press a key, mouse button or gamepad button… (Esc cancels)")
                    .color(theme::WARNING),
            );
        } else if ui
            .small_button("+")
            .on_hover_text("Bind another input: press it next")
            .clicked()
        {
            window.listening = Some((index, slot, 0));
        }
    });
    changed
}

/// Every action, with its kind and inputs, and adding and removing
/// actions. Returns whether anything changed.
fn actions_ui(ui: &mut Ui, window: &mut InputWindow, bindings: &mut Bindings, _pad: &[String]) -> bool {
    let mut changed = false;
    ui.label(
        "Game code asks for an action by its name, such as held(\"Jump\"), never for a key, \
         so players can rebind and gamepads work too. Axes give two numbers: Move which way to \
         go, Look how far to turn.",
    );
    ui.add_space(6.0);
    let mut remove = None;
    for (index, action) in bindings.actions.iter_mut().enumerate() {
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new(&action.name).strong());
            let kind = action.kind;
            egui::ComboBox::from_id_salt(("input kind", index))
                .selected_text(match kind {
                    ActionKind::Button => "Button",
                    ActionKind::Axis => "Axis",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut action.kind, ActionKind::Button, "Button");
                    ui.selectable_value(&mut action.kind, ActionKind::Axis, "Axis");
                });
            if action.kind != kind {
                changed = true;
            }
            if ui.small_button("Remove").clicked() {
                remove = Some(index);
            }
        });
        match action.kind {
            ActionKind::Button => {
                changed |= chips(ui, window, action, index, Slot::Inputs);
            }
            ActionKind::Axis => {
                for (slot, label) in [
                    (Slot::Up, "Up"),
                    (Slot::Down, "Down"),
                    (Slot::Left, "Left"),
                    (Slot::Right, "Right"),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(format!("{label}:"));
                        changed |= chips(ui, window, action, index, slot);
                    });
                }
                ui.horizontal(|ui| {
                    ui.label("Whole axis:");
                    for whole in [input::MOUSE_MOTION, "Pad LeftStick", "Pad RightStick"] {
                        let mut on = action.inputs.iter().any(|i| i == whole);
                        if ui.checkbox(&mut on, whole).changed() {
                            if on {
                                action.inputs.push(whole.to_string());
                            } else {
                                action.inputs.retain(|i| i != whole);
                            }
                            changed = true;
                        }
                    }
                });
                if action.is_motion() {
                    ui.horizontal(|ui| {
                        ui.label("Mouse speed");
                        changed |= ui
                            .add(egui::DragValue::new(&mut action.mouse_speed).range(0.0..=100.0).speed(0.01))
                            .on_hover_text("How far a pixel of mouse movement turns (degrees, for the ready-made cameras).")
                            .changed();
                        ui.label("Stick and key speed");
                        changed |= ui
                            .add(egui::DragValue::new(&mut action.pad_speed).range(0.0..=10_000.0).speed(1.0))
                            .on_hover_text("How far a stick or key held all the way turns in a second.")
                            .changed();
                    });
                }
            }
        }
    }
    if let Some(index) = remove {
        bindings.actions.remove(index);
        window.listening = None;
        changed = true;
    }
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("New action:");
        ui.add(egui::TextEdit::singleline(&mut window.new_name).desired_width(140.0));
        let name = window.new_name.trim().to_string();
        let usable = input::valid_action_name(&name) && bindings.get(&name).is_none();
        if ui
            .add_enabled(usable, egui::Button::new("Add button"))
            .on_disabled_hover_text("Letters, digits and _, starting with a letter, and not taken")
            .clicked()
        {
            bindings.actions.push(Action::button(&name, &[]));
            window.new_name.clear();
            changed = true;
        }
        if ui.add_enabled(usable, egui::Button::new("Add axis")).clicked() {
            bindings.actions.push(Action::axis(&name, &[], [&[], &[], &[], &[]]));
            window.new_name.clear();
            changed = true;
        }
    });
    ui.add_space(4.0);
    if ui
        .button("Restore the ready-made actions")
        .on_hover_text("Move, Look, Jump, Sprint and Interact with their usual keys; other actions stay")
        .clicked()
    {
        for ready in Bindings::default().actions {
            match bindings.actions.iter_mut().find(|a| a.name == ready.name) {
                Some(mine) => *mine = ready,
                None => bindings.actions.push(ready),
            }
        }
        changed = true;
    }
    changed
}
