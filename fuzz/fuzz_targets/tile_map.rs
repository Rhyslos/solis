//! A tile map file (`.soltilemap`, Phase 9, D4): refused or read, and
//! what's read is within the limits, writes back to itself, is autotiled
//! with the farm fixture's tileset (D5), and is changed cell by cell,
//! resized and summed without panicking.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(read) = tiles::map_file::read(text) else {
        return;
    };
    use tiles::map::{MAX_LAYERS, MAX_OPTIONS, MAX_SIDE, MAX_VALUES};
    use tiles::{CellValue, ValueKind};
    let mut map = read.map;
    let [w, h] = map.size();
    assert!((1..=MAX_SIDE).contains(&w) && (1..=MAX_SIDE).contains(&h));
    assert!(map.layers().len() <= MAX_LAYERS);
    assert!(map.values().len() <= MAX_VALUES);
    for v in map.values() {
        if let ValueKind::Choice(options) = &v.kind {
            assert!(options.len() <= MAX_OPTIONS);
        }
    }
    for l in 0..map.layers().len() {
        assert_eq!(map.layer_cells(l).map(<[u8]>::len), Some(w as usize * h as usize));
    }
    let again = tiles::map_file::read(&tiles::map_file::write(&map)).expect("what's written reads");
    assert_eq!(again.map, map);
    // Autotiled with a tileset of every kind, its first chunk.
    let set = tiles::tileset::read(include_str!("../../tests/fixtures/tileset/v2/farm.soltileset"))
        .expect("the fixture reads")
        .tileset;
    for l in 0..map.layers().len() {
        let _ = tiles::autotile::chunk_tiles(&map, l, &set, [0, 0]);
    }
    // Changed, at its corners and past them.
    let sum = map.checksum();
    for (x, y) in [(0, 0), (w - 1, h - 1), (w, 0), (0, h), (u32::MAX, 3)] {
        for l in 0..=map.layers().len() {
            let _ = map.set_code(l, x, y, b't');
        }
        for v in 0..=map.values().len() {
            for to in [CellValue::Number(i32::MAX), CellValue::Switch(true), CellValue::Choice(Some(200))] {
                let _ = map.set_value(v, x, y, to);
            }
        }
    }
    let _ = map.take_dirty();
    let _ = map.checksum() == sum;
    map.resize([w.min(40) + 1, h.min(40)]);
    for l in 0..map.layers().len() {
        let _ = map.counts(l);
    }
    let _ = map.checksum();
});

/// Panics Solis catches on purpose aren't failures: only one that escapes
/// Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
