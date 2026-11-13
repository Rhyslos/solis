//! Leaving the open area: the one check before it's left
//! (`Workbench::request`), quitting and closing, and the questions about
//! unsaved changes and files changed on disk.

use super::{Workbench, WorkbenchAction};
use egui::Ui;

impl Workbench {
    /// Asks for `action`. This is the one check before the open area is
    /// left ([`leaves_the_area`]: another area or project in its place, or
    /// the editor closed): first Keep or Undo all, if edits made while
    /// playing wait for it, then the question about unsaved changes. The
    /// action waits for each answer, so nothing is lost without a word.
    pub(crate) fn request(&mut self, action: WorkbenchAction, actions: &mut Vec<WorkbenchAction>) {
        if self.open.playing.is_some() && crate::play::waits_for_stop(&action) {
            // Refused, with a word in the Console, at the end of the frame.
            actions.push(action);
            return;
        }
        if leaves_the_area(&action) {
            // Only quitting gets here while playing. The game stops
            // first, which asks about the edits made meanwhile.
            if self.open.playing.is_some() {
                self.stop();
            }
            if let Some(changes) = &mut self.open.play_changes {
                // Asked for again once Keep or Undo all is chosen.
                changes.then = Some(action);
                return;
            }
            if self.open.document.is_dirty() {
                self.open.unsaved_prompt = Some(action);
                return;
            }
        }
        if action == WorkbenchAction::Quit {
            self.may_close = true;
        }
        actions.push(action);
    }

    /// The app called off a queued quit (the save before it was cancelled
    /// or failed): a later close asks about unsaved changes again.
    pub fn quit_cancelled(&mut self) {
        self.may_close = false;
    }

    /// Whether the window may close now. If edits made while playing wait
    /// for Keep or Undo all, or the area has unsaved changes, asks first
    /// (and the app should cancel this close).
    pub fn close_requested(&mut self) -> bool {
        // Edits made while playing are asked about at Stop, so the game
        // stops for the question, and the close waits for its answer.
        if self.edited_while_playing() {
            self.stop();
        }
        if let Some(changes) = &mut self.open.play_changes {
            changes.then = Some(WorkbenchAction::Quit);
            return false;
        }
        // While playing, the area that matters is the one from before Play.
        let dirty = match &self.open.playing {
            Some(playing) => playing.before.is_dirty(),
            None => self.open.document.is_dirty(),
        };
        if self.may_close || !dirty {
            if self.open.playing.is_some() {
                self.stop();
            }
            return true;
        }
        // Asked first: Cancel leaves the game running.
        self.open.unsaved_prompt = Some(WorkbenchAction::Quit);
        false
    }

    /// The questions about unsaved changes and files changed on disk.
    pub(super) fn prompts(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        if let Some(waiting) = self.open.unsaved_prompt.clone() {
            let name = self.open.document.name();
            use crate::widgets::dialog::{Answer, dialog};
            let words = format!("{name} has changes that aren't saved.");
            let answers = [
                Answer::left("Don't save"),
                Answer::right("Cancel"),
                Answer::main("Save"),
            ];
            let picked = dialog(
                ui.ctx(),
                "unsaved changes",
                "Unsaved changes",
                |ui| drop(ui.label(words)),
                &answers,
            );
            let answer = match picked {
                Some(0) => Some(false),
                Some(2) => Some(true),
                Some(_) => {
                    self.open.unsaved_prompt = None;
                    // A cancelled restart is no restart.
                    self.restarting = false;
                    None
                }
                None => None,
            };
            if let Some(save) = answer {
                self.open.unsaved_prompt = None;
                // Saving or leaving waits for Stop, which puts the area
                // back as it was before Play.
                if self.open.playing.is_some() {
                    self.stop();
                }
                if let Some(changes) = &mut self.open.play_changes {
                    // Edits made while this question was up (the game
                    // played on): Keep or Undo all comes first, and this
                    // is asked again after, about the area as it is then.
                    changes.then = Some(waiting);
                } else {
                    if save {
                        actions.push(WorkbenchAction::SaveArea);
                    }
                    if waiting == WorkbenchAction::Quit {
                        self.may_close = true;
                    }
                    actions.push(waiting);
                }
            }
        }
        // Asked after the game stops: loading it now would only be undone.
        if self.open.pending_reload.is_some() && self.open.playing.is_none() {
            let name = self.open.document.name();
            use crate::widgets::dialog::{Answer, dialog};
            let words = format!(
                "{name}'s file changed on disk (perhaps a Git pull), and you have changes that \
                 aren't saved."
            );
            let answers = [
                Answer::right("Keep mine"),
                Answer::main("Load the file").hint("Ctrl+Z brings yours back"),
            ];
            let answer = dialog(
                ui.ctx(),
                "changed on disk",
                "Changed on disk",
                |ui| drop(ui.label(words)),
                &answers,
            )
            .map(|index| index == 1);
            if let Some(load) = answer
                && let Some(area) = self.open.pending_reload.take()
                && load
            {
                self.reload_area(area);
            }
        }
    }
}

/// Whether `action` leaves the open area: puts another area or project
/// in its place, or closes the editor. [`Workbench::request`] asks about
/// what would be lost before any of these goes ahead.
pub(crate) fn leaves_the_area(action: &WorkbenchAction) -> bool {
    matches!(
        action,
        WorkbenchAction::NewArea
            | WorkbenchAction::OpenArea
            | WorkbenchAction::OpenAreaFile(_)
            | WorkbenchAction::OpenProject
            | WorkbenchAction::NewProject
            | WorkbenchAction::OpenRecent(_)
            | WorkbenchAction::MakeSampleGame
            | WorkbenchAction::MakeUiSample
            | WorkbenchAction::MakeBenchmark
            | WorkbenchAction::MakePlayground
            | WorkbenchAction::Quit
    )
}
