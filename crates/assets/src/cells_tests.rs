//! Cutting a picture into cells.

use super::*;

/// A picture of `size` whose every pixel says where it is: red its x,
/// green its y, fully opaque.
fn picture(size: [u32; 2]) -> SpriteSheet {
    let mut rgba = Vec::new();
    for y in 0..size[1] {
        for x in 0..size[0] {
            rgba.extend([x as u8, y as u8, 0, 255]);
        }
    }
    build_sheet(Aseprite {
        size,
        frames: vec![AseFrame {
            duration_ms: 100,
            rgba,
            events: Vec::new(),
        }],
        layers: Vec::new(),
        tags: Vec::new(),
        slices: Vec::new(),
        palette: Vec::new(),
        notes: Vec::new(),
    })
    .unwrap()
}

fn grid(cell: u32, between: u32, edge: u32) -> Cutting {
    Cutting {
        cell: [cell; 2],
        between: [between; 2],
        edge: [edge; 2],
        drawn: Vec::new(),
    }
}

#[test]
fn a_grid_with_gaps_and_an_edge_cuts_as_aseprite_numbers_frames() {
    let cut = grid(32, 2, 1);
    assert_eq!(cut.across_down([306, 170]), [9, 5]);
    let (boxes, notes) = cut.boxes([306, 170]);
    assert_eq!(boxes.len(), 45);
    assert!(notes.is_empty());
    assert_eq!(boxes[0], Some([1, 1, 32, 32]));
    assert_eq!(boxes[10], Some([35, 35, 32, 32]), "across, then down");
    assert_eq!(boxes[44], Some([273, 137, 32, 32]));
}

#[test]
fn a_drawn_box_moves_its_grid_cell_or_adds_one_after_the_grid() {
    let mut cut = grid(32, 2, 1);
    cut.drawn = vec![
        (23, [10, 20, 30, 40]),
        (45, [0, 0, 8, 8]),
        (48, [0, 0, 4, 4]),
    ];
    let (boxes, notes) = cut.boxes([306, 170]);
    assert_eq!(boxes[23], Some([10, 20, 30, 40]));
    assert_eq!(
        boxes[45],
        Some([0, 0, 8, 8]),
        "the next number after the grid's"
    );
    assert_eq!((boxes[46], boxes[47]), (None, None), "numbers no cell has");
    assert_eq!(boxes.len(), 49);
    assert!(notes.is_empty());
}

#[test]
fn what_cant_be_cut_is_left_out_with_a_note() {
    let (boxes, notes) = grid(64, 0, 0).boxes([40, 40]);
    assert!(boxes.is_empty());
    assert_eq!(notes.len(), 1, "a grid bigger than the picture gives none");
    let mut cut = grid(16, 0, 0);
    cut.drawn = vec![
        (0, [0, 0, 0, 8]),
        (1, [30, 30, 16, 16]),
        (MAX_CELLS, [0, 0, 1, 1]),
    ];
    let (boxes, notes) = cut.boxes([40, 40]);
    assert_eq!(
        boxes[0],
        Some([0, 0, 16, 16]),
        "a box with no width keeps the grid's"
    );
    assert_eq!(
        boxes[1],
        Some([16, 0, 16, 16]),
        "a box past the edge keeps the grid's"
    );
    assert_eq!(notes.len(), 3, "{notes:?}");
    assert_eq!(grid(0, 0, 0).boxes([40, 40]), (Vec::new(), Vec::new()));
}

#[test]
fn every_cell_stands_on_the_frame_s_bottom_middle() {
    let mut cut = grid(4, 0, 0);
    cut.drawn = vec![(2, [5, 1, 2, 2])];
    let (sheet, notes) = cut_sheet(&picture([8, 4]), &cut).unwrap();
    assert!(notes.is_empty());
    assert_eq!(sheet.size, [4, 4]);
    assert_eq!(sheet.frames.len(), 3);
    let at = |frame: &[u8], x: u32, y: u32| {
        let i = ((y * 4 + x) * 4) as usize;
        [frame[i], frame[i + 1], frame[i + 3]]
    };
    let second = sheet.frame_rgba(1).unwrap();
    assert_eq!(at(&second, 0, 0), [4, 0, 255], "the grid's second cell");
    let drawn = sheet.frame_rgba(2).unwrap();
    assert_eq!(at(&drawn, 1, 2), [5, 1, 255], "its top left");
    assert_eq!(
        at(&drawn, 2, 3),
        [6, 2, 255],
        "its bottom right, on the frame's bottom"
    );
    assert_eq!(at(&drawn, 0, 0)[2], 0, "see-through around it");
}

#[test]
fn too_many_big_cells_are_refused() {
    let mut cut = grid(1, 0, 0);
    cut.drawn = vec![(0, [0, 0, 1024, 1024])];
    let err = cut_sheet(&picture([1024, 1024]), &cut).unwrap_err();
    assert!(err.0.contains("too many"), "{}", err.0);
}
