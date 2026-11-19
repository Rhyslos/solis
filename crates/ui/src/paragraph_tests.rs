//! Text laid out as paragraphs (stage 1, text layout): line breaking,
//! alignment, "…", fallback fonts, the project's own fonts, and keeping
//! what's laid out.

use crate::paragraph::{Fit, Paragraph, TextStyle};
use crate::screen::Edge;
use crate::text::{FontFiles, Fonts, MAX_FONT_BYTES, MAX_PROJECT_FONTS};
use std::sync::Arc;

const INTER: &[u8] = include_bytes!("../../editor/assets/fonts/Inter-Regular.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../../editor/assets/fonts/Inter-SemiBold.ttf");
const MONO: &[u8] = include_bytes!("../../editor/assets/fonts/JetBrainsMonoNL-Regular.ttf");

fn fonts() -> Fonts {
    let (fonts, notes) = Fonts::new(vec![INTER.to_vec(), INTER_BOLD.to_vec()]);
    assert!(notes.is_empty(), "{notes:?}");
    fonts
}

/// The game's fonts with the project's JetBrains Mono as `fonts/Mono.ttf`.
fn with_mono() -> Fonts {
    let mut files = FontFiles::new();
    files.add("fonts/Mono.ttf", MONO.to_vec()).unwrap();
    let mut f = fonts();
    f.use_files(Some(&files));
    f
}

fn style(size: f32) -> TextStyle {
    TextStyle::new(size, 0)
}

fn wrapped(width: f32) -> Fit {
    Fit {
        width: Some(width),
        wrap: true,
        ..Fit::default()
    }
}

/// Each line's words, as the text's characters from its first letter to
/// its last (spaces at its end left out).
fn lines_of(text: &str, p: &Paragraph) -> Vec<String> {
    p.lines
        .iter()
        .map(|line| {
            let glyphs = p.glyphs_of(line);
            let (Some(first), Some(last)) = (glyphs.first(), glyphs.last()) else {
                return String::new();
            };
            let from = first.cluster as usize;
            let to = last.cluster as usize;
            let end = to + text[to..].chars().next().map_or(0, char::len_utf8);
            text[from..end].trim_end().to_string()
        })
        .collect()
}

#[test]
fn words_wrap_inside_the_width_by_the_line_break_rules() {
    let mut f = fonts();
    let text = "The quick brown fox jumps over the lazy dog";
    let one = f.paragraph(text, &style(20.0), &Fit::default());
    assert_eq!(one.lines.len(), 1, "no width: one line");
    let p = f.paragraph(text, &style(20.0), &wrapped(160.0));
    let lines = lines_of(text, &p);
    assert!(lines.len() >= 3, "{lines:?}");
    assert_eq!(lines.join(" "), text, "every word, in order, none lost");
    for line in &p.lines {
        assert!(line.width <= 160.5, "{} wide", line.width);
    }
    // Each line as full as it can be: the next line's first word wouldn't
    // have fitted on it.
    for pair in p.lines.windows(2) {
        let next_word = lines_of(text, &p)[p.lines.iter().position(|l| *l == pair[1]).unwrap()]
            .split(' ')
            .next()
            .unwrap()
            .to_string();
        let word = f.paragraph(&format!(" {next_word}"), &style(20.0), &Fit::default());
        assert!(pair[0].width + word.size[0] > 160.0, "{next_word}");
    }
    // Lines are a line apart, and the paragraph as tall as all of them.
    let gap = p.lines[1].baseline - p.lines[0].baseline;
    assert!((p.size[1] - gap * p.lines.len() as f32).abs() < 0.01);
    // Hyphens and slashes are places to break too.
    let parts = "well-known and/or";
    let p = f.paragraph(parts, &style(20.0), &wrapped(60.0));
    assert!(
        lines_of(parts, &p).contains(&"well-".to_string()),
        "{:?}",
        lines_of(parts, &p)
    );
}

#[test]
fn a_word_too_long_for_its_line_breaks_between_its_letters() {
    let mut f = fonts();
    let word = "Supercalifragilisticexpialidocious";
    let p = f.paragraph(word, &style(20.0), &wrapped(80.0));
    let lines = lines_of(word, &p);
    assert!(lines.len() > 2, "{lines:?}");
    assert_eq!(lines.concat(), word, "no letter lost or repeated");
    for line in &p.lines {
        assert!(line.width <= 80.5, "{}", line.width);
    }
    // Before and after it, words still wrap as words.
    let text = format!("a {word} b");
    let p = f.paragraph(&text, &style(20.0), &wrapped(80.0));
    let lines = lines_of(&text, &p);
    assert_eq!(lines.first().map(String::as_str), Some("a"), "{lines:?}");
    assert!(lines.last().unwrap().ends_with(" b"), "{lines:?}");
    assert_eq!(lines.join("").replace(' ', ""), format!("a{word}b"));
    // A box too narrow for one letter still shows each, a line each.
    let p = f.paragraph("abc", &style(20.0), &wrapped(1.0));
    assert_eq!(p.lines.len(), 3);
    assert_eq!(p.letters(), 3);
}

#[test]
fn spaces_at_a_line_s_end_take_no_room_and_new_lines_are_kept() {
    let mut f = fonts();
    let bare = f.paragraph("Gold", &style(20.0), &Fit::default());
    let spaced = f.paragraph("Gold    ", &style(20.0), &Fit::default());
    assert_eq!(bare.size[0], spaced.size[0], "trailing spaces aren't width");
    // A space that ends a wrapped line doesn't push the line over.
    let p = f.paragraph("Gold Gold", &style(20.0), &wrapped(bare.size[0] + 1.0));
    assert_eq!(p.lines.len(), 2);
    assert_eq!(p.lines[0].width, bare.size[0]);
    // \n, \r\n and a text ending in \n start lines, as before.
    assert_eq!(
        f.paragraph("a\nb\r\nc", &style(20.0), &Fit::default())
            .lines
            .len(),
        3
    );
    assert_eq!(
        f.paragraph("a\n", &style(20.0), &Fit::default())
            .lines
            .len(),
        2
    );
    let empty = f.paragraph("", &style(20.0), &Fit::default());
    assert_eq!(empty.lines.len(), 1, "an empty text is one empty line");
    assert_eq!(empty.size[0], 0.0);
    assert!(empty.size[1] > 0.0);
}

#[test]
fn lines_align_left_in_the_middle_or_right() {
    let mut f = fonts();
    let text = "A long first line\nshort";
    let p = f.paragraph(text, &style(20.0), &Fit::default());
    let short = p.lines[1].width;
    let place = |f: &mut Fonts, align| f.place_paragraph(&p, align, [0.0, 0.0], 1.0, None);
    let first_of_short = |placed: &[crate::text::Placed]| {
        // The short line's first letter: the first below the first line.
        placed
            .iter()
            .find(|l| l.rect[1] > p.lines[0].baseline)
            .unwrap()
            .rect[0]
    };
    let left = first_of_short(&place(&mut f, Edge::Start));
    let middle = first_of_short(&place(&mut f, Edge::Middle));
    let right = first_of_short(&place(&mut f, Edge::End));
    let room = p.size[0] - short;
    assert!((middle - left - room / 2.0).abs() < 0.01, "{left} {middle}");
    assert!((right - left - room).abs() < 0.01, "{left} {right}");
}

#[test]
fn text_that_doesn_t_fit_is_cut_with_an_ellipsis_when_its_box_asks() {
    let mut f = fonts();
    let text = "Iron sword, sharp and old";
    let whole = f.paragraph(text, &style(20.0), &Fit::default());
    // One line, too wide for its box: cut, with "…", inside the width.
    let cut = Fit {
        width: Some(whole.size[0] * 0.6),
        cut: true,
        ..Fit::default()
    };
    let p = f.paragraph(text, &style(20.0), &cut);
    assert!(p.cut);
    assert_eq!(p.lines.len(), 1);
    assert!(p.size[0] <= whole.size[0] * 0.6 + 0.5, "{}", p.size[0]);
    assert!(p.letters() < whole.letters());
    let ellipsis = f.paragraph("\u{2026}", &style(20.0), &Fit::default());
    let last = *p.glyphs.last().unwrap();
    assert_eq!(last.id, ellipsis.glyphs[0].id, "it ends in …");
    // Without asking, it isn't cut (the box lets it spill).
    let uncut = f.paragraph(text, &style(20.0), &Fit { cut: false, ..cut });
    assert!(!uncut.cut);
    assert_eq!(uncut.letters(), whole.letters());
    // Wrapped text with room for two lines: the second ends in "…".
    let two = Fit {
        lines: Some(2),
        cut: true,
        ..wrapped(90.0)
    };
    let p = f.paragraph(text, &style(20.0), &two);
    assert!(p.cut);
    assert_eq!(p.lines.len(), 2);
    assert_eq!(p.glyphs.last().unwrap().id, ellipsis.glyphs[0].id);
    assert!(p.lines[1].width <= 90.5);
    // Text that fits isn't cut.
    let fits = f.paragraph("Iron", &style(20.0), &two);
    assert!(!fits.cut);
    // A box too narrow even for "…" shows just "…".
    let tiny = f.paragraph(
        text,
        &style(20.0),
        &Fit {
            width: Some(1.0),
            ..cut
        },
    );
    assert_eq!(tiny.letters(), 1);
}

#[test]
fn a_letter_the_first_font_lacks_comes_from_the_next() {
    let mut f = with_mono();
    // A letter JetBrains Mono has and Inter doesn't, and one the other way.
    let (mut mono_only, mut inter_only) = (None, None);
    for c in (0x21u32..0x3000).filter_map(char::from_u32) {
        let in_mono = f.fonts[2].font.has(c);
        let in_inter = f.fonts[0].font.has(c);
        if in_mono && !in_inter && mono_only.is_none() && !c.is_whitespace() {
            mono_only = Some(c);
        }
        if in_inter && !in_mono && inter_only.is_none() && !c.is_whitespace() {
            inter_only = Some(c);
        }
    }
    let (mono_only, inter_only) = (mono_only.unwrap(), inter_only.unwrap());
    // Mono first: its letters from it, Inter's from Inter (slot 0).
    let mut first_mono = style(20.0);
    first_mono.fonts = vec!["fonts/Mono.ttf".into()];
    let text = format!("a{inter_only}b");
    let p = f.paragraph(&text, &first_mono, &Fit::default());
    let slots: Vec<u16> = p.glyphs.iter().map(|g| g.slot).collect();
    assert_eq!(slots, [2, 0, 2], "{inter_only:?} falls to Inter");
    // The game's Inter first: a letter it lacks comes from the project's
    // font named after it.
    let text = format!("a{mono_only}b");
    let p = f.paragraph(&text, &first_mono, &Fit::default());
    assert!(p.glyphs.iter().all(|g| g.slot == 2));
    let mut fallback = style(20.0);
    fallback.fonts = vec!["fonts/Missing.ttf".into()];
    let p = f.paragraph(&text, &fallback, &Fit::default());
    let slots: Vec<u16> = p.glyphs.iter().map(|g| g.slot).collect();
    assert_eq!(slots, [0, 0, 0], "a font not in use is skipped");
    // A letter no font has: the first font's "missing" letter, never lost.
    let p = f.paragraph("a\u{e000}b", &first_mono, &Fit::default());
    assert_eq!(p.letters(), 3);
    // Spaces and accents stay with the letter before them.
    let p = f.paragraph(
        &format!("{inter_only} e\u{301}"),
        &first_mono,
        &Fit::default(),
    );
    assert_eq!(p.glyphs[1].slot, 0, "the space after Inter's letter");
    // Bold picks the game's bold (slot 1) for letters the named fonts lack.
    let mut bold = first_mono.clone();
    bold.font = 1;
    let p = f.paragraph(&format!("{inter_only}"), &bold, &Fit::default());
    assert_eq!(p.glyphs[0].slot, 1);
}

#[test]
fn a_project_s_font_is_read_checked_and_used_by_name() {
    let mut files = FontFiles::new();
    assert!(files.is_empty());
    files.add("fonts/Mono.ttf", MONO.to_vec()).unwrap();
    // Not a font, too big, or one too many: refused with why.
    let not = files.add("fonts/Notes.ttf", b"hello".to_vec()).unwrap_err();
    assert!(not.contains("isn't a TrueType or OpenType font"), "{not}");
    let big = files
        .add("fonts/Big.ttf", vec![0; MAX_FONT_BYTES + 1])
        .unwrap_err();
    assert!(big.contains("bigger than 32 MB"), "{big}");
    assert_eq!(files.names().collect::<Vec<_>>(), ["fonts/Mono.ttf"]);
    let mut many = FontFiles::new();
    for i in 0..MAX_PROJECT_FONTS {
        many.add(&format!("f{i}.ttf"), MONO.to_vec()).unwrap();
    }
    assert!(many.add("one-more.ttf", MONO.to_vec()).is_err());
    // Used by its name (any capitals, either slash): Mono's letters are
    // all one width, Inter's aren't.
    let mut f = fonts();
    f.use_files(Some(&files));
    let mut mono = style(20.0);
    mono.fonts = vec!["Fonts\\MONO.ttf".into()];
    let i = f.paragraph("iiii", &mono, &Fit::default()).size[0];
    let w = f.paragraph("wwww", &mono, &Fit::default()).size[0];
    assert!((i - w).abs() < 0.01, "{i} {w}");
    let i = f.paragraph("iiii", &style(20.0), &Fit::default()).size[0];
    let w = f.paragraph("wwww", &style(20.0), &Fit::default()).size[0];
    assert!(w > i * 2.0, "Inter: {i} {w}");
    // Taken away: laid out again in the game's own.
    f.use_files(None);
    let i = f.paragraph("iiii", &mono, &Fit::default()).size[0];
    let w = f.paragraph("wwww", &mono, &Fit::default()).size[0];
    assert!(w > i * 2.0, "back to Inter: {i} {w}");
}

#[test]
fn damaged_project_fonts_are_refused_or_lay_text_out_safely() {
    // A fuzz sweep (rule 9) of the project's own fonts, a fixed sequence:
    // each damaged JetBrains Mono is refused with why, or added and used
    // for text with every kind of letter, never a panic or a box that
    // isn't finite.
    let mut state = 0x7e47_u64;
    let mut next = move |below: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % below as u64) as usize
    };
    let mut refused = 0;
    for case in 0..150 {
        let mut bytes = MONO.to_vec();
        for _ in 0..1 + case % 10 {
            let reach = if next(2) == 0 { 2048 } else { bytes.len() };
            let at = next(reach);
            bytes[at] = next(256) as u8;
        }
        if case % 6 == 0 {
            bytes.truncate(next(bytes.len()));
        }
        let mut files = FontFiles::new();
        if let Err(why) = files.add("fonts/Damaged.ttf", bytes) {
            assert!(!why.is_empty());
            refused += 1;
            continue;
        }
        let mut f = fonts();
        f.use_files(Some(&files));
        f.begin_frame();
        let mut damaged = style(24.0);
        damaged.fonts = vec!["fonts/Damaged.ttf".into()];
        for text in [
            "Ag→ fi ffl",
            "Ωμέγα Жж",
            "日本 é\u{301}",
            "Long words wrap here",
        ] {
            let p = f.paragraph(text, &damaged, &wrapped(120.0));
            assert!(p.size.iter().all(|v| v.is_finite()), "case {case}");
            for l in f.place_paragraph(&p, Edge::Middle, [0.0, 0.0], 2.0, None) {
                assert!(l.rect.iter().all(|v| v.is_finite()), "case {case}");
            }
        }
    }
    assert!(refused < 150, "some damaged fonts still read");
}

#[test]
fn a_paragraph_is_kept_while_its_text_look_and_width_stay_the_same() {
    let mut f = fonts();
    let a = f.paragraph("Gold: 120", &style(20.0), &wrapped(200.0));
    let again = f.paragraph("Gold: 120", &style(20.0), &wrapped(200.0));
    assert!(Arc::ptr_eq(&a, &again), "kept, not laid out again");
    assert_eq!(f.paragraphs_kept(), 1);
    // Anything different is another paragraph.
    let mut spaced = style(20.0);
    spaced.letter_spacing = 2.0;
    let mut tall = style(20.0);
    tall.line_height = 1.5;
    let others = [
        f.paragraph("Gold: 121", &style(20.0), &wrapped(200.0)),
        f.paragraph("Gold: 120", &style(21.0), &wrapped(200.0)),
        f.paragraph("Gold: 120", &style(20.0), &wrapped(199.0)),
        f.paragraph("Gold: 120", &spaced, &wrapped(200.0)),
        f.paragraph("Gold: 120", &tall, &wrapped(200.0)),
        f.paragraph("Gold: 120", &TextStyle::new(20.0, 1), &wrapped(200.0)),
    ];
    for other in &others {
        assert!(!Arc::ptr_eq(&a, other));
    }
    assert_eq!(f.paragraphs_kept(), 7);
    // Spacing and line height do what they say.
    assert!(others[3].size[0] > a.size[0] + 2.0 * 8.0);
    let two = |s: &TextStyle, f: &mut Fonts| f.paragraph("a\nb", s, &Fit::default()).size[1];
    let (plain, taller) = (two(&style(20.0), &mut f), two(&tall, &mut f));
    assert!((taller - plain * 1.5).abs() < 0.01, "{plain} {taller}");
    // A text changing every frame never keeps more than the limit.
    for i in 0..crate::text::MAX_PARAGRAPHS * 2 {
        let _ = f.paragraph(&format!("Score {i}"), &style(20.0), &Fit::default());
    }
    assert!(f.paragraphs_kept() <= crate::text::MAX_PARAGRAPHS);
    // The project's fonts changing lays everything out again.
    let mut files = FontFiles::new();
    files.add("fonts/Mono.ttf", MONO.to_vec()).unwrap();
    f.use_files(Some(&files));
    assert_eq!(f.paragraphs_kept(), 0);
    f.use_files(Some(&files));
    let kept = f.paragraph("Gold", &style(20.0), &Fit::default());
    f.use_files(Some(&files));
    assert!(Arc::ptr_eq(
        &kept,
        &f.paragraph("Gold", &style(20.0), &Fit::default())
    ));
}

#[test]
fn hostile_numbers_never_give_a_box_that_isn_t_finite() {
    let mut f = fonts();
    for v in [f32::NAN, f32::INFINITY, -1.0e9, 0.0, 1.0e9] {
        let s = TextStyle {
            size: v,
            line_height: v,
            letter_spacing: v,
            ..style(20.0)
        };
        let fit = Fit {
            width: Some(v),
            wrap: true,
            lines: Some(0),
            cut: true,
        };
        let p = f.paragraph("Hello there, world", &s, &fit);
        assert!(p.size.iter().all(|x| x.is_finite()), "{v}");
        for l in f.place_paragraph(&p, Edge::End, [0.0, 0.0], v, None) {
            assert!(l.rect.iter().all(|x| x.is_finite()), "{v}");
        }
    }
    // At most the most letters a text holds.
    let long = "word ".repeat(crate::text::MAX_LETTERS);
    let p = f.paragraph(&long, &style(10.0), &wrapped(500.0));
    assert!(p.letters() <= crate::text::MAX_LETTERS);
}

#[test]
#[ignore = "scale check: run by tools/scale/scale-check.sh"]
fn scale_check_a_list_of_2000_changing_lines() {
    // Stage 1's text layout: a screen with a big list whose 2,000 lines
    // all change every frame is laid out within the frame (the words seen
    // before are shaped once; the lines are new each frame).
    const TARGET_MS: f64 = 8.0;
    let mut f = fonts();
    let frame = |f: &mut Fonts, n: usize| {
        for i in 0..2000 {
            let text = format!(
                "Item {}: {} gold, {} left",
                i + n * 7,
                (i * 37 + n) % 1000,
                (i + n) % 13
            );
            let _ = f.paragraph(&text, &style(20.0), &wrapped(400.0));
        }
    };
    frame(&mut f, 0);
    let mut best = f64::MAX;
    for n in 1..=10 {
        let start = std::time::Instant::now();
        frame(&mut f, n);
        best = best.min(start.elapsed().as_secs_f64() * 1000.0);
    }
    let verdict = if best <= TARGET_MS { "ok" } else { "SLOW" };
    println!(
        "Scale check: laying out 2,000 changing lines {best:.3} ms (target {TARGET_MS} ms) {verdict}"
    );
    assert!(best <= TARGET_MS, "{best:.3} ms");
}
