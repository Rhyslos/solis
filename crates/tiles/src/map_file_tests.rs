//! Map files: round trips, one changed cell is one changed line, the
//! fixture, damaged parts skipped, limits, and the load scale case.

use super::*;
use proptest::prelude::*;

fn id(n: u64) -> Id {
    Id::from_raw(n).unwrap()
}

/// A 40 × 36 farm: tilled rows, a pond, a fence; wheat growing, some
/// watered.
pub(crate) fn farm() -> TileMap {
    let mut map = TileMap::bare([40, 36], Some(AssetId(id(77))));
    map.add_layer_with_id(id(1), "Ground", false).unwrap();
    map.add_layer_with_id(id(2), "Decoration", false).unwrap();
    map.add_layer_with_id(id(3), "Above", true).unwrap();
    map.add_value_with_id(
        id(10),
        "crop",
        ValueKind::Choice(vec!["wheat".into(), "corn".into()]),
    )
    .unwrap();
    map.add_value_with_id(id(11), "growth", ValueKind::Number)
        .unwrap();
    map.add_value_with_id(id(12), "watered", ValueKind::Switch)
        .unwrap();
    for y in 0..36 {
        for x in 0..40 {
            let code = match (x, y) {
                (4..=12, 3..=8) => b't',
                (30..=36, 2..=6) => b'~',
                _ => b'.',
            };
            map.set_code(0, x, y, code).unwrap();
        }
    }
    for x in 0..40 {
        map.set_code(1, x, 20, b'#').unwrap();
    }
    for (x, y) in [(4, 3), (5, 3), (33, 34)] {
        map.set_value(0, x, y, CellValue::Choice(Some(0))).unwrap();
        map.set_value(1, x, y, CellValue::Number(2)).unwrap();
    }
    map.set_value(2, 5, 3, CellValue::Switch(true)).unwrap();
    let _ = map.take_dirty();
    map
}

#[test]
fn a_map_round_trips() {
    let map = farm();
    let text = write(&map);
    assert!(text.contains("tilemap version 2;"));
    assert!(text.contains("size = [40, 36];"));
    assert!(text.contains(&format!("cells {}.1.0 {{", id(1))));
    assert!(
        text.contains("crop = [[4, 3, \"wheat\"], [5, 3, \"wheat\"]];"),
        "{text}"
    );
    assert!(text.contains("watered = [[5, 3]];"));
    assert!(text.contains("values 1.1 {"), "the chunk with (33, 34)");
    assert!(
        !text.contains(&format!("cells {}.", id(3))),
        "an empty layer writes nothing"
    );
    let back = read(&text).unwrap();
    assert!(back.notes.is_empty(), "{:?}", back.notes);
    assert_eq!(write(&back.map), text);
    let a = map.clone();
    let b = back.map.clone();
    assert_eq!(a.checksum(), b.checksum());
    assert_eq!(b.code(0, 6, 4), Some(b't'));
}

#[test]
fn one_changed_cell_is_one_changed_line() {
    let map = farm();
    let before = write(&map);
    let mut changed = map.clone();
    changed.set_code(0, 20, 30, b't').unwrap();
    let after = write(&changed);
    let diff: Vec<(&str, &str)> = before
        .lines()
        .zip(after.lines())
        .filter(|(a, b)| a != b)
        .collect();
    assert_eq!(before.lines().count(), after.lines().count());
    assert_eq!(diff.len(), 1, "{diff:?}");
}

#[test]
fn version_1_reads_with_every_cell_autotiled() {
    let text = include_str!("../../../tests/fixtures/tilemap/v1/farm.soltilemap");
    let read = read(text).unwrap();
    assert!(read.notes.is_empty(), "{:?}", read.notes);
    assert_eq!(read.map.size(), [40, 36]);
    assert_eq!(read.map.exact_tiles(0).count(), 0);
    // Written again, it's version 2 with the same cells.
    let again = write(&read.map);
    assert!(again.contains("tilemap version 2;"));
    assert_eq!(super::read(&again).unwrap().map, read.map);
}

#[test]
fn the_fixture_reads_without_notes() {
    let text = include_str!("../../../tests/fixtures/tilemap/v2/farm.soltilemap");
    let read = read(text).unwrap();
    assert!(read.notes.is_empty(), "{:?}", read.notes);
    assert_eq!(read.map.size(), [40, 36]);
    let piece = |s: &str| s.parse::<Id>().unwrap();
    assert_eq!(read.map.exact(0, 0, 0), Some(piece("0000000000035")));
    assert_eq!(read.map.exact(0, 33, 2), Some(piece("0000000000036")));
    assert_eq!(read.map.exact_tiles(0).count(), 2);
    assert_eq!(write(&read.map), text, "written as Solis writes it");
}

#[test]
fn exact_tiles_round_trip_and_go_with_their_cell() {
    let mut map = TileMap::new([70, 40], None, Some(b'.'));
    map.set_exact(0, 69, 39, Some(id(7))).unwrap();
    map.set_exact(0, 3, 4, Some(id(8))).unwrap();
    let text = write(&map);
    assert!(text.contains("exact "), "{text}");
    assert_eq!(read(&text).unwrap().map, map);
    // Painted over, the cell is autotiled again; an empty cell holds none.
    map.set_code(0, 3, 4, b't').unwrap();
    assert_eq!(map.exact(0, 3, 4), None);
    map.set_code(0, 5, 5, 0).unwrap();
    assert_eq!(
        map.set_exact(0, 5, 5, Some(id(9))),
        Err(crate::map::MapError::NotUsable)
    );
    // Removing a layer, or shrinking the map, takes its exact tiles along.
    map.resize([40, 40]);
    assert_eq!(map.exact_tiles(0).count(), 0);
}

#[test]
fn exact_tiles_it_can_t_use_are_skipped_with_a_note() {
    let g = id(1).to_string();
    let text = format!(
        "tilemap version 2;\nsize = [40, 36];\n\
         layer {g} {{ name = \"Ground\"; }}\n\
         cells {g}.0.0 {{ rows = [\"..\"]; }}\n\
         exact {g}.0.0 {{ cells = [[0, 0, \"{}\"], [5, 5, \"{}\"]]; }}\n\
         exact {g}.0.1 {{ cells = [[0, 40, \"{}\"]]; }}\n\
         exact {g}.1.0 {{ cells = [[0, 0, \"nope\"]]; }}\n\
         exact {g}.9.9 {{ cells = []; }}\n\
         exact {g}.1.1 {{ rows = 3; }}\n",
        id(5),
        id(6),
        id(7)
    );
    let read = read(&text).unwrap();
    let notes = read.notes.join("\n");
    // The one on a painted cell is kept; on an empty cell, skipped quietly.
    assert_eq!(read.map.exact(0, 0, 0), Some(id(5)));
    assert_eq!(read.map.exact_tiles(0).count(), 1);
    for line in [6, 7, 8, 9] {
        assert!(
            notes.contains(&format!("line {line}:")),
            "line {line}: {notes}"
        );
    }
}

#[test]
fn what_can_t_be_used_is_skipped_with_a_note() {
    let i = |n: u64| id(n).to_string();
    let text = format!(
        "tilemap version 1;\nsize = [40, 36];\ntileset = \"nope\";\ncolour = 1;\n\
         layer {g} {{ name = \"Ground\"; }}\n\
         layer {g} {{ name = \"Again\"; }}\n\
         layer {d} {{ name = \"Ground\"; }}\n\
         layer {e} {{ name = \"Top\"; over = 3; }}\n\
         value {v} {{ name = \"crop\"; kind = \"choice\"; options = [\"wheat\", \"wheat\"]; }}\n\
         value {w} {{ name = \"growth\"; kind = \"colour\"; }}\n\
         value {x} {{ name = \"water\"; kind = \"switch\"; }}\n\
         cells {g}.0.0 {{ rows = [\"..\\u{{e9}}\"]; }}\n\
         cells {g}.5.0 {{ rows = [\"..\"]; }}\n\
         cells {z}.0.0 {{ rows = [\"..\"]; }}\n\
         cells {g}.1.0 {{ rows = [\"tt\"]; }}\n\
         values 0.0 {{ sun = [[0, 0]]; water = [[1, 1], [2, 2, 5]]; }}\n\
         values 0.1 {{ water = [[40, 0]]; }}\n\
         blob 1 {{ }}\n",
        g = i(1),
        d = i(2),
        e = i(3),
        v = i(4),
        w = i(5),
        x = i(6),
        z = i(9)
    );
    let read = read(&text).unwrap();
    let notes = read.notes.join("\n");
    assert_eq!(read.map.layers().len(), 1);
    assert_eq!(read.map.values().len(), 1);
    assert_eq!(read.map.code(0, 32, 0), Some(b't'));
    assert_eq!(read.map.tileset, None);
    for expected in [
        "the tileset isn't an asset's ID",
        "\"colour\" isn't a tile map setting",
        "a second layer with this ID",
        "a second layer called \"Ground\"",
        "over should be true or false",
        "\"wheat\" is an option twice",
        "its kind should be",
        "isn't a terrain's code",
        "isn't a chunk of this map",
        "its layer isn't one of the map's",
        "doesn't fit its kind (switch)",
        "\"sun\" isn't one of the map's values",
        "should start with its x and y in the chunk",
        "\"blob\" isn't a part of a tile map",
    ] {
        assert!(notes.contains(expected), "{expected} in\n{notes}");
    }
}

#[test]
fn a_damaged_or_too_big_file_is_refused_with_its_line() {
    assert!(
        read("tileset version 1;")
            .unwrap_err()
            .message
            .contains("isn't a tilemap file")
    );
    assert!(
        read("tilemap version 3;")
            .unwrap_err()
            .message
            .contains("newer")
    );
    let size = read("tilemap version 1;\nsize = [0, 5];").unwrap_err();
    assert_eq!(size.line, 2);
    assert!(read("tilemap version 1;\nsize = [99999, 5];").is_err());
    assert!(
        read(&"-".repeat(MAX_BYTES + 1))
            .unwrap_err()
            .message
            .contains("MB")
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Any cells and values written read back the same.
    #[test]
    fn anything_written_reads_back(
        w in 1u32..80, h in 1u32..80,
        cells in proptest::collection::vec((0u32..80, 0u32..80, 0usize..3, 33u8..127), 0..200),
        numbers in proptest::collection::vec((0u32..80, 0u32..80, -1000i32..1000), 0..50),
        exact in proptest::collection::vec((0u32..80, 0u32..80, 0usize..3, 1u64..1000), 0..50),
    ) {
        let mut map = TileMap::new([w, h], None, None);
        map.add_value("n", ValueKind::Number).unwrap();
        for (x, y, l, c) in cells {
            if crate::tileset::usable_code(char::from(c)) {
                let _ = map.set_code(l, x % w, y % h, c);
            }
        }
        for (x, y, n) in numbers {
            let _ = map.set_value(0, x % w, y % h, CellValue::Number(n));
        }
        for (x, y, l, p) in exact {
            let _ = map.set_exact(l, x % w, y % h, Some(id(p)));
        }
        let text = write(&map);
        let back = read(&text).unwrap();
        prop_assert!(back.notes.is_empty(), "{:?}", back.notes);
        prop_assert_eq!(write(&back.map), text);
        prop_assert_eq!(back.map, map);
    }
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_reading_a_1000_by_1000_map() {
    // The plan's budget: a 1,000 × 1,000 map loads in under a second.
    const TARGET_MS: f64 = 1000.0;
    let mut map = TileMap::new([1000, 1000], None, Some(b'.'));
    map.add_value("growth", ValueKind::Number).unwrap();
    for i in 0..100_000u32 {
        let (x, y) = ((i * 37) % 1000, (i * 91) % 1000);
        map.set_code(0, x, y, b't').unwrap();
        map.set_code(1, x, (y + 3) % 1000, b'#').unwrap();
        map.set_value(0, x, y, CellValue::Number((i % 5) as i32))
            .unwrap();
    }
    let text = write(&map);
    let start = std::time::Instant::now();
    let back = read(&text).unwrap();
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(back.map.size(), [1000, 1000]);
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: reading a 1,000 × 1,000 tile map ({} MB) {ms:.0} ms (target {TARGET_MS} ms) {verdict}",
        text.len() >> 20
    );
    assert!(ms <= TARGET_MS, "{ms:.0} ms");
}
