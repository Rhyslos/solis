use super::*;

#[test]
fn a_project_with_no_file_is_written_in_english() {
    let languages = Languages::default();
    assert_eq!(languages.main(), "en");
    assert_eq!(languages.all(), ["en"]);
    assert!(languages.others().is_empty());
}

#[test]
fn languages_round_trip_with_the_main_one_first() {
    let mut languages = Languages::default();
    languages.add("nb").unwrap();
    languages.add("pt-BR").unwrap();
    assert!(languages.set_main("nb"));
    let text = write(&languages);
    let read = read(&text).unwrap();
    assert!(read.notes.is_empty(), "{:?}", read.notes);
    assert_eq!(read.languages, languages);
    assert_eq!(read.languages.main(), "nb");
    assert_eq!(read.languages.others(), ["en", "pt-BR"]);
}

#[test]
fn codes_are_checked_in_plain_words() {
    for good in ["en", "nb", "fil", "pt-BR", "zh-Hans", "sr-Latn-RS"] {
        assert_eq!(code_problem(good), None, "{good}");
    }
    for bad in [
        "",
        "E",
        "English",
        "EN",
        "en_GB",
        "en-",
        "en-x",
        "zh-Hans-TW-extra-long",
    ] {
        let why = code_problem(bad).unwrap_or_else(|| panic!("{bad} taken"));
        assert!(why.contains("such as en"), "{why}");
    }
}

#[test]
fn adding_twice_or_past_the_limit_is_refused() {
    let mut languages = Languages::default();
    assert!(languages.add("en").unwrap_err().contains("already"));
    for n in 0..MAX_LANGUAGES - 1 {
        let code = format!("x{}", (b'a' + (n % 26) as u8) as char);
        let code = if n >= 26 {
            format!("{code}-{n:02}")
        } else {
            code
        };
        languages.add(&code).unwrap();
    }
    assert!(languages.add("fr").unwrap_err().contains("at most"));
}

#[test]
fn the_main_language_cant_be_taken_out() {
    let mut languages = Languages::default();
    languages.add("de").unwrap();
    assert!(!languages.remove("en"));
    assert!(languages.remove("de"));
    assert!(!languages.remove("de"), "already gone");
}

#[test]
fn a_damaged_list_keeps_what_it_can() {
    let text = "-- x\nfetch \"solis/data/languages\":[Languages];\n\nlock int version = 1;\n\
                lock Languages languages = new Languages { main = \"Klingon\", all = [\"en\", \"fr\", \"fr\", \"x\"] };\nend\n";
    let read = read(text).unwrap();
    assert_eq!(read.languages.all(), ["en", "fr"]);
    assert_eq!(read.notes.len(), 2, "{:?}", read.notes);
    assert!(read.notes[0].contains("Klingon"));
}

#[test]
fn a_newer_file_is_refused() {
    let text = write(&Languages::default()).replace("version = 1", "version = 9");
    assert!(read(&text).unwrap_err().to_string().contains("newer"));
}

#[test]
fn hostile_text_never_panics() {
    let good = write(&Languages::default());
    for cut in 0..good.len() {
        if good.is_char_boundary(cut) {
            let _ = read(&good[..cut]);
        }
    }
    let _ = read(&"x".repeat(MAX_BYTES + 1));
}
