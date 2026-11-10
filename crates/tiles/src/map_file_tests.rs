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
    assert!(text.contains("lock int version = 3;"));
    assert!(text.contains("size = [40, 36]"));
    assert!(text.contains(&format!(
        "new Cells {{ layer = \"{}\", chunk = [1, 0]",
        id(1)
    )));
    assert!(
        text.contains("value = \"crop\", chunk = [0, 0], cells = [[4, 3], [5, 3]], picks = [\"wheat\", \"wheat\"]"),
        "{text}"
    );
    assert!(text.contains("value = \"watered\", chunk = [0, 0], cells = [[5, 3]]"));
    assert!(text.contains("chunk = [1, 1]"), "the chunk with (33, 34)");
    assert!(
        !text.contains(&format!("layer = \"{}\", chunk", id(3))),
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
fn a_map_with_as_many_parts_as_one_has_reads_back() {
    // Every layer and value a map can have, a whole chunk of each value,
    // exact tiles, and cells of the odder codes (`{`, `'`, `@`): every
    // shape a map file holds, in one, which the data reader's checks must
    // let through whole.
    let [w, h] = [96, 70];
    let mut map = TileMap::bare([w, h], Some(AssetId(id(77))));
    for n in 0..MAX_LAYERS {
        map.add_layer_with_id(id(100 + n as u64), &format!("layer_{n}"), n % 2 == 1)
            .unwrap();
    }
    let options: Vec<String> = (0..MAX_OPTIONS).map(|o| format!("option_{o}")).collect();
    for n in 0..MAX_VALUES {
        let kind = match n % 3 {
            0 => ValueKind::Number,
            1 => ValueKind::Switch,
            _ => ValueKind::Choice(options.clone()),
        };
        map.add_value_with_id(id(200 + n as u64), &format!("value_{n}"), kind)
            .unwrap();
    }
    let codes: Vec<u8> = (b'!'..=b'~').filter(|c| usable_code(*c as char)).collect();
    for layer in 0..MAX_LAYERS {
        for y in 0..h {
            for x in 0..w {
                let n = (x * 7 + y * 3) as usize + layer;
                if !n.is_multiple_of(5) {
                    map.set_code(layer, x, y, codes[n % codes.len()]).unwrap();
                    if n.is_multiple_of(11) {
                        map.set_exact(layer, x, y, Some(id(5000 + n as u64)))
                            .unwrap();
                    }
                }
            }
        }
    }
    for value in 0..MAX_VALUES {
        for y in 0..CHUNK {
            for x in CHUNK..2 * CHUNK {
                let to = match value % 3 {
                    0 => CellValue::Number(crate::map::MAX_NUMBER - (x * y) as i32),
                    1 => CellValue::Switch(true),
                    _ => CellValue::Choice(Some(((x + y) as usize % MAX_OPTIONS) as u8)),
                };
                map.set_value(value, x, y, to).unwrap();
            }
        }
    }
    let _ = map.take_dirty();
    let text = write(&map);
    let back = read(&text).unwrap();
    assert!(back.notes.is_empty(), "{:?}", back.notes);
    assert_eq!(write(&back.map), text);
    assert_eq!(back.map.checksum(), map.checksum());
}

#[test]
fn values_listed_a_cell_at_a_time_in_any_order_read_as_one_map() {
    // Solis writes a chunk's cells of a value as one thing; a file changed
    // by hand or made by another tool may list them a cell at a time, the
    // chunks in any order. Each cell still gets its value, and a cell
    // listed twice the later one, as when they were searched for.
    let mut map = farm();
    for (x, y) in [(4, 3), (5, 3), (33, 34)] {
        map.set_value(0, x, y, CellValue::Choice(None)).unwrap();
        map.set_value(1, x, y, CellValue::Number(0)).unwrap();
        map.set_value(2, x, y, CellValue::Switch(false)).unwrap();
    }
    let mut text = write(&map);
    let mut next = crate::tileset::tests::chooser(0x5EC0_9000_0C4A_11CE);
    let mut lists = [String::new(), String::new(), String::new()];
    for _ in 0..3000 {
        let (x, y) = (next(40) as u32, next(36) as u32);
        let at = format!("chunk = [{}, {}], cells = [[{}", x / 32, y / 32, x % 32);
        let at = format!("{at}, {}", y % 32);
        match next(3) {
            0 => {
                let n = next(9) as i32 - 4;
                map.set_value(1, x, y, CellValue::Number(n)).unwrap();
                lists[0] += &format!("    new Numbers {{ value = \"growth\", {at}, {n}]] }},\n");
            }
            1 => {
                map.set_value(2, x, y, CellValue::Switch(true)).unwrap();
                lists[1] += &format!("    new Switches {{ value = \"watered\", {at}]] }},\n");
            }
            _ => {
                let pick = next(2);
                let to = CellValue::Choice(Some(pick as u8));
                map.set_value(0, x, y, to).unwrap();
                let pick = ["wheat", "corn"][pick];
                lists[2] += &format!(
                    "    new Choices {{ value = \"crop\", {at}]], picks = [\"{pick}\"] }},\n"
                );
            }
        }
    }
    for (class, list, things) in [
        ("Numbers", "numbers", &lists[0]),
        ("Switches", "switches", &lists[1]),
        ("Choices", "choices", &lists[2]),
    ] {
        let empty = format!("lock {class} array {list} = [];\n");
        assert!(text.contains(&empty), "{text}");
        text = text.replace(
            &empty,
            &format!("lock {class} array {list} = [\n{things}];\n"),
        );
    }
    let back = read(&text).unwrap();
    assert!(back.notes.is_empty(), "{:?}", back.notes);
    assert_eq!(back.map, map);
    // Written again, each chunk's cells of a value are one thing.
    assert!(write(&back.map).len() < text.len() / 2);
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
    // Written again, it's the current version with the same cells.
    let again = write(&read.map);
    assert!(again.contains("lock int version = 3;"));
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
    // Written again, it's the version 3 example: the same, as Lust.
    let v3 = include_str!("../../../tests/fixtures/tilemap/v3/farm.tilemap.lst");
    assert_eq!(write(&read.map), v3, "written as Solis writes it");
}

#[test]
fn the_lust_fixture_reads_back_as_itself() {
    let text = include_str!("../../../tests/fixtures/tilemap/v3/farm.tilemap.lst");
    let read = read(text).unwrap();
    assert!(read.notes.is_empty(), "{:?}", read.notes);
    assert_eq!(read.map.exact_tiles(0).count(), 2);
    assert_eq!(write(&read.map), text);
    assert_eq!((FORMAT.version_of)(text), Some(3));
}

#[test]
fn a_mistake_in_the_lust_says_its_line() {
    let text = include_str!("../../../tests/fixtures/tilemap/v3/farm.tilemap.lst");
    let wrong = text.replacen("over = false", "over = 3", 1);
    let why = read(&wrong).unwrap_err();
    assert!(why.line > 0, "{why}");
    let newer = text.replacen("lock int version = 3;", "lock int version = 9;", 1);
    assert!(read(&newer).unwrap_err().message.contains("newer"));
    let runs = text.replacen("size = [40, 36]", "size = [40, Maths:abs(36)]", 1);
    assert!(read(&runs).is_err());
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
fn two_classes_with_one_name_are_refused_never_a_hang() {
    // What the tileset fuzzer found (see the tileset's tests) reached tile
    // maps the same way: the fixture with its `new Layer`s made `class
    // Layer` was never finished with.
    let fixture = include_str!("../../../tests/fixtures/tilemap/v3/farm.tilemap.lst");
    let damaged = fixture.replace("new Layer", "class Layer");
    let why = crate::tileset::tests::in_time(move || read(&damaged).map(|_| ())).unwrap_err();
    assert_eq!(why.line, 11);
    assert!(why.message.contains("only lock values"), "{why}");
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
    let big = read(&"-".repeat(MAX_BYTES + 1)).unwrap_err();
    assert_eq!(
        big.message,
        "the map is too big to read: its file is 64 MB, and tile maps up to 64 MB are read"
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
fn a_character_that_can_t_be_seen_is_refused() {
    // What the atlas fuzzer found (see `area::lust_data`) can't reach a
    // map, whose names and rows are all checked; the file is refused all
    // the same, on the line its list starts.
    let fixture = include_str!("../../../tests/fixtures/tilemap/v3/farm.tilemap.lst");
    for (line, from, to) in [
        (10, "\"Ground\"", "\"Gro\u{7f}und\""),
        (15, "\"wheat\", \"corn\"", "\"wheat\", \"corn\0\""),
    ] {
        let why = read(&fixture.replace(from, to)).unwrap_err();
        assert_eq!(why.line, line, "{to:?}");
        assert!(why.message.contains("can't be seen"), "{why}");
    }
}

/// What the fuzz target (`fuzz/fuzz_targets/tile_map.rs`) holds the
/// reader to, for `text`: refused, or read within the limits, every layer
/// with a cell for each of the map's, written back to itself, and then
/// changed every way without a panic ([`changed_every_way`]). Says
/// whether it was read.
fn keeps_its_promises(text: &str, set: &crate::Tileset) -> bool {
    use crate::map::{MAX_LAYERS, MAX_OPTIONS, MAX_SIDE, MAX_VALUES};
    let Ok(read) = read(text) else {
        return false;
    };
    let map = read.map;
    let [w, h] = map.size();
    assert!(
        (1..=MAX_SIDE).contains(&w) && (1..=MAX_SIDE).contains(&h),
        "{text:?}"
    );
    assert!(map.layers().len() <= MAX_LAYERS, "{text:?}");
    assert!(map.values().len() <= MAX_VALUES, "{text:?}");
    for value in map.values() {
        if let ValueKind::Choice(options) = &value.kind {
            assert!(options.len() <= MAX_OPTIONS, "{text:?}");
        }
    }
    for l in 0..map.layers().len() {
        let cells = map.layer_cells(l).map(<[u8]>::len);
        assert_eq!(cells, Some(w as usize * h as usize), "{text:?}");
    }
    let written = write(&map);
    let again = super::read(&written).unwrap_or_else(|e| panic!("{e}\n{text:?}"));
    assert_eq!(again.map, map, "{text:?}");
    changed_every_way(map, set);
    true
}

/// The rest of what the fuzz target does with a map it read: autotiled
/// with `set`, painted with a fill, a stamp and an exact tile, changed at
/// its corners and past them, and resized, none of which may panic.
fn changed_every_way(mut map: TileMap, set: &crate::Tileset) {
    let [w, h] = map.size();
    for l in 0..map.layers().len() {
        let _ = crate::autotile::chunk_tiles(&map, l, set, [0, 0]);
    }
    if !map.layers().is_empty() {
        let reached = crate::paint::flood_fill(&map, 0, [0, 0]);
        assert!(reached.len() <= w as usize * h as usize);
        if let Some(mut stamp) = crate::paint::Stamp::copy(&map, 0, [-2, -2], [3, 2]) {
            stamp.flip_x();
            stamp.rotate();
            stamp.flip_y();
            let cells: Vec<_> = stamp.placed([i64::from(w), 1], [w, h]).collect();
            for ([x, y], c) in cells {
                let _ = map.set_code(0, x, y, c.code);
                let _ = map.set_exact(0, x, y, c.exact);
            }
        }
        let _ = crate::paint::line_cells([0, 0], [i64::from(w), i64::from(h)]);
        let _ = map.set_exact(0, w - 1, h - 1, Id::from_raw(5));
        let last = [(w - 1) / crate::CHUNK, (h - 1) / crate::CHUNK];
        let _ = crate::autotile::chunk_tiles(&map, 0, set, last);
    }
    for (x, y) in [(0, 0), (w - 1, h - 1), (w, 0), (0, h), (u32::MAX, 3)] {
        for l in 0..=map.layers().len() {
            let _ = map.set_code(l, x, y, b't');
        }
        for v in 0..=map.values().len() {
            for to in [
                CellValue::Number(i32::MAX),
                CellValue::Switch(true),
                CellValue::Choice(Some(200)),
            ] {
                let _ = map.set_value(v, x, y, to);
            }
        }
    }
    let _ = map.take_dirty();
    map.resize([w.min(40) + 1, h.min(40)]);
    for l in 0..map.layers().len() {
        let _ = map.counts(l);
    }
    let _ = map.checksum();
}

#[test]
fn no_damaged_map_breaks_a_promise() {
    // Fuzz-style, the same every run: a thousand damaged copies of each
    // version's example, the older layouts too, each held to what the fuzz
    // target holds the reader to.
    let set = crate::tileset::read(include_str!(
        "../../../tests/fixtures/tileset/v2/farm.soltileset"
    ))
    .unwrap()
    .tileset;
    let mut next = crate::tileset::tests::chooser(0x5EC2_0000_711E_3A90);
    let mut read = 0;
    for good in [
        include_str!("../../../tests/fixtures/tilemap/v1/farm.soltilemap"),
        include_str!("../../../tests/fixtures/tilemap/v2/farm.soltilemap"),
        include_str!("../../../tests/fixtures/tilemap/v3/farm.tilemap.lst"),
    ] {
        for _ in 0..1000 {
            let text = crate::tileset::tests::damaged(good, &mut next);
            read += usize::from(keeps_its_promises(&text, &set));
        }
    }
    // Both sides are reached: files read, and files refused.
    assert!((750..2250).contains(&read), "{read}");
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

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_reading_the_biggest_map_with_three_full_layers() {
    // The biggest map (4,096 × 4,096) with a new map's three layers all
    // painted is about the most a map file holds that's read (57 of its
    // 64 MB): it reads back as it was, within the reader's ten seconds.
    const TARGET_MS: f64 = 10_000.0;
    let mut map = TileMap::new([MAX_SIDE, MAX_SIDE], None, None);
    for layer in 0..3 {
        for y in 0..MAX_SIDE {
            for x in 0..MAX_SIDE {
                let code = if (x + y + layer).is_multiple_of(3) {
                    b't'
                } else {
                    b'.'
                };
                map.set_code(layer as usize, x, y, code).unwrap();
            }
        }
    }
    let text = write(&map);
    assert!(text.len() <= MAX_BYTES, "{}", text.len());
    let start = std::time::Instant::now();
    let back = read(&text).unwrap();
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    assert!(back.notes.is_empty(), "{:?}", back.notes);
    assert!(back.map == map, "it reads back as it was");
    let verdict = if ms <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: reading a 4,096 × 4,096 tile map with three full layers ({} MB) {ms:.0} ms (target {TARGET_MS} ms) {verdict}",
        text.len() >> 20
    );
    assert!(ms <= TARGET_MS, "{ms:.0} ms");
    // What holds more than Lust is handed is refused, not tried: a number
    // in each of a million and a half cells of the same map with one layer
    // painted (13 million numbers, texts and symbols; 12 million are read).
    let mut map = TileMap::new([MAX_SIDE, MAX_SIDE], None, Some(b'.'));
    map.add_value("growth", ValueKind::Number).unwrap();
    for i in 0..1_500_000 {
        let to = CellValue::Number(1 + (i % 5) as i32);
        map.set_value(0, i % MAX_SIDE, i / MAX_SIDE, to).unwrap();
    }
    let text = write(&map);
    assert!(text.len() <= MAX_BYTES, "{}", text.len());
    let why = read(&text).unwrap_err();
    assert!(why.message.starts_with("it's too big to read"), "{why}");
}
