# Text layout: a library, or Solis's own

Stage 1's text layout milestone (the research: [`after-the-rethink/07-shared-5.md`](after-the-rethink/07-shared-5.md), M0 and its question 1) needs game screens' words to wrap inside a box, be shaped (kerning, joined letters, accents), align left, middle or right, take a line height and letter spacing, be cut with "…", and come from a project's own fonts with fallbacks. Two ways were compared (CONTRIBUTING.md, rule 22): fast, smooth and robust is the bar.

1. **`parley`** (Apache-2.0 or MIT): layout, line breaking, right-to-left text and shaping in one, the research's recommendation if it passes the checks.
2. **Solis's own:** where lines may break by Unicode's rules (`unicode-linebreak`, Apache-2.0), shaping with `harfrust` (MIT or Apache-2.0, already in the build: egui shapes with it), and the rest (choosing a font per letter, filling lines, alignment, "…") written in `ui`, about 450 lines.

Both keep the distance-image atlas (decision 0084): only where letters go is in question, not how they're drawn.

## How they were measured

The same paragraphs, the same font (Inter Regular at 20 pixels) and the same widths, in one program ([`text-layout-study/`](text-layout-study/README.md)), built for release and run on the build machine (two cores of a 2.1 GHz Xeon). Each set is laid out ten times, a frame each, with the text changing every frame (a list's numbers count up), and the best frame is kept. Every way gave the same lines and the same letters (2,000 lines and 53,282 letters for the list, 3,600 and 115,690 for the long paragraphs), so they did the same work.

Solis's own was measured twice: shaping each piece between two break chances afresh, and remembering each piece shaped by its words (text that changes every frame is mostly words seen before: "Item", "gold,", numbers below a thousand).

| Paragraphs | parley | own, shaped afresh | own, words remembered |
| --- | --- | --- | --- |
| 2,000 changing lines (a list) | 32.20 ms | 21.24 ms | 0.77 ms |
| 200 paragraphs, 4 sentences each | 49.66 ms | 37.97 ms | 1.19 ms |
| 200 Greek and Cyrillic paragraphs | 23.45 ms | 14.89 ms | 0.59 ms |

A frame at 60 frames a second has 16.7 ms for everything. parley lays the changing list out in twice that; it has no way to keep words shaped from one layout to the next (each layout is shaped from nothing), so a screen of changing text costs it the full price every frame. Remembering words makes Solis's own about forty times faster on the same text.

The milestone's scale case, laid out through the engine as screens use it (`ui`'s `scale_check_a_list_of_2000_changing_lines`: 2,000 changing lines a frame, wrapped at 400 pixels, each paragraph made and kept), takes **3.8 ms** a frame, under its target of 8.

## What each brings into the build

- **parley** brings nine new libraries, about 33 MB of source: `parley`, `fontique` (with `memmap2`), `parley_data`, `parlance`, `icu_segmenter` and its data (23 MB and 9 MB), and `icu_locale_fallback` with its data. Their licences (Apache-2.0, MIT, Unicode-3.0) all pass `cargo deny check`. Only parley 0.11.1 uses the `harfrust` and `skrifa` already in the build; the newest (0.13) needs `skrifa` 0.49 and `harfrust` 0.15, a second copy of each beside egui's, so Solis would be held to an older parley until egui moves.
- **Solis's own** brings one: `unicode-linebreak` (116 KB, Unicode's line-break table and its pairs), and uses `harfrust` directly, already in the build.

## What each does that the other doesn't

- **Right-to-left text** (Arabic, Hebrew) needs letters reordered by Unicode's bidirectional rules: parley does it; Solis's own doesn't yet. The project lead's writing systems (the research's question 2, recommended answer taken) are Latin, Greek and Cyrillic now, Chinese, Japanese and Korean through fallback fonts, and right-to-left later. When it comes, Solis's own adds the reordering of a line's runs (`unicode-bidi`, MIT or Apache-2.0, is small, or its rules written in `ui`).
- **Pictures inside text** (text effects' `[icon=Coin]`, the next milestone): parley has inline boxes; Solis's own lays a box out as a letter of a given width, a few lines in `paragraph.rs`, since each line is filled letter by letter already.
- **Thai, Lao and Khmer** break lines between words found with a dictionary: parley's `icu_segmenter` has one, `unicode-linebreak` breaks them only at spaces. Not among the writing systems asked for.
- **"…" and fallback by file:** neither library cuts text with "…", so it is Solis's either way. parley picks fonts by family name from the fonts' own tables, so two files of one family (a project's own copy of Inter beside the game's) can't be told apart; Solis's own picks by the file the screen names.

## What was chosen

**Solis's own layout** ([decision 0129](../decisions/0129-text-layout.md)): faster where it matters (changing text, every frame), one small library instead of nine, no held version, and the fonts a screen names are the files it means. parley passes the checks, so the research's first choice was possible; the measurement is why it wasn't taken. Right-to-left text is the one thing given up for now, and it's later in the plan anyway.

Built in `crates/ui`: `shaping.rs` (runs shaped with `harfrust`, remembered by their words, at most 16,384 a font), `paragraph.rs` (a font per letter from the style's list, lines filled at Unicode's break chances, long words broken between letters, "…"), `text.rs` (paragraphs kept while their text, look and width stay the same, at most 8,192; placing their letters from the atlas) and `font_files.rs` (a project's own fonts, checked as they're added).
