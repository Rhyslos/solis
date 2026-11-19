//! A project's own font (stage 1, text layout): any bytes are refused
//! with why, or added and used to lay out text of every kind (wrapped,
//! cut, falling back to the game's Inter) and draw its letters into the
//! atlas, never a panic or a box that isn't finite.
#![no_main]

const INTER: &[u8] = include_bytes!("../../crates/editor/assets/fonts/Inter-Regular.ttf");

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    fuzz_support::quiet_caught_panics();
    let mut files = ui::FontFiles::new();
    if let Err(why) = files.add("fonts/Fuzzed.ttf", data.to_vec()) {
        assert!(!why.is_empty());
        return;
    }
    let (mut fonts, _) = ui::Fonts::new(vec![INTER.to_vec()]);
    fonts.use_files(Some(&files));
    fonts.begin_frame();
    let style = ui::TextStyle {
        fonts: vec!["fonts/Fuzzed.ttf".into()],
        ..ui::TextStyle::new(24.0, 0)
    };
    let fit = ui::Fit {
        width: Some(120.0),
        wrap: true,
        lines: Some(3),
        cut: true,
    };
    for text in ["Ag→ fi ffl", "Ωμέγα Жж", "日本 é\u{301} \u{2026}", "a\nb\r\nc"] {
        let paragraph = fonts.paragraph(text, &style, &fit);
        assert!(paragraph.size.iter().all(|v| v.is_finite()));
        for letter in fonts.place_paragraph(&paragraph, ui::Edge::Middle, [0.0, 0.0], 2.0, None) {
            assert!(letter.rect.iter().all(|v| v.is_finite()));
        }
    }
});
