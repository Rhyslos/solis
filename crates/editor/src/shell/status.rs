//! The status bar along the window's bottom (spec 160-164; the frame of
//! decision 0099). On the left: a dot and "Ready" while the game's code
//! is clean (red, with the count, while it has mistakes), the branch
//! (not built yet), and the problems as coloured dots with counts, which
//! open the Problems tab when clicked. On the right: the context (playing
//! or paused, saved or not, the area's name with 2D or 3D and a 2D game's
//! size), then the version in mono, and Restart to update while a newer
//! build waits. For three seconds after a click on an unfinished part,
//! the context gives way to a line saying so (`unbuilt::said`).
//!
//! What it shows is read from counts kept by the code check and by
//! Problems (each worked out once per change to what it looks at), never
//! by walking the area here.

use crate::icons::Icon;
use crate::panel::Panel;
use crate::play::PlayState;
use crate::tokens::{self, size, text};
use crate::unbuilt::{self, Entry, UnbuiltId};
use crate::workbench::{Workbench, WorkbenchAction};
use egui::{Align, Color32, Layout, Sense, Ui, WidgetInfo, vec2};

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
    /// Of them, the mistakes in the game's code and pictures (red).
    pub code_mistakes: usize,
    /// Whether the area is saved.
    pub saved: Part,
    /// Playing or paused; none while editing.
    pub playing: Option<Part>,
    /// The area's name, 2D or 3D, and a 2D game's size.
    pub area: String,
    /// What moves snap to and which axes the gizmo follows: "Snap 0.25 ·
    /// World" (spec 233).
    pub snap: String,
}

impl Workbench {
    /// What the status bar says now.
    pub fn status(&self) -> Status {
        let mistakes = self.open.code_problems.len() + self.picture_problems().len();
        let code = self.open.code_checked.as_ref().map(|at| {
            match mistakes {
                0 => Part::new("Code checked", tokens::text_label()).marked(Mark::Tick),
                1 => Part::new("1 mistake in the code", tokens::ERROR_TEXT).marked(Mark::Warning),
                n => Part::new(format!("{n} mistakes in the code"), tokens::ERROR_TEXT)
                    .marked(Mark::Warning),
            }
            .more(format!(
                "The game's code was last checked at {at}, when the project opened or a code \
                 file was saved. Mistakes are listed in Problems."
            ))
        });
        let saved = if self.open.document.is_dirty() {
            Part::new("Unsaved changes", tokens::text_primary())
                .more("Home \u{203a} Save (Ctrl+S) keeps them.")
        } else if self.open.document.file().is_none() {
            Part::new("Not saved yet", tokens::text_label())
                .more("This area has no file yet: Home \u{203a} Save (Ctrl+S) gives it one.")
        } else {
            Part::new("Saved", tokens::text_label())
        };
        let playing = match self.play_state() {
            PlayState::Editing => None,
            PlayState::Playing => {
                Some(Part::new("Playing", tokens::text_primary()).marked(Mark::Playing))
            }
            PlayState::Paused => Some(Part::new("Paused", tokens::WARNING).marked(Mark::Paused)),
        };
        let area = if self.viewport.camera.kind == crate::camera::ViewKind::TwoD {
            let [w, h] = self.open.pixels.base;
            format!(
                "{} \u{b7} 2D \u{b7} {w} \u{d7} {h}",
                self.open.document.name()
            )
        } else {
            format!("{} \u{b7} 3D", self.open.document.name())
        };
        Status {
            code,
            problems: self.open.problems.list.len() + mistakes,
            code_mistakes: mistakes,
            saved,
            playing,
            area,
            snap: snap_words(
                &self.viewport.snapping,
                self.viewport.camera.kind == crate::camera::ViewKind::TwoD,
            ),
        }
    }

    /// Draws the status bar along the bottom of `ui`.
    pub(crate) fn status_bar(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        egui::Panel::bottom("status bar")
            .frame(crate::theme::status_bar_frame())
            .exact_size(size::STATUS_BAR_H)
            .resizable(false)
            .show_separator_line(false)
            .show(ui, |ui| self.status_parts(ui, actions));
    }

    fn status_parts(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        // The line along the top, the bar's whole width (spec 160).
        let whole = ui.max_rect().expand2(vec2(size::ROW_PAD, 0.0));
        tokens::hline(ui.painter(), whole.x_range(), whole.top(), tokens::line());
        let mut status = self.status();
        // While a file is dragged to place: what and how (spec 286).
        if let Some(dragged) = egui::DragAndDrop::payload::<crate::library::LibraryDrag>(ui.ctx())
            && dragged.0.kind.placeable()
        {
            let ctrl = ui.input(|i| i.modifiers.command);
            status.snap = placing_words(&dragged.0.name, &self.viewport.snapping, ctrl);
        }
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = size::STATUS_GAP;
            ready(ui, status.code.as_ref());
            let parts: Vec<Entry<()>> = vec![Entry::Unbuilt(UnbuiltId::ShellStatusBranch)];
            for entry in unbuilt::filter(parts) {
                if let Entry::Unbuilt(part) = entry {
                    unbuilt::status_item(ui, part);
                }
            }
            if counts(ui, &status).clicked() {
                self.show_panel(Panel::Problems);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                self.restart_button(ui, actions);
                let version = format!("v{}", self.info.version);
                words(ui, &version, text::mono(), tokens::text_label());
                words(ui, &status.snap, text::ribbon_small(), tokens::text_label());
                // For three seconds after a click on a part that isn't
                // built, the context says so instead.
                if let Some(said) = unbuilt::said(ui.ctx()).or_else(|| told(ui.ctx())) {
                    part(ui, &Part::new(said, tokens::text_label()));
                    return;
                }
                part(ui, &Part::new(status.area.clone(), tokens::text_label()));
                part(ui, &status.saved);
                if let Some(playing) = &status.playing {
                    part(ui, playing);
                }
            });
        });
    }
}

/// Words the status bar says for a moment in place of the context, as
/// for a click on an unfinished part: "Hints are off: File › Preferences
/// turns them back on".
pub(crate) fn tell(ctx: &egui::Context, words: impl Into<String>) {
    let at = ctx.input(|input| input.time);
    let told = (words.into(), at);
    ctx.data_mut(|data| data.insert_temp(told_id(), told));
    ctx.request_repaint();
}

fn told_id() -> egui::Id {
    egui::Id::new("status bar told")
}

/// What [`tell`] said, for three seconds after.
pub(crate) fn told(ctx: &egui::Context) -> Option<String> {
    let (words, at) = ctx.data(|data| data.get_temp::<(String, f64)>(told_id()))?;
    let left = unbuilt::SAID_FOR - (ctx.input(|input| input.time) - at);
    if left <= 0.0 || left > unbuilt::SAID_FOR {
        return None;
    }
    ctx.request_repaint_after(std::time::Duration::from_secs_f64(left));
    Some(words)
}

/// "Snap 0.25 · World", "Snap off · Object", or in the 2D view "Snap
/// whole pixels".
pub(crate) fn snap_words(snapping: &crate::transforming::Snapping, two_d: bool) -> String {
    if two_d {
        return format!("Snap {}", snapping.pixel_snap.name());
    }
    let step = if snapping.grid {
        snapping.grid_step.to_string()
    } else {
        "off".to_owned()
    };
    let space = if snapping.object_axes {
        "Object"
    } else {
        "World"
    };
    format!("Snap {step} \u{b7} {space}")
}

/// "Placing crate · Snap 0.25 · Grid" while a file is dragged to place:
/// its name, and what it would snap to.
pub(crate) fn placing_words(
    name: &str,
    snapping: &crate::transforming::Snapping,
    ctrl: bool,
) -> String {
    let how = match crate::placement::snap_word(snapping, ctrl) {
        "grid" => format!("Snap {} \u{b7} Grid", snapping.grid_step),
        _ => "Free".to_owned(),
    };
    format!("Placing {name} \u{b7} {how}")
}

/// The dot and its word: Ready while the code is clean (or not checked
/// yet), the mistakes in red while it has some.
fn ready(ui: &mut Ui, code: Option<&Part>) {
    let (word, colour, more) = match code {
        Some(part) if part.mark == Some(Mark::Warning) => {
            (part.text.as_str(), tokens::ERROR, part.more.as_deref())
        }
        Some(part) => ("Ready", tokens::SUCCESS, part.more.as_deref()),
        None => ("Ready", tokens::SUCCESS, None),
    };
    let shown = ui
        .scope(|ui| {
            ui.spacing_mut().item_spacing.x = size::GAP;
            dot(ui, colour);
            words(ui, word, text::ribbon_small(), tokens::text_label());
        })
        .response;
    if let Some(more) = more {
        shown.on_hover_text(more);
    }
}

/// The problems as coloured dots with counts: the code's mistakes in red,
/// the areas' problems in amber. Called "No problems" or "N problems"
/// for screen readers and tests.
fn counts(ui: &mut Ui, status: &Status) -> egui::Response {
    let others = status.problems - status.code_mistakes;
    let shown = ui
        .scope(|ui| {
            ui.spacing_mut().item_spacing.x = size::ROW_GAP;
            for (n, colour) in [
                (status.code_mistakes, tokens::ERROR),
                (others, tokens::WARNING),
            ] {
                dot(ui, colour);
                words(
                    ui,
                    &n.to_string(),
                    text::ribbon_small(),
                    tokens::text_label(),
                );
            }
        })
        .response
        .interact(Sense::click());
    let name = match status.problems {
        0 => "No problems".to_owned(),
        1 => "1 problem".to_owned(),
        n => format!("{n} problems"),
    };
    shown.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
    if shown.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    shown.on_hover_text(format!("{name}: opens the Problems tab"))
}

/// A 6 px round dot in `colour`.
fn dot(ui: &mut Ui, colour: Color32) {
    let side = size::STATUS_DOT;
    let (rect, _) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), side / 2.0, colour);
}

/// Words that can't be selected.
fn words(ui: &mut Ui, text: &str, font: egui::FontId, colour: Color32) -> egui::Response {
    ui.add(egui::Label::new(egui::RichText::new(text).font(font).color(colour)).selectable(false))
}

/// One part: its mark, then its words, with more on hover.
fn part(ui: &mut Ui, part: &Part) {
    let shown = ui
        .scope(|ui| {
            ui.spacing_mut().item_spacing.x = size::ROW_GAP;
            // Laid out from the right: the words first, then the mark.
            words(ui, &part.text, text::ribbon_small(), part.colour);
            if let Some(mark) = part.mark {
                let side = size::ICON_SMALL - 2.0;
                let (rect, _) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
                let icon = match mark {
                    Mark::Tick | Mark::Warning => Icon::Warning,
                    Mark::Playing => Icon::Play,
                    Mark::Paused => Icon::Pause,
                };
                crate::icons::icon(ui.painter(), rect, icon, part.colour);
            }
        })
        .response;
    if let Some(more) = &part.more {
        shown.on_hover_text(more);
    }
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
