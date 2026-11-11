//! The status bar along the editor's bottom (the editor overhaul's O1,
//! [decision 0085](../../../docs/decisions/0085-the-editor-overhaul.md)):
//! what's happening, at a glance. On the left, whether the game's code
//! was checked (or how many mistakes it has), the number of problems
//! (clicking opens Problems) and whether the area is saved; on the right,
//! whether the game is playing, and the area's name, whether it's seen in
//! 2D or 3D, and a 2D game's size in pixels. For three seconds after a
//! click on a part that isn't built yet, those last words give way to a
//! line saying so (`unbuilt::said`).

use crate::icons::Icon;
use crate::panel::Panel;
use crate::play::PlayState;
use crate::theme;
use crate::workbench::Workbench;
use egui::{Align, Color32, Layout, RichText, Sense, Ui};

/// The status bar's height, in points.
pub const HEIGHT: f32 = 24.0;

/// The small mark before a part's words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// A tick: all's well.
    Tick,
    /// A warning sign.
    Warning,
    /// The game is running.
    Playing,
    /// The game is paused.
    Paused,
}

/// One part of the status bar: its words and their colour.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    /// Its mark, if any.
    pub mark: Option<Mark>,
    /// What it says.
    pub text: String,
    /// The colour it's in.
    pub colour: Color32,
    /// More, on hover.
    pub more: Option<String>,
}

impl Part {
    fn new(text: impl Into<String>, colour: Color32) -> Self {
        Self {
            mark: None,
            text: text.into(),
            colour,
            more: None,
        }
    }

    fn marked(mut self, mark: Mark) -> Self {
        self.mark = Some(mark);
        self
    }

    fn more(mut self, more: impl Into<String>) -> Self {
        self.more = Some(more.into());
        self
    }
}

/// What the status bar says, worked out from the workbench (kept apart
/// from drawing, for the tests).
#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    /// The game's code: checked, or its mistakes. None before the first
    /// check (a project with no code is checked too).
    pub code: Option<Part>,
    /// The number of problems.
    pub problems: usize,
    /// Whether the area is saved.
    pub saved: Part,
    /// Playing or paused; none while editing.
    pub playing: Option<Part>,
    /// The area's name, 2D or 3D, and a 2D game's size.
    pub area: String,
}

impl Workbench {
    /// What the status bar says now.
    pub fn status(&self) -> Status {
        let code = self.open.code_checked.as_ref().map(|at| {
            match self.open.code_problems.len() + self.picture_problems().len() {
                0 => Part::new("Code checked", theme::text_weak()).marked(Mark::Tick),
                1 => Part::new("1 mistake in the code", theme::WARNING).marked(Mark::Warning),
                n => Part::new(format!("{n} mistakes in the code"), theme::WARNING)
                    .marked(Mark::Warning),
            }
            .more(format!(
                "The game's code was last checked at {at}, when the project opened or a code \
                 file was saved. Mistakes are listed in Problems."
            ))
        });
        let saved = if self.open.document.is_dirty() {
            Part::new("Unsaved changes", theme::text())
                .more("File > Save area (Ctrl+S) keeps them.")
        } else if self.open.document.file().is_none() {
            Part::new("Not saved yet", theme::text_weak())
                .more("This area has no file yet: File > Save area (Ctrl+S) gives it one.")
        } else {
            Part::new("Saved", theme::text_weak())
        };
        let playing = match self.play_state() {
            PlayState::Editing => None,
            PlayState::Playing => {
                Some(Part::new("Playing", theme::text_strong()).marked(Mark::Playing))
            }
            PlayState::Paused => Some(Part::new("Paused", theme::WARNING).marked(Mark::Paused)),
        };
        let area = if self.viewport.camera.kind == crate::camera::ViewKind::TwoD {
            let [w, h] = self.open.pixels.base;
            format!("{} · 2D · {w} × {h}", self.open.document.name())
        } else {
            format!("{} · 3D", self.open.document.name())
        };
        Status {
            code,
            problems: self.open.problems.list.len()
                + self.open.code_problems.len()
                + self.picture_problems().len(),
            saved,
            playing,
            area,
        }
    }

    /// Draws the status bar.
    pub(crate) fn status_bar(&mut self, ui: &mut Ui) {
        let status = self.status();
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 18.0;
            if let Some(code) = &status.code {
                part(ui, code, false);
            }
            let problems = match status.problems {
                0 => Part::new("No problems", theme::text_weak()),
                1 => Part::new("1 problem", theme::WARNING).marked(Mark::Warning),
                n => Part::new(format!("{n} problems"), theme::WARNING).marked(Mark::Warning),
            }
            .more("Opens the Problems panel");
            if part(ui, &problems, true).clicked() {
                crate::layout::bring_to_front(&mut self.dock, Panel::Problems);
            }
            part(ui, &status.saved, false);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // For three seconds after a click on a part that isn't
                // built, the right-hand words say so instead.
                let right = crate::unbuilt::said(ui.ctx()).unwrap_or_else(|| status.area.clone());
                part(ui, &Part::new(right, theme::text_weak()), false);
                if let Some(playing) = &status.playing {
                    part(ui, playing, false);
                }
            });
        });
    }
}

/// Draws one part: its mark, then its words, with more on hover.
fn part(ui: &mut Ui, part: &Part, clickable: bool) -> egui::Response {
    let sense = if clickable {
        Sense::click()
    } else {
        Sense::hover()
    };
    let shown = ui
        .scope(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            if let Some(mark) = part.mark {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::hover());
                draw_mark(ui.painter(), rect, mark, part.colour);
            }
            ui.add(
                egui::Label::new(RichText::new(&part.text).size(12.0).color(part.colour))
                    .selectable(false),
            );
        })
        .response
        .interact(sense);
    if clickable && shown.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    match &part.more {
        Some(more) => shown.on_hover_text(more),
        None => shown,
    }
}

/// A part's mark, drawn in its colour.
fn draw_mark(painter: &egui::Painter, rect: egui::Rect, mark: Mark, colour: Color32) {
    match mark {
        Mark::Tick => {
            let at = |x: f32, y: f32| rect.left_top() + egui::vec2(x, y) * rect.width() / 12.0;
            painter.line(
                vec![at(2.0, 6.5), at(5.0, 9.5), at(10.5, 3.0)],
                egui::Stroke::new(1.6, colour),
            );
        }
        Mark::Warning => crate::icons::icon(painter, rect, Icon::Warning, colour),
        Mark::Playing => crate::icons::icon(painter, rect, Icon::Play, colour),
        Mark::Paused => crate::icons::icon(painter, rect, Icon::Pause, colour),
    }
}

#[cfg(test)]
#[path = "status_bar_tests.rs"]
mod tests;
