//! The painting tools' keys, the terrain painted, and the piece an exact
//! tile shows.

use super::*;
use tiles::{Piece, Terrain};

fn set() -> Tileset {
    let t = |n: u64, name: &str, code, kind| Terrain {
        id: Id::from_raw(n).unwrap(),
        name: name.into(),
        code,
        colour: [0.5; 3],
        kind,
    };
    let piece = |n: u64, terrain: u64, slot| Piece {
        id: Id::from_raw(n).unwrap(),
        terrain: Id::from_raw(terrain).unwrap(),
        slot,
        at: [slot, 0],
        weight: 1.0,
        animation: tiles::Animation::None,
    };
    Tileset {
        terrains: vec![
            t(1, "grass", '.', TileKind::Plain),
            t(2, "tilled", 't', TileKind::Dual16),
            t(3, "rocks", 'r', TileKind::Rules),
        ],
        pieces: vec![
            piece(10, 2, 3),
            piece(11, 2, 15),
            piece(12, 3, 5),
            piece(13, 1, 0),
        ],
        ..Tileset::default()
    }
}

#[test]
fn every_tool_has_its_own_key_apart_from_the_transform_tools() {
    let mut keys: Vec<egui::Key> = PaintTool::ALL.iter().map(|t| t.key()).collect();
    keys.extend(crate::viewport::Tool::ALL.iter().map(|t| t.key()));
    let all = keys.len();
    keys.sort_by_key(|k| k.name());
    keys.dedup();
    assert_eq!(keys.len(), all);
    assert!(PaintTool::Fill.hint().starts_with("Fill (G)"));
}

#[test]
fn the_terrain_painted_is_the_one_chosen_or_the_first() {
    let set = set();
    let mut paint = PaintSettings::default();
    assert_eq!(paint.code_on(Some(&set)), Some(b'.'));
    paint.code = Some(b't');
    assert_eq!(paint.code_on(Some(&set)), Some(b't'));
    paint.code = Some(b'~');
    assert_eq!(
        paint.code_on(Some(&set)),
        Some(b'.'),
        "a code the tileset hasn't"
    );
    assert_eq!(paint.code_on(None), None);
    assert!(!paint.painting());
    paint.tool = Some(PaintTool::Brush);
    assert!(!paint.painting(), "not without a map");
    paint.allowed = true;
    assert!(paint.painting());
}

#[test]
fn an_exact_tile_is_the_piece_chosen_or_the_middle_of_a_patch() {
    let set = set();
    let id = |n| Id::from_raw(n).unwrap();
    assert_eq!(exact_piece(&set, b't', None), Some(id(11)), "slot 15");
    assert_eq!(exact_piece(&set, b't', Some(id(10))), Some(id(10)));
    // A piece of another terrain isn't taken.
    assert_eq!(exact_piece(&set, b't', Some(id(13))), Some(id(11)));
    assert_eq!(
        exact_piece(&set, b'r', None),
        Some(id(12)),
        "rules: its first"
    );
    assert_eq!(exact_piece(&set, b'.', None), Some(id(13)));
    assert_eq!(exact_piece(&set, b'~', None), None);
}
