//! The project's tags (`tags.lst`, stage 1): refused or read, within the
//! limits, each tag's name one game code can use and none twice, and
//! written and read back as the same list.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    fuzz_support::quiet_caught_panics();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(read) = level::tags::read(text) else {
        return;
    };
    let tags = read.tags;
    assert!(tags.len() <= level::tags::MAX_TAGS, "within the most");
    for tag in tags.iter() {
        assert_eq!(level::tags::name_problem(&tag.name), None, "{}", tag.name);
    }
    let again = level::tags::read(&level::tags::write(&tags)).expect("what's written reads");
    assert_eq!(again.tags, tags, "a list written reads back");
    assert!(again.notes.is_empty());
});
