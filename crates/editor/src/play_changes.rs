//! Playing is unmistakable, and nothing is lost (the editor overhaul's O5,
//! decision 0085). While the game runs, the window has a coloured edge
//! and the top and status bars take its colour: blue while playing, amber
//! while paused, so editing and playing can't be mixed up. Changes made to
//! the area while playing aren't thrown away silently on Stop, as other
//! engines do: Stop lists them and asks **Keep** (the area as you left it,
//! with every change still undoable) or **Undo all** (the area as it was
//! before Play).

use crate::document::Document;
use crate::play::PlayState;
use crate::theme;
use crate::workbench::Workbench;
use egui::{Color32, RichText, Ui};

/// The most changes listed by name; the rest are counted.
const LISTED: usize = 8;

/// Changes made while playing, waiting for Keep or Undo all.
#[derive(Clone)]
pub(crate) struct PlayChanges {
    /// The area as it was at Stop, with its history.
    pub(crate) document: Box<Document>,
    /// What was done, oldest first.
    pub(crate) labels: Vec<String>,
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

    /// Notes what was changed while playing, at Stop: `edited` is the
    /// area as it is now, `before` its state when Play was pressed. Returns
    /// whether there was anything (then the question is asked).
    pub(crate) fn note_play_changes(&mut self, edited: &Document, before: u64) -> bool {
        let history = edited.history();
        let mut labels = history.labels_since(before);
        if labels.is_empty() && history.state() != before {
            labels.push("Undid a change made before Play".into());
        }
        if labels.is_empty() {
            return false;
        }
        self.play_changes = Some(PlayChanges {
            document: Box::new(edited.clone()),
            labels,
        });
        true
    }

    /// The changes made while playing, waiting for an answer.
    pub fn play_changes(&self) -> Option<&[String]> {
        self.play_changes.as_ref().map(|c| c.labels.as_slice())
    }

    /// Keep: the area as it was at Stop, every change still undoable.
    pub fn keep_play_changes(&mut self) {
        if let Some(changes) = self.play_changes.take() {
            self.replace_document(*changes.document);
            self.document.touch();
            self.log(
                crate::console::LogLevel::Info,
                format!(
                    "Kept {} made while playing.",
                    count(changes.labels.len(), "change")
                ),
            );
        }
    }

    /// Undo all: the area stays as it was before Play.
    pub fn undo_play_changes(&mut self) {
        if let Some(changes) = self.play_changes.take() {
            self.log(
                crate::console::LogLevel::Info,
                format!(
                    "Undid {} made while playing.",
                    count(changes.labels.len(), "change")
                ),
            );
        }
    }

    /// The question after Stop, while there are changes to keep or undo.
    /// It waits for an answer: nothing is thrown away by accident.
    pub(crate) fn play_changes_window(&mut self, ui: &Ui) {
        let Some(changes) = &self.play_changes else {
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
                    .color(theme::TEXT_STRONG),
            );
            ui.add_space(4.0);
            ui.label(format!(
                "You made {} to the area while the game ran:",
                count(labels.len(), "change")
            ));
            ui.add_space(4.0);
            for label in labels.iter().take(LISTED) {
                ui.label(RichText::new(format!("•  {label}")).color(theme::TEXT));
            }
            if labels.len() > LISTED {
                ui.label(
                    RichText::new(format!("and {} more", labels.len() - LISTED))
                        .color(theme::TEXT_WEAK),
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
                        "The area as you left it, every change still undoable with Ctrl + Z",
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
