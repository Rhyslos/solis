//! Leaving the open level: the one check before it's left
//! (`Workbench::request`), quitting and closing, and the questions about
//! unsaved changes and files changed on disk.

use super::{Workbench, WorkbenchAction};
use egui::Ui;

impl Workbench {
    /// Asks for `action`. This is the one check before the open level is
    /// left ([`leaves_the_level`]: another level or project in its place, or
    /// the editor closed): first Keep or Undo all, if edits made while
    /// playing wait for it, then the question about unsaved changes. The
    /// action waits for each answer, so nothing is lost without a word.
    pub(crate) fn request(&mut self, action: WorkbenchAction, actions: &mut Vec<WorkbenchAction>) {
        // On a module's page, Save saves the module; anything that leaves
        // the level brings the level's tab to the front first.
        if self.page_in_front() {
            if matches!(
                action,
                WorkbenchAction::SaveLevel | WorkbenchAction::SaveLevelAs
            ) {
                self.save_module_page();
                return;
            }
            if leaves_the_level(&action) {
                self.level_to_front();
            }
        }
        // A table's save still being written is finished first.
        if leaves_the_project(&action) || action == WorkbenchAction::Quit {
            self.poll_table_saves(true);
        }
        if leaves_the_project(&action) && self.module_page_dirty() {
            // Its changes are asked about first: Save, Don't save or
            // Cancel, then this is asked for again.
            if let Some(page) = &mut self.open.module_page {
                page.asking_to_close = true;
            }
            return;
        }
        if self.open.playing.is_some() && crate::play::waits_for_stop(&action) {
            // Refused, with a word in the Console, at the end of the frame.
            actions.push(action);
            return;
        }
        if leaves_the_level(&action) {
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
    /// for Keep or Undo all, or the level has unsaved changes, asks first
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
        if self.module_page_dirty() && !self.may_close {
            if let Some(page) = &mut self.open.module_page {
                page.asking_to_close = true;
            }
            return false;
        }
        // While playing, the level that matters is the one from before Play.
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
                // Saving or leaving waits for Stop, which puts the level
                // back as it was before Play.
                if self.open.playing.is_some() {
                    self.stop();
                }
                if let Some(changes) = &mut self.open.play_changes {
                    // Edits made while this question was up (the game
                    // played on): Keep or Undo all comes first, and this
                    // is asked again after, about the level as it is then.
                    changes.then = Some(waiting);
                } else {
                    if save {
                        actions.push(WorkbenchAction::SaveLevel);
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
                && let Some(level) = self.open.pending_reload.take()
                && load
            {
                self.reload_level(level);
            }
        }
    }
}

/// Whether `action` leaves the open level: puts another level or project
/// in its place, or closes the editor. [`Workbench::request`] asks about
/// what would be lost before any of these goes ahead.
/// Whether `action` closes the project (and with it a module's page).
fn leaves_the_project(action: &WorkbenchAction) -> bool {
    matches!(
        action,
        WorkbenchAction::OpenProject
            | WorkbenchAction::CreateProject(_)
            | WorkbenchAction::CloseProject
            | WorkbenchAction::OpenRecent(_)
            | WorkbenchAction::MakeSampleGame
            | WorkbenchAction::MakeUiSample
            | WorkbenchAction::MakeBenchmark
            | WorkbenchAction::MakePlayground
            | WorkbenchAction::Quit
    )
}

pub(crate) fn leaves_the_level(action: &WorkbenchAction) -> bool {
    matches!(
        action,
        WorkbenchAction::NewLevel
            | WorkbenchAction::OpenLevel
            | WorkbenchAction::OpenLevelFile(_)
            | WorkbenchAction::OpenProject
            | WorkbenchAction::CreateProject(_)
            | WorkbenchAction::CloseProject
            | WorkbenchAction::OpenRecent(_)
            | WorkbenchAction::MakeSampleGame
            | WorkbenchAction::MakeUiSample
            | WorkbenchAction::MakeBenchmark
            | WorkbenchAction::MakePlayground
            | WorkbenchAction::Quit
    )
}
