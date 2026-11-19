# 0129. Text layout: Solis's own, shaped and wrapped, in a project's fonts with fallbacks

- **Status:** Accepted (builds on 0084, 0087, 0089, 0091, 0107 and 0122; the project lead asked for the recommended answer to every choice while they slept, 2026-11-18)
- **Date:** 2026-11-19

## Problem

Stage 1's next milestone (decision 0122; [report 07](../reports/after-the-rethink/07-shared-5.md), M0) is text layout. Game screens' text was laid out left to right, a line per `\n`, unshaped, wrapped only by the view between words in the game's Inter (decision 0084). Text effects, translations and dialogue all need words that wrap in a box, letters shaped (kerning, joined letters, accents), alignment, spacing, "…" when text doesn't fit, a project's own fonts and fallbacks for letters a font lacks.

## Options

- **The library** (the research's question 1): `parley` if it and what it brings pass the checks (recommended), or Solis's own with `unicode-linebreak` and `harfrust`. Compared in [the study](../reports/text-layout-study.md) (rule 22).
- **Writing systems** (question 2): Latin, Greek and Cyrillic now, Chinese, Japanese and Korean through fallback fonts, right-to-left later (recommended); all now; Latin only.
- **How a screen names its fonts:** Lust's own `font "fonts/Lora.ttf"` style (a file, as Lust's windows have it); a new style taking the project's `Font` names (a new Lust word).
- **How a box asks for "…", line height and letter spacing:** new Lust style words (a change to Lust, which stays as it is: decision 0087); or what Lust already has.

## Decision

- **Solis's own layout**, not parley. parley passes `cargo deny check`, but lays a changing list of 2,000 lines out in 32 ms a frame against Solis's 0.8 ms (each word shaped once and remembered), brings nine libraries (33 MB of source) against one, and only its older 0.11 shares egui's `harfrust` and `skrifa`. Lines break where Unicode's line-break rules allow (`unicode-linebreak`), letters are shaped with `harfrust` in runs of one font, a word too long for its line breaks between its letters, and spaces at a line's end take no room. The distance-image atlas stays (decision 0084).
- **Writing systems:** Latin, Greek and Cyrillic with the game's Inter; Chinese, Japanese and Korean when a project adds a font that has them and lists it as a fallback; **right-to-left text comes later** (it needs Unicode's bidirectional reordering), and Thai, Lao and Khmer break only at spaces.
- **A project's own fonts:** a TrueType or OpenType file (`.ttf`, `.otf`) in the Library is a **Font** (its own kind and icon; the project's names gain `state Font`, `Font.Fonts.Lora` for `fonts/Lora.ttf`, which fields can pick and which follows a rename). Each is read on a worker within 32 MB and checked (`ui::FontFiles::add`), refused with why in plain words; at most 32 a project's screens use. Fuzzed (`fuzz/fuzz_targets/font_file.rs`).
- **Screens name fonts with Lust's own `font` style:** `font "fonts/Lora.ttf"` on a piece, or on the view's own `[ ]` for every piece without one. **Fallbacks** are written in the same text between commas, first choice first: `font "fonts/Lora.ttf, fonts/NotoSansJP.otf"`. A letter the first lacks comes from the next, then from the game's Inter (its bold for `bold`); a letter none has shows the first font's box. A font named that isn't in the project, isn't a font or can't be read is in Problems at its line (or said once in the Console when worked out as the game runs).
- **Alignment** is Lust's `left`, `center` and `right` on a text, now per line of a wrapped paragraph.
- **"…": a box asks by limiting the text's own size**, with what Lust already has: a wrapping text with a `height` or `maxHeight` is cut after the last line that fits, its last line ending in "…"; a `nowrap` text with a `width` or `maxWidth` is cut on its line. Other text keeps its old behaviour, spilling past its box.
- **Line height and letter spacing are settings of the Screen component** (with its glow, outline and pixel size, decision 0089): **Line height** (50 to 300 %, 100 the font's own) and **Letter spacing** (−10 to 50 design pixels, 0). Properties draws them from the component's schema; nothing new to mock up.
- **Kept, not laid out again:** a paragraph is laid out in design pixels (so one layout serves every screen size) and kept by its text, look and width; at most 8,192 are kept, those asked for longest ago going first. Each run shaped is kept by its words (at most 16,384 a font). The player's text size (decision 0091) still grows text and the room round it.
- **The UI editor's canvas** draws the same layout through the renderer, with the project's fonts. The editor's own interface text stays egui's.

## Consequences

- `ui::ElementKind::Text` carries a `TextStyle` (size, the game's font, the project's fonts, line height, letter spacing) and a `Fit` (width, wrap, most lines, cut); a screen carries the project's fonts (`Screen::fonts`) as it carries its pictures. `ui::view::Measure` measures a whole text in its style and room, not a word at a time.
- Text is shaped now, so wrapped lines and kerned pairs sit a little differently from before (Inter's kerning was never read: it's in a table the old layout didn't read). No approved picture of the editor showed game UI text, so none changed; the new picture is `crates/render/tests/data/text-layout.png`.
- A request for Lust, ready to send (Lust stays as it is until it has them): style words `lineHeight 1.4` and `letterSpacing 2` on a text, `lines 2` (at most so many lines, the last ending in "…") and `ellipsis` (cut a `nowrap` line that doesn't fit with "…"), and `font` taking a list of files (`font ["fonts/Lora.ttf", "fonts/NotoSansJP.otf"]`), the first that has a letter drawing it. Lust's own windows would then lay text out as Solis's screens do; today `lust run` reads a fallback list as one file it can't find and uses Inter.
- Right-to-left text, dictionary line breaks and pictures inside text are later work; the study says what each needs.
