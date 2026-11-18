//! A ground's settings file (`.ground.lst`, decision 0116): refused or
//! read, within the limits (a size and height a ground can have, at least
//! one kind and at most 256, each named), and written back to itself.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    fuzz_support::quiet_caught_panics();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    use ground::settings as s;
    let Ok(read) = s::read(text) else {
        return;
    };
    let settings = &read.settings;
    assert!(s::Settings::check_size(settings.size, settings.height).is_ok());
    assert!(!settings.kinds.is_empty() && settings.kinds.len() <= s::MAX_KINDS);
    assert!(settings.kinds.iter().all(|k| !k.name.trim().is_empty()));
    let again = s::read(&s::write(settings)).expect("what's written reads back");
    assert_eq!(&again.settings, settings);
});
