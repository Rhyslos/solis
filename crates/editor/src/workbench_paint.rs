//! The workbench's side of the tile painter (Phase 9, D6): the painting
//! tools' keys, each stroke painted on a copy of the map shown at once and
//! saved as one undo step when the button comes up, the stamp copied,
//! flipped and turned, and the terrain picked with Alt.
//!
//! A stroke works on the map file's own copy: the map shown in the
//! viewport changes as it's painted (only the chunks touched are drawn
//! again), and the file is written once, at the end.

use crate::tile_paint::{PaintInput, PaintTool, Turn, exact_piece};
use crate::workbench::Workbench;
use std::sync::Arc;
use tiles::TileMap;
use tiles::paint::{Stamp, StampCell, flood_fill, line_cells, rect_within};

/// A stroke being painted: from the button going down to it coming up.
#[derive(Debug, Clone)]
pub(crate) struct Stroke {
    /// The map file's path.
    path: String,
    /// The tool it's painted with.
    tool: PaintTool,
    /// The right button: it empties cells.
    erase: bool,
    /// The cell it started on, and the last one painted.
    start: [i64; 2],
    last: [i64; 2],
    /// The map before it, for its undo step (and Esc).
    before: Arc<TileMap>,
    /// The map as painted so far.
    map: TileMap,
    /// What it paints with: the stamp held, or one cell of a terrain.
    brush: Stamp,
    /// The cell of the map the stamp's top-left cell is lined up with,
    /// so a stamp painted again and again repeats without seams.
    anchor: [i64; 2],
}

/// What a cell becomes, painted with `brush` lined up at `anchor` (the
/// brush repeated across the map like a pattern), or emptied.
fn brush_cell(brush: &Stamp, anchor: [i64; 2], at: [i64; 2], erase: bool) -> Option<StampCell> {
    if erase {
        return Some(StampCell::default());
    }
    let [w, h] = brush.size.map(i64::from);
    let x = (at[0] - anchor[0]).rem_euclid(w);
    let y = (at[1] - anchor[1]).rem_euclid(h);
    let cell = brush.cells[(y * w + x) as usize];
    (cell.code != 0).then_some(cell)
}

/// Makes `at` of `layer` what `cell` says (emptied for code 0). Whether
/// it changed.
fn put(map: &mut TileMap, layer: usize, at: [i64; 2], cell: StampCell) -> bool {
    let [w, h] = map.size();
    if at[0] < 0 || at[1] < 0 || at[0] >= i64::from(w) || at[1] >= i64::from(h) {
        return false;
    }
    let (x, y) = (at[0] as u32, at[1] as u32);
    let mut changed = map.set_code(layer, x, y, cell.code).unwrap_or(false);
    if cell.code != 0 {
        changed |= map.set_exact(layer, x, y, cell.exact).unwrap_or(false);
    }
    changed
}

impl Stroke {
    /// Paints the cells of `cells` with the brush, each as the pattern
    /// says.
    fn paint(&mut self, layer: usize, cells: impl IntoIterator<Item = [i64; 2]>) {
        for at in cells {
            if let Some(cell) = brush_cell(&self.brush, self.anchor, at, self.erase) {
                put(&mut self.map, layer, at, cell);
            }
        }
    }

    /// The brush's cells round `at` (the whole stamp, on its pattern), or
    /// the one cell when emptying.
    fn dab(&self, at: [i64; 2]) -> Vec<[i64; 2]> {
        if self.erase {
            return vec![at];
        }
        let [w, h] = self.brush.size.map(i64::from);
        // The repeat of the pattern the pointer is in.
        let x0 = self.anchor[0] + (at[0] - self.anchor[0]).div_euclid(w) * w;
        let y0 = self.anchor[1] + (at[1] - self.anchor[1]).div_euclid(h) * h;
        (y0..y0 + h)
            .flat_map(|y| (x0..x0 + w).map(move |x| [x, y]))
            .collect()
    }

    /// The rectangle from the start to `to`.
    fn rect_to(&self, to: [i64; 2]) -> [[i64; 2]; 2] {
        [
            [self.start[0].min(to[0]), self.start[1].min(to[1])],
            [self.start[0].max(to[0]), self.start[1].max(to[1])],
        ]
    }
}

impl Workbench {
    /// The tile map the painter works on: its file's path, while painting
    /// is possible.
    fn paint_path(&self) -> Option<String> {
        self.viewport
            .tile_overlay
            .paint
            .allowed
            .then(|| self.map_edits.path.clone())
            .flatten()
    }

    /// B, U, N, G and I pick a painting tool while a map is selected; Tab
    /// shows its art or its meaning; X, Y and Z flip and turn a copy
    /// held. Not while typing, nor while flying.
    pub(crate) fn paint_keys(&mut self, ui: &egui::Ui) {
        if ui.ctx().egui_wants_keyboard_input()
            || self.viewport.is_flying()
            || !self.viewport.tile_overlay.paint.allowed
        {
            return;
        }
        let holding = self.viewport.tile_overlay.paint.painting()
            && self.viewport.tile_overlay.paint.stamp.is_some();
        let (tool, tab, turn) = ui.input_mut(|i| {
            let tool = PaintTool::ALL
                .into_iter()
                .find(|t| i.key_pressed(t.key()) && i.modifiers.is_none());
            let tab = i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
            let mut turn = None;
            if holding {
                for (key, t) in [
                    (egui::Key::X, Turn::FlipX),
                    (egui::Key::Y, Turn::FlipY),
                    (egui::Key::Z, Turn::Rotate),
                ] {
                    // Taken, so they don't lock an axis too.
                    if i.consume_key(egui::Modifiers::NONE, key) {
                        turn = Some(t);
                    }
                }
            }
            (tool, tab, turn)
        });
        let paint = &mut self.viewport.tile_overlay.paint;
        if let Some(tool) = tool {
            paint.tool = Some(tool);
        }
        if tab {
            paint.meaning = !paint.meaning;
        }
        if let Some(turn) = turn {
            self.turn_stamp(turn);
        }
    }

    /// Esc while painting: a stroke is given up (the map as it was), or
    /// else the copy held is put away, or else the painting tool is. Whether
    /// it did anything.
    pub(crate) fn paint_escape(&mut self) -> bool {
        if let Some(stroke) = self.map_edits.stroke.take() {
            self.show_map_now(&stroke.path, (*stroke.before).clone());
            self.viewport.tile_overlay.preview.clear();
            return true;
        }
        let paint = &mut self.viewport.tile_overlay.paint;
        if !paint.painting() {
            return false;
        }
        if paint.stamp.take().is_none() {
            paint.tool = None;
        }
        true
    }

    /// Flips or turns the copy held.
    fn turn_stamp(&mut self, turn: Turn) {
        if let Some(stamp) = &mut self.viewport.tile_overlay.paint.stamp {
            match turn {
                Turn::FlipX => stamp.flip_x(),
                Turn::FlipY => stamp.flip_y(),
                Turn::Rotate => stamp.rotate(),
            }
        }
    }

    /// Shows `map` as the file at `path` at once, without saving it or
    /// making an undo step (a stroke under way).
    fn show_map_now(&mut self, path: &str, map: TileMap) {
        let sheets = Arc::make_mut(&mut self.viewport.sheets);
        sheets.tile_maps.insert(
            path.to_owned(),
            crate::sprite_sheets::MapFileState::Read(Arc::new(map), Vec::new()),
        );
        sheets.work_out_maps();
    }

    /// Paints from this frame's mouse (after the viewport is drawn).
    pub(crate) fn paint_frame(&mut self) {
        let overlay = &mut self.viewport.tile_overlay;
        let input: PaintInput = std::mem::take(&mut overlay.input);
        if let Some(asked) = overlay.turn_asked.take() {
            match asked {
                Some(turn) => self.turn_stamp(turn),
                None => self.viewport.tile_overlay.paint.stamp = None,
            }
        }
        let path = self.paint_path();
        let painting = self.viewport.tile_overlay.paint.painting();
        // A stroke whose map is no longer the one painted is given up.
        if self
            .map_edits
            .stroke
            .as_ref()
            .is_some_and(|s| !painting || path.as_deref() != Some(s.path.as_str()))
        {
            self.paint_escape();
        }
        let (Some(path), true) = (path, painting) else {
            self.viewport.tile_overlay.preview.clear();
            return;
        };
        let pointer = self.viewport.tile_overlay.pointer;
        match self.map_edits.stroke.take() {
            None => {
                let pressed = input.left.pressed || input.right.pressed;
                if let (true, true, Some(at)) = (pressed, input.on_view, pointer) {
                    self.start_stroke(&path, at, input);
                }
            }
            Some(mut stroke) => {
                let held = if stroke.erase {
                    input.right.down
                } else {
                    input.left.down
                };
                let at = pointer.unwrap_or(stroke.last);
                self.stroke_moved(&mut stroke, at);
                if held {
                    self.map_edits.stroke = Some(stroke);
                } else {
                    self.end_stroke(stroke);
                }
            }
        }
    }

    /// The button went down on cell `at`: Alt picks the terrain there;
    /// otherwise a stroke starts (a fill is done at once).
    fn start_stroke(&mut self, path: &str, at: [i64; 2], input: PaintInput) {
        let Some(before) = self.map_file(path) else {
            return;
        };
        let paint = self.viewport.tile_overlay.paint.clone();
        let Some(tool) = paint.tool else {
            return;
        };
        let layer = self.paint_layer(&before);
        if input.alt && input.left.pressed {
            self.pick_terrain(&before, layer, at);
            return;
        }
        let set = before
            .tileset
            .and_then(|t| self.viewport.sheets.tileset(t))
            .cloned();
        let erase = !input.left.pressed;
        let brush = match (&paint.stamp, tool) {
            (Some(stamp), t) if t != PaintTool::Eyedropper => stamp.clone(),
            _ => {
                let Some(code) = paint.code_on(set.as_deref()) else {
                    if !erase {
                        self.log(
                            crate::console::LogLevel::Warning,
                            "This map has no tileset to paint with: choose one in the map file's \
                             Properties (click it in the Library).",
                        );
                        return;
                    }
                    return self.begin(path, tool, true, at, before, Stamp::one(0, None));
                };
                let exact = (paint.exact || input.ctrl)
                    .then(|| {
                        set.as_deref()
                            .and_then(|s| exact_piece(s, code, paint.piece))
                    })
                    .flatten();
                Stamp::one(code, exact)
            }
        };
        self.begin(path, tool, erase, at, before, brush);
    }

    /// Starts a stroke with `brush` at `at`.
    fn begin(
        &mut self,
        path: &str,
        tool: PaintTool,
        erase: bool,
        at: [i64; 2],
        before: Arc<TileMap>,
        brush: Stamp,
    ) {
        let handle = brush.handle();
        let mut stroke = Stroke {
            path: path.to_owned(),
            tool,
            erase,
            start: at,
            last: at,
            map: (*before).clone(),
            before,
            anchor: [at[0] - handle[0], at[1] - handle[1]],
            brush,
        };
        let layer = self.paint_layer(&stroke.map);
        self.viewport.tile_overlay.preview_erase = erase;
        match tool {
            PaintTool::Brush => {
                let cells = stroke.dab(at);
                stroke.paint(layer, cells);
                self.show_map_now(path, stroke.map.clone());
                self.map_edits.stroke = Some(stroke);
            }
            PaintTool::Fill => {
                let [w, h] = stroke.map.size();
                if at[0] >= 0 && at[1] >= 0 && at[0] < i64::from(w) && at[1] < i64::from(h) {
                    let reached = flood_fill(&stroke.map, layer, [at[0] as u32, at[1] as u32]);
                    stroke.paint(
                        layer,
                        reached
                            .into_iter()
                            .map(|i| [i64::from(i % w), i64::from(i / w)]),
                    );
                }
                self.end_stroke(stroke);
            }
            PaintTool::Rectangle | PaintTool::Line | PaintTool::Eyedropper => {
                self.viewport.tile_overlay.preview = vec![[at, at]];
                self.map_edits.stroke = Some(stroke);
            }
        }
    }

    /// The layer painted on `map`: the one chosen, or its top one if
    /// that's gone.
    fn paint_layer(&self, map: &TileMap) -> usize {
        self.viewport
            .tile_overlay
            .paint
            .layer
            .min(map.layers().len().saturating_sub(1))
    }

    /// The pointer moved on to `at` during a stroke: the brush paints the
    /// cells between; the others show what they'll do.
    fn stroke_moved(&mut self, stroke: &mut Stroke, at: [i64; 2]) {
        if at == stroke.last && stroke.tool == PaintTool::Brush {
            return;
        }
        let layer = self.paint_layer(&stroke.map);
        let preview = &mut self.viewport.tile_overlay.preview;
        match stroke.tool {
            PaintTool::Brush => {
                // Every cell between, so a fast drag leaves no gaps; the
                // stamp's repeats along the way, each once.
                let mut dabs: Vec<[i64; 2]> = Vec::new();
                for cell in line_cells(stroke.last, at).into_iter().skip(1) {
                    let dab = stroke.dab(cell);
                    if dabs.last() != dab.first() {
                        dabs.push(dab[0]);
                        stroke.paint(layer, dab);
                    }
                }
                let map = stroke.map.clone();
                self.show_map_now(&stroke.path, map);
            }
            PaintTool::Rectangle | PaintTool::Eyedropper => {
                *preview = vec![stroke.rect_to(at)];
            }
            PaintTool::Line => {
                *preview = line_cells(stroke.start, at)
                    .into_iter()
                    .map(|c| [c, c])
                    .collect();
            }
            PaintTool::Fill => {}
        }
        stroke.last = at;
    }

    /// The button came up: the rectangle or line is painted, the copy
    /// taken, and the stroke saved as one undo step (if it changed
    /// anything).
    fn end_stroke(&mut self, mut stroke: Stroke) {
        self.viewport.tile_overlay.preview.clear();
        let layer = self.paint_layer(&stroke.map);
        let to = stroke.last;
        match stroke.tool {
            PaintTool::Rectangle => {
                let size = stroke.map.size();
                if let Some([[x0, y0], [x1, y1]]) = rect_within(stroke.start, to, size) {
                    let cells: Vec<[i64; 2]> = (y0..=y1)
                        .flat_map(|y| (x0..=x1).map(move |x| [i64::from(x), i64::from(y)]))
                        .collect();
                    stroke.paint(layer, cells);
                }
            }
            PaintTool::Line => {
                let cells = line_cells(stroke.start, to);
                stroke.paint(layer, cells);
            }
            PaintTool::Eyedropper => {
                self.copy_stamp(&stroke.before, layer, stroke.start, to);
                return;
            }
            PaintTool::Brush | PaintTool::Fill => {}
        }
        if stroke.map == *stroke.before {
            self.show_map_now(&stroke.path, (*stroke.before).clone());
            return;
        }
        self.map_edits
            .push_undo(stroke.path.clone(), (*stroke.before).clone());
        self.save_tile_map(&stroke.path, stroke.map);
    }

    /// The eyedropper let go: one cell picks its terrain (and its exact
    /// tile); more are copied to paint with, the brush taking them.
    fn copy_stamp(&mut self, map: &TileMap, layer: usize, a: [i64; 2], b: [i64; 2]) {
        if a == b {
            self.pick_terrain(map, layer, a);
            return;
        }
        let paint = &mut self.viewport.tile_overlay.paint;
        if let Some(stamp) = Stamp::copy(map, layer, a, b) {
            paint.stamp = Some(stamp);
            paint.tool = Some(PaintTool::Brush);
        }
    }

    /// Alt+click (or the eyedropper on one cell): paint with the terrain
    /// at `at` on the layer painted (and its exact tile, if it has one).
    fn pick_terrain(&mut self, map: &TileMap, layer: usize, at: [i64; 2]) {
        let (Ok(x), Ok(y)) = (u32::try_from(at[0]), u32::try_from(at[1])) else {
            return;
        };
        let Some(code) = map.code(layer, x, y).filter(|&c| c != 0) else {
            return;
        };
        let paint = &mut self.viewport.tile_overlay.paint;
        paint.code = Some(code);
        paint.stamp = None;
        if let Some(piece) = map.exact(layer, x, y) {
            paint.piece = Some(piece);
            paint.exact = true;
        }
        if paint.tool == Some(PaintTool::Eyedropper) {
            paint.tool = Some(PaintTool::Brush);
        }
    }
}

#[cfg(test)]
#[path = "workbench_paint_tests.rs"]
mod tests;
