//! A tileset (`.soltileset`, Phase 9, D4 and D5): refused or read, and
//! what's read is within the limits, has each name and code once, writes
//! back to itself, and autotiles a small map of all its terrains without
//! panicking, every tile it places inside the map's reach.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(read) = tiles::tileset::read(text) else {
        return;
    };
    use tiles::tileset as t;
    let set = &read.tileset;
    assert!(set.terrains.len() <= t::MAX_TERRAINS);
    for (i, terrain) in set.terrains.iter().enumerate() {
        assert!(t::usable_code(terrain.code), "{:?}", terrain.code);
        assert!(!terrain.name.is_empty() && terrain.name.chars().count() <= t::MAX_NAME);
        assert!(terrain.colour.iter().all(|c| (0.0..=1.0).contains(c)));
        assert_eq!(set.by_code(terrain.code).map(|f| f.id), Some(terrain.id), "{i}");
        assert_eq!(set.by_name(&terrain.name).map(|f| f.id), Some(terrain.id), "{i}");
    }
    let again = t::read(&t::write(set)).expect("what's written reads");
    assert_eq!(&again.tileset, set);
    if let Some(code) = set.free_code() {
        assert!(set.by_code(code).is_none() && t::usable_code(code));
    }
    for p in &set.pieces {
        let kind = set.terrain(p.terrain).expect("a piece's terrain is there").kind;
        assert!(p.slot < tiles::autotile::slot_count(kind));
    }
    // A 9 × 7 map of every terrain in turn, and gaps.
    let mut map = tiles::TileMap::new([9, 7], None, None);
    for y in 0..7u32 {
        for x in 0..9u32 {
            let i = (x * 7 + y * 3) as usize;
            if let Some(terrain) = set.terrains.get(i % (set.terrains.len() + 1)) {
                let _ = map.set_code(0, x, y, terrain.code as u8);
            }
        }
    }
    let placed = tiles::autotile::chunk_tiles(&map, 0, set, [0, 0]);
    for p in placed {
        assert!((-1..=18).contains(&p.x) && (-1..=14).contains(&p.y), "{p:?}");
        if let tiles::autotile::Art::Piece(i) = p.art {
            assert!(i < set.pieces.len());
        }
    }
    for t in &set.terrains {
        let _ = tiles::autotile::missing_slots(set, t);
    }
});

/// Panics Solis catches on purpose aren't failures: only one that escapes
/// Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
