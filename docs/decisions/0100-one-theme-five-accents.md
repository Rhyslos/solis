# 0100. One theme, five accents

- **Status:** Accepted (the project lead, 2026-11-11: "Replace both"; replaces [0096](0096-graphite-theme-and-rounded-panels.md); changes [0060](0060-amber-and-colours-that-mean-something.md); part of [0099](0099-the-g-layout.md))
- **Date:** 2026-11-11

## Problem

The editor had two themes, Standard and Graphite, a separate "Rounded panels" switch (decision 0096), and ten accent colours with white text on their fills (decisions 0004, 0060 and 0070). The project lead's design for the editor ([`docs/design/g-layout/`](../design/g-layout/README.md)) has one look: dark neutral greys with no warm or cool tint, five accents, and dark text on an accent fill. Asked what should happen to the two themes, the project lead answered: replace both.

## Options

1. **Add the design's look as a third theme.** Three palettes to keep readable and to test, and every screen of the design would need checking in looks it was never drawn for.
2. **Keep Standard beside the new look** for anyone who prefers it. The same cost, for a look whose layout is going away.
3. **One theme** (chosen): the design's, with its five accents.

## Decision

- **One theme, Neutral dark.** Every colour, size, radius and type style of the design is a named value in one file, `crates/editor/src/tokens.rs`, read through functions. The **Theme** and **Rounded panels** settings are gone; Standard and Graphite are removed.
- **Five accents:** Blue #5C9CFF (the standard), Grey #D4D4D4, Orange #F2994A, Teal #4FC2A8 and Purple #B48CFF. The accent marks what is selected, active or focused. **Text and icons on an accent fill are dark** (#0E0E0E); they were white.
- **An accent chosen before is mapped once:** Blue stays Blue; Petrol blue, Marine green and Olive become Teal; Red, Copper, Orange, Amber and Gold become Orange; Plum becomes Purple. Nothing maps to Grey, which is new. Nine of the ten old choices change colour.
- **Colours that mean something still keep theirs** (decision 0060's rule), now with the design's values: warnings #E8B04A, errors #F06A6A (as text #F08A84), success #4FC28A, links #7FB2FF, the axes X #E5534B, Y #6CC24A, Z #4C8DFF. They always come with a word or an icon, never colour alone.
- **Play is no longer blue.** Decision 0060 made Play and Build & Play petrol blue whatever the accent. In the design Play is **the one light button** (#E6E6E6 with dark text), which is what sets it apart, whatever the accent.
- **Two new settings: Interface scale** (100, 125, 150, 175 or 200%) and **Density** (Compact, Default, Comfortable: the height of rows, fields and shelf tabs).
- **Where the specification is silent** the values come from the mock-up it describes, or are Solis's own in the same language: menus, tooltips, scroll bars, a disabled control (faint text, no hover, a hover text saying why), the keyboard focus ring (a 1 px accent line), density's numbers and the scale's steps. They are listed in the plan's inventory, part (d).
- **Text is checked for contrast** as before (4.5 to 1 on its surface). Three faint greys are allowed below that, only for marks that repeat what something else says: a log line's time, a line number, the slash in a path.
- **The settings file:** the new look is saved under a new key. The old keys (the theme, rounded panels, the accent) are read once, to map the accent, and are never written again or removed, so an older build still finds what it saved.

## Consequences

- Today's whole editor turns Neutral dark in one step, before its layout changes. Some screens look half-way until their milestone, because egui's own controls can be pushed only so far towards the design's shapes.
- `theme` keeps giving the names the rest of the editor already uses (`theme::panel()`, `theme::text_weak()` …), now from the tokens, so no drawing code changes for the colours; each later milestone moves its own files to the tokens' names.
- The colour functions stay functions, not constants: a high-contrast palette, which the specification leaves open, could be added later without touching the code that draws.
- A colour written as numbers is allowed only in `tokens.rs` and `theme.rs`; a test checks.
- The design needs three font weights Solis didn't bundle (Inter Medium and Bold, JetBrains Mono Bold); they are bundled with it, from the families' own releases, under the same licence as the ones it had.
- The colours of the kinds of file (model, picture, sound and the rest) are not changed by this decision: the design has seven kinds and Solis sixteen, and which colour each of Solis's takes is asked separately.
