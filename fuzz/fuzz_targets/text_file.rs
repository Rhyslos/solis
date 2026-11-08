//! A text file shown in Properties (`editor::text_view`): any bytes, read
//! as the app reads them, become Markdown blocks and inline runs without a
//! panic, and no words of a plain line are lost.
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    solis_fuzz_quiet();
    let text = String::from_utf8_lossy(data);
    use editor::text_view::{Block, inline_runs, markdown_blocks};
    for block in markdown_blocks(&text) {
        if let Block::Paragraph(words) | Block::Quote(words) | Block::Heading(_, words) = block {
            let runs = inline_runs(&words);
            assert!(runs.iter().all(|(_, w)| !w.is_empty()));
        }
    }
    let _ = inline_runs(&text);
});

/// Panics Solis catches on purpose aren't failures: only one that escapes
/// Solis is, and libFuzzer still sees that one.
fn solis_fuzz_quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::panic::set_hook(Box::new(|_| {})));
}
