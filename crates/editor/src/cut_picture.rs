//! Cut picture… (Fixes 3, F3.4; the approved mock-up's frame (b), "Cut,
//! grid"): a pop-up over the editor showing a clips file's picture with
//! the cells its grid cuts, numbered as clips name them. The grid is a
//! cell's size, the gap between cells and the margin round the edge, each
//! across and down; Find grid guesses all three from the picture. Use
//! these cells keeps the grid in the clips file as one step Ctrl + Z
//! undoes, and Cancel leaves the file as it was.

use crate::widgets::dialog::{Answer, dialog_sized};
use crate::widgets::field::{Axis, whole_number_field_on};
use crate::widgets::picture_board::{picture_board, zoom_to_fit};
use animation::clips_file::{Grid, MAX_SIDE};
use assets::cells::Cutting;
use assets::sprite_sheet::SpriteSheet;
use egui::{Color32, Ui, Vec2};

/// The pop-up's width, and the picture's space in it.
const WIDTH: f32 = 860.0;
const PICTURE: Vec2 = Vec2::new(560.0, 420.0);

/// The cutting pop-up, open on a clips file.
#[derive(Debug, Clone, PartialEq)]
pub struct CutPicture {
    /// The clips file, by path inside the assets.
    pub path: String,
    /// Its picture, by path inside the assets.
    pub picture: String,
    /// The grid as it is in the pop-up, not yet kept.
    pub grid: Grid,
    /// Cells drawn by hand, by number, not yet kept: a grid cell's number
    /// moves that cell, a higher one adds a cell.
    pub drawn: Vec<(u32, [u32; 4])>,
    /// Grid or Free crop.
    pub mode: CutMode,
    /// The cell picked in Free crop, by number.
    pub picked: Option<u32>,
    drag: Option<Drag>,
    zoom: f32,
    pan: Vec2,
    fitted: bool,
}

/// How the pop-up cuts: by the grid, or by boxes drawn by hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutMode {
    /// Cell, Between and Edge cut the whole picture.
    Grid,
    /// Boxes drawn on the picture move a cell or add one.
    Free,
}

/// What a drag on the picture in Free crop is doing.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Drag {
    /// Drawing a new cell from this pixel.
    New([f32; 2]),
    /// Moving the picked cell, grabbed at this pixel when it was here.
    Move([f32; 2], [u32; 4]),
    /// Sizing the picked cell by a corner (0 top left, then clockwise),
    /// from where it was.
    Size(usize, [u32; 4]),
}

/// What the pop-up was told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutAnswer {
    /// Use these cells: keep the grid.
    Use,
    /// Cancel: leave the file as it was.
    Cancel,
}

impl CutPicture {
    /// The pop-up on the clips file at `path`, cutting `picture` by `grid`.
    pub fn new(path: String, picture: String, grid: Grid, drawn: Vec<(u32, [u32; 4])>) -> Self {
        Self {
            path,
            picture,
            grid,
            drawn,
            mode: CutMode::Grid,
            picked: None,
            drag: None,
            zoom: 1.0,
            pan: Vec2::ZERO,
            fitted: false,
        }
    }

    /// How the grid cuts the picture, with no cells drawn by hand.
    pub fn cutting(&self) -> Cutting {
        Cutting {
            cell: self.grid.cell,
            between: self.grid.between,
            edge: self.grid.edge,
            drawn: Vec::new(),
        }
    }

    /// How many cells the grid gives a picture `size` big.
    fn grid_cells(&self, size: [u32; 2]) -> u32 {
        let [across, down] = self.cutting().across_down(size);
        (across * down).min(assets::cells::MAX_CELLS)
    }

    /// Cell `n`'s box in a picture `size` big: drawn, or the grid's.
    pub fn box_of(&self, n: u32, size: [u32; 2]) -> Option<[u32; 4]> {
        if let Some(&(_, rect)) = self.drawn.iter().find(|(m, _)| *m == n) {
            return Some(rect);
        }
        let cutting = self.cutting();
        (n < self.grid_cells(size)).then(|| cutting.grid_box(n, cutting.across_down(size)[0]))
    }

    /// The number a new cell drawn by hand gets: the one after the grid's
    /// last and every drawn one.
    pub fn next_number(&self, size: [u32; 2]) -> u32 {
        let after_drawn = self.drawn.iter().map(|(n, _)| n + 1).max().unwrap_or(0);
        self.grid_cells(size).max(after_drawn)
    }

    /// The cell at the picture's pixel `at`: a drawn one first, the last
    /// drawn on top, then the grid's.
    pub fn cell_at(&self, at: [f32; 2], size: [u32; 2]) -> Option<u32> {
        let inside = |r: [u32; 4]| {
            let [x, y, w, h] = r.map(|v| v as f32);
            at[0] >= x && at[0] < x + w && at[1] >= y && at[1] < y + h
        };
        if let Some(&(n, _)) = self.drawn.iter().rev().find(|(_, r)| inside(*r)) {
            return Some(n);
        }
        (0..self.grid_cells(size)).find(|&n| self.box_of(n, size).is_some_and(inside))
    }

    /// Draws a new cell over `rect` and picks it.
    pub fn draw_new(&mut self, rect: [u32; 4], size: [u32; 2]) {
        let n = self.next_number(size);
        if n < assets::cells::MAX_CELLS && self.drawn.len() < animation::clips_file::MAX_CELLS {
            self.drawn.push((n, rect));
            self.picked = Some(n);
        }
    }

    /// Gives the picked cell the box `rect`, kept inside the picture and
    /// at least a pixel big: a grid cell becomes one drawn by hand.
    pub fn set_picked(&mut self, rect: [u32; 4], size: [u32; 2]) {
        let Some(n) = self.picked else {
            return;
        };
        let [x, y, w, h] = rect;
        let w = w.clamp(1, size[0].max(1));
        let h = h.clamp(1, size[1].max(1));
        let rect = [
            x.min(size[0].saturating_sub(w)),
            y.min(size[1].saturating_sub(h)),
            w,
            h,
        ];
        match self.drawn.iter_mut().find(|(m, _)| *m == n) {
            Some(cell) => cell.1 = rect,
            None => self.drawn.push((n, rect)),
        }
    }

    /// Moves the picked cell a pixel at a time (the arrow keys).
    pub fn nudge(&mut self, dx: i32, dy: i32, size: [u32; 2]) {
        let Some(rect) = self.picked.and_then(|n| self.box_of(n, size)) else {
            return;
        };
        let [x, y, w, h] = rect;
        let step = |v: u32, d: i32| v.saturating_add_signed(d);
        self.set_picked([step(x, dx), step(y, dy), w, h], size);
    }

    /// Back to the grid's box: the picked cell drawn by hand is forgotten
    /// (a grid cell's box is the grid's again; an added one is gone).
    pub fn back_to_grid(&mut self, size: [u32; 2]) {
        let Some(n) = self.picked else {
            return;
        };
        self.drawn.retain(|(m, _)| *m != n);
        if n >= self.grid_cells(size) {
            self.picked = None;
        }
    }

    /// The grid Solis finds in `picture` (Find grid): see-through gaps
    /// between cells and round the edge, or the tile size it guesses.
    pub fn find_grid(&mut self, picture: &SpriteSheet) {
        let Some(rgba) = picture.frame_rgba(0) else {
            return;
        };
        let [cell, between, edge] =
            tiles::autotile::detect_grid(picture.size, &rgba, self.grid.cell);
        self.grid = Grid {
            cell,
            between,
            edge,
        };
    }

    /// What the grid makes of a picture `size` big, in words ("45 cells:
    /// 9 across, 5 down").
    pub fn count_words(&self, size: [u32; 2]) -> String {
        let [across, down] = self.cutting().across_down(size);
        match across * down {
            0 => "No cells: a cell is bigger than the picture".to_owned(),
            1 => "1 cell".to_owned(),
            n => format!("{n} cells: {across} across, {down} down"),
        }
    }
}

/// Shows the pop-up for `cut`, on `picture` (its texture and the sheet
/// read from it) once it's read. Returns what it was told.
pub fn cut_dialog(
    ctx: &egui::Context,
    cut: &mut CutPicture,
    picture: Option<(&egui::TextureHandle, &SpriteSheet)>,
    accent: Color32,
) -> Option<CutAnswer> {
    let name = cut
        .picture
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_owned();
    if let Some((_, sheet)) = picture {
        arrow_keys(ctx, cut, sheet.size);
    }
    let note = picture.map_or_else(String::new, |(_, sheet)| cut.count_words(sheet.size));
    let answers = [
        Answer::right("Cancel"),
        Answer::main("Use these cells")
            .hint("Keeps this grid in the clips file: Ctrl + Z undoes it"),
    ];
    let chosen = dialog_sized(
        ctx,
        "cut picture",
        &format!("Cut {name} into cells"),
        WIDTH,
        |ui| {
            ui.horizontal_top(|ui| {
                let (space, _) = ui.allocate_exact_size(PICTURE, egui::Sense::hover());
                let mut board_ui = ui.new_child(egui::UiBuilder::new().max_rect(space));
                match picture {
                    Some((texture, sheet)) => {
                        picture_part(&mut board_ui, cut, texture, sheet, accent)
                    }
                    None => crate::panels::empty_note(&mut board_ui, "Reading the picture…"),
                }
                ui.add_space(crate::tokens::size::GAP * 4.0);
                ui.vertical(|ui| {
                    let modes = [(CutMode::Grid, "Grid"), (CutMode::Free, "Free crop")];
                    crate::widgets::segmented::segmented(ui, "cutting", &mut cut.mode, &modes);
                    ui.add_space(crate::tokens::size::GAP * 3.0);
                    match cut.mode {
                        CutMode::Grid => grid_part(ui, cut, picture.map(|(_, s)| s)),
                        CutMode::Free => picked_part(ui, cut, picture.map(|(_, s)| s.size)),
                    }
                });
            });
        },
        &note,
        &answers,
    );
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        return Some(CutAnswer::Cancel);
    }
    match chosen {
        Some(0) => Some(CutAnswer::Cancel),
        Some(_) => Some(CutAnswer::Use),
        None => None,
    }
}

/// The picture with its grid's cells, each numbered.
fn picture_part(
    ui: &mut Ui,
    cut: &mut CutPicture,
    texture: &egui::TextureHandle,
    sheet: &SpriteSheet,
    accent: Color32,
) {
    if !cut.fitted {
        cut.zoom = zoom_to_fit(sheet.size, PICTURE);
        cut.fitted = true;
    }
    let board = picture_board(ui, &mut cut.zoom, &mut cut.pan, texture, sheet.size);
    let cutting = cut.cutting();
    let [across, down] = cutting.across_down(sheet.size);
    let free = cut.mode == CutMode::Free;
    // In Free crop the grid's boxes are faint under the drawn ones; too
    // small to see one by one, none are drawn and the count says it.
    if (cutting.cell[0].min(cutting.cell[1]) as f32) * cut.zoom >= 6.0 {
        let colour = if free {
            accent.gamma_multiply(0.3)
        } else {
            accent
        };
        for n in 0..(across * down).min(assets::cells::MAX_CELLS) {
            if !free || !cut.drawn.iter().any(|(m, _)| *m == n) {
                let label = if free { String::new() } else { n.to_string() };
                board.boxed(ui, cutting.grid_box(n, across), &label, colour, 1.0);
            }
        }
    }
    if free {
        free_crop(ui, cut, &board, sheet.size, accent);
    }
}

/// How near a corner, in screen points, grabs its handle.
const HANDLE: f32 = 6.0;

/// Free crop on the picture: the drawn cells, the picked one with its
/// handles; drag on nothing to draw a cell, on the picked one to move
/// it, on a handle to size it; click to pick.
fn free_crop(
    ui: &Ui,
    cut: &mut CutPicture,
    board: &crate::widgets::picture_board::Board,
    size: [u32; 2],
    accent: Color32,
) {
    for &(n, rect) in &cut.drawn {
        let picked = cut.picked == Some(n);
        board.boxed(
            ui,
            rect,
            &n.to_string(),
            accent,
            if picked { 2.0 } else { 1.0 },
        );
    }
    let corners_of = |r: egui::Rect| {
        [
            r.left_top(),
            r.right_top(),
            r.right_bottom(),
            r.left_bottom(),
        ]
    };
    let picked_box = cut.picked.and_then(|n| cut.box_of(n, size));
    if let Some(rect) = picked_box {
        if !cut.drawn.iter().any(|(m, _)| Some(*m) == cut.picked) {
            board.boxed(ui, rect, "", accent, 2.0);
        }
        let painter = ui.painter_at(board.response.rect);
        for corner in corners_of(board.screen_box(rect)) {
            let handle = egui::Rect::from_center_size(corner, Vec2::splat(HANDLE));
            painter.rect_filled(handle, 1.0, accent);
        }
    }
    let response = &board.response;
    let whole = |v: f32| v.round().max(0.0) as u32;
    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(at) = response.interact_pointer_pos()
    {
        let grabbed = picked_box.and_then(|rect| {
            let r = board.screen_box(rect);
            match corners_of(r).iter().position(|c| c.distance(at) <= HANDLE) {
                Some(corner) => Some(Drag::Size(corner, rect)),
                None => r
                    .contains(at)
                    .then_some(Drag::Move(board.to_pixel(at), rect)),
            }
        });
        cut.drag = Some(grabbed.unwrap_or(Drag::New(board.to_pixel(at))));
    }
    let ended = response.drag_stopped_by(egui::PointerButton::Primary);
    if let (Some(drag), Some(now)) = (cut.drag, response.interact_pointer_pos()) {
        let [nx, ny] = board.to_pixel(now);
        match drag {
            Drag::New([sx, sy]) => {
                let (x0, y0) = (whole(sx.min(nx)), whole(sy.min(ny)));
                let x1 = whole(sx.max(nx)).min(size[0]);
                let y1 = whole(sy.max(ny)).min(size[1]);
                let rect = [x0, y0, x1.saturating_sub(x0), y1.saturating_sub(y0)];
                if !ended {
                    board.boxed(ui, rect, "", accent, 2.0);
                } else if rect[2] > 0 && rect[3] > 0 {
                    cut.draw_new(rect, size);
                }
            }
            Drag::Move([sx, sy], [x, y, w, h]) => {
                let to = |v: u32, d: f32| (v as f32 + d).round().max(0.0) as u32;
                cut.set_picked([to(x, nx - sx), to(y, ny - sy), w, h], size);
            }
            Drag::Size(corner, [x, y, w, h]) => {
                let (mut left, mut top, mut right, mut bottom) = (x, y, x + w, y + h);
                let (px, py) = (whole(nx), whole(ny));
                match corner {
                    0 => (left, top) = (px, py),
                    1 => (right, top) = (px, py),
                    2 => (right, bottom) = (px, py),
                    _ => (left, bottom) = (px, py),
                }
                let (left, right) = (left.min(right), left.max(right));
                let (top, bottom) = (top.min(bottom), top.max(bottom));
                cut.set_picked(
                    [left, top, (right - left).max(1), (bottom - top).max(1)],
                    size,
                );
            }
        }
    }
    if ended {
        cut.drag = None;
    } else if response.clicked()
        && let Some(at) = response.interact_pointer_pos()
    {
        cut.picked = cut.cell_at(board.to_pixel(at), size);
    }
}

/// The arrow keys move the picked cell a pixel in Free crop.
fn arrow_keys(ctx: &egui::Context, cut: &mut CutPicture, size: [u32; 2]) {
    if cut.mode != CutMode::Free || cut.picked.is_none() || ctx.egui_wants_keyboard_input() {
        return;
    }
    let keys = [
        (egui::Key::ArrowLeft, -1, 0),
        (egui::Key::ArrowRight, 1, 0),
        (egui::Key::ArrowUp, 0, -1),
        (egui::Key::ArrowDown, 0, 1),
    ];
    for (key, dx, dy) in keys {
        if ctx.input(|i| i.key_pressed(key)) {
            cut.nudge(dx, dy, size);
        }
    }
}

/// The grid's settings: a cell's size, the gap between and the edge, each
/// across and down, and Find grid.
fn grid_part(ui: &mut Ui, cut: &mut CutPicture, sheet: Option<&SpriteSheet>) {
    let rows: [(&str, &str, u32); 3] = [
        ("Cell", "A cell's width and height, in pixels", 1),
        ("Between", "The gap between cells, in pixels", 0),
        ("Edge", "The margin round the picture's edge, in pixels", 0),
    ];
    for (i, (name, hint, low)) in rows.into_iter().enumerate() {
        crate::widgets::rows::section_caps(ui, name).on_hover_text(hint);
        ui.horizontal(|ui| {
            let value = match i {
                0 => &mut cut.grid.cell,
                1 => &mut cut.grid.between,
                _ => &mut cut.grid.edge,
            };
            for (axis, v) in [Axis::W, Axis::H].into_iter().zip(value.iter_mut()) {
                let mut shown = f64::from(*v);
                let label = format!("{name} {}", if axis == Axis::W { "across" } else { "down" });
                if whole_number_field_on(ui, &label, &mut shown, Some(axis), 96.0).changed() {
                    *v = (shown.max(0.0) as u32).clamp(low, MAX_SIDE);
                }
            }
        });
        ui.add_space(crate::tokens::size::GAP * 2.0);
    }
    let find = crate::widgets::button::Button::plain("Find grid")
        .show(ui)
        .on_hover_text(
            "Looks for see-through gaps between the cells and round the edge, or the size \
         the picture's art changes at",
        );
    if find.clicked()
        && let Some(sheet) = sheet
    {
        cut.find_grid(sheet);
    }
}

/// Free crop's settings: the picked cell's place and size, and Back to
/// the grid's box.
fn picked_part(ui: &mut Ui, cut: &mut CutPicture, size: Option<[u32; 2]>) {
    let (Some(n), Some(size)) = (cut.picked, size) else {
        ui.label(
            egui::RichText::new(
                "Drag on the picture to draw a cell, or click one to pick it. The arrow keys \
                 move the picked cell a pixel.",
            )
            .color(crate::tokens::text_meta()),
        );
        return;
    };
    let Some(rect) = cut.box_of(n, size) else {
        return;
    };
    crate::widgets::rows::section_caps(ui, &format!("Cell {n}"));
    let mut shown = rect.map(f64::from);
    let mut changed = false;
    for (pair, axes) in [(0, [Axis::X, Axis::Y]), (2, [Axis::W, Axis::H])] {
        ui.horizontal(|ui| {
            for (i, axis) in axes.into_iter().enumerate() {
                let name = ["X", "Y", "Width", "Height"][pair + i];
                let field = whole_number_field_on(ui, name, &mut shown[pair + i], Some(axis), 96.0);
                changed |= field.changed();
            }
        });
        ui.add_space(crate::tokens::size::GAP * 2.0);
    }
    if changed {
        cut.set_picked(shown.map(|v| v.max(0.0) as u32), size);
    }
    let drawn = cut.drawn.iter().any(|(m, _)| *m == n);
    let back = ui
        .add_enabled_ui(drawn, |ui| {
            crate::widgets::button::Button::plain("Back to the grid's box").show(ui)
        })
        .inner
        .on_hover_text(
            "Forgets the box drawn for this cell: a grid cell's is the grid's again, an added \
             one is gone",
        );
    if back.clicked() {
        cut.back_to_grid(size);
    }
}

#[cfg(test)]
#[path = "cut_picture_tests.rs"]
mod tests;
