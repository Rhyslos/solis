//! An Aseprite file (`.aseprite`, Phase 9, D2): refused or read within the
//! limits, every frame the canvas's size, tags within the frames, and
//! packed into a sheet whose frames draw back the same.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
    let limits = assets::aseprite::AseLimits {
        max_side: 256,
        max_frames: 64,
        ..assets::aseprite::AseLimits::default()
    };
    let Ok(file) = assets::aseprite::read_aseprite_within(data, limits) else {
        return;
    };
    let [w, h] = file.size;
    assert!(w >= 1 && h >= 1 && w <= 256 && h <= 256);
    for frame in &file.frames {
        assert_eq!(frame.rgba.len(), (w * h * 4) as usize);
    }
    for tag in &file.tags {
        assert!(tag.from <= tag.to && (tag.to as usize) < file.frames.len());
    }
    let frames: Vec<Vec<u8>> = file.frames.iter().map(|f| f.rgba.clone()).collect();
    let Ok(sheet) = assets::sprite_sheet::build_sheet(file) else {
        return;
    };
    for (i, rgba) in frames.iter().enumerate() {
        // See-through pixels may differ in colour, never in what shows.
        let back = sheet.frame_rgba(i).expect("every frame is in the sheet");
        for (a, b) in rgba.chunks(4).zip(back.chunks(4)) {
            if a[3] != 0 || b[3] != 0 {
                assert_eq!(a, b);
            }
        }
    }
});

/// Panics Solis catches on purpose aren't failures: only one that escapes
/// Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
