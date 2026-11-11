//! Playing is unmistakable, and nothing is lost (the editor overhaul's O5,
//! decision 0085). While the game runs, the window has a coloured edge
//! and the top and status bars take its colour: blue while playing, amber
//! while paused, so editing and playing can't be mixed up. Changes made to
//! the area while playing aren't thrown away silently on Stop, as other
//! engines do: Stop lists them and asks **Keep** (the area as it was
//! before Play with those changes made, every one still undoable) or
//! **Undo all** (the area as it was before Play). Only the user's own
//! edits are kept, never what the game did: their edits are undo steps
//! and the game's aren't (`Document::edit_by_game`), so Stop makes the
//! steps again on the area from before Play (`Document::follow`).
//!
//! Nothing leaves the area while the question waits: quitting, closing
//! the window, another area or project (`Workbench::request`) and a file
//! changed on disk wait for the answer and go on after it, and the
//! question is asked over the launcher too.

use crate::document::Document;
use crate::play::PlayState;
use crate::theme;
use crate::workbench::{Workbench, WorkbenchAction};
use area::text::AreaFile;
use egui::{Color32, RichText, Ui};
use reflect::AssetId;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The most changes listed by name; the rest are counted.
const LISTED: usize = 8;

/// The most changes that went with the game named in the Console.
const NAMED: usize = 3;

/// Changes made while playing, waiting for Keep or Undo all.
#[derive(Clone)]
pub(crate) struct PlayChanges {
    /// The area as it was before Play with the user's changes made, and
    /// its history with a step for each.
    pub(crate) document: Box<Document>,
    /// The assemblies as they were at Play: what that area's copies were
    /// built from, so Keep can rebuild the ones whose files changed since.
    assemblies: BTreeMap<AssetId, Arc<AreaFile>>,
    /// What was done, oldest first.
    pub(crate) labels: Vec<String>,
    /// The area as its file holds it now, if the file changed on disk
    /// while playing or since: offered once the question is answered,
    /// since Keep puts another area in the open one's place.
    pub(crate) reload: Option<area::Area>,
    /// What waits for the answer: something that leaves the area (a quit,
    /// another area or project), asked for again once it's given.
    pub(crate) then: Option<WorkbenchAction>,
}

/// The colour while playing: the Play button's blue, lighter.
pub const PLAYING: Color32 = Color32::from_rgb(0x2f, 0x8f, 0xd8);

impl Workbench {
    /// The colour the editor takes while the game runs: blue playing,
    /// amber paused; none while editing.
    pub fn play_tint(&self) -> Option<Color32> {
        match self.play_state() {
            PlayState::Editing => None,
            PlayState::Playing => Some(PLAYING),
            PlayState::Paused => Some(theme::WARNING),
        }
    }

    /// A bar's fill: `base`, tinted while the game runs.
    pub(crate) fn tinted(&self, base: Color32) -> Color32 {
        match self.play_tint() {
            Some(tint) => blend(base, tint, 0.22),
            None => base,
        }
    }

    /// The coloured edge round the window while the game runs.
    pub(crate) fn play_edge(&self, ctx: &egui::Context) {
        if let Some(tint) = self.play_tint() {
            let rect = ctx.content_rect().shrink(1.0);
            ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("play edge"),
            ))
            .rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(2.0, tint),
                egui::StrokeKind::Inside,
            );
        }
    }

    /// Notes what the user changed while playing, at Stop: `before` is
    /// the document as it was when Play was pressed (and `assemblies` the
    /// assemblies then), and the open one is as the user and the game
    /// left it. Returns whether there was anything of the user's (then
    /// the question is asked). A change to something only the game had
    /// (an entity it spawned) can't be kept, and the Console says which.
    pub(crate) fn note_play_changes(
        &mut self,
        before: &Document,
        assemblies: &BTreeMap<AssetId, Arc<AreaFile>>,
    ) -> bool {
        let start = before.history().state();
        if self.open.document.history().state() == start {
            return false;
        }
        let mut kept = before.clone();
        let followed = kept.follow(&self.open.document);
        if !followed.left_out.is_empty() {
            let mut named = followed
                .left_out
                .iter()
                .take(NAMED)
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            if followed.left_out.len() > NAMED {
                named.push_str(&format!(" and {} more", followed.left_out.len() - NAMED));
            }
            let (was, it) = if followed.left_out.len() == 1 {
                ("was", "it")
            } else {
                ("were", "they")
            };
            self.log(
                crate::console::LogLevel::Info,
                format!(
                    "{} made while playing {was} to things the game made, which went with the \
                     game, so {it} can't be kept: {named}.",
                    count(followed.left_out.len(), "change"),
                ),
            );
        }
        let mut labels = followed.made;
        if labels.is_empty() && kept.history().state() != start {
            labels.push("Undid a change made before Play".into());
        }
        if labels.is_empty() {
            return false;
        }
        self.open.play_changes = Some(PlayChanges {
            document: Box::new(kept),
            assemblies: assemblies.clone(),
            labels,
            reload: None,
            then: None,
        });
        true
    }

    /// The changes made while playing, waiting for an answer.
    pub fn play_changes(&self) -> Option<&[String]> {
        self.open.play_changes.as_ref().map(|c| c.labels.as_slice())
    }

    /// Keep: the area as it was before Play with the user's changes made,
    /// every one still undoable. Nothing the game did comes with them.
    pub fn keep_play_changes(&mut self) {
        if let Some(changes) = self.open.play_changes.take() {
            // Its copies of assemblies are as they were at Play; the open
            // area's were rebuilt at Stop (or since) if their files
            // changed, and these are now.
            let now = std::mem::replace(&mut self.open.assemblies, changes.assemblies);
            self.replace_document(*changes.document);
            self.open.document.touch();
            self.update_copies(now);
            self.log(
                crate::console::LogLevel::Info,
                format!(
                    "Kept {} made while playing.",
                    count(changes.labels.len(), "change")
                ),
            );
            self.answered(changes.reload, changes.then);
        }
    }

    /// Undo all: the area stays as it was before Play.
    pub fn undo_play_changes(&mut self) {
        if let Some(changes) = self.open.play_changes.take() {
            self.log(
                crate::console::LogLevel::Info,
                format!(
                    "Undid {} made while playing.",
                    count(changes.labels.len(), "change")
                ),
            );
            self.answered(changes.reload, changes.then);
        }
    }

    /// Keep or Undo all was chosen: what waited for it goes on. A file
    /// changed on disk is offered (`reload`), and what was leaving the
    /// area (`then`) is asked for again, so it asks about unsaved changes
    /// as it would have; the app gets it with the next actions.
    fn answered(&mut self, reload: Option<area::Area>, then: Option<WorkbenchAction>) {
        if let Some(area) = reload {
            self.offer_reload(area);
        }
        if let Some(action) = then {
            let mut actions = Vec::new();
            self.request(action, &mut actions);
            self.open.outbox.append(&mut actions);
        }
    }

    /// The question after Stop, while there are changes to keep or undo.
    /// It waits for an answer: nothing is thrown away by accident.
    pub(crate) fn play_changes_window(&mut self, ui: &Ui) {
        let Some(changes) = &self.open.play_changes else {
            return;
        };
        let labels = changes.labels.clone();
        let mut answer = None;
        egui::Modal::new(egui::Id::new("play changes")).show(ui.ctx(), |ui| {
            ui.set_width(380.0);
            ui.label(
                RichText::new("Keep the changes made while playing?")
                    .strong()
                    .size(15.0)
                    .color(theme::text_strong()),
            );
            ui.add_space(4.0);
            ui.label(format!(
                "You made {} to the area while the game ran:",
                count(labels.len(), "change")
            ));
            ui.add_space(4.0);
            for label in labels.iter().take(LISTED) {
                ui.label(RichText::new(format!("•  {label}")).color(theme::text()));
            }
            if labels.len() > LISTED {
                ui.label(
                    RichText::new(format!("and {} more", labels.len() - LISTED))
                        .color(theme::text_weak()),
                );
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let colors = self.accent.colors();
                let keep = ui
                    .add(
                        egui::Button::new(RichText::new("Keep").color(Color32::WHITE).strong())
                            .fill(colors.fill)
                            .min_size(egui::vec2(90.0, 26.0)),
                    )
                    .on_hover_text(
                        "Your changes, each still undoable with Ctrl + Z. What the game did isn't \
                         kept",
                    );
                if keep.clicked() {
                    answer = Some(true);
                }
                let undo = ui
                    .add(egui::Button::new("Undo all").min_size(egui::vec2(90.0, 26.0)))
                    .on_hover_text("The area as it was before Play");
                if undo.clicked() {
                    answer = Some(false);
                }
            });
        });
        match answer {
            Some(true) => self.keep_play_changes(),
            Some(false) => self.undo_play_changes(),
            None => {}
        }
    }
}

/// `n` things, in words: "1 change", "3 changes".
fn count(n: usize, thing: &str) -> String {
    if n == 1 {
        format!("1 {thing}")
    } else {
        format!("{n} {thing}s")
    }
}

/// `a` moved `t` of the way to `b`.
fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    let mix = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}

#[cfg(test)]
#[path = "play_changes_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "play_changes_leaving_tests.rs"]
mod leaving_tests;
