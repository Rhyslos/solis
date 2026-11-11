//! An exported game ([decision 0093](../../../docs/decisions/0093-the-playtest-export.md)):
//! the same program, showing only the game. It fills the window, has the
//! mouse and keys from the start (Esc included), and closes when game code
//! calls `Window:quit()`. The app opens the project and area, starts play
//! once the code is ready, and makes the window follow the player's
//! settings.

use crate::LogLevel;
use crate::viewport::ViewportRenderer;
use crate::workbench::{Workbench, WorkbenchAction};
use egui::Ui;

/// What an exported game shows while it isn't playing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotPlaying {
    /// Its code is being read: it starts in a moment.
    Starting,
    /// It can't start: why, in words a player can pass on.
    Cannot(String),
}

impl Workbench {
    /// Makes this an exported game's workbench: no launcher, and only the
    /// game shows.
    pub fn set_play_only(&mut self) {
        self.play_only = true;
        self.launcher.open = false;
    }

    /// Whether this is an exported game's workbench.
    pub fn play_only(&self) -> bool {
        self.play_only
    }

    /// Whether game code asked to close the game (`Window:quit()`).
    pub fn quit_asked(&self) -> bool {
        self.open.quit_asked
    }

    /// Whether the game's code is read and ready (or there's none): an
    /// exported game starts then.
    pub fn code_ready(&self) -> bool {
        self.open.program.is_some()
    }

    /// Why an exported game can't start, if its code has mistakes: the
    /// first few, as the Problems panel says them.
    pub fn code_mistakes(&self) -> Option<String> {
        let problems = self.all_code_problems();
        let mistakes: Vec<String> = problems.iter().take(3).map(ToString::to_string).collect();
        (!mistakes.is_empty()).then(|| mistakes.join("\n"))
    }

    /// Starts an exported game: plays the open area, giving the game the
    /// mouse and keys.
    pub fn start_play_only(&mut self, ctx: &egui::Context) {
        if self.open.playing.is_some() {
            return;
        }
        self.play();
        if self.open.playing.is_some() {
            self.give_game_input(ctx);
        } else {
            self.log(LogLevel::Error, "The game couldn't start.");
        }
    }

    /// Draws an exported game filling `ui`: the game while it plays, or
    /// `waiting` before it does. Returns what the app must do (a Save game,
    /// say, or Quit once game code asked).
    pub fn show_play_only(
        &mut self,
        ui: &mut Ui,
        renderer: &mut dyn ViewportRenderer,
        waiting: &NotPlaying,
    ) -> Vec<WorkbenchAction> {
        let mut actions = std::mem::take(&mut self.open.outbox);
        let ctx = ui.ctx().clone();
        self.refresh_sheet_pictures(&ctx);
        // The window came back (after Alt+Tab, say): the pointer is held
        // again, as it was, unless a screen's buttons want it.
        let focused = ctx.input(|i| i.focused);
        let id = egui::Id::new("play only focused");
        let was = ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(focused);
        ctx.data_mut(|d| d.insert_temp(id, focused));
        if focused && !was && self.game_input && !self.open.game_screens.freed {
            ctx.send_viewport_cmd(egui::ViewportCommand::CursorGrab(egui::CursorGrab::Locked));
            ctx.send_viewport_cmd(egui::ViewportCommand::CursorVisible(false));
        }
        self.feed_input(&ctx);
        self.run_game(ui.input(|i| i.stable_dt));
        self.refresh_screen_pictures();
        let rect = ui.max_rect();
        ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);
        let game = self.game_scene_now().map(|(key, mut scene)| {
            scene.play_only = true;
            (key, scene)
        });
        let mut game_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        let out = match &game {
            Some((_, scene)) => crate::game_panel::game_panel(
                &mut game_ui,
                Some(scene),
                true,
                self.game_input,
                &self.viewport.camera,
                self.open.pixels,
                &self.viewport.sheets,
                renderer,
                self.open.game_screens.pictures.clone(),
            ),
            None => {
                let text = match (self.open.quit_asked, waiting) {
                    (true, _) => "Closing…".to_owned(),
                    (false, NotPlaying::Starting) => "Starting…".to_owned(),
                    (false, NotPlaying::Cannot(why)) => why.clone(),
                };
                game_ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new(text)
                            .color(crate::tokens::text_body())
                            .size(16.0),
                    );
                });
                crate::game_panel::GamePanelOut::default()
            }
        };
        ui.advance_cursor_after_rect(rect);
        self.game_panel_done(&ctx, out);
        self.open.game_scene = game;
        actions.append(&mut self.open.outbox);
        // Only what an exported game does: saving and loading games, and
        // closing. Nothing of the editor's.
        actions.retain(|a| {
            matches!(
                a,
                WorkbenchAction::SaveGame { .. } | WorkbenchAction::LoadGame(_)
            )
        });
        if self.open.quit_asked {
            actions.push(WorkbenchAction::Quit);
        }
        if self.open.playing.is_some() {
            ctx.request_repaint();
        }
        actions
    }
}

#[cfg(test)]
#[path = "play_only_tests.rs"]
mod tests;
