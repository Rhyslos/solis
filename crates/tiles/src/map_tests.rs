//! Tile maps: cells and values set and read, chunks marked, layers and
//! values changed, resizing, the checksum, and the scale cases.

use super::*;

fn farm() -> TileMap {
    let mut map = TileMap::new([70, 40], None, Some(b'.'));
    map.add_value(
        "crop",
        ValueKind::Choice(vec!["wheat".into(), "corn".into()]),
    )
    .unwrap();
    map.add_value("growth", ValueKind::Number).unwrap();
    map.add_value("watered", ValueKind::Switch).unwrap();
    let _ = map.take_dirty();
    map
}

#[test]
fn a_new_map_has_three_layers_and_its_ground_filled() {
    let map = TileMap::new([10, 5], None, Some(b'.'));
    let names: Vec<&str> = map.layers().iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["Ground", "Decoration", "Above"]);
    assert!(map.layers()[2].over);
    assert_eq!(map.code(0, 9, 4), Some(b'.'));
    assert_eq!(map.code(1, 9, 4), Some(0));
    assert_eq!(map.code(0, 10, 0), None, "outside");
    assert_eq!(map.counts(0)[usize::from(b'.')], 50);
    assert_eq!(TileMap::new([0, 99_999], None, None).size(), [1, MAX_SIDE]);
}

#[test]
fn cells_and_values_change_and_mark_their_chunk() {
    let mut map = farm();
    assert_eq!(map.set_code(0, 33, 2, b't'), Ok(true));
    assert_eq!(map.set_code(0, 33, 2, b't'), Ok(false), "no change");
    assert_eq!(map.set_code(0, 70, 0, b't'), Err(MapError::Outside));
    assert_eq!(map.set_code(9, 0, 0, b't'), Err(MapError::NoSuch));
    assert_eq!(
        map.set_value(0, 33, 2, CellValue::Choice(Some(1))),
        Ok(true)
    );
    assert_eq!(map.value(0, 33, 2), Some(CellValue::Choice(Some(1))));
    assert_eq!(
        map.set_value(0, 33, 2, CellValue::Choice(Some(7))),
        Err(MapError::NotUsable)
    );
    assert_eq!(
        map.set_value(1, 33, 2, CellValue::Switch(true)),
        Err(MapError::WrongKind("number"))
    );
    assert_eq!(
        map.set_value(1, 1, 1, CellValue::Number(i32::MAX)),
        Err(MapError::NotUsable)
    );
    map.set_value(1, 33, 2, CellValue::Number(3)).unwrap();
    map.set_value(2, 0, 39, CellValue::Switch(true)).unwrap();
    let mut dirty = map.take_dirty();
    dirty.sort_unstable();
    assert_eq!(dirty, [[0, 1], [1, 0]]);
    assert!(map.take_dirty().is_empty());
    assert_eq!(map.cells_where(1, |v| v == CellValue::Number(3)), [[33, 2]]);
    assert_eq!(
        map.cells_where(2, |v| v == CellValue::Switch(true)),
        [[0, 39]]
    );
    assert!(map.cells_where(9, |_| true).is_empty());
}

#[test]
fn layers_and_values_are_added_moved_changed_and_taken_away() {
    let mut map = farm();
    map.set_code(1, 0, 0, b'#').unwrap();
    assert!(map.move_layer(1, true));
    assert_eq!(map.layers()[2].name, "Decoration");
    assert_eq!(map.code(2, 0, 0), Some(b'#'), "its cells moved with it");
    assert!(!map.move_layer(2, true), "already on top");
    assert!(!map.move_layer(0, false));
    assert!(map.set_layer(2, "Fences", true));
    assert!(!map.set_layer(2, "Fences", true));
    assert!(map.remove_layer(2));
    assert_eq!(map.layers().len(), 2);
    // A choice's options changed: cells keep their option by name.
    map.set_value(0, 5, 5, CellValue::Choice(Some(1))).unwrap();
    assert!(map.set_value_kind(0, ValueKind::Choice(vec!["corn".into(), "beet".into()])));
    assert_eq!(map.value(0, 5, 5), Some(CellValue::Choice(Some(0))));
    // Another kind: back to its start.
    assert!(map.set_value_kind(0, ValueKind::Number));
    assert_eq!(map.value(0, 5, 5), Some(CellValue::Number(0)));
    assert!(map.rename_value(0, "seed"));
    assert_eq!(map.value_named("seed"), Some(0));
    assert!(map.remove_value(0));
    assert!(!map.remove_value(9));
    for n in 0..MAX_LAYERS {
        let _ = map.add_layer(&format!("L{n}"), false);
    }
    assert_eq!(map.add_layer("one more", false), Err(MapError::Full));
}

#[test]
fn resizing_keeps_what_fits() {
    let mut map = farm();
    map.set_code(0, 2, 3, b't').unwrap();
    map.set_value(1, 2, 3, CellValue::Number(4)).unwrap();
    map.set_code(0, 60, 30, b't').unwrap();
    assert!(map.resize([40, 10]));
    assert_eq!(map.size(), [40, 10]);
    assert_eq!(map.code(0, 2, 3), Some(b't'));
    assert_eq!(map.value(1, 2, 3), Some(CellValue::Number(4)));
    assert!(map.resize([45, 12]));
    assert_eq!(map.code(0, 44, 11), Some(0), "new cells are empty");
    assert_eq!(map.chunks(), [2, 1]);
    assert!(!map.resize([45, 12]));
}

#[test]
fn the_checksum_follows_every_cell_and_value() {
    let mut a = farm();
    let mut b = farm();
    assert_eq!(a.checksum(), b.checksum());
    a.set_code(0, 69, 39, b't').unwrap();
    assert_ne!(a.checksum(), b.checksum());
    b.set_code(0, 69, 39, b't').unwrap();
    assert_eq!(a.checksum(), b.checksum());
    b.set_value(2, 69, 39, CellValue::Switch(true)).unwrap();
    assert_ne!(a.checksum(), b.checksum());
    // Copies share cells until changed.
    let copy = a.clone();
    a.set_code(0, 0, 0, b'~').unwrap();
    assert_eq!(copy.code(0, 0, 0), Some(b'.'));
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_10000_cell_changes() {
    // A day's farming in one tick: 10,000 cells tilled and watered on a
    // 1,000 × 1,000 map, then the checksum.
    const TARGET_MS: f64 = 1.0;
    let mut map = TileMap::new([1000, 1000], None, Some(b'.'));
    map.add_value("watered", ValueKind::Switch).unwrap();
    // As in a running game, every cell's memory is in use (a map read
    // from its file has had every cell written; resizing writes them).
    map.resize([999, 999]);
    map.resize([1000, 1000]);
    let _ = map.checksum();
    let _ = map.take_dirty();
    let start = std::time::Instant::now();
    for i in 0..10_000u32 {
        let (x, y) = ((i * 37) % 1000, (i * 91) % 1000);
        map.set_code(0, x, y, b't').unwrap();
        map.set_value(0, x, y, CellValue::Switch(true)).unwrap();
    }
    let changes = start.elapsed().as_secs_f64() * 1000.0;
    let verdict = if changes <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: 10,000 tile cell changes {changes:.3} ms (target {TARGET_MS} ms) {verdict}"
    );
    assert!(changes <= TARGET_MS, "{changes:.3} ms");
}
