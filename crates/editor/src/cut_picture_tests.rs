//! Cut picture…: its grid, its count and Find grid.

use super::*;

fn cut() -> CutPicture {
    CutPicture::new(
        "knight.clips.lst".into(),
        "knight.png".into(),
        Grid {
            cell: [32, 32],
            between: [2, 2],
            edge: [1, 1],
        },
        Vec::new(),
    )
}

#[test]
fn the_count_says_what_the_grid_cuts() {
    let cut = cut();
    assert_eq!(cut.count_words([306, 170]), "45 cells: 9 across, 5 down");
    assert_eq!(cut.count_words([34, 34]), "1 cell");
    assert_eq!(
        cut.count_words([20, 20]),
        "No cells: a cell is bigger than the picture"
    );
}

#[test]
fn find_grid_finds_see_through_gaps() {
    // 3 × 2 opaque 8-pixel cells, 2 apart, 1 from the edge.
    let size = [2 + 3 * 8 + 2 * 2, 2 + 2 * 8 + 2];
    let inside = |p: u32| p >= 1 && (p - 1) % 10 < 8;
    let mut rgba = vec![0u8; (size[0] * size[1] * 4) as usize];
    for y in 0..size[1] {
        for x in 0..size[0] {
            if inside(x) && inside(y) && x < size[0] - 1 && y < size[1] - 1 {
                let i = ((y * size[0] + x) * 4) as usize;
                rgba[i..i + 4].copy_from_slice(&[(x * 9) as u8, (y * 13) as u8, 40, 255]);
            }
        }
    }
    let sheet = assets::sprite_sheet::build_sheet(assets::aseprite::Aseprite {
        size,
        frames: vec![assets::aseprite::AseFrame {
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
    .unwrap();
    let mut cut = cut();
    cut.find_grid(&sheet);
    assert_eq!(
        cut.grid,
        Grid {
            cell: [8, 8],
            between: [2, 2],
            edge: [1, 1],
        }
    );
}

/// A 306 × 170 picture of 9 × 5 coloured 32-pixel cells, 2 apart and 1
/// from the edge, each a little figure on see-through ground.
pub(crate) fn knight_sheet() -> SpriteSheet {
    let size = [306u32, 170];
    let mut rgba = vec![0u8; (size[0] * size[1] * 4) as usize];
    for n in 0..45u32 {
        let (cx, cy) = (1 + (n % 9) * 34, 1 + (n / 9) * 34);
        for y in 6..30 {
            for x in 10..22 {
                let i = (((cy + y) * size[0] + cx + x) * 4) as usize;
                let shade = (40 + n * 4) as u8;
                rgba[i..i + 4].copy_from_slice(&[shade, 120, 230 - shade, 255]);
            }
        }
    }
    assets::sprite_sheet::build_sheet(assets::aseprite::Aseprite {
        size,
        frames: vec![assets::aseprite::AseFrame {
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

#[test]
fn the_pop_up_cutting_by_a_grid() {
    use crate::picture_support::{drawn, editor_window, matches_approved, sample_project};
    use crate::sprite_sheets::SheetState;
    let mut w = sample_project();
    let knight = reflect::AssetId::random();
    let mut sheets = (*w.viewport.sheets).clone();
    sheets.by_path.insert(
        "knight.png".into(),
        SheetState::Read(std::sync::Arc::new(knight_sheet())),
    );
    sheets.paths.insert(knight, "knight.png".into());
    w.set_sprite_sheets(sheets);
    w.library_request(
        crate::library::LibraryRequest::MakeClipsFile("knight.png".into()),
        &mut Vec::new(),
    );
    w.open.cut_picture.as_mut().expect("open").grid = Grid {
        cell: [32, 32],
        between: [2, 2],
        edge: [1, 1],
    };
    let Some(mut h) = editor_window(
        w,
        crate::interface_look::InterfaceLook::default(),
        [1440.0, 900.0],
    ) else {
        return;
    };
    for _ in 0..4 {
        h.run();
    }
    let picture = drawn(&mut h).cropped([270, 180], [900, 540]);
    matches_approved("cut-picture-grid", &picture);
}

/// The 306 × 170 picture's size.
const KNIGHT: [u32; 2] = [306, 170];

#[test]
fn a_drawn_cell_gets_the_next_number_and_is_picked() {
    let mut cut = cut();
    assert_eq!(cut.next_number(KNIGHT), 45, "after the grid's 0 to 44");
    cut.draw_new([5, 5, 10, 12], KNIGHT);
    assert_eq!(cut.drawn, [(45, [5, 5, 10, 12])]);
    assert_eq!(cut.picked, Some(45));
    cut.draw_new([20, 5, 10, 12], KNIGHT);
    assert_eq!(cut.picked, Some(46));
    assert_eq!(
        cut.cell_at([22.0, 6.0], KNIGHT),
        Some(46),
        "drawn cells over the grid's"
    );
    assert_eq!(
        cut.cell_at([40.0, 40.0], KNIGHT),
        Some(10),
        "else the grid's"
    );
}

#[test]
fn the_arrows_move_the_picked_cell_a_pixel_within_the_picture() {
    let mut cut = cut();
    cut.picked = Some(0);
    cut.nudge(1, 0, KNIGHT);
    assert_eq!(
        cut.drawn,
        [(0, [2, 1, 32, 32])],
        "a grid cell moved is drawn"
    );
    cut.nudge(0, -1, KNIGHT);
    cut.nudge(0, -1, KNIGHT);
    assert_eq!(
        cut.box_of(0, KNIGHT),
        Some([2, 0, 32, 32]),
        "kept inside the picture"
    );
    cut.set_picked([300, 160, 32, 32], KNIGHT);
    assert_eq!(cut.box_of(0, KNIGHT), Some([274, 138, 32, 32]));
    cut.set_picked([0, 0, 0, 999], KNIGHT);
    assert_eq!(
        cut.box_of(0, KNIGHT),
        Some([0, 0, 1, 170]),
        "at least a pixel, at most the picture"
    );
}

#[test]
fn back_to_the_grid_s_box_forgets_what_was_drawn() {
    let mut cut = cut();
    cut.picked = Some(3);
    cut.nudge(1, 1, KNIGHT);
    cut.back_to_grid(KNIGHT);
    assert!(cut.drawn.is_empty());
    assert_eq!(cut.box_of(3, KNIGHT), Some([103, 1, 32, 32]));
    assert_eq!(cut.picked, Some(3), "a grid cell stays picked");
    cut.draw_new([0, 0, 4, 4], KNIGHT);
    cut.back_to_grid(KNIGHT);
    assert!(
        cut.drawn.is_empty() && cut.picked.is_none(),
        "an added cell is gone"
    );
}

#[test]
fn the_pop_up_in_free_crop() {
    use crate::picture_support::{drawn, editor_window, matches_approved, sample_project};
    use crate::sprite_sheets::SheetState;
    let mut w = sample_project();
    let knight = reflect::AssetId::random();
    let mut sheets = (*w.viewport.sheets).clone();
    sheets.by_path.insert(
        "knight.png".into(),
        SheetState::Read(std::sync::Arc::new(knight_sheet())),
    );
    sheets.paths.insert(knight, "knight.png".into());
    w.set_sprite_sheets(sheets);
    w.library_request(
        crate::library::LibraryRequest::MakeClipsFile("knight.png".into()),
        &mut Vec::new(),
    );
    let cut = w.open.cut_picture.as_mut().expect("open");
    cut.grid = Grid {
        cell: [32, 32],
        between: [2, 2],
        edge: [1, 1],
    };
    cut.mode = CutMode::Free;
    cut.picked = Some(10);
    cut.set_picked([37, 33, 30, 34], KNIGHT);
    cut.draw_new([200, 120, 20, 20], KNIGHT);
    cut.picked = Some(10);
    let Some(mut h) = editor_window(
        w,
        crate::interface_look::InterfaceLook::default(),
        [1440.0, 900.0],
    ) else {
        return;
    };
    for _ in 0..4 {
        h.run();
    }
    let picture = drawn(&mut h).cropped([270, 180], [900, 540]);
    matches_approved("cut-picture-free", &picture);
}
