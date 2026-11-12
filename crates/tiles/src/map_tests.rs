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

#[test]
fn a_chunk_s_revision_moves_on_only_when_it_changes() {
    let mut map = TileMap::new([40, 40], None, Some(b'.'));
    let first = map.chunk_revision([0, 0]);
    let other = map.chunk_revision([1, 1]);
    map.set_code(0, 3, 3, b't').unwrap();
    assert!(map.chunk_revision([0, 0]) > first);
    assert_eq!(map.chunk_revision([1, 1]), other);
    assert_eq!(map.chunk_revision([9, 9]), 0, "past the edge");
    map.set_code(0, 3, 3, b't').unwrap();
    let now = map.chunk_revision([0, 0]);
    map.set_code(0, 3, 3, b't').unwrap();
    assert_eq!(
        map.chunk_revision([0, 0]),
        now,
        "no change, no new revision"
    );
    map.resize([10, 10]);
    assert!(
        map.chunk_revision([0, 0]) > now,
        "a resize changes every chunk"
    );
}

#[test]
fn a_map_s_layers_and_values_stay_within_their_memory() {
    // The stability review: the biggest map, filled with every value a
    // file may list, would take gigabytes.
    let mut map = TileMap::bare([MAX_SIDE, MAX_SIDE], None);
    let mut layers = 0;
    while map.add_layer("layer", false).is_ok() {
        layers += 1;
    }
    let mut numbers = 0;
    while map.add_value("n", ValueKind::Number).is_ok() {
        numbers += 1;
    }
    let cells = u64::from(MAX_SIDE) * u64::from(MAX_SIDE);
    let bytes = (layers as u64 + numbers as u64 * 4) * cells;
    assert!(bytes <= MAX_MAP_BYTES, "{bytes}");
    assert!((3..=MAX_LAYERS).contains(&layers));
    // A small map has room for all of them.
    let mut small = TileMap::bare([64, 64], None);
    for _ in 0..MAX_LAYERS {
        small.add_layer("layer", false).unwrap();
    }
    for _ in 0..MAX_VALUES {
        small.add_value("n", ValueKind::Number).unwrap();
    }
    // Growing it to past the budget is refused, and changes nothing.
    assert!(!small.resize([MAX_SIDE, MAX_SIDE]));
    assert_eq!(small.size(), [64, 64]);
}

#[test]
fn a_value_s_kind_changes_only_within_the_map_s_memory() {
    // The code review: adding a value checked the map's memory, but
    // changing one's kind didn't, so on a full map each switch (a byte a
    // cell) made a number (four) took 48 MB past the limit, and 32 of them
    // a gigabyte and a half. The biggest map, full: 16 layers and 16
    // switches.
    let mut map = TileMap::bare([MAX_SIDE, MAX_SIDE], None);
    for n in 0..MAX_LAYERS {
        map.add_layer(&format!("layer{n}"), false).unwrap();
    }
    for n in 0..16 {
        map.add_value(&format!("switch{n}"), ValueKind::Switch)
            .unwrap();
    }
    assert_eq!(
        map.add_value("more", ValueKind::Switch),
        Err(MapError::Full)
    );
    map.set_value(0, 5, 5, CellValue::Switch(true)).unwrap();
    let before = map.clone();
    // A bigger kind doesn't fit: refused, and nothing changes.
    assert!(!map.set_value_kind(0, ValueKind::Number));
    assert!(map == before);
    assert_eq!(map.revision(), before.revision());
    assert_eq!(map.value(0, 5, 5), Some(CellValue::Switch(true)));
    // A kind of the same size does.
    assert!(map.set_value_kind(0, ValueKind::Choice(vec!["wheat".into()])));
    assert_eq!(map.value(0, 5, 5), Some(CellValue::Choice(None)));
    // With three values gone there is exactly room for one number, and for
    // no second; made a switch again, it leaves that room to another.
    for _ in 0..3 {
        assert!(map.remove_value(15 - 3));
    }
    assert!(map.set_value_kind(1, ValueKind::Number));
    assert!(!map.set_value_kind(2, ValueKind::Number));
    assert!(map.set_value_kind(1, ValueKind::Switch));
    assert!(map.set_value_kind(2, ValueKind::Number));
    let cells = u64::from(MAX_SIDE) * u64::from(MAX_SIDE);
    assert_eq!(map.bytes_per_cell() * cells, MAX_MAP_BYTES);
}

/// The review, BUG-10: what's made from a layer (its picture) knew the
/// layer by where its cells were in memory, a place the next cells made
/// can be given. A layer's stamp says the same without that: two maps
/// have the same one only while they share the layer's very cells.
#[test]
fn a_layer_s_stamp_is_shared_only_while_its_cells_are() {
    let mut a = TileMap::new([8, 8], None, Some(b'.'));
    let stamps =
        |m: &TileMap| -> Vec<u64> { (0..m.layers().len()).map(|i| m.layer_stamp(i)).collect() };
    let first = stamps(&a);
    assert!(first.iter().all(|&s| s != 0), "{first:?}");
    assert!(first[0] != first[1] && first[1] != first[2] && first[0] != first[2]);
    assert_eq!(a.layer_stamp(3), 0, "past the last layer");
    // Changed where nothing shares its cells: the same cells, the same
    // stamp, and its revision says it changed.
    let revision = a.layer_revision(0);
    assert_eq!(a.set_code(0, 1, 1, b't'), Ok(true));
    assert_eq!(stamps(&a), first);
    assert!(a.layer_revision(0) > revision);
    // A copy shares every layer's cells, and so its stamps.
    let mut b = a.clone();
    assert_eq!(stamps(&b), first);
    // The copy changed: it has cells of its own on that layer, under a
    // stamp no cells have had. The other layers are still shared.
    assert_eq!(b.set_code(0, 2, 2, b't'), Ok(true));
    assert_ne!(b.layer_stamp(0), first[0]);
    assert_eq!(stamps(&b)[1..], first[1..]);
    assert_eq!(stamps(&a), first, "the first map keeps its own");
    // Setting a cell to what it is already changes nothing.
    let before = stamps(&b);
    assert_eq!(b.set_code(1, 0, 0, 0), Ok(false));
    assert_eq!(stamps(&b), before);
    // Another map made the same way is another map: no stamp in common,
    // though each cell is the same.
    let other = TileMap::new([8, 8], None, Some(b'.'));
    assert!(
        stamps(&other)
            .iter()
            .all(|s| !first.contains(s) && !before.contains(s))
    );
    // The copy let go: the first map's cells aren't shared any more, and
    // changing them again keeps their stamp.
    drop(b);
    assert_eq!(a.set_code(0, 3, 3, b't'), Ok(true));
    assert_eq!(stamps(&a), first);
}

#[test]
fn stamps_go_with_their_layers_and_new_cells_get_new_ones() {
    let mut map = TileMap::new([8, 8], None, Some(b'.'));
    let stamps =
        |m: &TileMap| -> Vec<u64> { (0..m.layers().len()).map(|i| m.layer_stamp(i)).collect() };
    let first = stamps(&map);
    // A layer added: a stamp of its own, the others as they were.
    map.add_layer("Roofs", true).unwrap();
    let four = stamps(&map);
    assert_eq!(four[..3], first[..]);
    assert!(four[3] != 0 && !first.contains(&four[3]));
    // Moved: each layer's stamp goes with its cells.
    assert!(map.move_layer(0, true));
    assert_eq!(stamps(&map), [four[1], four[0], four[2], four[3]]);
    // Taken away: the rest keep theirs.
    assert!(map.remove_layer(1));
    assert_eq!(stamps(&map), [four[1], four[2], four[3]]);
    // Resized: every layer has new cells, so new stamps.
    assert!(map.resize([4, 4]));
    let resized = stamps(&map);
    assert!(
        resized.iter().all(|s| *s != 0 && !four.contains(s)),
        "{resized:?}"
    );
    assert!(resized[0] != resized[1] && resized[1] != resized[2]);
}

/// Each layer's counts, worked out from its cells.
fn counted(map: &TileMap) -> Vec<[usize; 256]> {
    (0..map.layers().len())
        .map(|l| {
            let mut counts = [0; 256];
            for &c in map.layer_cells(l).unwrap() {
                counts[usize::from(c)] += 1;
            }
            counts
        })
        .collect()
}

/// Each layer's counts, as the map keeps them.
fn kept(map: &TileMap) -> Vec<[usize; 256]> {
    (0..map.layers().len()).map(|l| map.counts(l)).collect()
}

/// BUG-22: the counts are kept as cells change, not counted when asked:
/// whatever changes a map, they are what counting its cells gives.
#[test]
fn a_map_s_counts_follow_every_change() {
    let mut map = farm();
    assert_eq!(map.counts(0)[usize::from(b'.')], 70 * 40);
    assert_eq!(map.counts(1)[0], 70 * 40, "an empty layer");
    assert_eq!(map.counts(9), [0; 256], "no such layer");
    let mut seed = 11u64;
    let mut next = |most: u32| {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) as u32 % most.max(1)
    };
    let mut copies = Vec::new();
    for step in 0..3_000 {
        let layers = map.layers().len() as u32;
        match next(40) {
            0 => {
                let _ = map.add_layer(&format!("L{step}"), false);
            }
            1 if layers > 1 => {
                map.remove_layer(next(layers) as usize);
            }
            2 => {
                map.move_layer(next(layers) as usize, next(2) == 0);
            }
            3 => {
                let [w, h] = map.size();
                map.resize([
                    (w + next(9)).saturating_sub(4),
                    (h + next(9)).saturating_sub(4),
                ]);
            }
            4 => copies.push(map.clone()),
            _ => {
                let [w, h] = map.size();
                let code = [0, b'.', b't', b'~'][next(4) as usize];
                let _ = map.set_code(next(layers) as usize, next(w), next(h), code);
            }
        }
        assert_eq!(kept(&map), counted(&map), "after step {step}");
    }
    // Copies keep the counts they had, though the map changed since.
    assert!(!copies.is_empty());
    for copy in &copies {
        assert_eq!(kept(copy), counted(copy));
    }
    // A map read from its file has its counts too.
    let read = crate::map_file::read(&crate::map_file::write(&map).unwrap()).unwrap();
    assert_eq!(kept(&read.map), counted(&read.map));
    assert_eq!(kept(&read.map), kept(&map));
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_counting_a_big_map_s_terrains() {
    // BUG-22: the Map tab's palette counts each terrain's cells on every
    // layer, every frame it is shown.
    const TARGET_MS: f64 = 0.05;
    let mut map = TileMap::new([2048, 2048], None, Some(b'.'));
    map.add_layer("Paths", false).unwrap();
    map.set_code(3, 7, 7, b't').unwrap();
    let start = std::time::Instant::now();
    let mut grass = 0;
    for l in 0..map.layers().len() {
        grass += map.counts(l)[usize::from(b'.')];
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(grass, 2048 * 2048);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: counting the terrains of a 2048 × 2048 map's four layers {ms:.3} ms \
         (target {TARGET_MS} ms) {verdict}"
    );
    assert!(ms <= TARGET_MS, "{ms:.3} ms");
}
