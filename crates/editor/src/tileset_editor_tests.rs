//! A tileset open for editing: each change one undo step, terrains,
//! kinds, tiles filling slots (variations), marking up from a template,
//! rules, and the try-it patch.

use super::*;
use reflect::AssetId;

/// grass (plain), tilled (16-tile) with no art, a picture.
fn editor() -> TilesetEditor {
    let mut set = crate::workbench_tiles::starter_tileset();
    set.terrains[1].kind = TileKind::Dual16;
    set.picture = Some(AssetId::random());
    TilesetEditor::new("sprites/farm.soltileset".into(), set)
}

fn tilled(e: &TilesetEditor) -> Id {
    e.file.by_name("tilled").unwrap().id
}

#[test]
fn it_opens_on_a_terrain_with_a_kind_and_steps_through_setting_it_up() {
    let mut e = editor();
    assert_eq!(e.terrain, Some(tilled(&e)), "the first terrain with a kind");
    assert_eq!(e.step(), Step::MarkUp);
    e.terrain = e.file.by_name("grass").map(|t| t.id);
    assert_eq!(e.step(), Step::Kind, "plain and no tiles yet");
    e.file.picture = None;
    assert_eq!(e.step(), Step::Picture);
    assert_eq!(e.try_cells.iter().filter(|c| **c).count(), 16);
}

#[test]
fn every_change_is_one_undo_step_and_undo_redo_round_trip() {
    let mut e = editor();
    let before = e.file.clone();
    assert!(e.assign(15, [3, 3]));
    assert!(e.assign(15, [2, 3]), "a second tile is a variation");
    assert!(!e.assign(15, [2, 3]), "the same tile twice isn't");
    assert!(!e.assign(16, [0, 0]), "no slot 16 in a 16-tile terrain");
    assert_eq!(e.file.pieces_of(tilled(&e), 15).count(), 2);
    assert_eq!(e.slot, Some(15));
    assert!(e.undo());
    assert_eq!(e.file.pieces.len(), 1);
    assert!(e.undo());
    assert_eq!(e.file, before);
    assert!(!e.undo(), "nothing more");
    assert!(e.redo() && e.redo());
    assert_eq!(e.file.pieces.len(), 2);
    assert!(!e.redo());
    // A change that changes nothing isn't a step.
    let id = e.file.pieces[0].id;
    assert!(!e.set_piece(id, 1.0, Animation::None));
}

#[test]
fn terrains_are_added_changed_and_taken_away_with_their_art() {
    let mut e = editor();
    assert!(e.add_terrain());
    let added = e.chosen().unwrap().clone();
    assert_eq!(added.name, "terrain");
    assert_eq!(added.kind, TileKind::Plain);
    assert!(e.add_terrain());
    assert_eq!(e.chosen().unwrap().name, "terrain2");
    // Names and codes stay unique; a usable code only.
    assert!(!e.set_terrain(added.id, Some("grass".into()), None, None));
    assert!(!e.set_terrain(added.id, None, Some('.'), None));
    assert!(
        !e.set_terrain(added.id, None, Some('?'), None),
        "a pattern's character"
    );
    assert!(e.set_terrain(
        added.id,
        Some("mud".into()),
        Some('m'),
        Some([0.1, 0.2, 0.3])
    ));
    let mud = e.file.by_name("mud").unwrap();
    assert_eq!((mud.code, mud.colour), ('m', [0.1, 0.2, 0.3]));
    // Taking tilled away takes its tiles; the pick moves on.
    let t = tilled(&e);
    e.terrain = Some(t);
    e.assign(1, [0, 0]);
    assert!(e.remove_terrain(t));
    assert!(e.file.pieces.is_empty());
    assert!(e.chosen().is_some());
}

#[test]
fn a_new_kind_lets_go_of_tiles_that_no_longer_fit() {
    let mut e = editor();
    let t = tilled(&e);
    e.assign(9, [1, 2]);
    assert!(e.set_kind(t, TileKind::Blob47));
    assert!(e.file.pieces.is_empty());
    assert_eq!(e.file.terrain(t).unwrap().kind, TileKind::Blob47);
    assert!(!e.set_kind(t, TileKind::Blob47), "the same kind: no step");
    assert!(e.undo());
    assert_eq!(e.file.pieces.len(), 1, "undo brings them back");
}

#[test]
fn marking_up_from_a_template_fills_each_slot_from_its_place() {
    let mut e = editor();
    // Every tile of a 4 × 4 block from (2, 1) is drawn but the first.
    assert!(e.mark_up([2, 1], |at| at != [2, 1]));
    let t = tilled(&e);
    assert_eq!(e.file.pieces.len(), 15);
    let p = e.file.pieces_of(t, 5).next().unwrap().1;
    assert_eq!(p.at, [2 + 1, 1 + 1]);
    // Again replaces them; nothing drawn changes nothing.
    assert!(e.mark_up([0, 0], |_| true));
    assert_eq!(e.file.pieces.len(), 16);
    assert!(!e.mark_up([0, 0], |_| false));
}

#[test]
fn pieces_are_weighted_and_animated_within_limits() {
    let mut e = editor();
    e.assign(3, [0, 0]);
    let id = e.file.pieces[0].id;
    assert!(e.set_piece(id, 9999.0, Animation::Columns { count: 1, ms: 1 }));
    let p = &e.file.pieces[0];
    assert_eq!(p.weight, tiles::tileset::MAX_WEIGHT);
    assert_eq!(
        p.animation,
        Animation::Columns {
            count: 2,
            ms: tiles::tileset::FRAME_MS.0
        }
    );
    assert!(e.set_piece(id, f32::NAN, Animation::Frames));
    assert_eq!(e.file.pieces[0].weight, 1.0);
    assert!(e.remove_piece(id));
    assert!(e.file.pieces.is_empty());
}

#[test]
fn rules_are_added_changed_resized_and_moved() {
    let mut e = editor();
    assert!(!e.add_rule(), "tilled isn't of its own rules");
    let t = tilled(&e);
    e.set_kind(t, TileKind::Rules);
    assert!(e.add_rule());
    let first = e.rule.unwrap();
    assert!(e.add_rule());
    let second = e.rule.unwrap();
    assert!(e.set_cond(first, 1, Cond::Terrain('.')));
    assert!(
        !e.set_cond(first, 4, Cond::Any),
        "the middle is the cell itself"
    );
    assert!(e.set_rule_slot(first, 999));
    assert_eq!(e.file.rules[0].slot, tiles::autotile::RULE_SLOTS - 1);
    // Bigger: the 3 × 3 stays round the middle.
    assert!(e.set_rule_size(first, 5));
    let r = &e.file.rules[0];
    assert_eq!((r.size, r.pattern.len()), (5, 25));
    assert_eq!(r.pattern[7], Cond::Terrain('.'), "north of the middle");
    assert_eq!(r.pattern[12], Cond::Same);
    assert!(!e.set_rule_size(first, 4), "made odd: 5 stays, no change");
    assert!(e.set_rule_size(first, 1));
    assert_eq!(e.file.rules[0].pattern, [Cond::Same]);
    // Order: the second one first.
    assert!(e.move_rule(second, true));
    assert_eq!(e.file.rules[0].id, second);
    assert!(!e.move_rule(second, true), "already first");
    assert!(e.remove_rule(second));
    assert_eq!(e.file.rules.len(), 1);
}

#[test]
fn the_try_it_patch_is_the_terrain_over_its_ground() {
    let e = editor();
    let map = e.try_map().unwrap();
    assert_eq!(map.size(), [TRY_SIZE[0] as u32, TRY_SIZE[1] as u32]);
    assert_eq!(map.code(0, 0, 0), Some(b'.'), "grass under");
    assert_eq!(map.code(1, 1, 1), Some(b't'));
    assert_eq!(map.code(1, 0, 0), Some(0));
}

#[test]
fn its_own_saves_are_told_from_changes_on_disk() {
    let mut e = editor();
    e.assign(1, [0, 0]);
    let _ = e.saving();
    assert!(e.is_own(&e.file.clone()));
    let mut by_hand = e.file.clone();
    by_hand.tile = [8, 8];
    assert!(!e.is_own(&by_hand));
    e.taken_in(by_hand.clone());
    assert_eq!(e.file, by_hand);
    assert!(e.undo(), "taken in as one step");
    assert_eq!(e.file.tile, [16, 16]);
    // A new picture and tile size, within limits.
    assert!(e.set_picture(None, [0, 9999]));
    assert_eq!(e.file.tile, [1, tiles::tileset::MAX_TILE]);
}
