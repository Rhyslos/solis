# 0096: The Graphite theme and rounded panels

- **Status:** Replaced by [decision 0100](0100-one-theme-five-accents.md) (the project lead, 2026-11-11: both themes and the rounded panels switch are removed; one theme and five accents remain)
- **Date:** 2026-11-10

## Problem

The project lead brought a design of their own (from their video editor project) whose colours and layout they like, and wants to try it on Solis, swappable in the settings. Solis's neutral colours were fixed in code, so the look couldn't change while the editor runs.

## Options

- **Replace the standard look.** Simple, but nothing to compare it with, and no way back.
- **A theme with its own layout.** Picking it would change colours and panel shapes together, so neither could be judged alone.
- **A theme and a separate layout switch** (chosen from the mock-ups, 2026-11-10).

## Decision

- The neutral colours (panels, lines, fields, text, the viewport's backdrop) become a palette, chosen in Edit > Settings… > Appearance > **Theme**: **Standard**, the greys of decision 0004, or **Graphite**, the project lead's near-black palette (panels #141414, hairlines #262626, text #fafafa).
- **Rounded panels** is its own switch: panels become cards with an 8 px radius and 6 px gaps of the window's colour, the front tab a soft pill, and the workspaces one group of pills.
- Colours that mean something (the accent, Play, Stop, warnings, the kinds of things, the axes) stay the same in every theme (decision 0060). Both palettes pass the same contrast checks.
- The editor draws on one thread, so the chosen look is kept there and read through colour functions (`theme::panel()` …), which replace the old constants.

## Consequences

- Standard stays the default; nothing changes for anyone who doesn't choose.
- A new neutral colour is added to both palettes, and the contrast tests run in every theme.
- More themes can be added as palettes without touching the code that draws.
