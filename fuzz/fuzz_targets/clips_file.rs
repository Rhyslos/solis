//! A clips file (`.clips.lst`, Fixes 3): refused or read, and what's read
//! is within the limits, each clip's name good and used once, each row a
//! direction its clip has (none on a mirrored west side), and writes back
//! to itself.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    fuzz_support::quiet_caught_panics();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    use animation::clips_file as c;
    let Ok(read) = c::read(text) else {
        return;
    };
    let file = &read.file;
    assert!(file.clips.len() + file.tags.len() <= c::MAX_CLIPS);
    assert!(file.looks.len() <= c::MAX_LOOKS && file.cells.len() <= c::MAX_CELLS);
    for (i, clip) in file.clips.iter().enumerate() {
        assert!(c::good_name(&clip.name));
        assert!(!file.clips[..i].iter().any(|o| o.name == clip.name));
        assert!(matches!(clip.directions, 1 | 4 | 8));
        for way in &clip.ways {
            assert!(!(clip.mirror && way.facing.is_west()));
            assert!(way.cells.len() <= c::MAX_FRAMES);
        }
    }
    let again = c::read(&c::write(file, "picture.png")).expect("what's written reads back");
    assert_eq!(&again.file, file);
});
