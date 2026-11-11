# 0101. Unfinished parts are shown as placeholders, and one setting hides them

- **Status:** Accepted (the project lead, 2026-11-11; changes [0070](0070-usable-first.md) and [0085](0085-the-editor-overhaul.md), "nothing that isn't built is shown"; part of [0099](0099-the-g-layout.md))
- **Date:** 2026-11-11

## Problem

Decision 0070 took every dead button out of the editor: nothing that isn't built is shown, and decision 0085 repeated it. The project lead's design for the editor shows a whole engine, and about half of what it draws Solis doesn't have yet (164 of 332 elements, three whole screens among them). Building only what works would leave a ribbon full of holes and a layout that isn't the one designed. The project lead asked for the opposite of 0070 for the new interface: features the engine lacks get a placeholder page or element, and a setting hides all of it.

## Options

For how a placeholder looks, three were drawn on the design's own screens and shown to the project lead:

- **A: quiet and in place** (chosen): the real size and position, the label in the lowest grey that still reads, a small hollow square as its mark, no hover highlight.
- **B: a dashed outline.** Dashes already mean "add something here" in the design.
- **C: a small "soon" tag.** No room for it on a 50 px ribbon button.

## Decision

- **An unfinished part is drawn where the design puts the real one, at its real size,** so the layout seen is the layout designed. It has three signs, always together: its label in the lowest readable grey (#8C8C8C) with its icon fainter still (#5A5A5A); a 5 px hollow square mark; and no hover highlight. It never looks broken or switched off for now: nothing is struck through or red.
- **A whole unbuilt screen** is its ribbon and shelf tabs as placeholders, and one centred card in the middle: the feature's name, one plain sentence saying what it will do, and the phase that builds it (or "not planned yet").
- **Hovering says what it is and when it comes; clicking does nothing but say so** in the status bar.
- **One setting, Hide unfinished parts, hides them all.** It is off at first: placeholders show. With it on they are taken out before the layout is worked out, so nothing leaves a gap: an emptied ribbon group goes, and so does a tab or page with nothing real in it.
- **One mechanism.** Every unfinished part is one line in one table, with its name, its sentence and its phase, and is drawn only by that table's functions. Tests check that nothing unfinished is drawn any other way, that everything in the table is drawn somewhere, and that with the setting on none of it is on screen.
- **The command search never lists what can't be done,** with the setting on or off.

## Consequences

- Decision 0070's rule now reads: nothing unfinished is shown *unmarked*. A dead button that looks like a working one is still not allowed, and a control that exists but can't be used right now still says why when hovered.
- The setting is saved from the first milestone of Shared 1a, with the rest of the editor's look; the mechanism, the table and the setting's row in the settings arrive with the second ("the kit"), and the first placeholders with the third. (As built: the second also shows one, the design's Code editor split on the Appearance page, so the whole path is proved in the real editor before the frame arrives.)
- A placeholder's hover text names a phase by its title in ROADMAP.md, and a test fails if that title isn't a heading there, so a renamed phase can't leave a wrong promise behind.
- The manual gains a page on unfinished parts when the first one is on screen.
- When a feature is built, its line leaves the table and the compiler points at every place that still draws its placeholder.
