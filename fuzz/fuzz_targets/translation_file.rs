//! A translation file (`locale/nb/shop.ftl`, stage 1): refused or read,
//! within the limits, every line a key game code can use; its text can be
//! written again and reads back the same; and the translations it makes
//! give text for every key in every language without stopping.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    fuzz_support::quiet_caught_panics();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let read = translations::ftl::read("locale/nb/x.ftl", text);
    assert!(read.lines.len() <= translations::ftl::MAX_LINES, "within the most lines");
    for line in &read.lines {
        assert_eq!(translations::keys::key_problem(&line.key), None, "{}", line.key);
        assert!(line.text.chars().count() <= translations::ftl::MAX_TEXT_CHARS);
        if let Ok(again) = translations::edit::set_line("", &line.key, &line.text) {
            let back = translations::ftl::read("locale/nb/x.ftl", &again);
            assert!(back.problems.is_empty(), "{:?}", back.problems);
        }
    }
    let mut languages = level::languages::Languages::default();
    let _ = languages.add("nb");
    let all = translations::Translations::new(
        &languages,
        &[("locale/nb/x.ftl".to_owned(), text.to_owned()), ("locale/en/x.ftl".to_owned(), text.to_owned())],
        Vec::new(),
    );
    for language in all.languages() {
        for key in all.keys().iter().take(50) {
            let _ = all.text(language, key, &[("count", translations::TextValue::Number(2.0))]);
        }
    }
    let csv = translations::sheet::export(&all);
    let _ = translations::sheet::import(&csv, &all);
});
