//! A tileset's picture (Phase 9, D5): a PNG, JPEG or Aseprite file (the
//! first byte chooses which reader), refused or read whole within the
//! limits, every frame the picture's size; then its grid found and its
//! tiles weighed without panicking.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
    let Some((&first, bytes)) = data.split_first() else {
        return;
    };
    let Ok(art) = assets::tile_art::read_tile_art(bytes, first & 1 == 1) else {
        return;
    };
    let [w, h] = art.size;
    assert!(w <= assets::tile_art::MAX_SIDE && h <= assets::tile_art::MAX_SIDE);
    assert_eq!(art.frames.len(), art.durations.len());
    assert!(!art.frames.is_empty());
    for f in &art.frames {
        assert_eq!(f.len(), (w * h * 4) as usize);
    }
    let tile = tiles::autotile::detect_tile(art.size, &art.frames[0], [16, 16]);
    assert!(tile[0] >= 1 && tile[1] >= 1);
    let grid = [w / tile[0].max(1), h / tile[1].max(1)];
    let fill = |at| tiles::autotile::tile_fill(art.size, &art.frames[0], tile, at);
    let _ = tiles::autotile::guess_kind(fill, [0, 0], grid);
    let _ = tiles::autotile::mark_up(tiles::TileKind::Blob47, [0, 0], |at| fill(at).0);
});

/// Panics Solis catches on purpose aren't failures: only one that escapes
/// Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
