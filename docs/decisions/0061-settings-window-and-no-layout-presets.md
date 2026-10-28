# 0061. A Settings window, and no layout presets

- **Status:** Accepted (changes 0004's Window → Layouts)
- **Date:** 2026-10-28

## Problem

Decision 0004 gave the Window menu three layout presets (Default, Wide screen, Small screen) and Reset layout, and the accent colour sat in Window → Accent colour. Using the editor, the project lead found the presets redundant: every panel docks freely, so any arrangement is a drag away. And there was nowhere for settings to grow: each new one would have become another menu.

## Decision

Chosen by the project lead, from a mock-up drawn into a screenshot of the editor:

- **Edit → Settings…** opens a Settings window, shaped like Unreal's preferences: a list of pages on the left with a search box above it, the chosen page's settings on the right (each with a name, a line saying what it does, and its control), and **Reset this page** at the bottom. Search looks through every page's settings, names and descriptions. A page appears once it has a setting: the first is **Appearance**, with the accent colour (moved from the Window menu) and **Reset layout**.
- **No layout presets.** Window → Layouts and its presets go. The standard layout (the old Default) is what a first start shows and what Reset layout brings back.
- **Window → Reset layout stays**, for when the panels are too tangled to reach Settings.
- Settings in the window are the editor's, kept on this computer, never undo steps. A setting that belongs to a project goes on a "This project" page when one arrives.

## Consequences

- Each new editor setting is a row on a page (`settings_window.rs`: a `Setting` with its page, name and hint, which the search reads), not a new menu.
- Saved layouts keep working; a damaged one still falls back to the standard layout.
- The Window menu will also list panels that aren't open by default (the Mixer, S4).
