//! The tile painter (Phase 9, D6), as the project lead chose it (mock-up
//! B): while a tile map is selected, its tools join the viewport's tool
//! bar after the transform tools, with Exact, Art or Meaning, and the
//! layer painted; the Map tab's terrain swatches are the palette.
//!
//! This module holds what the viewport and the Map tab share: the tools
//! and their keys, the painter's settings, and the pointer and buttons
//! the viewport saw this frame. The workbench does the painting
//! (`workbench_paint.rs`), each stroke one undo step.

use crate::theme::{self, Accent};
use egui::{Color32, Rect, RichText, Sense, Ui, vec2};
use reflect::Id;
use tiles::paint::Stamp;
use tiles::{TileKind, Tileset};

/// A painting tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaintTool {
    /// Paints the cells dragged over (left), or empties them (right).
    Brush,
    /// Paints a rectangle, from where the drag starts to where it ends.
    Rectangle,
    /// Paints a straight line of cells.
    Line,
    /// Paints every cell joined to the one clicked with the same terrain.
    Fill,
    /// Copies the cells dragged over, to paint with (a stamp).
    Eyedropper,
}

impl PaintTool {
    /// Every tool, in the order the tool bar shows them.
    pub const ALL: [Self; 5] = [
        Self::Brush,
        Self::Rectangle,
        Self::Line,
        Self::Fill,
        Self::Eyedropper,
    ];

    /// The tool's name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Brush => "Brush",
            Self::Rectangle => "Rectangle",
            Self::Line => "Line",
            Self::Fill => "Fill",
            Self::Eyedropper => "Eyedropper",
        }
    }

    /// The key that picks it while a map is selected (chosen with the
    /// project lead, 2026-10-29).
    pub fn key(self) -> egui::Key {
        match self {
            Self::Brush => egui::Key::B,
            Self::Rectangle => egui::Key::U,
            Self::Line => egui::Key::N,
            Self::Fill => egui::Key::G,
            Self::Eyedropper => egui::Key::I,
        }
    }

    /// What its button says when pointed at.
    pub fn hint(self) -> String {
        let what = match self {
            Self::Brush => "drag to paint, right-drag to empty",
            Self::Rectangle => "drag a rectangle to paint",
            Self::Line => "drag a straight line to paint",
            Self::Fill => "click to paint the whole patch of one terrain",
            Self::Eyedropper => "drag over cells to copy them, then paint with the copy",
        };
        format!("{} ({}): {what}", self.name(), self.key().name())
    }
}

/// The painter's settings: what the tool bar and the Map tab change.
/// They stay while the editor runs (never saved).
#[derive(Debug, Clone, Default)]
pub struct PaintSettings {
    /// The tool in use, or none for the transform tools.
    pub tool: Option<PaintTool>,
    /// Place exact tiles: the piece chosen, not autotiled (Ctrl while
    /// painting does the same).
    pub exact: bool,
    /// Show the selected map's meaning (each terrain's colour) instead of
    /// its art (Tab).
    pub meaning: bool,
    /// The layer painted, by index.
    pub layer: usize,
    /// The terrain painted, by its code: the first of the tileset's if
    /// none is chosen.
    pub code: Option<u8>,
    /// The piece an exact tile shows, if one is chosen.
    pub piece: Option<Id>,
    /// The cells copied with the eyedropper, painted instead of a terrain.
    pub stamp: Option<Stamp>,
    /// Whether painting is possible now: a map is selected and the game
    /// isn't running (the workbench sets it each frame).
    pub allowed: bool,
    /// The selected map's layers' names, for the tool bar.
    pub layers: Vec<String>,
}

impl PaintSettings {
    /// Whether a painting tool has the viewport's mouse now.
    pub fn painting(&self) -> bool {
        self.allowed && self.tool.is_some()
    }

    /// The terrain painted on `set`: the one chosen if the tileset has
    /// it, otherwise its first.
    pub fn code_on(&self, set: Option<&Tileset>) -> Option<u8> {
        let set = set?;
        self.code
            .filter(|&c| set.by_code(char::from(c)).is_some())
            .or_else(|| set.terrains.first().map(|t| t.code as u8))
    }
}

/// One mouse button this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Button {
    /// Pressed this frame.
    pub pressed: bool,
    /// Held down.
    pub down: bool,
}

/// The mouse as the viewport saw it this frame, for painting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PaintInput {
    /// Over the view (not its corner controls or a menu).
    pub on_view: bool,
    /// The left button: paints.
    pub left: Button,
    /// The right button: empties.
    pub right: Button,
    /// Ctrl: exact tiles.
    pub ctrl: bool,
    /// Alt: a click picks the terrain under the pointer.
    pub alt: bool,
}

/// How a stamp is turned, asked for by a key or a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Turn {
    /// Left and right swapped (X).
    FlipX,
    /// Top and bottom swapped (Y).
    FlipY,
    /// A quarter turn clockwise (Z).
    Rotate,
}

/// The slot whose piece shows a terrain all round (its middle), for an
/// exact tile when no piece is chosen.
fn full_slot(kind: TileKind) -> Option<u32> {
    match kind {
        TileKind::Plain => Some(0),
        TileKind::Dual16 | TileKind::Edges => Some(15),
        TileKind::Blob47 => Some(46),
        TileKind::Rules => None,
    }
}

/// The piece an exact tile of terrain `code` shows: `chosen` if it's one
/// of that terrain's, otherwise its piece for the middle of a patch, or
/// its first. `None` if the terrain has no art.
pub fn exact_piece(set: &Tileset, code: u8, chosen: Option<Id>) -> Option<Id> {
    let t = set.by_code(char::from(code))?;
    let mine = |p: &&tiles::Piece| p.terrain == t.id;
    if let Some(c) = chosen
        && set.pieces.iter().filter(mine).any(|p| p.id == c)
    {
        return Some(c);
    }
    full_slot(t.kind)
        .and_then(|s| set.pieces.iter().filter(mine).find(|p| p.slot == s))
        .or_else(|| set.pieces.iter().find(mine))
        .map(|p| p.id)
}

/// The painting tools on the viewport's tool bar, then Exact, Art or
/// Meaning, and the layer painted. Changes `paint`; gives back the tool
/// clicked, if one was.
pub(crate) fn paint_bar(
    ui: &mut Ui,
    paint: &mut PaintSettings,
    accent: Accent,
) -> Option<PaintTool> {
    let mut clicked = None;
    let (rect, _) = ui.allocate_exact_size(vec2(7.0, 22.0), Sense::hover());
    ui.painter().line_segment(
        [rect.center_top(), rect.center_bottom()],
        egui::Stroke::new(1.0, theme::TEXT_WEAK),
    );
    for tool in PaintTool::ALL {
        let chosen = paint.tool == Some(tool);
        let (rect, response) = ui.allocate_exact_size(vec2(28.0, 26.0), Sense::click());
        let response = response.on_hover_text(tool.hint());
        let fill = if chosen {
            accent.colors().fill
        } else if response.hovered() {
            theme::HOVER
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 3.0, fill);
        let colour = if chosen {
            Color32::WHITE
        } else {
            theme::TEXT_STRONG
        };
        crate::icons::paint_icon(
            ui.painter(),
            Rect::from_center_size(rect.center() - vec2(2.0, 1.0), vec2(15.0, 15.0)),
            tool,
            colour,
        );
        ui.painter().text(
            rect.right_bottom() - vec2(3.0, 1.0),
            egui::Align2::RIGHT_BOTTOM,
            tool.key().name(),
            egui::FontId::proportional(8.0),
            colour.gamma_multiply(0.8),
        );
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, chosen, tool.name())
        });
        if response.clicked() {
            clicked = Some(tool);
        }
    }
    ui.add_space(2.0);
    let exact = ui
        .add(egui::Button::new("Exact").selected(paint.exact))
        .on_hover_text(
            "Place the exact tile chosen in the Map tab, not the one autotiling picks \
             (hold Ctrl while painting for the same)",
        );
    if exact.clicked() {
        paint.exact = !paint.exact;
    }
    let shown = if paint.meaning { "Meaning" } else { "Art" };
    egui::ComboBox::from_id_salt("paint view")
        .selected_text(shown)
        .width(66.0)
        .show_ui(ui, |ui| {
            for (meaning, text) in [(false, "Art"), (true, "Meaning")] {
                if ui
                    .selectable_label(paint.meaning == meaning, text)
                    .on_hover_text(if meaning {
                        "Each terrain in its colour: what the game reads (Tab)"
                    } else {
                        "The tileset's art, as the game draws it (Tab)"
                    })
                    .clicked()
                {
                    paint.meaning = meaning;
                }
            }
        })
        .response
        .on_hover_text("What the selected map shows: its art, or its meaning (Tab)");
    let layer = paint.layers.get(paint.layer).cloned().unwrap_or_default();
    egui::ComboBox::from_id_salt("paint layer")
        .selected_text(RichText::new(layer))
        .width(64.0)
        .show_ui(ui, |ui| {
            // Top first, as the Map tab lists them.
            for i in (0..paint.layers.len()).rev() {
                if ui
                    .selectable_label(paint.layer == i, &paint.layers[i])
                    .clicked()
                {
                    paint.layer = i;
                }
            }
        })
        .response
        .on_hover_text("The layer painted");
    clicked
}

/// The stamp's buttons, while one is held: flip it either way, turn it,
/// or put it away. Gives back what was asked for (`None` in the outer
/// option for nothing; `Some(None)` to drop it).
pub(crate) fn stamp_bar(ui: &mut Ui, stamp: &Stamp) -> Option<Option<Turn>> {
    let mut asked = None;
    theme::overlay_frame().show(ui, |ui| {
        // The corner it sits in builds rows from the right, so its parts
        // go in last first: they read "Copy 3 × 2  X  Y  Z  ×".
        ui.horizontal(|ui| {
            if ui
                .button("×")
                .on_hover_text("Put the copy away and paint the terrain again (Esc)")
                .clicked()
            {
                asked = Some(None);
            }
            for (text, turn, hint) in [
                ("Z", Turn::Rotate, "Turn a quarter clockwise (Z)"),
                ("Y", Turn::FlipY, "Flip top and bottom (Y)"),
                ("X", Turn::FlipX, "Flip left and right (X)"),
            ] {
                if ui.button(text).on_hover_text(hint).clicked() {
                    asked = Some(Some(turn));
                }
            }
            ui.label(
                RichText::new(format!("Copy {} × {}", stamp.size[0], stamp.size[1]))
                    .color(theme::TEXT_WEAK),
            );
        });
    });
    asked
}

#[cfg(test)]
#[path = "tile_paint_tests.rs"]
mod tests;
