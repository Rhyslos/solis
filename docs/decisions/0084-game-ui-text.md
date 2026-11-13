# 0084. Game UI text: letters as distance images from ab_glyph

- **Status:** Accepted (carries out the text half of 0083's first milestone; changes the game UI plan's choice of `parley` and `fdsm`). Fonts are read with `skrifa` since [0107](0107-game-ui-fonts-read-with-skrifa.md): `ab_glyph` was no longer in egui.
- **Date:** 2026-11-05

## Problem

Game UI needs text (the day, the time, money, labels) that stays sharp at any size, from Full HD to 4K, and hard-edged for pixel-art screens. The game UI plan chose `parley` to lay text out and `fdsm` to make each letter a multi-channel distance image. Both are larger than the first text needs, and `fdsm` pulls in another font reader beside the one already in the build.

## Options

- **The plan's `parley` and `fdsm`:** line breaking, right-to-left text and shaping, and multi-channel letters with the sharpest corners. Two new libraries and a second font reader; most of it unused until wrapping and other scripts arrive.
- **`cosmic-text` and `glyphon`:** good, but letters are bitmaps at each size, so outlines and glow would need extra passes (the plan's reason for passing them over).
- **`ab_glyph` and our own single-channel distance images (chosen):** `ab_glyph` is already in the build (egui reads its fonts with it), reads TrueType and OpenType outlines, kerning and advances; each letter is drawn once at 48 pixels and turned into a single-channel distance image by a small search (about 30 lines, tested). Corners are a touch rounder than multi-channel at very large sizes, which UI text doesn't reach.

## Decision

- A text element (`ElementKind::Text`: words, size in design pixels, which font, and alignment across its box) is laid out by `ui::Fonts`: left to right with each letter's advance and the font's kerning, a line per `\n`, the lines centred down the element's box. Long lines aren't wrapped yet.
- Each letter is made once, the first time it's shown, into a 1024 × 1024 one-byte atlas (128 on the letter's edge, reaching 6 pixels past it), and drawn at any size from that one image. `render`'s UI shader reads the atlas and works out the edge as it does for boxes, so outlines and glow can follow.
- Under the **whole steps** rule, each letter lands on the game's pixel grid and is drawn hard-edged, like the shapes.
- The fonts are Inter Regular and Inter SemiBold (the editor's, already in the program) until a project can pick its own fonts and the pixel font the plan asks for. A font file that isn't a font, or is over 32 MB, is left out with a note; a text holds at most 4,096 letters; a full atlas leaves later letters out and says so.

## Consequences

- No new library in the build; `ab_glyph` is now used by `ui` directly (docs/dependencies.md).
- Letters are made on the frame they first appear: a few milliseconds each in a debug build, once. Making them on a worker, wrapping, shaping and right-to-left text come when they're needed (the plan's later milestones), and may bring `parley` back for layout.
- The atlas never frees letters yet; with one or two fonts at UI sizes it holds every Latin letter several times over.
