//! A screen's Lust file as the UI editor reads it (decision 0089): read
//! or refused, never a panic, and every piece's places inside the text;
//! setting a style, words, taking a piece out and moving it never panic.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    fuzz_support::quiet_caught_panics();
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    use ui::source as s;
    let Ok(screens) = s::parse(text) else {
        return;
    };
    for screen in &screens {
        let mut path = vec![0];
        while let Some(piece) = screen.view.at(&path) {
            assert!(piece.span.end <= text.len() && piece.span.start < piece.span.end);
            assert!(
                text.is_char_boundary(piece.span.start) && text.is_char_boundary(piece.span.end)
            );
            let _ = s::set_style(text, piece, "gap", Some("4"));
            let _ = s::remove_styles(text, piece, &["gap"]);
            let _ = s::set_words(text, piece, "x");
            let _ = s::remove_piece(text, piece);
            let _ = s::move_piece(text, &screen.name, &path, &[], 0);
            path.push(0);
        }
    }
});
