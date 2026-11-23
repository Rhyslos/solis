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
fn knight_sheet() -> SpriteSheet {
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
