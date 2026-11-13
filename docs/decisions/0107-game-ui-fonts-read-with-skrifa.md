# 0107. Game UI's fonts are read with `skrifa`

- **Status:** Accepted (changes how 0084 reads fonts; the rest of 0084 stands)
- **Date:** 2026-11-13

## Problem

Decision 0084 chose `ab_glyph` to read game UI's fonts because it was "already in the build (egui reads its fonts with it)". The November 2026 review (DEP-01) found that no longer true: egui 0.36 reads fonts with `skrifa`, and `ab_glyph` was in the Windows build only because `ui` used it. Underneath, `ab_glyph` reads fonts with `ttf-parser`, whose author has said it is unmaintained (RustSec advisory RUSTSEC-2026-0192). `deny.toml` let the advisory through as "font reading inside winit's Linux window decorations", which was wrong for the one platform Solis ships on: on Windows the unmaintained reader was there for game UI's text, reading font files up to 32 MB.

## Options

- **Keep `ab_glyph`,** correct the reasons in `deny.toml`, the register and `Cargo.toml`, and add a fuzzer for fonts. Two font readers ship, one unmaintained.
- **Read fonts with `skrifa` (chosen),** the reader egui already uses, at egui's version: maintained (the advisory names it as the alternative), and already compiled into the program. Its outlines are drawn into coverage by `ab_glyph_rasterizer`, the small rasterizer `ab_glyph` itself used (no font reading in it), so letters come out exactly as before.
- **Shape text with `harfrust`** (also in egui): kerning from the font's positioning tables and other scripts. More than game UI needs until wrapping and other scripts arrive (0084's later milestones).

## Decision

- `ui::Fonts` reads each font with `skrifa` (`crates/ui/src/font.rs`, the only place that uses it): letters from the font's character map, advances, the line's ascent, descent and gap, and kerning from the `kern` table's horizontal pairs and classes, as before.
- Sizes keep 0084's meaning: pixels from the top of the font's ascent to the bottom of its descent. A font whose height isn't above zero is left out with a note.
- Each letter's outline is drawn by `ab_glyph_rasterizer` at the distance images' size (48 pixels) over the same whole pixels as before. The letters' boxes, places in the atlas and distance images are the same as `ab_glyph` gave (checked by hand for Latin and accented letters, arrows and the euro sign).

## Consequences

- The Windows build no longer carries `ab_glyph`, `owned_ttf_parser` or `ttf-parser`; one font reader ships. `ttf-parser` stays in the Linux build only, inside winit's window decorations, which is now what `deny.toml` says.
- `ab_glyph_rasterizer` is a direct dependency of `ui` (it was in the build already, under `ab_glyph`). Both libraries are in the register (docs/dependencies.md).
- `skrifa` moves when egui moves: the workspace asks for egui's version, so the program keeps one copy.
- A sweep of damaged fonts (text_tests.rs) checks that a broken font is left out or laid out with finite boxes, never a crash.
