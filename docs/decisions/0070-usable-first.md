# 0070. Usable first: the engine audit, a new build order, blue, and nothing shown that isn't built

- **Status:** Accepted (changes the build order in ROADMAP.md, and 0060's standard colour). Changed by [decision 0101](0101-unfinished-parts-as-placeholders.md): unfinished parts are now shown, marked as placeholders, and one setting hides them; nothing unfinished is shown unmarked.
- **Date:** 2026-11-04

## Problem

After Phase 9's tile painter, the project lead said the engine felt far from usable, and wondered whether to start over (1 and 3 November). In their words:
- the UI "doesn't do it for me";
- right-clicking can't make new files;
- half the buttons and menus don't work;
- there's no UI generator;
- looking at the engine, it's hard to see how anything works;
- not knowing what they wanted made us build things in the wrong order.

The goal is clear now: everything needed to make a Stardew Valley-like farming game from scratch in Solis, written in Lust (in the Ledger editor).

## Options

- **Start over.** It would throw away the parts the farming game needs most, which work: tile maps and the painter, sprites, animation and animators, sound, saves, input actions and play mode. It would take months to get back to where Solis is.
- **Carry on with the old order** (2D physics, the 2D camera, lamps, then Phase 10). The everyday basics would stay missing until much later.
- **Revamp.** Keep what works, fix the editor's surface first, and rebuild the order around what the farming game needs.

## Decision

- **Revamp, not restart.** The audit (the project lead's notes, 3 November) found that what's wrong is mostly the editor's surface and the order things were built in.
- **A new build order**, usable basics first (ROADMAP.md, "Usable first"):
  1. **Editor basics (E1):** blue, nothing shown that isn't built, New in the Library, a start page.
  2. **One project file, the rest as Lust:** what the editor makes is written as Lust data files (with Lust's `lust::read_values`).
  3. **The API milestone:** game code changes tiles and their values, drives animators and plays clips (Lust changes 1 to 4, approved by the Lust worker on 3 November).
  4. **Game-wide objects** (Lust's `object`, ticked and saved by Solis).
  5. **2D physics and top-down movement:** 8 directions for the player, 4 for NPCs.
  6. **The 2D camera.**
  7. **Game UI and the UI editor:** menus, HUD, inventory, dialogue, a pause menu with key rebinding.
  8. **Day and night, and lamps.**
  9. **The playtest export.**

  Then the farming game itself. Phase 10's other 2D parts, and Phases 13 onwards, come after; renaming the engine waits until it feels right.
- **Blue is the standard accent** (the project lead chose Blue over Petrol blue from two real screenshots). Amber, 0060's standard, stays one of the ten. An accent saved while amber was the standard turns blue; one chosen otherwise stays. Play stays its own blue, as 0060 decided.
- **Nothing that isn't built is shown.** No greyed-out or dead buttons promising a later phase: Build, Build & Play, the Build menu, Play with two players, the Layers panel, the new-tab +, the viewport's Show menu and Wireframe are gone until they work. A layout saved with the Layers tab loses it.
- **New, everywhere in the Library** (mock-up e1-library, approved): right-click empty space, a folder or a file for **New**: Lust code file, folder, area, tile map, tileset, animator, sound event. A **+ New** button above the Library has the same list; **Import** keeps importing.
  - The first code file is `main.lst` at the top of the assets (game code starts there); later ones are `component.lst`. Each starts with a component that compiles, and opens in the computer's program for `.lst` files: Ledger, the project lead's editor. There's no code editor inside Solis.
  - A new area is saved where it was asked for, then opened.
- **A start page** (mock-up e1-start, approved): when Solis starts with no project, and from File > Start page. New project, Open project, recent projects, the three examples, four first steps, and a link to the manual. "Don't show this at start" is saved per computer.

## Consequences

- The README's tracker follows the new order. Phase 9 keeps its finished milestones; its last three (2D physics, the 2D camera, lamps) move into the new order.
- Things hidden now come back as they're built, each in the milestone that builds it.
- The editor's look as a whole (beyond blue) gets its own mock-ups, as part of the UI overhaul.
