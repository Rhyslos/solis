//! The status line along the window's bottom (spec 160-164; one thin
//! line, as board 05 of decision 0118 draws it). On the left: a dot and
//! "Ready" while the game's code is clean (red, with the count, while it
//! has mistakes), the branch (not built yet), whether the level is saved
//! (a click saves it: Save left Home's ribbon), playing or paused, and
//! what is selected. On the right: what moves snap to, the level's name
//! with 2D or 3D and a 2D game's size, then the version in mono, and
//! Restart to update while a newer build waits. The problems' counts are
//! the bottom panel's now, on its Problems tab. For three seconds after a
//! click on an unfinished part, the left's words give way to a line
//! saying so (`unbuilt::said`).
//!
//! What it shows is read from counts kept by the code check and by
//! Problems (each worked out once per change to what it looks at), never
//! by walking the level here.

use crate::icons::Icon;
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
    /// Whether the level is saved.
    pub saved: Part,
    /// Playing or paused; none while editing.
    pub playing: Option<Part>,
    /// A 2D or 3D project's name and kind ("Farm Days · 2D"); otherwise
    /// the level's name, 2D or 3D, and a 2D game's size.
    pub level: String,
    /// In the 2D view, the tile map's cell under the pointer ("Tile 13,
    /// 9"), while a map is selected.
    pub tile: Option<String>,
    /// What is selected: "Crate 1 selected", "3 selected"; none while
    /// nothing is.
    pub selected: Option<String>,
    /// What moves snap to and which axes the gizmo follows: "Snap 0.25 ·
    /// World" (spec 233).
    pub snap: String,
    /// While the brush list is open or a brush is drawn out (Tools 1a,
    /// B2), what is going on: "Choosing a shape brush: Ramp"; while a
    /// brush's insides are edited (B3), "Editing Box_03 · Faces · 1 face
    /// picked"; or for a moment why a change was put back. The saved and
    /// playing parts give way to it.
    pub brushing: Option<String>,
}

impl Workbench {
    /// What the status bar says now.
    pub fn status(&self) -> Status {
        let mistakes = self.open.code_problems.len()
            + self.picture_problems().len()
            + self.font_problems().len()
            + self.markup_problems().len();
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
                .more("Click to save them (Ctrl+S).")
        } else if self.open.document.file().is_none() {
            Part::new("Not saved yet", tokens::text_label())
                .more("This level has no file yet: click to give it one (Ctrl+S).")
        } else {
            Part::new("Saved", tokens::text_label()).more("Click to save it again (Ctrl+S).")
        };
        let playing = match self.play_state() {
            PlayState::Editing => None,
            PlayState::Playing => {
                Some(Part::new("Playing", tokens::text_primary()).marked(Mark::Playing))
            }
            PlayState::Paused => Some(Part::new("Paused", tokens::WARNING).marked(Mark::Paused)),
        };
        let two_d = self.viewport.camera.kind == crate::camera::ViewKind::TwoD;
        let kind = self.open.kind;
        let level =
            if let Some(project) = self.open.project.as_ref().filter(|_| kind.note().is_some()) {
                // A 2D or 3D project says so (board 08): "Farm Days · 2D".
                // Both says the level, as before the kind was asked.
                format!("{} \u{b7} {}", project.name, kind.badge())
            } else if two_d {
                let [w, h] = self.open.pixels.base;
                format!(
                    "{} \u{b7} 2D \u{b7} {w} \u{d7} {h}",
                    self.open.document.name()
                )
            } else {
                format!("{} \u{b7} 3D", self.open.document.name())
            };
        let selection = &self.open.document.selection;
        let selected = match selection.len() {
            0 => None,
            1 => selection
                .primary()
                .and_then(|id| self.open.document.level().get(id))
                .map(|entity| format!("{} selected", entity.name)),
            n => Some(format!("{n} selected")),
        };
        Status {
            code,
            problems: self.open.problems.list.len() + mistakes,
            code_mistakes: mistakes,
            saved,
            playing,
            level,
            selected,
            snap: snap_words(
                &self.viewport.snapping,
                two_d.then_some(self.open.pixels.tile),
            ),
            tile: self
                .viewport
                .tile_overlay
                .pointer
                .filter(|_| two_d)
                .map(|[x, y]| format!("Tile {x}, {y}")),
            brushing: self.brush_status().or_else(|| self.zone_status()),
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
            self.status_left(ui, &status, actions);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                self.restart_button(ui, actions);
                let version = format!("v{}", self.info.version);
                words(ui, &version, text::mono(), tokens::text_label());
                part(ui, &Part::new(status.level.clone(), tokens::text_label()));
                words(ui, &status.snap, text::ribbon_small(), tokens::text_label());
                if let Some(tile) = &status.tile {
                    words(ui, tile, text::ribbon_small(), tokens::text_label());
                }
            });
        });
    }

    /// The status line's left after Ready: saved or not (a click saves),
    /// playing, and what is selected; or what's going on with a brush; or
    /// for three seconds what a click on an unfinished part said.
    fn status_left(&mut self, ui: &mut Ui, status: &Status, actions: &mut Vec<WorkbenchAction>) {
        if let Some(said) = unbuilt::said(ui.ctx()).or_else(|| told(ui.ctx())) {
            part(ui, &Part::new(said, tokens::text_label()));
            return;
        }
        if let Some(brushing) = &status.brushing {
            part(ui, &Part::new(brushing.clone(), tokens::text_primary()));
            return;
        }
        if saved_part(ui, &status.saved).clicked() {
            crate::shell::ShellCommand::SaveLevel.run(self, &ui.ctx().clone(), actions);
        }
        if let Some(playing) = &status.playing {
            part(ui, playing);
        }
        if let Some(selected) = &status.selected {
            words(ui, selected, text::ribbon_small(), tokens::text_label());
        }
    }
}

/// Whether the level is saved, as a button: a click saves it.
fn saved_part(ui: &mut Ui, saved: &Part) -> egui::Response {
    let shown = words(ui, &saved.text, text::ribbon_small(), saved.colour).interact(Sense::click());
    let name = format!("{}: click to save", saved.text);
    shown.widget_info(|| WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
    if shown.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    match &saved.more {
        Some(more) => shown.on_hover_text(more),
        None => shown,
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

/// "Snap 0.25 · World", "Snap off · Object", or in the 2D view (`tile`
/// is the project's tile size there) "Snap whole pixels" or, on tiles,
/// "Snap 16 px" (board 08).
pub(crate) fn snap_words(
    snapping: &crate::transforming::Snapping,
    tile: Option<[u32; 2]>,
) -> String {
    if let Some([x, y]) = tile {
        return match snapping.pixel_snap {
            crate::transforming::PixelSnap::Tiles if x == y => format!("Snap {x} px"),
            crate::transforming::PixelSnap::Tiles => format!("Snap {x} \u{d7} {y} px"),
            other => format!("Snap {}", other.name()),
        };
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
