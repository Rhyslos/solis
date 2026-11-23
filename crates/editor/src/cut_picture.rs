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
    zoom: f32,
    pan: Vec2,
    fitted: bool,
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
    pub fn new(path: String, picture: String, grid: Grid) -> Self {
        Self {
            path,
            picture,
            grid,
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
                ui.vertical(|ui| grid_part(ui, cut, picture.map(|(_, s)| s)));
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
    // Too small to see one by one: none drawn, the count says it.
    if (cutting.cell[0].min(cutting.cell[1]) as f32) * cut.zoom < 6.0 {
        return;
    }
    for n in 0..(across * down).min(assets::cells::MAX_CELLS) {
        board.boxed(ui, cutting.grid_box(n, across), &n.to_string(), accent, 1.0);
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

#[cfg(test)]
#[path = "cut_picture_tests.rs"]
mod tests;
