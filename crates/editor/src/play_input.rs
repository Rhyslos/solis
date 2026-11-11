//! The game's input in the editor ([decision 0037](../../../docs/decisions/0037-input-as-named-actions.md)):
//! clicking the Game panel gives the game the mouse and keys, Esc gives
//! them back; what they do meanwhile feeds `input::Devices`, sampled once
//! per tick by `run_game`. Also recording a play's input and playing it
//! back (Play > Play and record inputs, Replay a recording…).

use crate::console::LogLevel;
use crate::workbench::{Workbench, WorkbenchAction};
use std::sync::Arc;

/// An egui key's name as bindings use it (egui's own names: `W`,
/// `Space`, `Up`).
pub(crate) fn key_name(key: egui::Key) -> &'static str {
    key.name()
}

/// A mouse button's name as bindings use it.
pub(crate) fn mouse_name(button: egui::PointerButton) -> &'static str {
    match button {
        egui::PointerButton::Primary => "Mouse Left",
        egui::PointerButton::Secondary => "Mouse Right",
        egui::PointerButton::Middle => "Mouse Middle",
        egui::PointerButton::Extra1 => "Mouse Back",
        egui::PointerButton::Extra2 => "Mouse Forward",
    }
}

impl Workbench {
    /// The project's input actions changed (it opened, or its
    /// `input.solinput` was read): the next Play uses them.
    pub fn set_bindings(&mut self, bindings: input::Bindings) {
        self.open.bindings = bindings;
    }

    /// The project's input actions.
    pub fn bindings(&self) -> &input::Bindings {
        &self.open.bindings
    }

    /// The gamepad's state this frame (none: not connected), from the
    /// app. It reaches the game while playing, whether or not the game
    /// has the mouse and keys: a gamepad has nothing of the editor's.
    pub fn set_gamepad(&mut self, pad: Option<input::Pad>) {
        self.pad_now.clone_from(&pad);
        if self.open.playing.is_some() {
            self.devices.set_pad(pad);
        } else {
            self.devices.set_pad(None);
        }
    }

    /// Whether the game has the mouse and keys: the editor's own keys
    /// are off meanwhile.
    pub fn game_has_input(&self) -> bool {
        self.game_input
    }

    /// Gives the game the mouse and keys (the Game panel was clicked
    /// while playing): the pointer is hidden and held in the window.
    pub(crate) fn give_game_input(&mut self, ctx: &egui::Context) {
        if self.open.playing.is_none() || self.game_input {
            return;
        }
        self.game_input = true;
        // A screen with buttons keeps the pointer free (free_pointer).
        self.open.game_screens.freed = self.open.game_screens.wants_pointer();
        if !self.open.game_screens.freed {
            ctx.send_viewport_cmd(egui::ViewportCommand::CursorGrab(egui::CursorGrab::Locked));
            ctx.send_viewport_cmd(egui::ViewportCommand::CursorVisible(false));
        }
        if !self.play_only {
            self.log(
                LogLevel::Info,
                "The game has the mouse and keys; press Esc to give them back to the editor.",
            );
        }
    }

    /// What the Game panel saw this frame: the pointer over the game's
    /// screens' buttons, and a click, which presses the button under it or
    /// else gives the game the mouse and keys (decision 0088). And the
    /// pointer freed or held as screens with buttons come and go.
    pub(crate) fn game_panel_done(
        &mut self,
        ctx: &egui::Context,
        out: crate::game_panel::GamePanelOut,
    ) {
        if self.open.playing.is_none() {
            self.open.game_screens.hover = None;
            return;
        }
        let wants = self.open.game_screens.wants_pointer();
        // The pointer is only over buttons while it can be seen.
        let pointer = out
            .pointer
            .filter(|_| wants && (!self.game_input || self.open.game_screens.freed));
        self.open.game_screens.point_at(pointer, out.area);
        if out.clicked {
            if self.open.game_screens.hover.is_some() {
                self.press_hovered();
            } else {
                self.give_game_input(ctx);
            }
        }
        if self.game_input && wants != self.open.game_screens.freed {
            self.open.game_screens.freed = wants;
            let (grab, visible) = if wants {
                (egui::CursorGrab::None, true)
            } else {
                (egui::CursorGrab::Locked, false)
            };
            ctx.send_viewport_cmd(egui::ViewportCommand::CursorGrab(grab));
            ctx.send_viewport_cmd(egui::ViewportCommand::CursorVisible(visible));
        }
    }

    /// Takes the mouse and keys back from the game: everything it held is
    /// let go. The pointer comes back the next frame.
    pub(crate) fn give_back_input(&mut self) {
        if self.game_input {
            self.game_input = false;
            self.release_pointer = true;
        }
        self.open.game_screens.freed = false;
        self.devices.release_all();
    }

    /// Feeds this frame's keys, mouse buttons and mouse movement to the
    /// game while it has them, and takes them back on Esc or when the
    /// window loses focus. Called at the start of each frame.
    pub(crate) fn feed_input(&mut self, ctx: &egui::Context) {
        if std::mem::take(&mut self.release_pointer) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CursorGrab(egui::CursorGrab::None));
            ctx.send_viewport_cmd(egui::ViewportCommand::CursorVisible(true));
        }
        if !self.game_input {
            return;
        }
        if self.open.playing.is_none() {
            self.give_back_input();
            return;
        }
        let (events, modifiers, focused) =
            ctx.input(|i| (i.events.clone(), i.modifiers, i.focused));
        // An exported game keeps the keys when its window loses focus: it
        // has nothing else to give them to.
        let mut give_back = !focused && !self.play_only;
        let mut alt_enter = false;
        for event in events {
            match event {
                // In an exported game, Esc is the game's.
                egui::Event::Key {
                    key: egui::Key::Escape,
                    pressed: true,
                    ..
                } if !self.play_only => give_back = true,
                // Alt+Enter switches the player's fullscreen setting
                // (decision 0091), as players expect of any game; the
                // game doesn't see it.
                egui::Event::Key {
                    key: egui::Key::Enter,
                    pressed: true,
                    repeat,
                    modifiers,
                    ..
                } if modifiers.alt => {
                    alt_enter |= !repeat;
                }
                egui::Event::Key {
                    key,
                    pressed,
                    repeat: false,
                    ..
                } => self.devices.set(key_name(key), pressed),
                // A click on a screen's button is the screen's, not the
                // game's; while the pointer is free for buttons, moving it
                // doesn't turn the game's camera.
                egui::Event::PointerButton {
                    button, pressed, ..
                } if !(pressed && self.open.game_screens.hover.is_some()) => {
                    self.devices.set(mouse_name(button), pressed);
                }
                egui::Event::MouseMoved(delta) if !self.open.game_screens.freed => {
                    self.devices.mouse_moved(glam::Vec2::new(delta.x, delta.y));
                }
                egui::Event::MouseWheel { unit, delta, .. } => {
                    self.devices.wheel_turned(match unit {
                        egui::MouseWheelUnit::Line => delta.y,
                        egui::MouseWheelUnit::Point => delta.y / crate::viewport::POINTS_PER_NOTCH,
                        egui::MouseWheelUnit::Page => delta.y * 3.0,
                    });
                }
                _ => {}
            }
        }
        self.devices.set("Shift", modifiers.shift);
        self.devices.set("Ctrl", modifiers.ctrl);
        self.devices.set("Alt", modifiers.alt);
        if alt_enter {
            self.switch_fullscreen();
        }
        if give_back {
            // The Esc that gave the editor its keys back isn't also the
            // editor's (it would clear the selection).
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            self.give_back_input();
            if !self.play_only {
                self.log(LogLevel::Info, "The editor has the mouse and keys again.");
            }
        }
    }

    /// Alt+Enter while the game has the keys: the player's fullscreen
    /// setting switches, as `Window:setFullscreen` would. While playing in
    /// the editor the game stays in the Game panel, so it's said.
    pub(crate) fn switch_fullscreen(&mut self) {
        let on = !self.player_settings.fullscreen;
        self.player_settings = self
            .player_settings
            .with(script::settings::Setting::Fullscreen(on));
        if let Some(playing) = &mut self.open.playing {
            playing.view.settings = self.player_settings;
        }
        if self.play_only {
            return;
        }
        self.log(
            LogLevel::Info,
            format!(
                "Alt+Enter: the exported game would {} (Window:fullscreen() is {on} now). While \
                 playing in the editor, the game stays in the Game panel.",
                if on {
                    "fill the screen"
                } else {
                    "sit in a window"
                }
            ),
        );
    }

    /// Plays, recording each tick's input and checksums; Stop saves the
    /// recording in the project's `recordings` folder (Play > Play and
    /// record inputs).
    pub fn play_and_record(&mut self) {
        if self.open.project.is_none() {
            self.log(
                LogLevel::Warning,
                "Recordings are kept in a project: open or make one first (File > New project…).",
            );
            return;
        }
        if self.open.playing.is_some() {
            self.log(
                LogLevel::Info,
                "A recording starts from Play: press Stop first, then Play and record inputs.",
            );
            return;
        }
        // The area it's recorded in, inside the project's assets, so the
        // project's check can play it back on its own (Phase 8, C4).
        let area = self.recorded_area();
        // Played back on its own (a project's check), the area is read as
        // saved: unsaved changes would make it play back differently.
        if area.is_some() && self.open.document.is_dirty() {
            self.log(
                LogLevel::Warning,
                "The area has unsaved changes: save it before recording, or this recording \
                 won't play back the same in a project's check (which reads the saved area).",
            );
        }
        self.play();
        if let Some(playing) = &mut self.open.playing {
            playing.recording = Some(input::Recording {
                seed: playing.start_seed,
                start: playing.world.checksum(),
                ticks: Vec::new(),
                area,
            });
            self.log(
                LogLevel::Info,
                "Recording the game's input from now; Stop saves it.",
            );
        }
    }

    /// The open area's file inside the project's assets, with `/` between
    /// folders, when it's saved there (unsaved changes aside).
    fn recorded_area(&self) -> Option<String> {
        let assets = &self.open.project.as_ref()?.assets;
        let file = self.open.document.file()?;
        // The same folder may be spelt differently (letter case on Windows,
        // a link): compared as the system resolves them, when it can.
        let canonical = |p: &std::path::Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
        let (file, assets) = (canonical(file), canonical(assets));
        let inside = file.strip_prefix(&assets).ok()?;
        let parts: Vec<String> = inside
            .components()
            .map(|c| match c {
                std::path::Component::Normal(part) => part.to_str().map(str::to_owned),
                _ => None,
            })
            .collect::<Option<_>>()?;
        Some(parts.join("/"))
    }

    /// Plays a recording back (Play > Replay a recording…, the app hands
    /// over its text): from Play, with the recorded seed and each tick's
    /// recorded input, checking each tick's checksums; the game pauses
    /// and says whether it matched, or where it stopped matching.
    pub fn replay(&mut self, text: &str, name: &str) {
        let recording = match input::Recording::read(text) {
            Ok(r) => r,
            Err(why) => {
                self.log(
                    LogLevel::Error,
                    format!("Couldn't play back {name}: {why}."),
                );
                return;
            }
        };
        if recording.ticks.is_empty() {
            self.log(
                LogLevel::Warning,
                format!("{name} has no ticks to play back."),
            );
            return;
        }
        if self.open.playing.is_some() {
            self.stop();
        }
        let kept = self.open.fixed_seed.replace(recording.seed);
        self.play();
        self.open.fixed_seed = kept;
        let Some(playing) = &mut self.open.playing else {
            return;
        };
        let same_start = playing.world.checksum() == recording.start;
        let count = recording.ticks.len();
        playing.replay = Some((recording, 0));
        self.log(
            LogLevel::Info,
            format!(
                "Playing back {name}: {count} ticks ({:.1} s) of recorded input.",
                count as f64 * world::TICK
            ),
        );
        if !same_start {
            self.log(
                LogLevel::Warning,
                "The area isn't as it was when this was recorded, so it may not play back the \
                 same.",
            );
        }
    }

    /// What the editor asks of the app, taken (for tools that run the game
    /// without a window, which answer saves and loads themselves).
    pub fn take_outbox(&mut self) -> Vec<WorkbenchAction> {
        std::mem::take(&mut self.open.outbox)
    }

    /// The recording being made, as its file's text (for tools and tests
    /// that run the game without a window; Stop saves it as usual).
    pub fn recording_text(&self) -> Option<String> {
        let recording = self.open.playing.as_ref()?.recording.as_ref()?;
        Some(recording.write())
    }

    /// How the last playback in this play ended: `Ok` with the ticks when
    /// it matched the recording on every one, `Err` with the tick it
    /// stopped matching by; `None` while it plays or with none.
    pub fn replayed(&self) -> Option<Result<usize, usize>> {
        self.open.playing.as_ref()?.replayed
    }

    /// The bindings a running game uses now (with game code's rebinding).
    pub fn playing_bindings(&self) -> Option<Arc<input::Bindings>> {
        self.open.playing.as_ref().map(|p| Arc::clone(&p.bindings))
    }
}

#[cfg(test)]
#[path = "play_input_tests.rs"]
mod tests;
