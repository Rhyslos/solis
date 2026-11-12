//! The tile painter (Phase 9, D6): while a tile map is selected, its
//! tools, Exact, Art or Meaning and the layer painted are on the ribbon
//! of HOME TOOLS · Tile map (the approved additions, round 3); the Map
//! tab's terrain swatches are the palette.
//!
//! This module holds what the viewport and the Map tab share: the tools
//! and their keys, the painter's settings, and the pointer and buttons
//! the viewport saw this frame. The workbench does the painting
//! (`workbench_paint.rs`), each stroke one undo step.

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

    /// The tool's icon.
    pub fn icon(self) -> crate::icons::Icon {
        use crate::icons::Icon;
        match self {
            Self::Brush => Icon::Brush,
            Self::Rectangle => Icon::Rectangle,
            Self::Line => Icon::Line,
            Self::Fill => Icon::Fill,
            Self::Eyedropper => Icon::Eyedropper,
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

    /// What it does, in a few words.
    pub fn what(self) -> &'static str {
        match self {
            Self::Brush => "Drag to paint, right-drag to empty",
            Self::Rectangle => "Drag a rectangle to paint",
            Self::Line => "Drag a straight line to paint",
            Self::Fill => "Click to paint the whole patch of one terrain",
            Self::Eyedropper => "Drag over cells to copy them, then paint with the copy",
        }
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

    /// The keys of the tool in use, for the hint over the view: the
    /// copy's while one is held. None while nothing paints.
    pub(crate) fn hint_keys(&self) -> Option<&'static [(&'static str, &'static str)]> {
        if !self.painting() {
            return None;
        }
        Some(if self.stamp.is_some() {
            &[
                ("Drag", "paint the copy"),
                ("X", "flip across"),
                ("Y", "flip down"),
                ("Z", "turn"),
                ("Esc", "put it away"),
            ]
        } else {
            &[
                ("Drag", "paint"),
                ("Right-drag", "empty"),
                ("Alt click", "pick a terrain"),
                ("Ctrl", "exact tile"),
                ("Esc", "give up"),
            ]
        })
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[cfg(test)]
#[path = "tile_paint_tests.rs"]
mod tests;
